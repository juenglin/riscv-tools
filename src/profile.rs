//! Definitions of the ten ratified RISC-V profiles and the partial order
//! between them.
//!
//! The ordering relation is: **Profile A ≤ Profile B** when every instruction
//! that is mandatory in A is also mandatory in B — i.e. A's mandatory
//! [`ExtSet`] is a subset of B's.  This is strictly defined over instruction-
//! bearing extensions only; behavioral extensions (Ziccif, Zic64b, Za64rs,
//! Zkt, …) are excluded.

use crate::ext::{extset, Ext, ExtSet};

/// Privilege mode for a profile (U = user, S = supervisor).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    U,
    S,
}

/// A single ratified RISC-V profile.
#[derive(Debug)]
pub struct Profile {
    /// Short name used in output (e.g. "RVA23U64").
    pub name: &'static str,
    /// 32 or 64.
    pub xlen: u8,
    pub mode: Mode,
    /// Instruction-bearing mandatory extensions (used for containment checks).
    pub mandatory: ExtSet,
}

impl Profile {
    /// Returns `true` when `self ≤ other` in the profile partial order,
    /// meaning every instruction mandatory in `self` is also mandatory in
    /// `other`.  Profiles of different XLEN are never comparable.
    pub fn le(&self, other: &Profile) -> bool {
        self.xlen == other.xlen && self.mandatory.is_subset_of(&other.mandatory)
    }

    /// Returns `true` when `self < other` (strictly).
    pub fn lt(&self, other: &Profile) -> bool {
        self.le(other) && self.mandatory != other.mandatory
    }
}

// ── Mandatory instruction-bearing extension sets (from ISA Manual Vol. III) ─

/// RVI20U32: just base RV32I, no mandatory extensions.
const MAND_RVI20U32: ExtSet = ExtSet::empty();

/// RVI20U64: just base RV64I, no mandatory extensions.
const MAND_RVI20U64: ExtSet = ExtSet::empty();

/// RVA20U64 adds M, A, F, D, C (= Zca+Zcd for RV64), Zicsr, Zicntr.
const MAND_RVA20U64: ExtSet = extset!(
    Ext::M,
    Ext::A,
    Ext::F,
    Ext::D,
    Ext::Zca,
    Ext::Zcd,
    Ext::Zicsr,
    Ext::Zicntr
);

/// RVA20S64 = RVA20U64 + Zifencei.
const MAND_RVA20S64: ExtSet = ExtSet(MAND_RVA20U64.0 | extset!(Ext::Zifencei).0);

/// RVA22U64 = RVA20U64 + B (Zba+Zbb+Zbs), Zihpm, Zicbom, Zicboz, Zfhmin.
/// (Zihintpause, Zicbop, Zic64b, Zkt, Za64rs are behavioral/hint — excluded.)
const MAND_RVA22U64: ExtSet = ExtSet(
    MAND_RVA20U64.0
        | extset!(
            Ext::Zba,
            Ext::Zbb,
            Ext::Zbs,
            Ext::Zihpm,
            Ext::Zicbom,
            Ext::Zicboz,
            Ext::Zfhmin
        )
        .0,
);

/// RVA22S64 = RVA22U64 + Zifencei.
const MAND_RVA22S64: ExtSet = ExtSet(MAND_RVA22U64.0 | extset!(Ext::Zifencei).0);

/// RVA23U64 = RVA22U64 + V, Zvfhmin, Zvbb, Zicond, Zimop, Zcmop, Zcb, Zfa, Zawrs.
/// (Zvkt, Zihintntl, Supm are behavioral/hint — excluded.)
const MAND_RVA23U64: ExtSet = ExtSet(
    MAND_RVA22U64.0
        | extset!(
            Ext::V,
            Ext::Zvfhmin,
            Ext::Zvbb,
            Ext::Zicond,
            Ext::Zimop,
            Ext::Zcmop,
            Ext::Zcb,
            Ext::Zfa,
            Ext::Zawrs
        )
        .0,
);

/// RVA23S64 = RVA23U64 + Zifencei + Sha (H-extension mandatory).
const MAND_RVA23S64: ExtSet = ExtSet(MAND_RVA23U64.0 | extset!(Ext::Zifencei, Ext::Sha).0);

/// RVB23U64 = RVA20U64 + B, Zihpm, Zicbom, Zicboz + Zicond, Zimop, Zcmop, Zcb, Zfa, Zawrs.
/// Notably NOT Zfhmin (optional in RVB23), NOT V/Zvfhmin/Zvbb.
const MAND_RVB23U64: ExtSet = ExtSet(
    MAND_RVA20U64.0
        | extset!(
            Ext::Zba,
            Ext::Zbb,
            Ext::Zbs,
            Ext::Zihpm,
            Ext::Zicbom,
            Ext::Zicboz,
            Ext::Zicond,
            Ext::Zimop,
            Ext::Zcmop,
            Ext::Zcb,
            Ext::Zfa,
            Ext::Zawrs
        )
        .0,
);

