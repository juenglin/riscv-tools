//! Opcode tables for 32-bit and 16-bit RISC-V instructions.
//!
//! Each entry is `(mask, matchv, exts, is_hint)`.  The `exts` slice lists all
//! extensions that individually make the instruction legal — the decoder adds
//! the **entire** slice to the used-extension set (caller then intersects with
//! profile mandatories as needed).  Most instructions have exactly one ext;
//! those shared between two extension families (e.g. rev8 ∈ Zbb ∧ Zbkb) list
//! both.
//!
//! Table order matters: first-match semantics.  More-specific masks come
//! before less-specific ones so that (e.g.) Zfhmin fcvt.s.h is matched before
//! the broad F opcode catch.

use crate::ext::Ext;

/// A 32-bit instruction table row.
pub struct Row32 {
    pub mask: u32,
    pub matchv: u32,
    /// Extensions that provide this instruction.
    pub exts: &'static [Ext],
    /// True for harmless hint instructions (pause, ntl.*, prefetch.*).
    pub is_hint: bool,
}

impl Row32 {
    const fn ext(mask: u32, matchv: u32, exts: &'static [Ext]) -> Self {
        Row32 {
            mask,
            matchv,
            exts,
            is_hint: false,
        }
    }
    const fn hint(mask: u32, matchv: u32, exts: &'static [Ext]) -> Self {
        Row32 {
            mask,
            matchv,
            exts,
            is_hint: true,
        }
    }
}

/// A 16-bit compressed instruction table row.
pub struct Row16 {
    pub mask: u16,
    pub matchv: u16,
    pub exts: &'static [Ext],
    pub is_hint: bool,
}

