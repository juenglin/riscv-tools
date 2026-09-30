//! `/proc/cpuinfo` ISA string parser.
//!
//! Parses the `isa` field from every hart and returns the intersection of
//! extensions across all harts (the guaranteed capability).

use crate::ext::{Ext, ExtSet};

/// Parse the ISA string from `/proc/cpuinfo` and return an `ExtSet`.
///
/// The intersection of all harts' extensions is returned.  If no `isa:` line
/// is found, returns an empty `ExtSet`.
pub fn parse_cpuinfo(cpuinfo: &str) -> ExtSet {
    let mut result: Option<ExtSet> = None;

    for line in cpuinfo.lines() {
        let line = line.trim();
        // Look for lines like "isa : rv64imafdc_zicsr_..."
        let isa_str = if let Some(s) = line.strip_prefix("isa\t:") {
            s.trim()
        } else if let Some(s) = line.strip_prefix("isa :") {
            s.trim()
        } else if let Some(s) = line.strip_prefix("isa:") {
            s.trim()
        } else {
            continue;
        };

        let exts = parse_isa_string(isa_str);
        result = Some(match result {
            None => exts,
            Some(prev) => ExtSet(prev.0 & exts.0), // intersection
        });
    }

    result.unwrap_or_default()
}

/// Parse a single ISA string like `rv64imafdc_zba_zbb_zbs_v_zvfhmin`.
///
/// The leading `rv64`/`rv32` part is stripped, then single-letter and
/// underscore-separated multi-letter extensions are decoded.
pub fn parse_isa_string(s: &str) -> ExtSet {
    let mut exts = ExtSet::empty();

    // Strip leading "rv64" or "rv32" (case-insensitive).
    let rest = s.to_lowercase();
    let rest = rest
        .strip_prefix("rv64")
        .or_else(|| rest.strip_prefix("rv32"))
        .unwrap_or(&rest);

    // Split on underscore: first "word" may be single-letter combined
    // (e.g. "imafdc"), rest are multi-letter extensions.
    let mut parts = rest.split('_');

    if let Some(first) = parts.next() {
        // Strip optional version suffix from first word (e.g. "gc2p0" → "gc")
        let alpha: String = first.chars().take_while(|c| c.is_alphabetic()).collect();
        for ch in alpha.chars() {
            match ch {
                'i' | 'e' => {} // base ISA, always present
                'm' => {
                    exts.insert(Ext::M);
                }
                'a' => {
                    exts.insert(Ext::A);
                }
                'f' => {
                    exts.insert(Ext::F);
                    exts.insert(Ext::Zicsr);
                }
                'd' => {
                    exts.insert(Ext::D);
                    exts.insert(Ext::Zicsr);
                }
                'c' => {
                    exts.insert(Ext::Zca);
                    exts.insert(Ext::Zcd);
                }
                'b' => {
                    exts.insert(Ext::Zba);
                    exts.insert(Ext::Zbb);
                    exts.insert(Ext::Zbs);
                }
                'v' => {
                    exts.insert(Ext::V);
                }
                'g' => {
                    // G = IMAFD + Zicsr + Zifencei
                    exts.insert(Ext::M);
                    exts.insert(Ext::A);
                    exts.insert(Ext::F);
                    exts.insert(Ext::D);
                    exts.insert(Ext::Zicsr);
                    exts.insert(Ext::Zifencei);
                }
                'h' => {
                    exts.insert(Ext::Sha);
                }
                _ => {} // unknown single letter
            }
        }
    }

    for part in parts {
        // Strip version suffix (e.g. "zba2p0" → "zba")
        let name: String = part
            .chars()
            .take_while(|c| c.is_alphabetic() || *c == '_')
            .collect();
        let name = name.trim_end_matches('_');
        apply_multiext(&name.to_lowercase(), &mut exts);
    }

    // Ensure implied extensions.
    if exts.contains(Ext::F) || exts.contains(Ext::D) {
        exts.insert(Ext::Zicsr);
    }

    exts
}

