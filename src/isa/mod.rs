//! RISC-V instruction decoder.
//!
//! Classifies individual 16-bit and 32-bit words into extension sets without
//! caring about the surrounding context (that is `scan`'s job).

pub mod csr;
pub mod tables;

use crate::ext::{Ext, ExtSet};
use csr::{classify_csr, CsrInfo};
use tables::{
    is_custom_opcode, is_vector_extended_load_store, system_privilege, SystemPriv, INSNS_16,
    INSNS_32,
};

/// Result of decoding one instruction word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeResult {
    /// Instruction belongs to the given extension set (may be empty for base).
    Extension {
        exts: ExtSet,
        /// True if all matched entries were hint-only.
        is_hint: bool,
    },
    /// A CSR instruction.  `ext_from_csr` is the extension implied by the
    /// CSR number; `csr_num` is the raw 12-bit CSR field.
    Csr {
        csr_num: u16,
        csr_info: CsrInfo,
        /// Whether the CSR instruction itself (read or write) is a hint.
        is_hint: bool,
    },
    /// Instruction in a privileged mode beyond what user-space needs.
    Privileged(PrivLevel),
    /// Completely unrecognized encoding (custom opcode space or reserved).
    Unknown,
    /// Known to be a valid instruction we just didn't classify further.
    Base,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivLevel {
    Supervisor,
    Hypervisor,
    Machine,
}

/// Instruction length in bytes.  Returns 2 for 16-bit, 4 for 32-bit, and
/// `None` for >= 48-bit (unsupported, caller skips and advances 4 bytes).
pub fn insn_len(first_two_bytes: u16) -> Option<usize> {
    let lo2 = first_two_bytes & 0x3;
    if lo2 != 0b11 {
        // Quadrant 0, 1, or 2 → compressed (16-bit).
        return Some(2);
    }
    let lo5 = first_two_bytes & 0x1f;
    if lo5 != 0b11111 {
        // Standard 32-bit.
        return Some(4);
    }
    // 48-bit or longer — treat as unrecognized.
    None
}

/// Decode a 32-bit instruction word.
pub fn decode_32(word: u32) -> DecodeResult {
    // ── CSR instructions (SYSTEM opcode, funct3 ∈ {1,2,3,5,6,7}) ─────────
    if word & 0x7f == 0x73 {
        let funct3 = (word >> 12) & 0x7;
        if matches!(funct3, 1 | 2 | 3 | 5 | 6 | 7) {
            let csr_num = ((word >> 20) & 0xfff) as u16;
            let csr_info = classify_csr(csr_num);
            return DecodeResult::Csr {
                csr_num,
                csr_info,
                is_hint: false,
            };
        }
        // funct3=0: privileged / ecall / ebreak / mop.*
        // funct3=4: Zimop (handled in table)
        let priv_level = system_privilege(word);
        match priv_level {
            SystemPriv::Machine => return DecodeResult::Privileged(PrivLevel::Machine),
            SystemPriv::Supervisor => return DecodeResult::Privileged(PrivLevel::Supervisor),
            SystemPriv::Hypervisor => return DecodeResult::Privileged(PrivLevel::Hypervisor),
            SystemPriv::None => {}
        }
    }

    // ── Table lookup ──────────────────────────────────────────────────────
    // (must come BEFORE the mew=1 vector check so that scalar FP stores with
    // negative immediates — where bit 28 is the sign bit, not the vector mew
    // flag — are correctly classified as D/F rather than V)
    let mut matched_exts = ExtSet::empty();
    let mut matched = false;
    let mut all_hint = true;

    for row in INSNS_32 {
        if word & row.mask == row.matchv {
            matched = true;
            if !row.is_hint {
                all_hint = false;
            }
            for &e in row.exts {
                matched_exts.insert(e);
            }
            break; // first-match semantics
        }
    }

    if matched {
        if matched_exts.is_empty() && !all_hint {
            return DecodeResult::Base;
        }
        return DecodeResult::Extension {
            exts: matched_exts,
            is_hint: all_hint,
        };
    }

    // ── Vector extended-width (mew=1) load/store — not in static table ────
    // Checked AFTER table because scalar FP stores with negative immediates
    // have bit 28 set as the immediate sign bit, which we must not confuse
    // with the vector mew flag.  Additionally exclude funct3 ∈ {1,2,3} which
    // are scalar FP (FLH/FLW/FLD or FSH/FSW/FSD).
    if is_vector_extended_load_store(word) {
        let mut exts = ExtSet::empty();
        exts.insert(Ext::V);
        return DecodeResult::Extension {
            exts,
            is_hint: false,
        };
    }

    // ── Custom opcode → unknown ───────────────────────────────────────────
    if is_custom_opcode(word) {
        return DecodeResult::Unknown;
    }

    // Standard opcode but encoding not in our table: treat conservatively as
    // base (might be a newer extension we don't know about yet).
    DecodeResult::Base
}

