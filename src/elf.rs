//! ELF parsing via the [`object`] crate.
//!
//! Extracts executable code regions, mapping symbols (`$x`/`$d`), and the
//! `.riscv.attributes` section from a RISC-V ELF binary.

use object::{
    Architecture, Endianness, Object, ObjectSection, ObjectSegment, ObjectSymbol,
    SectionKind, SegmentFlags, SymbolSection,
};

use crate::error::AppError;

// ── Public types ──────────────────────────────────────────────────────────────

/// A contiguous run of bytes from an executable section (or PT_LOAD segment).
#[derive(Debug, Clone)]
pub struct CodeRegion {
    /// Virtual address of the first byte (section-relative for ET_REL).
    pub vaddr: u64,
    /// Raw bytes of this region.
    pub data: Vec<u8>,
    /// Section index (0 when derived from a PT_LOAD segment).
    pub section_idx: usize,
}

/// A `$x` or `$d` mapping symbol extracted from the symbol table.
#[derive(Debug, Clone)]
pub struct MappingSymbol {
    /// Section index the symbol belongs to.
    pub section_idx: usize,
    /// Section-relative byte offset.
    pub offset: u64,
    /// `true` = code region (`$x`), `false` = data region (`$d`).
    pub is_code: bool,
}

/// Parsed information from a RISC-V ELF file.
pub struct ElfInfo {
    /// 32 or 64, derived from the ELF class.
    pub xlen: u8,
    /// All executable code regions, in section order.
    pub regions: Vec<CodeRegion>,
    /// Mapping symbols sorted by `(section_idx, offset)`.
    pub mapping: Vec<MappingSymbol>,
    /// Raw bytes of `.riscv.attributes`, if present.
    pub riscv_attributes: Option<Vec<u8>>,
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse a byte slice as a RISC-V little-endian ELF file.
pub fn parse(bytes: &[u8]) -> Result<ElfInfo, AppError> {
    // Quick up-front checks so we return typed errors instead of the generic
    // object::Error.
    if bytes.len() < 4 || &bytes[0..4] != b"\x7fELF" {
        return Err(AppError::NotElf);
    }
    // EI_DATA byte: 1 = LE, 2 = BE.
    if bytes.get(5).copied() == Some(2) {
        return Err(AppError::BigEndian);
    }

    let file = object::File::parse(bytes)
        .map_err(|e| AppError::Corrupt(e.to_string()))?;

    if file.endianness() != Endianness::Little {
        return Err(AppError::BigEndian);
    }

    let xlen: u8 = match file.architecture() {
        Architecture::Riscv32 => 32,
        Architecture::Riscv64 => 64,
        _ => {
            // Read e_machine directly from the raw bytes for the error message.
            let e_machine = u16::from_le_bytes([
                bytes.get(18).copied().unwrap_or(0),
                bytes.get(19).copied().unwrap_or(0),
            ]);
            return Err(AppError::NotRiscV { e_machine });
        }
    };

    // ── Code regions ─────────────────────────────────────────────────────────

    let mut regions: Vec<CodeRegion> = Vec::new();

    let sections: Vec<_> = file.sections().collect();
    if sections.is_empty() {
        // Stripped binary — fall back to executable PT_LOAD segments.
        for seg in file.segments() {
            // In object 0.40 p_flags is a ProgramFlags newtype; use .0 for the u32.
            let executes = match seg.flags() {
                SegmentFlags::Elf { p_flags, .. } => p_flags.0 & 0x1 != 0, // PF_X
                _ => false,
            };
            if executes {
                if let Ok(data) = seg.data() {
                    if !data.is_empty() {
                        regions.push(CodeRegion {
                            vaddr: seg.address(),
                            data: data.to_vec(),
                            section_idx: 0,
                        });
                    }
                }
            }
        }
    } else {
        for section in &sections {
            // SectionKind::Text = SHT_PROGBITS + SHF_EXECINSTR, which is
            // exactly what we want.  Avoids touching the SectionFlags newtype.
            if section.kind() != SectionKind::Text {
                continue;
            }
            if let Ok(data) = section.data() {
                if !data.is_empty() {
                    regions.push(CodeRegion {
                        vaddr: section.address(),
                        data: data.to_vec(),
                        section_idx: section.index().0,
                    });
                }
            }
        }
    }

    // ── Mapping symbols ───────────────────────────────────────────────────────

    let mut mapping: Vec<MappingSymbol> = Vec::new();

    for sym in file.symbols() {
        let name = match sym.name() {
            Ok(n) => n,
            Err(_) => continue,
        };
        let is_code = if name == "$x" || name.starts_with("$x.") {
            true
        } else if name == "$d" || name.starts_with("$d.") {
            false
        } else {
            continue;
        };

        let section_idx = match sym.section() {
            SymbolSection::Section(idx) => idx.0,
            _ => continue,
        };

        // Normalise to a section-relative offset.
        // For ET_REL, section.address() == 0 and sym.address() is already
        // section-relative.  For ET_EXEC/DYN, sym.address() is a VA.
        let section_vaddr = file
            .section_by_index(object::SectionIndex(section_idx))
            .map(|s| s.address())
            .unwrap_or(0);
        let offset = sym.address().saturating_sub(section_vaddr);

        mapping.push(MappingSymbol { section_idx, offset, is_code });
    }

    mapping.sort_by_key(|m| (m.section_idx, m.offset));

    // ── .riscv.attributes ─────────────────────────────────────────────────────

    let riscv_attributes = file
        .section_by_name(".riscv.attributes")
        .and_then(|s| s.data().ok())
        .map(|d| d.to_vec());

    Ok(ElfInfo { xlen, regions, mapping, riscv_attributes })
}

// ── Mapping-symbol helper ─────────────────────────────────────────────────────

/// Given the sorted mapping symbols, returns `true` if `offset` bytes into
/// section `section_idx` is code.  Defaults to `true` when no symbols exist.
pub fn is_code_at(mapping: &[MappingSymbol], section_idx: usize, offset: u64) -> bool {
    let last = mapping
        .iter()
        .filter(|m| m.section_idx == section_idx && m.offset <= offset)
        .last();
    last.map(|m| m.is_code).unwrap_or(true)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal valid ELF64 RISC-V file in memory.
    fn minimal_elf64(text: &[u8]) -> Vec<u8> {
        let shstrtab: &[u8] = b"\x00.text\x00.shstrtab\x00";
        let text_off: u64 = 0x40; // immediately after ELF header
        let shstrtab_off = (text_off + text.len() as u64 + 7) & !7;
        let sh_off = (shstrtab_off + shstrtab.len() as u64 + 7) & !7;

        let mut v: Vec<u8> = Vec::new();

        // ELF identifier (16 bytes)
        v.extend_from_slice(b"\x7fELF");
        v.push(2); // ELFCLASS64
        v.push(1); // ELFDATA2LSB
        v.push(1); // EV_CURRENT
        v.push(0); // ELFOSABI_NONE
        v.extend_from_slice(&[0u8; 8]); // padding
        // e_type=ET_EXEC, e_machine=EM_RISCV
        v.extend_from_slice(&2u16.to_le_bytes());
        v.extend_from_slice(&243u16.to_le_bytes());
        v.extend_from_slice(&1u32.to_le_bytes());   // e_version
        v.extend_from_slice(&0u64.to_le_bytes());   // e_entry
        v.extend_from_slice(&0u64.to_le_bytes());   // e_phoff (none)
        v.extend_from_slice(&sh_off.to_le_bytes()); // e_shoff
        v.extend_from_slice(&0u32.to_le_bytes());   // e_flags
        v.extend_from_slice(&64u16.to_le_bytes());  // e_ehsize
        v.extend_from_slice(&56u16.to_le_bytes());  // e_phentsize
        v.extend_from_slice(&0u16.to_le_bytes());   // e_phnum
        v.extend_from_slice(&64u16.to_le_bytes());  // e_shentsize
        v.extend_from_slice(&3u16.to_le_bytes());   // e_shnum
        v.extend_from_slice(&2u16.to_le_bytes());   // e_shstrndx
        assert_eq!(v.len(), 64);

        // Section data
        v.extend_from_slice(text);
        while v.len() < shstrtab_off as usize { v.push(0); }
        v.extend_from_slice(shstrtab);
        while v.len() < sh_off as usize { v.push(0); }

        // Section headers (3 × 64 bytes)
        // [0] null
        v.extend_from_slice(&[0u8; 64]);
        // [1] .text
        let text_flags: u64 = 0x6; // SHF_ALLOC | SHF_EXECINSTR
        v.extend_from_slice(&1u32.to_le_bytes());                     // sh_name
        v.extend_from_slice(&1u32.to_le_bytes());                     // sh_type = SHT_PROGBITS
        v.extend_from_slice(&text_flags.to_le_bytes());               // sh_flags
        v.extend_from_slice(&text_off.to_le_bytes());                 // sh_addr
        v.extend_from_slice(&text_off.to_le_bytes());                 // sh_offset
        v.extend_from_slice(&(text.len() as u64).to_le_bytes());      // sh_size
        v.extend_from_slice(&[0u8; 24]);                              // rest of shdr
        // [2] .shstrtab
        v.extend_from_slice(&7u32.to_le_bytes());                     // sh_name
        v.extend_from_slice(&3u32.to_le_bytes());                     // sh_type = SHT_STRTAB
        v.extend_from_slice(&0u64.to_le_bytes());                     // sh_flags
        v.extend_from_slice(&0u64.to_le_bytes());                     // sh_addr
        v.extend_from_slice(&shstrtab_off.to_le_bytes());             // sh_offset
        v.extend_from_slice(&(shstrtab.len() as u64).to_le_bytes()); // sh_size
        v.extend_from_slice(&[0u8; 24]);

        v
    }

    #[test]
    fn parse_minimal_elf64() {
        let text = [0x13u8, 0x00, 0x00, 0x00, 0x67, 0x80, 0x00, 0x00];
        let elf = minimal_elf64(&text);
        let info = parse(&elf).expect("parse");
        assert_eq!(info.xlen, 64);
        assert_eq!(info.regions.len(), 1);
        assert_eq!(info.regions[0].data, text);
    }

    #[test]
    fn bad_magic_returns_not_elf() {
        assert!(matches!(parse(b"not an elf"), Err(AppError::NotElf)));
    }

    #[test]
    fn big_endian_rejected() {
        let mut elf = minimal_elf64(&[]);
        elf[5] = 2; // ELFDATA2MSB
        assert!(matches!(parse(&elf), Err(AppError::BigEndian)));
    }

    #[test]
    fn wrong_machine_returns_not_riscv() {
        let mut elf = minimal_elf64(&[0x13, 0x00, 0x00, 0x00]);
        // Patch e_machine to 62 (EM_X86_64)
        elf[18] = 62;
        elf[19] = 0;
        assert!(matches!(parse(&elf), Err(AppError::NotRiscV { .. })));
    }

    #[test]
    fn truncated_rejected() {
        let elf = minimal_elf64(&[0x13, 0x00, 0x00, 0x00]);
        assert!(parse(&elf[..10]).is_err());
    }

    #[test]
    fn mapping_symbol_default_code() {
        assert!(is_code_at(&[], 1, 100));
    }

    #[test]
    fn mapping_symbol_data_region() {
        let syms = vec![
            MappingSymbol { section_idx: 1, offset: 0, is_code: true },
            MappingSymbol { section_idx: 1, offset: 16, is_code: false },
            MappingSymbol { section_idx: 1, offset: 32, is_code: true },
        ];
        assert!(is_code_at(&syms, 1, 0));
        assert!(is_code_at(&syms, 1, 15));
        assert!(!is_code_at(&syms, 1, 16));
        assert!(!is_code_at(&syms, 1, 24));
        assert!(is_code_at(&syms, 1, 32));
    }
}