/// RVB23S64 = RVB23U64 + Zifencei.  (Sha/H is only OPTIONAL in RVB23S64.)
const MAND_RVB23S64: ExtSet = ExtSet(MAND_RVB23U64.0 | extset!(Ext::Zifencei).0);

// ── The ten ratified profiles ─────────────────────────────────────────────

pub static PROFILES: &[Profile] = &[
    Profile {
        name: "RVI20U32",
        xlen: 32,
        mode: Mode::U,
        mandatory: MAND_RVI20U32,
    },
    Profile {
        name: "RVI20U64",
        xlen: 64,
        mode: Mode::U,
        mandatory: MAND_RVI20U64,
    },
    Profile {
        name: "RVA20U64",
        xlen: 64,
        mode: Mode::U,
        mandatory: MAND_RVA20U64,
    },
    Profile {
        name: "RVA20S64",
        xlen: 64,
        mode: Mode::S,
        mandatory: MAND_RVA20S64,
    },
    Profile {
        name: "RVA22U64",
        xlen: 64,
        mode: Mode::U,
        mandatory: MAND_RVA22U64,
    },
    Profile {
        name: "RVA22S64",
        xlen: 64,
        mode: Mode::S,
        mandatory: MAND_RVA22S64,
    },
    Profile {
        name: "RVA23U64",
        xlen: 64,
        mode: Mode::U,
        mandatory: MAND_RVA23U64,
    },
    Profile {
        name: "RVA23S64",
        xlen: 64,
        mode: Mode::S,
        mandatory: MAND_RVA23S64,
    },
    Profile {
        name: "RVB23U64",
        xlen: 64,
        mode: Mode::U,
        mandatory: MAND_RVB23U64,
    },
    Profile {
        name: "RVB23S64",
        xlen: 64,
        mode: Mode::S,
        mandatory: MAND_RVB23S64,
    },
];

// ── Profile selection ──────────────────────────────────────────────────────

/// Find the **smallest** profiles (in the partial order) whose mandatory
/// extension set is a **superset** of `used`.
///
/// `xlen` must be 32 or 64.  If `need_supervisor` is true only S-mode
/// profiles are considered.  If `need_hypervisor` is true only profiles
/// where [`Ext::Sha`] is mandatory are considered (i.e. RVA23S64).
///
/// Returns an empty `Vec` when no profile covers `used`.
pub fn smallest_containing(
    used: &ExtSet,
    xlen: u8,
    need_supervisor: bool,
    need_hypervisor: bool,
) -> Vec<&'static Profile> {
    // Filter: xlen match, mode requirement, hypervisor requirement, coverage.
    let candidates: Vec<&'static Profile> = PROFILES
        .iter()
        .filter(|p| {
            p.xlen == xlen
                && if need_supervisor || need_hypervisor {
                    p.mode == Mode::S
                } else {
                    p.mode == Mode::U
                }
                && (!need_hypervisor || p.mandatory.contains(Ext::Sha))
                && used.is_subset_of(&p.mandatory)
        })
        .collect();

    minimal_elements(&candidates)
}

/// Find the **largest** profiles whose mandatory set is a **subset** of
/// `host_exts` — i.e. the profiles the host provably supports.
pub fn largest_supported(
    host_exts: &ExtSet,
    xlen: u8,
    has_supervisor: bool,
) -> Vec<&'static Profile> {
    let candidates: Vec<&'static Profile> = PROFILES
        .iter()
        .filter(|p| {
            p.xlen == xlen
                && if has_supervisor {
                    p.mode == Mode::S
                } else {
                    p.mode == Mode::U
                }
                && p.mandatory.is_subset_of(host_exts)
        })
        .collect();

    maximal_elements(&candidates)
}

/// Returns the minimal elements of `profiles` under the profile partial order:
/// those for which no other element is strictly smaller.
fn minimal_elements<'a>(profiles: &[&'a Profile]) -> Vec<&'a Profile> {
    profiles
        .iter()
        .copied()
        .filter(|&p| {
            // p is minimal if no other q in the set satisfies q < p
            !profiles.iter().copied().any(|q| {
                !std::ptr::eq(q, p)
                    && q.mandatory.is_subset_of(&p.mandatory)
                    && q.mandatory != p.mandatory
            })
        })
        .collect()
}

