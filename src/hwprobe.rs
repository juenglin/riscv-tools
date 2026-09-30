//! RISC-V `riscv_hwprobe` syscall wrapper and bit-to-extension mapping.
//!
//! On non-riscv64 builds the syscall stubs return `ENOSYS` so that
//! `probe-host` compiles everywhere (the fallback path then reads
//! `/proc/cpuinfo`).

use crate::ext::{Ext, ExtSet};

// ── Key / value pairs as defined in <asm/hwprobe.h> ─────────────────────────

/// A single key/value pair for `riscv_hwprobe`.  A returned key of -1 means
/// the kernel doesn't know this key; the caller must skip it.
#[repr(C)]
pub struct HwprobeKv {
    pub key: i64,
    pub value: u64,
}

// Well-known keys (kernel 6.4+).
pub const KEY_BASE_BEHAVIOR: i64 = 3;
pub const KEY_IMA_EXT_0: i64 = 4;
pub const KEY_ZICBOZ_BLOCK_SIZE: i64 = 6;

// Bit positions in KEY_IMA_EXT_0.
const IMA_FD: u64 = 1 << 0;
const IMA_C: u64 = 1 << 1;
const IMA_V: u64 = 1 << 2;
const EXT_ZBA: u64 = 1 << 3;
const EXT_ZBB: u64 = 1 << 4;
const EXT_ZBS: u64 = 1 << 5;
const EXT_ZICBOZ: u64 = 1 << 6;
const EXT_ZBC: u64 = 1 << 7;
const EXT_ZBKB: u64 = 1 << 8;
const EXT_ZBKC: u64 = 1 << 9;
const EXT_ZBKX: u64 = 1 << 10;
const EXT_ZKND: u64 = 1 << 11;
const EXT_ZKNE: u64 = 1 << 12;
const EXT_ZKNH: u64 = 1 << 13;
const EXT_ZKSED: u64 = 1 << 14;
const EXT_ZKSH: u64 = 1 << 15;
const _EXT_ZKT: u64 = 1 << 16;  // behavioral, not mapped to an Ext variant
const EXT_ZVBB: u64 = 1 << 17;
const _EXT_ZVBC: u64 = 1 << 18; // Zbc handles this via cpuinfo
const EXT_ZIHINTNTL: u64 = 1 << 27;
const EXT_ZICOND: u64 = 1 << 28;
const EXT_ZIFENCEI: u64 = 1 << 29;
const EXT_ZCA: u64 = 1 << 30;
const EXT_ZCB: u64 = 1 << 31;
const EXT_ZCD: u64 = 1 << 32;
const _EXT_ZCF: u64 = 1 << 33; // RV32-only, not applicable here
const EXT_ZCMOP: u64 = 1 << 34;
const EXT_ZAWRS: u64 = 1 << 35;

// ── Syscall ──────────────────────────────────────────────────────────────────

/// Invoke the `riscv_hwprobe` syscall (number 258).
///
/// Returns 0 on success, a negative errno on failure.
/// On non-riscv64 hosts always returns -38 (ENOSYS).
#[cfg(target_arch = "riscv64")]
pub fn riscv_hwprobe(
    pairs: &mut [HwprobeKv],
    cpusetsize: usize,
    cpuset: *const u8,
    flags: u64,
) -> i64 {
    use std::arch::asm;
    let ret: i64;
    unsafe {
        asm!(
            "ecall",
            in("a7") 258usize,
            in("a0") pairs.as_mut_ptr(),
            in("a1") pairs.len(),
            in("a2") cpusetsize,
            in("a3") cpuset,
            in("a4") flags as usize,
            lateout("a0") ret,
            options(nostack),
        );
    }
    ret
}

#[cfg(not(target_arch = "riscv64"))]
pub fn riscv_hwprobe(
    _pairs: &mut [HwprobeKv],
    _cpusetsize: usize,
    _cpuset: *const u8,
    _flags: u64,
) -> i64 {
    -38 // ENOSYS
}

// ── Bitmask → ExtSet ─────────────────────────────────────────────────────────

