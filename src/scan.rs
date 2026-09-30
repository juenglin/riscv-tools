//! Scan a parsed ELF for used RISC-V extensions.

use crate::{
    elf::{ElfInfo, MappingSymbol, is_code_at},
    ext::{Ext, ExtSet},
    isa::{
        csr::CsrInfo,
        DecodeResult, PrivLevel, decode_16, decode_32, insn_len,
    },
};

/// Summary of extensions found while scanning a binary.
#[derive(Debug, Default)]
pub struct ScanResult {
    /// Instruction-bearing extensions used (excluding hints).
    pub ext_used: ExtSet,
    /// Hint-only extensions observed.
    pub hint_exts: ExtSet,
    /// Any `sret`, `wfi`, `sfence.vma`, or S-mode CSR access found.
    pub has_supervisor: bool,
    /// Any hypervisor instruction (hfence.*, hlv.*, hsv.*) found.
    pub has_hypervisor: bool,
    /// Any machine-mode instruction (mret, M-mode CSR) found.
    pub has_machine: bool,
    /// Total instructions decoded.
    pub insn_count: u64,
    /// Count of unrecognized instruction encodings.
    pub unknown_count: u64,
    /// Virtual address of the first unrecognized instruction (for diagnostics).
    pub unknown_first_vaddr: Option<u64>,
    /// XLEN from the ELF header.
    pub xlen: u8,
}

/// Scan all code regions in a parsed ELF file and return the extension summary.
pub fn scan(elf: &ElfInfo) -> ScanResult {
    let mut result = ScanResult { xlen: elf.xlen, ..Default::default() };

    for region in &elf.regions {
        scan_region(
            region.vaddr,
            &region.data,
            region.section_idx,
            &elf.mapping,
            &mut result,
        );
    }

    result
}

fn scan_region(
    base_vaddr: u64,
    data: &[u8],
    section_idx: usize,
    mapping: &[MappingSymbol],
    result: &mut ScanResult,
) {
    let mut offset = 0usize;

    while offset < data.len() {
        // Skip data regions per mapping symbols.
        if !is_code_at(mapping, section_idx, offset as u64) {
            offset += 4;
            continue;
        }

        if offset + 2 > data.len() {
            break;
        }

        let hw = u16::from_le_bytes([data[offset], data[offset + 1]]);
        let len = match insn_len(hw) {
            Some(l) => l,
            None => {
                // >= 48-bit, skip 4 bytes.
                offset += 4;
                continue;
            }
        };

        if offset + len > data.len() {
            break;
        }

        let vaddr = base_vaddr + offset as u64;

        if len == 2 {
            let decoded = decode_16(hw);
            apply_decoded(decoded, vaddr, result);
            offset += 2;
        } else {
            let word = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            let decoded = decode_32(word);
            apply_decoded(decoded, vaddr, result);
            offset += 4;
        }
    }
}

