/// Every ISA-visible extension that matters for the ten ratified profiles.
///
/// Variants are numbered 0–63 so they fit into a single u64 `ExtSet`.
/// "Behavioral" extensions (Ziccif, Zic64b, Za64rs, Zkt, Zvkt, …) add no
/// new instructions and are therefore omitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Ext {
    // ── Unprivileged, instruction-bearing ──────────────────────────────────
    /// Integer multiply / divide  (M)
    M = 0,
    /// Atomics  (A)
    A = 1,
    /// Single-precision FP  (F)
    F = 2,
    /// Double-precision FP  (D)
    D = 3,
    /// Basic compressed instructions for RV64  (Zca portion of C)
    Zca = 4,
    /// Double-precision compressed loads/stores  (Zcd portion of C for RV64)
    Zcd = 5,
    /// CSR instructions  (Zicsr — implied by F)
    Zicsr = 6,
    /// Base counters & timers CSRs  (Zicntr)
    Zicntr = 7,
    /// Hardware performance-counter CSRs  (Zihpm)
    Zihpm = 8,
    /// Instruction-fetch fence  (Zifencei — mandatory in all S64 profiles)
    Zifencei = 9,
    /// Bit-manipulation address  (Zba)
    Zba = 10,
    /// Bit-manipulation base  (Zbb)
    Zbb = 11,
    /// Bit-manipulation single-bit  (Zbs)
    Zbs = 12,
    /// Carryless multiply  (Zbc — expansion option in RVA23, not mandatory)
    Zbc = 13,
    /// Bit-manip for crypto (overlaps Zbb)  (Zbkb)
    Zbkb = 14,
    /// Carryless for crypto (= Zbc)  (Zbkc)
    Zbkc = 15,
    /// Crossbar perm for crypto  (Zbkx)
    Zbkx = 16,
    /// AES decrypt  (Zknd)
    Zknd = 17,
    /// AES encrypt  (Zkne)
    Zkne = 18,
    /// SHA hashes  (Zknh)
    Zknh = 19,
    /// SM4 block cipher  (Zksed)
    Zksed = 20,
    /// SM3 hash  (Zksh)
    Zksh = 21,
    /// Half-precision FP transfer & convert  (Zfhmin)
    Zfhmin = 22,
    /// Full half-precision FP  (Zfh — expansion option)
    Zfh = 23,
    /// Vector  (V)
    V = 24,
    /// Vector minimal half-precision FP  (Zvfhmin)
    Zvfhmin = 25,
    /// Vector basic bit-manipulation  (Zvbb)
    Zvbb = 26,
    /// Cache-block management (cbo.inval/clean/flush)  (Zicbom)
    Zicbom = 27,
    /// Cache-block zero (cbo.zero)  (Zicboz)
    Zicboz = 28,
    /// Integer conditional (czero.eqz / czero.nez)  (Zicond)
    Zicond = 29,
    /// May-be-operations (mop.r.N / mop.rr.N)  (Zimop)
    Zimop = 30,
    /// Compressed may-be-ops (c.mop.N)  (Zcmop)
    Zcmop = 31,
    /// Additional compressed instructions  (Zcb)
    Zcb = 32,
    /// Additional FP instructions  (Zfa)
    Zfa = 33,
    /// Wait-on-reservation-set  (Zawrs)
    Zawrs = 34,
    /// Hypervisor extension instructions  (Sha / H)
    Sha = 35,

    // ── Hint extensions — tracked separately, never raise profile alone ────
    /// Pause hint  (Zihintpause)
    Zihintpause = 48,
    /// Non-temporal locality hints  (Zihintntl)
    Zihintntl = 49,
    /// Cache-block prefetch hints  (Zicbop — harmless on old CPUs)
    Zicbop = 50,
}

impl Ext {
    /// Returns `true` if this extension is hint-only and should not by itself
    /// raise the minimum required profile.
    pub const fn is_hint(self) -> bool {
        matches!(self, Ext::Zihintpause | Ext::Zihintntl | Ext::Zicbop)
    }
}

/// A compact bitset of up to 64 [`Ext`] variants.
///
/// Extension variants have `repr(u8)` discriminants < 64.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct ExtSet(pub u64);

impl ExtSet {
    pub const fn empty() -> Self {
        ExtSet(0)
    }