/// Convert the `KEY_IMA_EXT_0` bitmask into an [`ExtSet`].
pub fn ima_ext_0_to_extset(bits: u64) -> ExtSet {
    let mut s = ExtSet::empty();

    // Base IMA / FD / C
    // IMA_FD bit means F+D are present (implied by the profile).
    if bits & IMA_FD != 0 {
        s.insert(Ext::F);
        s.insert(Ext::D);
    }
    if bits & IMA_C != 0 {
        s.insert(Ext::Zca);
        s.insert(Ext::Zcd); // RV64 C includes Zca+Zcd
    }
    if bits & IMA_V != 0 {
        s.insert(Ext::V);
    }
    if bits & EXT_ZBA != 0 { s.insert(Ext::Zba); }
    if bits & EXT_ZBB != 0 { s.insert(Ext::Zbb); }
    if bits & EXT_ZBS != 0 { s.insert(Ext::Zbs); }
    if bits & EXT_ZICBOZ != 0 { s.insert(Ext::Zicboz); }
    if bits & EXT_ZBC != 0 { s.insert(Ext::Zbc); }
    if bits & EXT_ZBKB != 0 { s.insert(Ext::Zbkb); }
    if bits & EXT_ZBKC != 0 { s.insert(Ext::Zbkc); }
    if bits & EXT_ZBKX != 0 { s.insert(Ext::Zbkx); }
    if bits & EXT_ZKND != 0 { s.insert(Ext::Zknd); }
    if bits & EXT_ZKNE != 0 { s.insert(Ext::Zkne); }
    if bits & EXT_ZKNH != 0 { s.insert(Ext::Zknh); }
    if bits & EXT_ZKSED != 0 { s.insert(Ext::Zksed); }
    if bits & EXT_ZKSH != 0 { s.insert(Ext::Zksh); }
    if bits & EXT_ZVBB != 0 { s.insert(Ext::Zvbb); }
    if bits & EXT_ZIHINTNTL != 0 { s.insert(Ext::Zihintntl); }
    if bits & EXT_ZICOND != 0 { s.insert(Ext::Zicond); }
    if bits & EXT_ZIFENCEI != 0 { s.insert(Ext::Zifencei); }
    if bits & EXT_ZCA != 0 { s.insert(Ext::Zca); }
    if bits & EXT_ZCB != 0 { s.insert(Ext::Zcb); }
    if bits & EXT_ZCD != 0 { s.insert(Ext::Zcd); }
    if bits & EXT_ZCMOP != 0 { s.insert(Ext::Zcmop); }
    if bits & EXT_ZAWRS != 0 { s.insert(Ext::Zawrs); }

    s
}

/// Query the kernel via `riscv_hwprobe` and return the host extension set.
///
/// Returns `None` if the syscall is unavailable or returns an error.
pub fn query() -> Option<ExtSet> {
    let mut pairs = [
        HwprobeKv { key: KEY_IMA_EXT_0, value: 0 },
    ];
    let ret = riscv_hwprobe(&mut pairs, 0, std::ptr::null(), 0);
    if ret != 0 {
        return None;
    }
    // If the kernel doesn't recognize the key it sets it to -1.
    if pairs[0].key == -1 {
        return None;
    }

    let mut exts = ima_ext_0_to_extset(pairs[0].value);
    // M and A are implied by the base behavior (KEY_BASE_BEHAVIOR bit 0).
    // We assume any modern kernel has IMA, so always set M and A when hwprobe
    // returns success.
    exts.insert(Ext::M);
    exts.insert(Ext::A);
    exts.insert(Ext::Zicsr);
    exts.insert(Ext::Zicntr);

    Some(exts)
}

// ── Zvfhmin detection ────────────────────────────────────────────────────────
// hwprobe currently (kernel 6.10) doesn't have a bit for Zvfhmin.  We infer
// it from V being present and the cpuinfo isa string containing "zvfhmin".
pub fn has_zvfhmin_from_isa(isa_str: &str) -> bool {
    isa_str.contains("zvfhmin")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ima_fd_sets_f_d() {
        let s = ima_ext_0_to_extset(IMA_FD);
        assert!(s.contains(Ext::F));
        assert!(s.contains(Ext::D));
    }

    #[test]
    fn ima_v_sets_v() {
        let s = ima_ext_0_to_extset(IMA_V);
        assert!(s.contains(Ext::V));
    }

    #[test]
    fn zba_zbb_zbs() {
        let bits = EXT_ZBA | EXT_ZBB | EXT_ZBS;
        let s = ima_ext_0_to_extset(bits);
        assert!(s.contains(Ext::Zba));
        assert!(s.contains(Ext::Zbb));
        assert!(s.contains(Ext::Zbs));
    }

    #[test]
    fn all_rva23_bits() {
        // Simulate a full RVA23 hwprobe response.
        let bits = IMA_FD | IMA_C | IMA_V | EXT_ZBA | EXT_ZBB | EXT_ZBS
            | EXT_ZICBOZ | EXT_ZVBB | EXT_ZICOND | EXT_ZCMOP | EXT_ZCB
            | EXT_ZAWRS | EXT_ZIFENCEI;
        let s = ima_ext_0_to_extset(bits);
        assert!(s.contains(Ext::V));
        assert!(s.contains(Ext::Zvbb));
        assert!(s.contains(Ext::Zicond));
        assert!(s.contains(Ext::Zcmop));
        assert!(s.contains(Ext::Zcb));
        assert!(s.contains(Ext::Zawrs));
    }

    #[test]
    fn enosys_returns_none() {
        // On x86 (not riscv64) the stub always returns ENOSYS → None.
        #[cfg(not(target_arch = "riscv64"))]
        assert!(query().is_none());
    }
}