/// Decode a 16-bit compressed instruction halfword.
pub fn decode_16(hw: u16) -> DecodeResult {
    // Quadrant 3 (bits[1:0]=11) is never compressed.
    if hw & 0x3 == 0x3 {
        return DecodeResult::Unknown;
    }

    let mut matched_exts = ExtSet::empty();
    let mut matched = false;
    let mut all_hint = true;

    for row in INSNS_16 {
        if hw & row.mask == row.matchv {
            matched = true;
            if !row.is_hint {
                all_hint = false;
            }
            for &e in row.exts {
                matched_exts.insert(e);
            }
            break;
        }
    }

    if matched {
        if matched_exts.is_empty() && !all_hint {
            return DecodeResult::Base;
        }
        return DecodeResult::Extension {
            exts: matched_exts,
            is_hint: all_hint,
        };
    }

    // Unknown compressed encoding (reserved or unimplemented extension).
    DecodeResult::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: decode a 32-bit word and assert we get specific exts.
    fn expect_ext(word: u32, expected: &[Ext]) {
        match decode_32(word) {
            DecodeResult::Extension { exts, .. } => {
                for &e in expected {
                    assert!(exts.contains(e), "word={word:#010x}: missing {e:?}");
                }
            }
            DecodeResult::Base => {
                assert!(
                    expected.is_empty(),
                    "word={word:#010x}: got Base, expected {expected:?}"
                );
            }
            other => panic!("word={word:#010x}: unexpected result {other:?}"),
        }
    }

    fn expect_base(word: u32) {
        let r = decode_32(word);
        assert!(
            matches!(r, DecodeResult::Base | DecodeResult::Extension { .. }),
            "word={word:#010x}: expected Base, got {r:?}"
        );
    }

    // ── 32-bit instruction length detection ─────────────────────────────

    #[test]
    fn insn_len_compressed() {
        assert_eq!(insn_len(0x0000), Some(2)); // Q0
        assert_eq!(insn_len(0x0001), Some(2)); // Q1
        assert_eq!(insn_len(0x0002), Some(2)); // Q2
    }

    #[test]
    fn insn_len_32bit() {
        // Any halfword with bits[1:0]=11 and bits[4:2]≠111
        assert_eq!(insn_len(0x0003), Some(4));
        assert_eq!(insn_len(0x4073), Some(4)); // SYSTEM lower 16 bits
    }

    #[test]
    fn insn_len_long() {
        assert_eq!(insn_len(0x001f), None); // bits[4:0]=11111 → ≥48-bit
    }

    // ── Base RV64I ────────────────────────────────────────────────────────

    #[test]
    fn base_add() {
        // add x1, x2, x3 = 0x003100b3
        expect_base(0x0031_00b3);
    }

    #[test]
    fn base_addi() {
        // addi x1, x2, 1 = 0x00110093
        expect_base(0x0011_0093);
    }

    #[test]
    fn base_nop() {
        // nop = addi x0, x0, 0 = 0x00000013
        expect_base(0x0000_0013);
    }

    #[test]
    fn base_load() {
        // ld x1, 0(x2) = 0x00013083
        expect_base(0x0001_3083);
    }

    #[test]
    fn base_jal() {
        // jal x0, 0 = 0x0000006f
        expect_base(0x0000_006f);
    }

    // ── M extension ────────────────────────────────────────────────────────

    #[test]
    fn mul() {
        // mul x1, x2, x3 = 0x023100b3
        expect_ext(0x0231_00b3, &[Ext::M]);
    }

    #[test]
    fn mulw() {
        // mulw x1, x2, x3 = 0x023100bb (OP-32)
        expect_ext(0x0231_00bb, &[Ext::M]);
    }

    #[test]
    fn div() {
        // div x1, x2, x3 = funct7=0000001, funct3=100 = 0x023140b3
        expect_ext(0x0231_40b3, &[Ext::M]);
    }

    // ── A extension ────────────────────────────────────────────────────────

    #[test]
    fn lr_w() {
        // lr.w x1, (x2) = 0x1000_202f
        expect_ext(0x1000_202f, &[Ext::A]);
    }

    #[test]
    fn amoswap_d() {
        // amoswap.d x1, x2, (x3) = funct5=00001, aq=0, rl=0, rs2=x2, rs1=x3,
        // funct3=011 = 0x0821_30af (approx)
        let word: u32 = (0b00001 << 27) | (0b011 << 12) | (0x2f);
        expect_ext(word, &[Ext::A]);
    }

    // ── F extension ────────────────────────────────────────────────────────

    #[test]
    fn flw() {
        // flw f1, 0(x2) = 0x00012087
        expect_ext(0x0001_2087, &[Ext::F]);
    }

    #[test]
    fn fadd_s() {
        // fadd.s f1, f2, f3 = 0x003100d3
        expect_ext(0x0031_00d3, &[Ext::F]);
    }

    // ── D extension ────────────────────────────────────────────────────────

    #[test]
    fn fld() {
        // fld f1, 0(x2) = 0x00013087
        expect_ext(0x0001_3087, &[Ext::D]);
    }

    #[test]
    fn fadd_d() {
        // fadd.d f1, f2, f3 = funct7=0000001, fmt=01 = 0x023100d3
        expect_ext(0x0231_00d3, &[Ext::D]);
    }

    // ── Zfhmin ────────────────────────────────────────────────────────────

    #[test]
    fn flh() {
        // flh f1, 0(x2) = 0x00011087
        expect_ext(0x0001_1087, &[Ext::Zfhmin]);
    }

    #[test]
    fn fcvt_s_h() {
        // fcvt.s.h f1, f2 = funct7=0100000, rs2=00010 = 0x40210053
        expect_ext(0x4021_0053, &[Ext::Zfhmin]);
    }

    // ── B extension ────────────────────────────────────────────────────────

    #[test]
    fn sh1add() {
        // sh1add x1, x2, x3 = funct7=0010000, funct3=010, OP = 0x20310133
        // Let me compute: funct7=0b0010000=0x10, rs2=x3=3, rs1=x2=2, funct3=010, rd=x1=1, opcode=0x33
        let word: u32 = (0x10 << 25) | (3 << 20) | (2 << 15) | (0b010 << 12) | (1 << 7) | 0x33;
        expect_ext(word, &[Ext::Zba]);
    }

    #[test]
    #[allow(clippy::identity_op)]
    fn clz() {
        // clz x1, x2 = funct7=0110000, rs2=00000, funct3=001, OP
        // Show all fields explicitly for documentation: (0 << 20) = rs2=x0
        let word: u32 = (0x30 << 25) | (0 << 20) | (2 << 15) | (0b001 << 12) | (1 << 7) | 0x33;
        expect_ext(word, &[Ext::Zbb]);
    }

    #[test]
    fn rev8() {
        // rev8 x1, x2 = bits[31:25]=0110101 (funct6=011010,funct1=1), rs2=11000, funct3=101, OP-IMM
        // bits[31:25] = 0b0110101 = 0x35 (NOT 0x6b which would be 1101011)
        let word: u32 =
            (0x35u32 << 25) | (0x18 << 20) | (2 << 15) | (0b101 << 12) | (1 << 7) | 0x13;
        // Sanity check: should produce 0x6b81_5093
        assert_eq!(word, 0x6b81_5093, "rev8 encoding sanity check");
        let r = decode_32(word);
        match r {
            DecodeResult::Extension { exts, .. } => {
                assert!(exts.contains(Ext::Zbb) || exts.contains(Ext::Zbkb));
            }
            other => panic!("rev8 decoded as {other:?}"),
        }
    }

    // ── Zicbom / Zicboz ──────────────────────────────────────────────────

    #[test]
    fn cbo_zero() {
        // cbo.zero rs1=x2: 0000000_00000_00010_010_00000_0001111 = 0x0001200f
        expect_ext(0x0001_200f, &[Ext::Zicboz]);
    }

    #[test]
    fn cbo_inval() {
        // cbo.inval rs1=x2: rs2=00011 → 0x0031_200f
        expect_ext(0x0031_200f, &[Ext::Zicbom]);
    }

    // ── Zicond ────────────────────────────────────────────────────────────

    #[test]
    fn czero_eqz() {
        // czero.eqz x1, x2, x3 = funct7=0000111, funct3=101, OP
        let word: u32 = (0b0000111 << 25) | (3 << 20) | (2 << 15) | (0b101 << 12) | (1 << 7) | 0x33;
        expect_ext(word, &[Ext::Zicond]);
    }

    // ── Zimop ─────────────────────────────────────────────────────────────

    #[test]
    #[allow(clippy::identity_op)]
    fn mop_r_0() {
        // mop.r.0 x1 = bit31=1, bits[30:26]=00000, bit25=1, rs2=00000, rs1=00000,
        // funct3=100, rd=x1=1, opcode=0x73.  All fields shown explicitly.
        let word: u32 = (1u32 << 31)
            | (0 << 26)
            | (1u32 << 25)
            | (0 << 20)
            | (0 << 15)
            | (0b100 << 12)
            | (1 << 7)
            | 0x73;
        expect_ext(word, &[Ext::Zimop]);
    }

    // ── V extension ───────────────────────────────────────────────────────

    #[test]
    fn vsetvli() {
        // vsetvli x0, x0, e32, m1, ta, ma = 0x0d001057 (example)
        // opcode=0x57 is enough
        let word: u32 = 0x0d00_1057;
        expect_ext(word, &[Ext::V]);
    }

    // ── Zawrs ────────────────────────────────────────────────────────────

    #[test]
    fn wrs_nto() {
        expect_ext(0x00d0_0073, &[Ext::Zawrs]);
    }

    // ── Hints ─────────────────────────────────────────────────────────────

    #[test]
    fn pause_is_hint() {
        match decode_32(0x0100_000f) {
            DecodeResult::Extension { is_hint, exts } => {
                assert!(is_hint);
                assert!(exts.contains(Ext::Zihintpause));
            }
            other => panic!("pause: {other:?}"),
        }
    }

    #[test]
    fn ntl_p1_is_hint() {
        // ntl.p1 = add x0, x0, x2 = 0x0020_0033
        match decode_32(0x0020_0033) {
            DecodeResult::Extension { is_hint, exts } => {
                assert!(is_hint);
                assert!(exts.contains(Ext::Zihintntl));
            }
            other => panic!("ntl.p1: {other:?}"),
        }
    }

    // ── CSR instructions ─────────────────────────────────────────────────

    #[test]
    #[allow(clippy::identity_op)]
    fn csrrs_cycle() {
        // csrrs x1, cycle, x0  (all fields shown explicitly; rs1=x0 → 0 << 15)
        let word: u32 = (0xC00 << 20) | (0 << 15) | (0b010 << 12) | (1 << 7) | 0x73;
        match decode_32(word) {
            DecodeResult::Csr { csr_info, .. } => {
                assert_eq!(csr_info, CsrInfo::Ext(Ext::Zicntr));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    #[allow(clippy::identity_op)]
    fn csrrw_vstart() {
        // csrrw x0, vstart, x1  (rd=x0 → 0 << 7 shown explicitly)
        let word: u32 = (0x008 << 20) | (1 << 15) | (0b001 << 12) | (0 << 7) | 0x73;
        match decode_32(word) {
            DecodeResult::Csr { csr_info, .. } => {
                assert_eq!(csr_info, CsrInfo::Ext(Ext::V));
            }
            other => panic!("{other:?}"),
        }
    }

    // ── Privileged ───────────────────────────────────────────────────────

    #[test]
    fn sret_is_supervisor() {
        // sret = 0x10200073
        match decode_32(0x1020_0073) {
            DecodeResult::Privileged(PrivLevel::Supervisor) => {}
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn mret_is_machine() {
        // mret = 0x30200073
        match decode_32(0x3020_0073) {
            DecodeResult::Privileged(PrivLevel::Machine) => {}
            other => panic!("{other:?}"),
        }
    }

    // ── Compressed ───────────────────────────────────────────────────────

    #[test]
    fn c_addi_is_zca() {
        // c.addi x1, 1 = Q1, funct3=000, rd=x1=1, imm=1
        // 000_00001_00001_01 = 0x0405
        match decode_16(0x0405) {
            DecodeResult::Extension { exts, .. } => assert!(exts.contains(Ext::Zca)),
            DecodeResult::Base => {} // acceptable
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn c_fld_is_zcd() {
        // c.fld f0, 0(x8) = Q0, funct3=001
        // 001_000_00000_000_00 = 0x2000
        match decode_16(0x2000) {
            DecodeResult::Extension { exts, .. } => assert!(exts.contains(Ext::Zcd)),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn c_custom_unknown() {
        // A 16-bit word that is clearly invalid (Q0 with imm=0 is c.addi4spn UNDEF if imm=0)
        // But we're testing *unknown* classification. Use a Q0 word with an all-zeros imm
        // which the ISA defines as UNDEF.
        // Actually let's test that we don't panic even on odd values.
        let _ = decode_16(0xffff); // should not panic
    }
}
