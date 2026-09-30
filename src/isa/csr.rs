//! CSR-number → extension / privilege mapping.

use crate::ext::Ext;

/// What a CSR access implies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsrInfo {
    /// Extension whose presence the CSR access implies.
    Ext(Ext),
    /// Supervisor-mode CSR (implies S-mode binary).
    Supervisor,
    /// Hypervisor CSR (implies H extension).
    Hypervisor,
    /// Machine-mode CSR (no A-class profile supports M-mode).
    Machine,
    /// Unknown / not in our table.
    Unknown,
}

/// Classify a CSR number (upper 12 bits of a CSR instruction).
pub fn classify_csr(csr: u16) -> CsrInfo {
    match csr {
        // Floating-point status/rounding/flag CSRs (F extension)
        0x001 | 0x002 | 0x003 => CsrInfo::Ext(Ext::F),

        // Vector CSRs (V extension)
        0x008 | 0x009 | 0x00A | 0x00F => CsrInfo::Ext(Ext::V),
        0xC20 | 0xC21 | 0xC22 => CsrInfo::Ext(Ext::V),

        // Zicntr: unprivileged performance counters
        0xC00 | 0xC01 | 0xC02 => CsrInfo::Ext(Ext::Zicntr), // cycle, time, instret
        0xC80 | 0xC81 | 0xC82 => CsrInfo::Ext(Ext::Zicntr), // *h (RV32)

        // Zihpm: hpmcounterN (3..31) and their RV32 high halves
        0xC03..=0xC1F => CsrInfo::Ext(Ext::Zihpm),
        0xC83..=0xC9F => CsrInfo::Ext(Ext::Zihpm),

        // Entropy (zkr) — not mandatory in any profile but not a problem
        0x015 => CsrInfo::Unknown,

        // Supervisor-mode CSRs — specific addresses first, then the full range.
        // The range 0x100..=0x1FF catches everything not listed explicitly.
        0x100..=0x1FF => CsrInfo::Supervisor,
        // VS* CSRs and hypervisor CSRs (0x200–0x6FF range that contains H/VS)
        0x200..=0x28F => CsrInfo::Supervisor, // sstatus mirrors / sip / sie through delegates
        0x600..=0x6FF => CsrInfo::Hypervisor,
        // Hypervisor CSRs (0xE00–0xE80 range, hstatus etc.)
        0xE00..=0xE80 => CsrInfo::Hypervisor,

        // Machine-mode CSRs
        0x300..=0x3FF => CsrInfo::Machine,
        0x700..=0x7FF => CsrInfo::Machine,
        0xB00..=0xB9F => CsrInfo::Machine, // mcycle, minstret, mhpmcounterN
        0xF11..=0xF15 => CsrInfo::Machine, // mvendorid, marchid, mimpid, mhartid, mconfigptr

        _ => CsrInfo::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fp_csr() {
        assert_eq!(classify_csr(0x001), CsrInfo::Ext(Ext::F)); // fflags
        assert_eq!(classify_csr(0x003), CsrInfo::Ext(Ext::F)); // fcsr
    }

    #[test]
    fn vector_csr() {
        assert_eq!(classify_csr(0x008), CsrInfo::Ext(Ext::V)); // vstart
        assert_eq!(classify_csr(0xC20), CsrInfo::Ext(Ext::V)); // vl
    }

    #[test]
    fn counter_csrs() {
        assert_eq!(classify_csr(0xC00), CsrInfo::Ext(Ext::Zicntr)); // cycle
        assert_eq!(classify_csr(0xC03), CsrInfo::Ext(Ext::Zihpm)); // hpmcounter3
    }

    #[test]
    fn supervisor_csr() {
        assert_eq!(classify_csr(0x100), CsrInfo::Supervisor); // sstatus
        assert_eq!(classify_csr(0x180), CsrInfo::Supervisor); // satp
    }

    #[test]
    fn machine_csr() {
        assert_eq!(classify_csr(0x300), CsrInfo::Machine); // mstatus
    }
}
