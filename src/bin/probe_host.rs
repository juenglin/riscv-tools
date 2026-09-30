#![deny(clippy::all)]
//! `probe-host` — largest RISC-V profile the current CPU supports.
//!
//! Reports a verifiable U64 profile line (from hwprobe / cpuinfo) and a
//! best-effort S64 line with unverified supervisor extensions noted.
//! On non-RISC-V hosts prints "not a RISC-V host" and exits 0.

use clap::Parser;

// Only needed on the actual RISC-V target.
#[cfg(target_arch = "riscv64")]
use riscv_tools::{
    cpuinfo,
    ext::{Ext, ExtSet},
    hwprobe,
    profile::largest_supported,
};

#[derive(Parser)]
#[command(
    name = "probe-host",
    about = "Report the largest RISC-V profile this CPU supports"
)]
struct Args {
    /// Print extension sources and raw extension bitmasks
    #[arg(short, long)]
    verbose: bool,
}

fn main() {
    let args = Args::parse();

    #[cfg(not(target_arch = "riscv64"))]
    {
        println!("not a RISC-V host");
        let _ = args;
    }

    #[cfg(target_arch = "riscv64")]
    probe_and_report(args.verbose);
}

#[cfg(target_arch = "riscv64")]
fn probe_and_report(verbose: bool) {
    // ── Collect host extensions ───────────────────────────────────────────────

    let (mut host_exts, source) = if let Some(exts) = hwprobe::query() {
        (exts, "hwprobe")
    } else {
        (cpuinfo::from_proc_cpuinfo(), "cpuinfo")
    };

    // hwprobe doesn't yet have a bit for Zvfhmin; infer it from cpuinfo.
    if host_exts.contains(Ext::V) {
        let cpuinfo_str = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
        if hwprobe::has_zvfhmin_from_isa(&cpuinfo_str) {
            host_exts.insert(Ext::Zvfhmin);
        }
    }

    if verbose {
        eprintln!("source     : {source}");
        eprintln!("extensions : {host_exts}");
    }

    // ── U64 profile (fully verifiable) ───────────────────────────────────────

    let u64_profiles = largest_supported(&host_exts, 64, false);
    let u64_name = profile_names(&u64_profiles).unwrap_or_else(|| "RVI20U64".into());
    println!("U64 profile: {u64_name}");

    // ── S64 profile (best-effort) ─────────────────────────────────────────────

    let S64Result {
        verified,
        unverified,
    } = build_s64_exts(&host_exts);
    let s64_profiles = largest_supported(&verified, 64, true);
    let s64_name = profile_names(&s64_profiles).unwrap_or_else(|| "RVI20U64".into());

    let note = if unverified.is_empty() {
        String::new()
    } else {
        format!(" (unverified: {unverified})")
    };
    println!("S64 profile: {s64_name}{note}");
}

#[cfg(target_arch = "riscv64")]
struct S64Result {
    verified: ExtSet,
    unverified: ExtSet,
}

#[cfg(target_arch = "riscv64")]
fn build_s64_exts(u64_exts: &ExtSet) -> S64Result {
    let cpuinfo_str = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let cpuinfo_exts = cpuinfo::parse_cpuinfo(&cpuinfo_str);

    let mut verified = *u64_exts;
    let mut unverified = ExtSet::empty();

    // Sha (H extension) — check cpuinfo for the 'h' or 'sha' marker.
    if cpuinfo_exts.contains(Ext::Sha) {
        verified.insert(Ext::Sha);
    } else {
        unverified.insert(Ext::Sha);
    }

    // Pick up any additional S-mode extensions from cpuinfo.
    if cpuinfo_exts.contains(Ext::Zvfhmin) {
        verified.insert(Ext::Zvfhmin);
    }

    S64Result {
        verified,
        unverified,
    }
}

#[cfg(target_arch = "riscv64")]
fn profile_names(profiles: &[&riscv_tools::profile::Profile]) -> Option<String> {
    if profiles.is_empty() {
        None
    } else {
        Some(
            profiles
                .iter()
                .map(|p| p.name)
                .collect::<Vec<_>>()
                .join(", "),
        )
    }
}