fn apply_decoded(decoded: DecodeResult, vaddr: u64, result: &mut ScanResult) {
    result.insn_count += 1;

    match decoded {
        DecodeResult::Base => {}
        DecodeResult::Extension { exts, is_hint } => {
            if is_hint {
                result.hint_exts = result.hint_exts.union(&exts);
            } else {
                result.ext_used = result.ext_used.union(&exts);
            }
        }
        DecodeResult::Csr { csr_info, .. } => {
            // Every CSR access implies Zicsr.
            result.ext_used.insert(Ext::Zicsr);
            match csr_info {
                CsrInfo::Ext(e) => {
                    if e.is_hint() {
                        result.hint_exts.insert(e);
                    } else {
                        result.ext_used.insert(e);
                    }
                }
                CsrInfo::Supervisor => result.has_supervisor = true,
                CsrInfo::Hypervisor => result.has_hypervisor = true,
                CsrInfo::Machine => result.has_machine = true,
                CsrInfo::Unknown => {}
            }
        }
        DecodeResult::Privileged(level) => match level {
            PrivLevel::Supervisor => result.has_supervisor = true,
            PrivLevel::Hypervisor => {
                result.has_hypervisor = true;
                result.ext_used.insert(Ext::Sha);
            }
            PrivLevel::Machine => result.has_machine = true,
        },
        DecodeResult::Unknown => {
            result.unknown_count += 1;
            if result.unknown_first_vaddr.is_none() {
                result.unknown_first_vaddr = Some(vaddr);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elf::parse;

    fn make_elf(text: &[u8]) -> ElfInfo {
        parse(&build_elf64(text)).expect("parse")
    }

    fn build_elf64(text: &[u8]) -> Vec<u8> {
        let mut v: Vec<u8> = Vec::new();
        let shstrtab: Vec<u8> = b"\x00.text\x00.shstrtab\x00".to_vec();
        let text_off: u64 = 0x40;
        let shstrtab_off = ((text_off + text.len() as u64) + 7) & !7;
        let sh_off = ((shstrtab_off + shstrtab.len() as u64) + 7) & !7;

        v.extend_from_slice(&[0x7f, b'E', b'L', b'F']);
        v.push(2); v.push(1); v.push(1); v.push(0);
        v.extend_from_slice(&[0u8; 8]);
        v.extend_from_slice(&2u16.to_le_bytes());   // e_type=ET_EXEC
        v.extend_from_slice(&243u16.to_le_bytes()); // e_machine=EM_RISCV
        v.extend_from_slice(&1u32.to_le_bytes());   // e_version
        v.extend_from_slice(&0u64.to_le_bytes());   // e_entry
        v.extend_from_slice(&0u64.to_le_bytes());   // e_phoff
        v.extend_from_slice(&sh_off.to_le_bytes()); // e_shoff
        v.extend_from_slice(&0u32.to_le_bytes());   // e_flags
        v.extend_from_slice(&64u16.to_le_bytes());  // e_ehsize
        v.extend_from_slice(&56u16.to_le_bytes());  // e_phentsize
        v.extend_from_slice(&0u16.to_le_bytes());   // e_phnum
        v.extend_from_slice(&64u16.to_le_bytes());  // e_shentsize
        v.extend_from_slice(&3u16.to_le_bytes());   // e_shnum
        v.extend_from_slice(&2u16.to_le_bytes());   // e_shstrndx
        assert_eq!(v.len(), 64);

        v.extend_from_slice(text);
        while v.len() < shstrtab_off as usize { v.push(0); }
        v.extend_from_slice(&shstrtab);
        while v.len() < sh_off as usize { v.push(0); }

        // null shdr
        v.extend_from_slice(&[0u8; 64]);
        // .text shdr
        let text_flags: u64 = 6; // SHF_ALLOC | SHF_EXECINSTR
        v.extend_from_slice(&1u32.to_le_bytes());           // sh_name
        v.extend_from_slice(&1u32.to_le_bytes());           // sh_type=PROGBITS
        v.extend_from_slice(&text_flags.to_le_bytes());
        v.extend_from_slice(&text_off.to_le_bytes());       // sh_addr
        v.extend_from_slice(&text_off.to_le_bytes());       // sh_offset
        v.extend_from_slice(&(text.len() as u64).to_le_bytes());
        v.extend_from_slice(&[0u8; 24]);
        // .shstrtab shdr
        v.extend_from_slice(&7u32.to_le_bytes());           // sh_name
        v.extend_from_slice(&3u32.to_le_bytes());           // sh_type=STRTAB
        v.extend_from_slice(&0u64.to_le_bytes());
        v.extend_from_slice(&0u64.to_le_bytes());           // sh_addr
        v.extend_from_slice(&shstrtab_off.to_le_bytes());
        v.extend_from_slice(&(shstrtab.len() as u64).to_le_bytes());
        v.extend_from_slice(&[0u8; 24]);
        v
    }

    #[test]
    fn scan_empty_text() {
        let r = scan(&make_elf(&[]));
        assert_eq!(r.insn_count, 0);
        assert!(r.ext_used.is_empty());
    }

    #[test]
    fn scan_nop_no_ext() {
        let text = 0x00000013u32.to_le_bytes(); // addi x0, x0, 0
        let r = scan(&make_elf(&text));
        assert_eq!(r.insn_count, 1);
        assert!(r.ext_used.is_empty());
    }

    #[test]
    fn scan_mul_detects_m() {
        let text = 0x023100b3u32.to_le_bytes(); // mul x1, x2, x3
        let r = scan(&make_elf(&text));
        assert!(r.ext_used.contains(Ext::M));
    }

    #[test]
    fn scan_v_opcode_detects_v() {
        let word: u32 = 0x0d00_1057; // vsetvli-like OP-V
        let r = scan(&make_elf(&word.to_le_bytes()));
        assert!(r.ext_used.contains(Ext::V));
    }

    #[test]
    fn scan_sret_sets_supervisor() {
        let r = scan(&make_elf(&0x10200073u32.to_le_bytes())); // sret
        assert!(r.has_supervisor);
        assert!(!r.has_hypervisor);
        assert!(!r.has_machine);
    }

    #[test]
    fn scan_mret_sets_machine() {
        let r = scan(&make_elf(&0x30200073u32.to_le_bytes())); // mret
        assert!(r.has_machine);
    }

    #[test]
    fn scan_pause_is_hint_only() {
        let r = scan(&make_elf(&0x0100_000fu32.to_le_bytes())); // pause
        assert!(!r.ext_used.contains(Ext::Zihintpause));
        assert!(r.hint_exts.contains(Ext::Zihintpause));
    }

    #[test]
    fn scan_csrrs_cycle_adds_zicntr() {
        // csrrs x1, cycle, x0
        let word: u32 = (0xC00 << 20) | (0b010 << 12) | (1 << 7) | 0x73;
        let r = scan(&make_elf(&word.to_le_bytes()));
        assert!(r.ext_used.contains(Ext::Zicntr));
        assert!(r.ext_used.contains(Ext::Zicsr));
    }

    #[test]
    fn scan_hfence_sets_hypervisor() {
        // hfence.vvma x0, x0 = 0x22000073
        let r = scan(&make_elf(&0x2200_0073u32.to_le_bytes()));
        assert!(r.has_hypervisor);
        assert!(r.ext_used.contains(Ext::Sha));
    }
}