/// Returns the maximal elements: those for which no other element is strictly
/// larger.
fn maximal_elements<'a>(profiles: &[&'a Profile]) -> Vec<&'a Profile> {
    profiles
        .iter()
        .copied()
        .filter(|&p| {
            !profiles.iter().copied().any(|q| {
                !std::ptr::eq(q, p)
                    && p.mandatory.is_subset_of(&q.mandatory)
                    && p.mandatory != q.mandatory
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(name: &str) -> &'static Profile {
        PROFILES
            .iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("unknown profile {name}"))
    }

    // ── Partial-order correctness (agreed examples from planning) ──────────

    #[test]
    fn rvi20u64_le_rva20u64() {
        assert!(profile("RVI20U64").le(profile("RVA20U64")));
        assert!(!profile("RVA20U64").le(profile("RVI20U64")));
    }

    #[test]
    fn rva20u64_le_rva22u64_le_rva23u64() {
        let p20 = profile("RVA20U64");
        let p22 = profile("RVA22U64");
        let p23 = profile("RVA23U64");
        assert!(p20.le(p22));
        assert!(p22.le(p23));
        assert!(p20.le(p23));
        assert!(!p23.le(p22));
    }

    #[test]
    fn rvb23u64_le_rva23u64() {
        assert!(profile("RVB23U64").le(profile("RVA23U64")));
        assert!(!profile("RVA23U64").le(profile("RVB23U64")));
    }

    #[test]
    fn rva22u64_and_rvb23u64_incomparable() {
        let a = profile("RVA22U64");
        let b = profile("RVB23U64");
        assert!(!a.le(b), "RVA22U64 should not be ≤ RVB23U64");
        assert!(!b.le(a), "RVB23U64 should not be ≤ RVA22U64");
    }

    #[test]
    fn rvi20u32_and_rvi20u64_incomparable() {
        // Different XLEN — never comparable.
        let u32p = profile("RVI20U32");
        let u64p = profile("RVI20U64");
        assert!(!u32p.le(u64p));
        assert!(!u64p.le(u32p));
    }

    // ── smallest_containing ───────────────────────────────────────────────

    #[test]
    fn smallest_containing_empty_is_rvi20u64() {
        let res = smallest_containing(&ExtSet::empty(), 64, false, false);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].name, "RVI20U64");
    }

    #[test]
    fn smallest_containing_mafd_is_rva20u64() {
        let used = extset!(
            Ext::M,
            Ext::A,
            Ext::F,
            Ext::D,
            Ext::Zca,
            Ext::Zcd,
            Ext::Zicsr,
            Ext::Zicntr
        );
        let res = smallest_containing(&used, 64, false, false);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].name, "RVA20U64");
    }

    #[test]
    fn smallest_containing_v_is_rva23u64() {
        let mut used = MAND_RVA22U64;
        used.insert(Ext::V);
        let res = smallest_containing(&used, 64, false, false);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].name, "RVA23U64");
    }

    #[test]
    fn smallest_containing_zicond_only_is_rvb23u64() {
        // A binary using only Zicond (and none of the rest): RVB23U64 is
        // smaller than RVA23U64.
        let used = extset!(Ext::Zicond);
        let res = smallest_containing(&used, 64, false, false);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].name, "RVB23U64");
    }

    #[test]
    fn smallest_containing_zfhmin_is_rva22u64() {
        let mut used = MAND_RVA20U64;
        used.insert(Ext::Zfhmin);
        let res = smallest_containing(&used, 64, false, false);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].name, "RVA22U64");
    }

    #[test]
    fn smallest_containing_supervisor_zifencei() {
        // A binary using only Zifencei with supervisor mode needs RVA20S64.
        let used = extset!(
            Ext::M,
            Ext::A,
            Ext::F,
            Ext::D,
            Ext::Zca,
            Ext::Zcd,
            Ext::Zicsr,
            Ext::Zicntr,
            Ext::Zifencei
        );
        let res = smallest_containing(&used, 64, true, false);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].name, "RVA20S64");
    }

    #[test]
    fn smallest_containing_hypervisor_is_rva23s64() {
        let mut used = MAND_RVA20S64;
        used.insert(Ext::Sha);
        let res = smallest_containing(&used, 64, true, true);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].name, "RVA23S64");
    }

    #[test]
    fn smallest_containing_no_profile_for_machine_mode() {
        // No profile mandates machine-mode—caller checks has_machine before
        // calling, so this tests Zbc (optional, not mandatory anywhere).
        let used = extset!(Ext::Zbc);
        let res = smallest_containing(&used, 64, false, false);
        // Zbc is NOT mandatory in any U64 profile → empty result.
        assert!(
            res.is_empty(),
            "Zbc is not mandatory in any profile; got {res:?}"
        );
    }

    // ── largest_supported ─────────────────────────────────────────────────

    #[test]
    fn largest_supported_full_rva23_gives_rva23u64() {
        let res = largest_supported(&MAND_RVA23U64, 64, false);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].name, "RVA23U64");
    }

    #[test]
    fn largest_supported_only_rva20_gives_rva20u64() {
        let res = largest_supported(&MAND_RVA20U64, 64, false);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].name, "RVA20U64");
    }

    #[test]
    fn largest_supported_empty_gives_rvi20u64() {
        let res = largest_supported(&ExtSet::empty(), 64, false);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].name, "RVI20U64");
    }
}