    /// Construct from a slice of extensions (const-compatible via loop).
    pub const fn from_exts(exts: &[Ext]) -> Self {
        let mut bits = 0u64;
        let mut i = 0;
        while i < exts.len() {
            bits |= 1u64 << (exts[i] as u8);
            i += 1;
        }
        ExtSet(bits)
    }

    pub fn insert(&mut self, ext: Ext) {
        self.0 |= 1u64 << (ext as u8);
    }

    pub fn remove(&mut self, ext: Ext) {
        self.0 &= !(1u64 << (ext as u8));
    }

    pub fn contains(&self, ext: Ext) -> bool {
        self.0 & (1u64 << (ext as u8)) != 0
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Returns `true` iff every bit set in `self` is also set in `other`.
    pub fn is_subset_of(&self, other: &ExtSet) -> bool {
        (self.0 & !other.0) == 0
    }

    pub fn union(&self, other: &ExtSet) -> ExtSet {
        ExtSet(self.0 | other.0)
    }

    pub fn intersect(&self, other: &ExtSet) -> ExtSet {
        ExtSet(self.0 & other.0)
    }

    pub fn difference(&self, other: &ExtSet) -> ExtSet {
        ExtSet(self.0 & !other.0)
    }

    /// Iterate over the extensions present in this set.
    pub fn iter(&self) -> impl Iterator<Item = Ext> + '_ {
        ALL_EXTS.iter().copied().filter(move |e| self.contains(*e))
    }
}

impl std::fmt::Display for ExtSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for ext in self.iter() {
            if !first {
                write!(f, "+")?;
            }
            write!(f, "{ext:?}")?;
            first = false;
        }
        if first {
            write!(f, "(none)")?;
        }
        Ok(())
    }
}

/// All defined extension variants in discriminant order, for iteration.
pub static ALL_EXTS: &[Ext] = &[
    Ext::M,
    Ext::A,
    Ext::F,
    Ext::D,
    Ext::Zca,
    Ext::Zcd,
    Ext::Zicsr,
    Ext::Zicntr,
    Ext::Zihpm,
    Ext::Zifencei,
    Ext::Zba,
    Ext::Zbb,
    Ext::Zbs,
    Ext::Zbc,
    Ext::Zbkb,
    Ext::Zbkc,
    Ext::Zbkx,
    Ext::Zknd,
    Ext::Zkne,
    Ext::Zknh,
    Ext::Zksed,
    Ext::Zksh,
    Ext::Zfhmin,
    Ext::Zfh,
    Ext::V,
    Ext::Zvfhmin,
    Ext::Zvbb,
    Ext::Zicbom,
    Ext::Zicboz,
    Ext::Zicond,
    Ext::Zimop,
    Ext::Zcmop,
    Ext::Zcb,
    Ext::Zfa,
    Ext::Zawrs,
    Ext::Sha,
    Ext::Zihintpause,
    Ext::Zihintntl,
    Ext::Zicbop,
];

// ── Convenience const constructors ────────────────────────────────────────

macro_rules! extset {
    ($($e:expr),* $(,)?) => {
        ExtSet::from_exts(&[$($e),*])
    };
}
pub(crate) use extset;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_ops() {
        let mut s = ExtSet::empty();
        assert!(s.is_empty());
        s.insert(Ext::M);
        assert!(s.contains(Ext::M));
        assert!(!s.contains(Ext::A));
        s.insert(Ext::A);
        assert!(s.contains(Ext::A));
        s.remove(Ext::M);
        assert!(!s.contains(Ext::M));
    }

    #[test]
    fn subset() {
        let a = extset!(Ext::M, Ext::A);
        let b = extset!(Ext::M, Ext::A, Ext::F);
        assert!(a.is_subset_of(&b));
        assert!(!b.is_subset_of(&a));
        assert!(a.is_subset_of(&a));
    }

    #[test]
    fn from_exts_const() {
        const S: ExtSet = ExtSet::from_exts(&[Ext::M, Ext::V]);
        assert!(S.contains(Ext::M));
        assert!(S.contains(Ext::V));
        assert!(!S.contains(Ext::A));
    }

    #[test]
    fn discriminants_fit_u64() {
        for e in ALL_EXTS {
            assert!((*e as u8) < 64, "Ext::{e:?} discriminant >= 64");
        }
    }

    #[test]
    fn hint_flags() {
        assert!(Ext::Zihintpause.is_hint());
        assert!(Ext::Zihintntl.is_hint());
        assert!(Ext::Zicbop.is_hint());
        assert!(!Ext::M.is_hint());
        assert!(!Ext::V.is_hint());
    }
}