impl Row16 {
    const fn ext(mask: u16, matchv: u16, exts: &'static [Ext]) -> Self {
        Row16 {
            mask,
            matchv,
            exts,
            is_hint: false,
        }
    }
    const fn hint(mask: u16, matchv: u16, exts: &'static [Ext]) -> Self {
        Row16 {
            mask,
            matchv,
            exts,
            is_hint: true,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 32-bit instruction table
// ─────────────────────────────────────────────────────────────────────────────
//
// Opcode reference (bits [6:0]):
//   0x03  LOAD         0x07  LOAD-FP       0x0F  MISC-MEM
//   0x13  OP-IMM       0x17  AUIPC         0x1B  OP-IMM-32
//   0x23  STORE        0x27  STORE-FP      0x2F  AMO
//   0x33  OP           0x37  LUI           0x3B  OP-32
//   0x43  MADD         0x47  MSUB          0x4B  NMSUB     0x4F  NMADD
//   0x53  OP-FP        0x57  OP-V
//   0x63  BRANCH       0x67  JALR          0x6F  JAL
//   0x73  SYSTEM
//   0x0B  custom-0     0x2B  custom-1      0x5B  custom-2  0x7B  custom-3

pub static INSNS_32: &[Row32] = &[
    // ── V extension: entire OP-V opcode space (0x57) ─────────────────────
    // Catch-all first; more-specific Zvfhmin/Zvbb entries are not needed for
    // profile matching since they push to the same RVA23U64 level as V.
    Row32::ext(0x0000_007f, 0x0000_0057, &[Ext::V]),
    // ── Vector loads (LOAD-FP = 0x07, funct3 ∉ {001,010,011}) ────────────
    Row32::ext(0x0000_707f, 0x0000_0007, &[Ext::V]), // EEW8  (funct3=000)
    Row32::ext(0x0000_707f, 0x0000_5007, &[Ext::V]), // EEW16 (funct3=101)
    Row32::ext(0x0000_707f, 0x0000_6007, &[Ext::V]), // EEW32 (funct3=110)
    Row32::ext(0x0000_707f, 0x0000_7007, &[Ext::V]), // EEW64 (funct3=111)
    Row32::ext(0x0000_707f, 0x0000_4007, &[Ext::V]), // mask/segment (funct3=100)
    // ── Vector stores (STORE-FP = 0x27, funct3 ∉ {001,010,011}) ─────────
    Row32::ext(0x0000_707f, 0x0000_0027, &[Ext::V]),
    Row32::ext(0x0000_707f, 0x0000_5027, &[Ext::V]),
    Row32::ext(0x0000_707f, 0x0000_6027, &[Ext::V]),
    Row32::ext(0x0000_707f, 0x0000_7027, &[Ext::V]),
    Row32::ext(0x0000_707f, 0x0000_4027, &[Ext::V]),
    // ── Zfhmin: specific entries BEFORE the broad F/D catch-alls ─────────
    // fcvt.s.h rd, rs1 : funct7=0100000, rs2=00010 (H→S), fmt=00 → Zfhmin
    Row32::ext(0xfff0_007f, 0x4020_0053, &[Ext::Zfhmin]),
    // fcvt.h.s rd, rs1 : funct7=0100010, rs2=00000 (S→H), fmt=10 → Zfhmin
    Row32::ext(0xfff0_007f, 0x4400_0053, &[Ext::Zfhmin]),
    // fcvt.h.d rd, rs1 : funct7=0100010, rs2=00001 (D→H), fmt=10 → Zfhmin
    Row32::ext(0xfff0_007f, 0x4410_0053, &[Ext::Zfhmin]),
    // fcvt.d.h rd, rs1 : funct7=0100001, rs2=00010 (H→D), fmt=01 → Zfhmin
    Row32::ext(0xfff0_007f, 0x4220_0053, &[Ext::Zfhmin]),
    // fmv.x.h, fmv.h.x (Zfhmin)
    Row32::ext(0xfff0_007f, 0xe400_0053, &[Ext::Zfhmin]), // fmv.x.h
    Row32::ext(0xfff0_007f, 0xf400_0053, &[Ext::Zfhmin]), // fmv.h.x
    // FLH (funct3=001, opcode=0x07) → Zfhmin
    Row32::ext(0x0000_707f, 0x0000_1007, &[Ext::Zfhmin]),
    // FSH (funct3=001, opcode=0x27) → Zfhmin
    Row32::ext(0x0000_707f, 0x0000_1027, &[Ext::Zfhmin]),
    // ── Zfa: additional FP instructions ──────────────────────────────────
    // fli.s (funct7=1111000, rs2=00001, fmt=00) : 0xF0100053
    Row32::ext(0xfff0_007f, 0xf010_0053, &[Ext::Zfa]),
    // fli.d (funct7=1111001, rs2=00001, fmt=01) : 0xF2100053
    Row32::ext(0xfff0_007f, 0xf210_0053, &[Ext::Zfa]),
    // fminm.s: funct7=0010100, fmt=00 → 0x2800_0053 (with variable rs1/rs2)
    Row32::ext(0xfe00_707f, 0x2800_0053, &[Ext::Zfa]),
    // fmaxm.s: funct7=0010100, funct3=001
    Row32::ext(0xfe00_707f, 0x2810_0053, &[Ext::Zfa]),
    // fminm.d: funct7=0010110
    Row32::ext(0xfe00_707f, 0x2a00_0053, &[Ext::Zfa]),
    // fmaxm.d: funct7=0010110, funct3=001
    Row32::ext(0xfe00_707f, 0x2a10_0053, &[Ext::Zfa]),
    // fround.s: funct7=0100000, rs2=00100 → 0x4040_0053
    Row32::ext(0xfff0_007f, 0x4040_0053, &[Ext::Zfa]),
    // froundnx.s: rs2=00101
    Row32::ext(0xfff0_007f, 0x4050_0053, &[Ext::Zfa]),
    // fround.d: funct7=0100001, rs2=00100
    Row32::ext(0xfff0_007f, 0x4240_0053, &[Ext::Zfa]),
    // froundnx.d: rs2=00101
    Row32::ext(0xfff0_007f, 0x4250_0053, &[Ext::Zfa]),
    // fcvtmod.w.d: funct7=1100001, rs2=01000 → 0xC280_1053
    Row32::ext(0xfff0_707f, 0xc280_1053, &[Ext::Zfa]),
    // fleq.s: funct7=1010000, funct3=100 → 0xa004_0053
    Row32::ext(0xfe00_707f, 0xa004_0053, &[Ext::Zfa]),
    // fltq.s: funct3=101
    Row32::ext(0xfe00_707f, 0xa005_0053, &[Ext::Zfa]),
    // fleq.d: funct7=1010001
    Row32::ext(0xfe00_707f, 0xa204_0053, &[Ext::Zfa]),
    // fltq.d: funct3=101
    Row32::ext(0xfe00_707f, 0xa205_0053, &[Ext::Zfa]),
    // ── Zfh full half-precision ───────────────────────────────────────────
    // OP-FP fmt=10 (excluding Zfhmin converts already listed above)
    Row32::ext(0x0600_007f, 0x0400_0053, &[Ext::Zfh]),
    // ── F single-precision ────────────────────────────────────────────────
    // OP-FP fmt=00 (broad catch, after Zfa/Zfhmin specific entries)
    Row32::ext(0x0600_007f, 0x0000_0053, &[Ext::F]),
    // FP fused-multiply opcodes fmt=00
    Row32::ext(0x0600_007f, 0x0000_0043, &[Ext::F]), // MADD.S
    Row32::ext(0x0600_007f, 0x0000_0047, &[Ext::F]), // MSUB.S
    Row32::ext(0x0600_007f, 0x0000_004b, &[Ext::F]), // NMSUB.S
    Row32::ext(0x0600_007f, 0x0000_004f, &[Ext::F]), // NMADD.S
    // FLW (funct3=010, LOAD-FP)
    Row32::ext(0x0000_707f, 0x0000_2007, &[Ext::F]),
    // FSW (funct3=010, STORE-FP)
    Row32::ext(0x0000_707f, 0x0000_2027, &[Ext::F]),
    // ── D double-precision ────────────────────────────────────────────────
    Row32::ext(0x0600_007f, 0x0200_0053, &[Ext::D]), // OP-FP fmt=01
    Row32::ext(0x0600_007f, 0x0200_0043, &[Ext::D]), // MADD.D
    Row32::ext(0x0600_007f, 0x0200_0047, &[Ext::D]), // MSUB.D
    Row32::ext(0x0600_007f, 0x0200_004b, &[Ext::D]), // NMSUB.D
    Row32::ext(0x0600_007f, 0x0200_004f, &[Ext::D]), // NMADD.D
    // FLD (funct3=011, LOAD-FP)
    Row32::ext(0x0000_707f, 0x0000_3007, &[Ext::D]),
    // FSD (funct3=011, STORE-FP)
    Row32::ext(0x0000_707f, 0x0000_3027, &[Ext::D]),
    // ── M extension ───────────────────────────────────────────────────────
    // All M instructions in OP (0x33) have funct7=0000001.
    Row32::ext(0xfe00_007f, 0x0200_0033, &[Ext::M]),
    // All M instructions in OP-32 (0x3B).
    Row32::ext(0xfe00_007f, 0x0200_003b, &[Ext::M]),
    // ── A extension (AMO opcode = 0x2F) ───────────────────────────────────
    // word AMOs (funct3=010)
    Row32::ext(0x0000_707f, 0x0000_202f, &[Ext::A]),
    // double AMOs (funct3=011)
    Row32::ext(0x0000_707f, 0x0000_302f, &[Ext::A]),
    // ── B extension ───────────────────────────────────────────────────────
    // Zba: sh1add, sh2add, sh3add in OP (0x33)
    Row32::ext(0xfe00_707f, 0x2000_2033, &[Ext::Zba]), // sh1add
    Row32::ext(0xfe00_707f, 0x2000_4033, &[Ext::Zba]), // sh2add
    Row32::ext(0xfe00_707f, 0x2000_6033, &[Ext::Zba]), // sh3add
    // Zba RV64-only (OP-32 = 0x3B and OP-IMM-32 = 0x1B)
    Row32::ext(0xfe00_707f, 0x0800_003b, &[Ext::Zba]), // add.uw
    Row32::ext(0xfe00_707f, 0x2000_203b, &[Ext::Zba]), // sh1add.uw
    Row32::ext(0xfe00_707f, 0x2000_403b, &[Ext::Zba]), // sh2add.uw
    Row32::ext(0xfe00_707f, 0x2000_603b, &[Ext::Zba]), // sh3add.uw
    Row32::ext(0xfc00_707f, 0x0800_101b, &[Ext::Zba]), // slli.uw (OP-IMM-32, funct6=000010)
    // Zbb: clz, ctz, cpop, andn, orn, xnor, min, minu, max, maxu,
    //      sext.b, sext.h, zext.h, rol, ror, rori, orc.b, rev8
    // clz: funct7=0110000, rs2=00000, funct3=001 in OP (0x33)
    Row32::ext(0xfff0_707f, 0x6000_1033, &[Ext::Zbb]),
    // clzw: same but OP-32
    Row32::ext(0xfff0_707f, 0x6000_103b, &[Ext::Zbb]),
    // ctz: rs2=00001
    Row32::ext(0xfff0_707f, 0x6010_1033, &[Ext::Zbb]),
    // ctzw
    Row32::ext(0xfff0_707f, 0x6010_103b, &[Ext::Zbb]),
    // cpop: rs2=00010
    Row32::ext(0xfff0_707f, 0x6020_1033, &[Ext::Zbb]),
    // cpopw
    Row32::ext(0xfff0_707f, 0x6020_103b, &[Ext::Zbb]),
    // andn: funct7=0100000, funct3=111
    Row32::ext(0xfe00_707f, 0x4000_7033, &[Ext::Zbb]),
    // orn: funct7=0100000, funct3=110
    Row32::ext(0xfe00_707f, 0x4000_6033, &[Ext::Zbb]),
    // xnor: funct7=0100000, funct3=100
    Row32::ext(0xfe00_707f, 0x4000_4033, &[Ext::Zbb]),
    // min: funct7=0000101, funct3=100
    Row32::ext(0xfe00_707f, 0x0a00_4033, &[Ext::Zbb]),
    // minu: funct3=101
    Row32::ext(0xfe00_707f, 0x0a00_5033, &[Ext::Zbb]),
    // max: funct3=110
    Row32::ext(0xfe00_707f, 0x0a00_6033, &[Ext::Zbb]),
    // maxu: funct3=111
    Row32::ext(0xfe00_707f, 0x0a00_7033, &[Ext::Zbb]),
    // sext.b: funct7=0110000, rs2=00100, funct3=001 (OP)
    Row32::ext(0xfff0_707f, 0x6040_1033, &[Ext::Zbb]),
    // sext.h: rs2=00101
    Row32::ext(0xfff0_707f, 0x6050_1033, &[Ext::Zbb]),
    // zext.h: funct7=0000100, rs2=00000, funct3=100 (OP-32)
    Row32::ext(0xfff0_707f, 0x0800_403b, &[Ext::Zbb]),
    // rol: funct7=0110000, funct3=001 (OP) — note clz also has funct7=0110000/funct3=001/rs2=00000; rol has rs2 nonzero
    // Distinguish: rol is OP funct7=0110000 funct3=001 (mask covers funct7+funct3, rs2 is variable)
    // Actually clz = 0110000_00000_rs1_001_rd_0110011; rol = 0110000_rs2_rs1_001_rd_0110011.
    // Since clz is specific (rs2=00000) and listed first, the catch-all OP+funct7+funct3 won't double-count.
    // But to be safe, we list rol as: funct7=0110000, funct3=001, rs2≠0 — we can't express rs2≠0 in mask/match.
    // Instead, rely on first-match: if clz (rs2=00000) is listed first and has a tighter mask, rol would then match.
    // Actually we already listed clz with mask 0xfff0_707f (covers funct7+rs2+funct3+opcode). So rol can be:
    Row32::ext(0xfe00_707f, 0x6000_1033, &[Ext::Zbb]), // rol (and potentially re-matches clz, but clz listed first)
    // rolw
    Row32::ext(0xfe00_707f, 0x6000_103b, &[Ext::Zbb]),
    // ror: funct7=0110000, funct3=101 (OP)
    Row32::ext(0xfe00_707f, 0x6000_5033, &[Ext::Zbb]),
    // rorw
    Row32::ext(0xfe00_707f, 0x6000_503b, &[Ext::Zbb]),
    // rori: OP-IMM (0x13) funct6=011000, funct3=101 (SRI-like, with high bit of imm)
    Row32::ext(0xfc00_707f, 0x6000_5013, &[Ext::Zbb]),
    // roriw: OP-IMM-32 (0x1B)
    Row32::ext(0xfe00_707f, 0x6000_501b, &[Ext::Zbb]),
    // orc.b: funct7=0010100, rs2=00111, funct3=101 (OP, OP-IMM-like but uses funct7 space)
    Row32::ext(0xfff0_707f, 0x2870_5013, &[Ext::Zbb]),
    // rev8 (RV64): funct7=0110101, rs2=11000, funct3=101 (OP-IMM)
    // rev8 is in both Zbb and Zbkb:
    Row32::ext(0xfff0_707f, 0x6b80_5013, &[Ext::Zbb, Ext::Zbkb]),
    // Zbs: bclr, bext, binv, bset and their immediate variants
    // bclr: funct7=0100100, funct3=001 (OP)
    Row32::ext(0xfe00_707f, 0x4800_1033, &[Ext::Zbs]),
    // bclri: funct6=010010, funct3=001 (OP-IMM, upper 6 bits of imm12=010010)
    Row32::ext(0xfc00_707f, 0x4800_1013, &[Ext::Zbs]),
    // bext: funct7=0100100, funct3=101
    Row32::ext(0xfe00_707f, 0x4800_5033, &[Ext::Zbs]),
    // bexti: funct6=010010, funct3=101
    Row32::ext(0xfc00_707f, 0x4800_5013, &[Ext::Zbs]),
    // binv: funct7=0110100, funct3=001
    Row32::ext(0xfe00_707f, 0x6800_1033, &[Ext::Zbs]),
    // binvi: funct6=011010, funct3=001
    Row32::ext(0xfc00_707f, 0x6800_1013, &[Ext::Zbs]),
    // bset: funct7=0010100, funct3=001
    Row32::ext(0xfe00_707f, 0x2800_1033, &[Ext::Zbs]),
    // bseti: funct6=001010, funct3=001
    Row32::ext(0xfc00_707f, 0x2800_1013, &[Ext::Zbs]),
    // Zbc (carryless multiply — optional in RVA23, not mandatory in any profile)
    Row32::ext(0xfe00_707f, 0x0a00_1033, &[Ext::Zbc, Ext::Zbkc]), // clmul
    Row32::ext(0xfe00_707f, 0x0a00_3033, &[Ext::Zbc, Ext::Zbkc]), // clmulh
    Row32::ext(0xfe00_707f, 0x0a00_2033, &[Ext::Zbc, Ext::Zbkc]), // clmulr
    // ── Zicbom: cache-block management ──────────────────────────────────
    // cbo.inval rs1: 0000000_00011_rs1_010_00000_0001111
    Row32::ext(0xfff0_7fff, 0x0030_200f, &[Ext::Zicbom]),
    // cbo.clean rs1: rs2=00001
    Row32::ext(0xfff0_7fff, 0x0010_200f, &[Ext::Zicbom]),
    // cbo.flush rs1: rs2=00010
    Row32::ext(0xfff0_7fff, 0x0020_200f, &[Ext::Zicbom]),
    // ── Zicboz: cache-block zero ─────────────────────────────────────────
    // cbo.zero rs1: 0000000_00000_rs1_010_00000_0001111
    Row32::ext(0xfff0_7fff, 0x0000_200f, &[Ext::Zicboz]),
    // ── Zicbop: prefetch hints (harmless, tracked as hints) ──────────────
    // prefetch.i: OP-IMM (0x13) funct7=0000000, rs2=00000, rd=0, funct3=110
    Row32::hint(0xfff0_707f, 0x0000_6013, &[Ext::Zicbop]),
    // prefetch.r: imm[11:5]=0000001
    Row32::hint(0xfff0_707f, 0x0010_6013, &[Ext::Zicbop]),
    // prefetch.w: imm[11:5]=0000011
    Row32::hint(0xfff0_707f, 0x0030_6013, &[Ext::Zicbop]),
    // ── Zicond ────────────────────────────────────────────────────────────
    // czero.eqz: funct7=0000111, funct3=101 (OP)
    Row32::ext(0xfe00_707f, 0x0e00_5033, &[Ext::Zicond]),
    // czero.nez: funct3=111
    Row32::ext(0xfe00_707f, 0x0e00_7033, &[Ext::Zicond]),
    // ── Zimop: may-be-operations (mop.r.N, mop.rr.N) ─────────────────────
    // mop.r.N: SYSTEM, funct3=100, bit31=1, bits[30:26]=00000, bit25=1, rs1=00000
    //   mask: funct3+opcode + bits[31:25]+rs1 fields
    Row32::ext(0xfe0f_f07f, 0x8200_4073, &[Ext::Zimop]),
    // mop.rr.N: bit25=0, rs1 is variable
    Row32::ext(0xfe00_707f, 0x8000_4073, &[Ext::Zimop]),
    // ── Zawrs: wait-on-reservation-set ───────────────────────────────────
    // wrs.nto: SYSTEM, imm=0000000_00000, rs1=0, funct3=000, rd=0
    //   Encoding: 0x00d00073
    Row32::ext(0xffff_ffff, 0x00d0_0073, &[Ext::Zawrs]),
    // wrs.sto: 0x01d00073
    Row32::ext(0xffff_ffff, 0x01d0_0073, &[Ext::Zawrs]),
    // ── Zihintpause: pause ────────────────────────────────────────────────
    // pause = fence w, 0: MISC-MEM, imm=0001, pred=0001, succ=0000
    //   Encoding: 0x0100_000F
    Row32::hint(0xffff_ffff, 0x0100_000f, &[Ext::Zihintpause]),
    // ── Zihintntl: non-temporal locality hints ────────────────────────────
    // ntl.p1   = add x0, x0, x2 = 0x0020_0033  (rs1=0, rs2=2, rd=0)
    Row32::hint(0xffff_ffff, 0x0020_0033, &[Ext::Zihintntl]),
    // ntl.pall = add x0, x0, x3
    Row32::hint(0xffff_ffff, 0x0030_0033, &[Ext::Zihintntl]),
    // ntl.s1   = add x0, x0, x4
    Row32::hint(0xffff_ffff, 0x0040_0033, &[Ext::Zihintntl]),
    // ntl.all  = add x0, x0, x5
    Row32::hint(0xffff_ffff, 0x0050_0033, &[Ext::Zihintntl]),
    // ── CSR instructions (Zicsr) ─────────────────────────────────────────
    // CSRRW: funct3=001 in SYSTEM (0x73)
    Row32::ext(0x0000_707f, 0x0000_1073, &[Ext::Zicsr]),
    // CSRRS: funct3=010
    Row32::ext(0x0000_707f, 0x0000_2073, &[Ext::Zicsr]),
    // CSRRC: funct3=011
    Row32::ext(0x0000_707f, 0x0000_3073, &[Ext::Zicsr]),
    // CSRRWI: funct3=101
    Row32::ext(0x0000_707f, 0x0000_5073, &[Ext::Zicsr]),
    // CSRRSI: funct3=110
    Row32::ext(0x0000_707f, 0x0000_6073, &[Ext::Zicsr]),
    // CSRRCI: funct3=111
    Row32::ext(0x0000_707f, 0x0000_7073, &[Ext::Zicsr]),
    // ── Privileged instructions ───────────────────────────────────────────
    // sret: 0001000_00010_00000_000_00000_1110011
    Row32::ext(0xffff_ffff, 0x1020_0073, &[Ext::Zifencei]), // sentinel: marks S-mode; we handle in scan
    // wfi: 0001000_00101_00000_000_00000_1110011
    Row32::ext(0xffff_ffff, 0x1050_0073, &[Ext::Zifencei]),
    // sfence.vma: funct7=0001001, funct3=000, rd=0 in SYSTEM
    Row32::ext(0xfe00_707f, 0x1200_0073, &[Ext::Zifencei]),
    // sinval.vma (Svinval): funct7=0001011
    Row32::ext(0xfe00_707f, 0x1600_0073, &[Ext::Zifencei]),
    // sfence.w.inval, sfence.inval.ir (Svinval): specific encodings
    Row32::ext(0xffff_ffff, 0x1800_0073, &[Ext::Zifencei]),
    Row32::ext(0xffff_ffff, 0x1810_0073, &[Ext::Zifencei]),
    // Hypervisor (Sha / H extension):
    // hfence.vvma: funct7=0010001
    Row32::ext(0xfe00_707f, 0x2200_0073, &[Ext::Sha]),
    // hfence.gvma: funct7=0110001
    Row32::ext(0xfe00_707f, 0x6200_0073, &[Ext::Sha]),
    // hinval.vvma: funct7=0010011
    Row32::ext(0xfe00_707f, 0x2600_0073, &[Ext::Sha]),
    // hinval.gvma: funct7=0110011
    Row32::ext(0xfe00_707f, 0x6600_0073, &[Ext::Sha]),
    // hlv.b:  funct7=0110000, rs2=00000, funct3=100 (SYSTEM)
    Row32::ext(0xfff0_707f, 0x6000_4073, &[Ext::Sha]),
    // hlv.bu: funct7=0110000, rs2=00001
    Row32::ext(0xfff0_707f, 0x6010_4073, &[Ext::Sha]),
    // hlv.h:  funct7=0110010, rs2=00000
    Row32::ext(0xfff0_707f, 0x6400_4073, &[Ext::Sha]),
    // hlv.hu: rs2=00001
    Row32::ext(0xfff0_707f, 0x6410_4073, &[Ext::Sha]),
    // hlvx.hu: rs2=00011
    Row32::ext(0xfff0_707f, 0x6430_4073, &[Ext::Sha]),
    // hlv.w:  funct7=0110100, rs2=00000
    Row32::ext(0xfff0_707f, 0x6800_4073, &[Ext::Sha]),
    // hlv.wu: rs2=00001
    Row32::ext(0xfff0_707f, 0x6810_4073, &[Ext::Sha]),
    // hlvx.wu: rs2=00011
    Row32::ext(0xfff0_707f, 0x6830_4073, &[Ext::Sha]),
    // hlv.d:  funct7=0110110, rs2=00000
    Row32::ext(0xfff0_707f, 0x6c00_4073, &[Ext::Sha]),
    // hsv.b:  funct7=0110001, rd=0
    Row32::ext(0xfe00_707f, 0x6200_4073, &[Ext::Sha]),
    // hsv.h:  funct7=0110011
    Row32::ext(0xfe00_707f, 0x6600_4073, &[Ext::Sha]),
    // hsv.w:  funct7=0110101
    Row32::ext(0xfe00_707f, 0x6a00_4073, &[Ext::Sha]),
    // hsv.d:  funct7=0110111
    Row32::ext(0xfe00_707f, 0x6e00_4073, &[Ext::Sha]),
    // ── Zifencei ─────────────────────────────────────────────────────────
    // fence.i: MISC-MEM (0x0F), funct3=001
    Row32::ext(0x0000_707f, 0x0000_100f, &[Ext::Zifencei]),
    // ── Base RV64I / RV32I (no extension needed) ─────────────────────────
    // These are listed so we don't treat them as "unknown".
    // LOAD (0x03): lb, lh, lw, lbu, lhu; RV64: ld, lwu
    Row32::ext(0x0000_007f, 0x0000_0003, &[]),
    // STORE (0x23): sb, sh, sw; RV64: sd
    Row32::ext(0x0000_007f, 0x0000_0023, &[]),
    // BRANCH (0x63)
    Row32::ext(0x0000_007f, 0x0000_0063, &[]),
    // JAL (0x6F)
    Row32::ext(0x0000_007f, 0x0000_006f, &[]),
    // JALR (0x67)
    Row32::ext(0x0000_007f, 0x0000_0067, &[]),
    // LUI (0x37)
    Row32::ext(0x0000_007f, 0x0000_0037, &[]),
    // AUIPC (0x17)
    Row32::ext(0x0000_007f, 0x0000_0017, &[]),
    // OP-IMM (0x13): addi, slti, sltiu, xori, ori, andi, slli, srli, srai
    Row32::ext(0x0000_007f, 0x0000_0013, &[]),
    // OP (0x33): add, sub, sll, slt, sltu, xor, srl, sra, or, and (base)
    //   Note: M and B instructions in 0x33 are already handled above with
    //   more-specific masks. The broad catch here picks up base ALU insns
    //   that weren't matched above.
    Row32::ext(0x0000_007f, 0x0000_0033, &[]),
    // OP-IMM-32 (0x1B): addiw, slliw, srliw, sraiw
    Row32::ext(0x0000_007f, 0x0000_001b, &[]),
    // OP-32 (0x3B): addw, subw, sllw, srlw, sraw
    Row32::ext(0x0000_007f, 0x0000_003b, &[]),
    // MISC-MEM (0x0F): fence (general)
    Row32::ext(0x0000_007f, 0x0000_000f, &[]),
    // SYSTEM (0x73): ecall, ebreak (base)
    Row32::ext(0x0000_007f, 0x0000_0073, &[]),
];

// ─────────────────────────────────────────────────────────────────────────────
// 16-bit (compressed) instruction table
// ─────────────────────────────────────────────────────────────────────────────
//
// Quadrants: bits[1:0] = 00 (Q0), 01 (Q1), 10 (Q2).
// bits[15:13] = funct3 within the quadrant.

pub static INSNS_16: &[Row16] = &[
    // ── Zcd (double-precision compressed) ────────────────────────────────
    // Q0 funct3=001: c.fld (RV64) — bits[15:13]=001, bits[1:0]=00
    Row16::ext(0xe003, 0x2000, &[Ext::Zcd]),
    // Q0 funct3=101: c.fsd
    Row16::ext(0xe003, 0xa000, &[Ext::Zcd]),
    // Q2 funct3=001: c.fldsp
    Row16::ext(0xe003, 0x2002, &[Ext::Zcd]),
    // Q2 funct3=101: c.fsdsp
    Row16::ext(0xe003, 0xa002, &[Ext::Zcd]),
    // ── Zcb: additional compressed (more-specific before broad Zca) ───────
    // Q0 funct3=100 (0x8000 base, bits[12:10] discriminate):
    // c.lbu: bits[15:10]=100_000 → mask=0xfc03, match=0x8000
    Row16::ext(0xfc03, 0x8000, &[Ext::Zcb]),
    // c.lhu: bits[15:10]=100_001
    Row16::ext(0xfc03, 0x8400, &[Ext::Zcb]),
    // c.lh:  bits[15:10]=100_010
    Row16::ext(0xfc03, 0x8800, &[Ext::Zcb]),
    // c.sb:  bits[15:10]=100_100
    Row16::ext(0xfc03, 0x9000, &[Ext::Zcb]),
    // c.sh:  bits[15:10]=100_101
    Row16::ext(0xfc03, 0x9400, &[Ext::Zcb]),
    // Q1 Zcb arithmetic (funct6=100_011, various funct2):
    // c.zext.b: bits[15:10]=100_011, bits[6:2]=11000 → 0x9c61 (approx)
    //   Exact: 100_0_11_rs1'_11_000_01
    Row16::ext(0xfc7f, 0x9c61, &[Ext::Zcb]),
    // c.sext.b: bits[6:2]=11001
    Row16::ext(0xfc7f, 0x9c69, &[Ext::Zcb]),
    // c.zext.h: bits[6:2]=11010
    Row16::ext(0xfc7f, 0x9c71, &[Ext::Zcb]),
    // c.sext.h: bits[6:2]=11011
    Row16::ext(0xfc7f, 0x9c79, &[Ext::Zcb]),
    // c.zext.w: bits[6:2]=11100 (RV64 only)
    Row16::ext(0xfc7f, 0x9c7d, &[Ext::Zcb]), // tentative — verify against spec
    // c.not: bits[6:2]=11101
    Row16::ext(0xfc7f, 0x9c65, &[Ext::Zcb]),
    // c.mul: funct6=100_011, funct2=10 → bits[6:5]=10, bits[4:2]=000
    Row16::ext(0xfc63, 0x9c41, &[Ext::Zcb]),
    // ── Zcmop: compressed may-be-ops ─────────────────────────────────────
    // c.mop.N for odd N=1,3,5,7,9,11,13,15 in Q1.
    // Encoding: 100_N[3]_N[2:1]_0_1_000_01 (tentative; verify against spec)
    // For now, use a placeholder catch covering the funct3=100, Q1 range that
    // doesn't conflict with standard Q1 instructions already listed.
    // TODO: verify exact bit patterns from Zcmop spec.

    // ── Zihintntl: compressed NTL hints ──────────────────────────────────
    // c.ntl.p1  = c.add x0, x2 (Q2, funct3=100, bit12=1, rd=x0, rs2=x2)
    //   Encoding: 1001_0000_0000_1010 = 0x900a
    //   NOT 0x8082! That is c.mv x8, x2 / c.jr x1 (ret).
    Row16::hint(0xffff, 0x900a, &[Ext::Zihintntl]),
    // c.ntl.pall = c.add x0, x3 = 0x900e
    Row16::hint(0xffff, 0x900e, &[Ext::Zihintntl]),
    // c.ntl.s1  = c.add x0, x4 = 0x9012
    Row16::hint(0xffff, 0x9012, &[Ext::Zihintntl]),
    // c.ntl.all = c.add x0, x5 = 0x9016
    Row16::hint(0xffff, 0x9016, &[Ext::Zihintntl]),
    // ── Zca: common compressed (base for RV64) ────────────────────────────
    // Q0: c.addi4spn (funct3=000)
    Row16::ext(0xe003, 0x0000, &[Ext::Zca]),
    // Q0: c.lw (funct3=010)
    Row16::ext(0xe003, 0x4000, &[Ext::Zca]),
    // Q0: c.ld (funct3=011, RV64)
    Row16::ext(0xe003, 0x6000, &[Ext::Zca]),
    // Q0: c.sw (funct3=110)
    Row16::ext(0xe003, 0xc000, &[Ext::Zca]),
    // Q0: c.sd (funct3=111, RV64)
    Row16::ext(0xe003, 0xe000, &[Ext::Zca]),
    // Q1: all non-Zcb, non-Zcmop Q1 instructions
    Row16::ext(0x0003, 0x0001, &[Ext::Zca]), // all Q1
    // Q2: c.slli, c.lwsp, c.ldsp, c.jr, c.mv, c.ebreak, c.jalr, c.add, c.swsp, c.sdsp
    //   (Zcd Q2 already listed above with funct3=001/101)
    Row16::ext(0x0003, 0x0002, &[Ext::Zca]), // broad Q2 catch
];

// ── Opcode classification ─────────────────────────────────────────────────

/// Returns `true` if bits[6:0] indicate a custom (non-standard) opcode.
pub fn is_custom_opcode(word: u32) -> bool {
    matches!(word & 0x7f, 0x0b | 0x2b | 0x5b | 0x7b)
}

/// Returns `true` if the 32-bit word is a known vector instruction other
/// than those already in INSNS_32 (e.g. uses intermediate mew=1 EEW).
pub fn is_vector_extended_load_store(word: u32) -> bool {
    let opcode = word & 0x7f;
    // mew=1 (bit 28) with LOAD-FP or STORE-FP
    (opcode == 0x07 || opcode == 0x27) && (word & 0x1000_0000) != 0
}

/// Privilege level implied by a SYSTEM (0x73) instruction that doesn't match
/// any extension table entry.  Returns `None` for ecall/ebreak/base.
pub fn system_privilege(word: u32) -> SystemPriv {
    if word & 0x7f != 0x73 {
        return SystemPriv::None;
    }
    // mret (machine return): 0x30200073
    if word == 0x3020_0073 {
        return SystemPriv::Machine;
    }
    // The funct7 field (bits[31:25]) is the discriminator for priv insns.
    let funct7 = (word >> 25) & 0x7f;
    match funct7 {
        0b000_1000 => SystemPriv::Supervisor,              // sret / wfi
        0b000_1001 => SystemPriv::Supervisor,              // sfence.vma
        0b000_1011 => SystemPriv::Supervisor,              // sinval.vma
        0b001_0001 | 0b001_0011 => SystemPriv::Hypervisor, // hfence.vvma / hinval.vvma
        0b011_0001 | 0b011_0011 => SystemPriv::Hypervisor, // hfence.gvma / hinval.gvma
        _ => SystemPriv::None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemPriv {
    None,
    Supervisor,
    Hypervisor,
    Machine,
}
