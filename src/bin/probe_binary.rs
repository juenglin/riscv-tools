#![deny(clippy::all)]
//! `probe-binary` — smallest RISC-V profile containing all instructions in an ELF.
//!
//! Exit codes: 0 ok · 1 I/O · 2 not ELF · 3 not RISC-V · 4 unsupported · 5 no profile

use std::process;

use anyhow::Context;
use clap::Parser;

use riscv_tools::{
    elf,
    error::{exit, AppError},
    profile::{smallest_containing, Mode, PROFILES},
    scan,
};

#[derive(Parser)]
#[command(
    name = "probe-binary",
    about = "Report the smallest RISC-V profile that covers a binary"
)]
struct Args {
    /// Print per-section extension sets and profile reasoning
    #[arg(short, long)]
    verbose: bool,

    /// ELF file to analyse
    path: String,
}

fn main() {
    let args = Args::parse();
    process::exit(run(&args));
}

fn run(args: &Args) -> i32 {
    let bytes = match std::fs::read(&args.path).with_context(|| format!("reading {}", args.path)) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("probe-binary: {e}");
            return exit::IO;
        }
    };

    let elf_info = match elf::parse(&bytes) {
        Ok(e) => e,
        Err(AppError::NotElf) => {
            eprintln!("probe-binary: {}: not an ELF file", args.path);
            return exit::NOT_ELF;
        }
        Err(AppError::NotRiscV { e_machine }) => {
            eprintln!(
                "probe-binary: {}: not a RISC-V ELF (e_machine={e_machine:#x})",
                args.path
            );
            return exit::NOT_RISCV;
        }
        Err(AppError::BigEndian) => {
            eprintln!(
                "probe-binary: {}: big-endian RISC-V ELF (unsupported)",
                args.path
            );
            return exit::UNSUPPORTED;
        }
        Err(e) => {
            eprintln!("probe-binary: {}: {e}", args.path);
            return exit::UNSUPPORTED;
        }
    };

    if elf_info.regions.is_empty() {
        eprintln!("probe-binary: {}: no executable sections found", args.path);
        return exit::NO_PROFILE;
    }

    let result = scan::scan(&elf_info);

    if args.verbose {
        eprintln!("--- verbose ---");
        eprintln!("XLEN        : {}", result.xlen);
        eprintln!("Instructions: {}", result.insn_count);
        eprintln!("Extensions  : {}", result.ext_used);
        eprintln!("Hints       : {}", result.hint_exts);
        if result.has_supervisor {
            eprintln!("Supervisor  : yes");
        }
        if result.has_hypervisor {
            eprintln!("Hypervisor  : yes");
        }
        if result.has_machine {
            eprintln!("Machine     : yes");
        }
        if result.unknown_count > 0 {
            eprintln!(
                "Unknown     : {} instruction(s), first at {:#x}",
                result.unknown_count,
                result.unknown_first_vaddr.unwrap_or(0)
            );
        }
        if let Some(a) = &elf_info.riscv_attributes {
            eprintln!(".riscv.attributes: {} bytes", a.len());
        }
        eprintln!("--- result ---");
    }

    if result.has_machine {
        eprintln!(
            "probe-binary: {}: contains machine-mode instructions; \
             no A-class profile covers M-mode",
            args.path
        );
        return exit::NO_PROFILE;
    }

    if result.unknown_count > 0 {
        eprintln!(
            "probe-binary: {}: {} unrecognised instruction(s) at {:#x}+; no profile found",
            args.path,
            result.unknown_count,
            result.unknown_first_vaddr.unwrap_or(0)
        );
        return exit::NO_PROFILE;
    }

    let need_super = result.has_supervisor || result.has_hypervisor;
    let candidates = smallest_containing(
        &result.ext_used,
        result.xlen,
        need_super,
        result.has_hypervisor,
    );

    if candidates.is_empty() {
        let uncovered = uncovered_exts(&result, need_super);
        eprintln!(
            "probe-binary: {}: no profile found (not mandatory anywhere: {uncovered})",
            args.path
        );
        return exit::NO_PROFILE;
    }

    for p in &candidates {
        println!("{}", p.name);
    }
    exit::OK
}

fn uncovered_exts(result: &scan::ScanResult, need_super: bool) -> String {
    use riscv_tools::ext::ExtSet;
    let mut uncovered = ExtSet::empty();
    for ext in result.ext_used.iter() {
        let covered = PROFILES.iter().any(|p| {
            p.xlen == result.xlen && (!need_super || p.mode == Mode::S) && p.mandatory.contains(ext)
        });
        if !covered {
            uncovered.insert(ext);
        }
    }
    format!("{uncovered}")
}