/// Map a multi-letter extension name to [`ExtSet`] bits.
fn apply_multiext(name: &str, exts: &mut ExtSet) {
    match name {
        "zicsr" => exts.insert(Ext::Zicsr),
        "zicntr" => {
            exts.insert(Ext::Zicntr);
            exts.insert(Ext::Zicsr);
        }
        "zihpm" => {
            exts.insert(Ext::Zihpm);
            exts.insert(Ext::Zicsr);
        }
        "zifencei" => exts.insert(Ext::Zifencei),
        "zihintpause" => exts.insert(Ext::Zihintpause),
        "zihintntl" => exts.insert(Ext::Zihintntl),
        "zba" => exts.insert(Ext::Zba),
        "zbb" => exts.insert(Ext::Zbb),
        "zbc" => exts.insert(Ext::Zbc),
        "zbs" => exts.insert(Ext::Zbs),
        "zbkb" => exts.insert(Ext::Zbkb),
        "zbkc" => exts.insert(Ext::Zbkc),
        "zbkx" => exts.insert(Ext::Zbkx),
        "zknd" => exts.insert(Ext::Zknd),
        "zkne" => exts.insert(Ext::Zkne),
        "zknh" => exts.insert(Ext::Zknh),
        "zksed" => exts.insert(Ext::Zksed),
        "zksh" => exts.insert(Ext::Zksh),
        "zkn" => {
            exts.insert(Ext::Zbkb);
            exts.insert(Ext::Zbkc);
            exts.insert(Ext::Zbkx);
            exts.insert(Ext::Zknd);
            exts.insert(Ext::Zkne);
            exts.insert(Ext::Zknh);
        }
        "zks" => {
            exts.insert(Ext::Zbkb);
            exts.insert(Ext::Zbkc);
            exts.insert(Ext::Zbkx);
            exts.insert(Ext::Zksed);
            exts.insert(Ext::Zksh);
        }
        "zfhmin" => exts.insert(Ext::Zfhmin),
        "zfh" => {
            exts.insert(Ext::Zfh);
            exts.insert(Ext::Zfhmin);
        }
        "zic64b" => {} // behavioral
        "zicbom" => exts.insert(Ext::Zicbom),
        "zicbop" => exts.insert(Ext::Zicbop),
        "zicboz" => exts.insert(Ext::Zicboz),
        "zicond" => exts.insert(Ext::Zicond),
        "zimop" => exts.insert(Ext::Zimop),
        "zcmop" => exts.insert(Ext::Zcmop),
        "zca" => exts.insert(Ext::Zca),
        "zcb" => exts.insert(Ext::Zcb),
        "zcd" => exts.insert(Ext::Zcd),
        "zcf" => {} // RV32 only
        "zfa" => exts.insert(Ext::Zfa),
        "zawrs" => exts.insert(Ext::Zawrs),
        "v" => exts.insert(Ext::V),
        "zvfhmin" => exts.insert(Ext::Zvfhmin),
        "zvfh" => exts.insert(Ext::Zvfhmin), // Zvfh implies Zvfhmin
        "zvbb" => exts.insert(Ext::Zvbb),
        "supm" | "ssnpm" | "sscofpmf" | "sscounterenw" | "ssccptr" | "sstvecd" | "sstvala"
        | "svbare" | "sv39" | "sv48" | "sv57" | "svade" | "svnapot" | "svpbmt" | "svinval"
        | "svadu" | "svvptc" | "sstc" | "ssu64xl" | "ssstrict" => {
            // Supervisor/hypervisor platform extensions — noted but not mapped
            // to instruction-bearing ExtSet bits here (scan.rs handles them
            // from instruction patterns).
        }
        "sha" => {
            exts.insert(Ext::Sha);
        }
        _ => {} // unknown extension — ignore
    }
}

/// Read `/proc/cpuinfo` and return the host extension set.
pub fn from_proc_cpuinfo() -> ExtSet {
    match std::fs::read_to_string("/proc/cpuinfo") {
        Ok(s) => parse_cpuinfo(&s),
        Err(_) => ExtSet::empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rv64gc() {
        let s = parse_isa_string("rv64gc");
        assert!(s.contains(Ext::M), "G implies M");
        assert!(s.contains(Ext::A));
        assert!(s.contains(Ext::F));
        assert!(s.contains(Ext::D));
        assert!(s.contains(Ext::Zca));
        assert!(s.contains(Ext::Zcd));
        assert!(s.contains(Ext::Zifencei));
    }

    #[test]
    fn parse_rv64imafdc() {
        let s = parse_isa_string("rv64imafdc");
        assert!(s.contains(Ext::M));
        assert!(s.contains(Ext::A));
        assert!(s.contains(Ext::F));
        assert!(s.contains(Ext::D));
        assert!(s.contains(Ext::Zca));
        assert!(s.contains(Ext::Zcd));
        // No G ⇒ no Zifencei from the G macro, but 'c' gives Zca/Zcd.
    }

    #[test]
    fn parse_with_underscore_exts() {
        let s = parse_isa_string(
            "rv64gc_zba_zbb_zbs_v_zvfhmin_zvbb_zicond_zimop_zcmop_zcb_zfa_zawrs_zfhmin",
        );
        assert!(s.contains(Ext::Zba));
        assert!(s.contains(Ext::Zbb));
        assert!(s.contains(Ext::Zbs));
        assert!(s.contains(Ext::V));
        assert!(s.contains(Ext::Zvfhmin));
        assert!(s.contains(Ext::Zvbb));
        assert!(s.contains(Ext::Zicond));
        assert!(s.contains(Ext::Zimop));
        assert!(s.contains(Ext::Zcmop));
        assert!(s.contains(Ext::Zcb));
        assert!(s.contains(Ext::Zfa));
        assert!(s.contains(Ext::Zawrs));
        assert!(s.contains(Ext::Zfhmin));
    }

    #[test]
    fn version_suffix_stripped() {
        // "zba2p0" should be treated the same as "zba"
        let s = parse_isa_string("rv64imafdc_zba2p0_zbb1p0");
        assert!(s.contains(Ext::Zba));
        assert!(s.contains(Ext::Zbb));
    }

    #[test]
    fn case_insensitive() {
        let s = parse_isa_string("RV64IMAFDC_ZBA_ZBB");
        assert!(s.contains(Ext::Zba));
        assert!(s.contains(Ext::Zbb));
    }

    #[test]
    fn multihart_intersection() {
        let cpuinfo = "\
processor\t: 0
isa\t: rv64gc_zba_zbb_zbs
mmu\t: sv39

processor\t: 1
isa\t: rv64gc_zba_zbb
mmu\t: sv39
";
        let s = parse_cpuinfo(cpuinfo);
        assert!(s.contains(Ext::Zba));
        assert!(s.contains(Ext::Zbb));
        // Zbs is only on hart 0 → not in intersection
        assert!(!s.contains(Ext::Zbs));
    }

    #[test]
    fn zkn_expands() {
        let s = parse_isa_string("rv64gc_zkn");
        assert!(s.contains(Ext::Zbkb));
        assert!(s.contains(Ext::Zknd));
        assert!(s.contains(Ext::Zkne));
        assert!(s.contains(Ext::Zknh));
    }
}
