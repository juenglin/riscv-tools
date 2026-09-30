//! Integration tests for `probe-binary`.
//!
//! These tests run the native x86 `probe-binary` binary on RISC-V ELF
//! fixtures that are either generated at test time (synthetic) or built by
//! `scripts/build-fixtures.sh` (checked into `fixtures/`).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

/// Path to the `probe-binary` binary as compiled for the host (x86).
fn bin() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_probe-binary"))
}

/// Generate a unique temp-file path so parallel tests don't collide.
fn temp_elf() -> std::path::PathBuf {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("pbt_{}_{n}.elf", std::process::id()))
}

/// Run probe-binary on the given bytes and return (stdout, stderr, exit_code).
fn run_on_bytes(bytes: &[u8]) -> (String, String, i32) {
    let path = temp_elf();
    std::fs::write(&path, bytes).expect("write temp file");
    let out = Command::new(bin())
        .arg(&path)
        .output()
        .expect("probe-binary failed to run");
    let _ = std::fs::remove_file(&path);
    (
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
        String::from_utf8_lossy(&out.stderr).trim().to_string(),
        out.status.code().unwrap_or(-1),
    )
}

/// Run probe-binary with -v on given bytes.
fn run_verbose(bytes: &[u8]) -> (String, String, i32) {
    let path = temp_elf();
    std::fs::write(&path, bytes).expect("write temp file");
    let out = Command::new(bin())
        .args(["-v", path.to_str().unwrap()])
        .output()
        .expect("probe-binary -v failed");
    let _ = std::fs::remove_file(&path);
    (
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
        String::from_utf8_lossy(&out.stderr).trim().to_string(),
        out.status.code().unwrap_or(-1),
    )
}

/// Build a minimal ELF64 RISC-V with the given .text bytes.
fn minimal_elf64(text: &[u8]) -> Vec<u8> {
    let mut v: Vec<u8> = Vec::new();
    let shstrtab: Vec<u8> = b"\x00.text\x00.shstrtab\x00".to_vec();
    let text_off: u64 = 0x40;
    let shstrtab_off = ((text_off + text.len() as u64) + 7) & !7;
    let sh_off = ((shstrtab_off + shstrtab.len() as u64) + 7) & !7;

    v.extend_from_slice(&[0x7f, b'E', b'L', b'F']);
    v.push(2); v.push(1); v.push(1); v.push(0);
    v.extend_from_slice(&[0u8; 8]);
    v.extend_from_slice(&2u16.to_le_bytes());   // ET_EXEC
    v.extend_from_slice(&243u16.to_le_bytes()); // EM_RISCV
    v.extend_from_slice(&1u32.to_le_bytes());
    v.extend_from_slice(&0u64.to_le_bytes());   // e_entry
    v.extend_from_slice(&0u64.to_le_bytes());   // e_phoff
    v.extend_from_slice(&sh_off.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());   // e_flags
    v.extend_from_slice(&64u16.to_le_bytes());  // e_ehsize
    v.extend_from_slice(&56u16.to_le_bytes());
    v.extend_from_slice(&0u16.to_le_bytes());   // e_phnum
    v.extend_from_slice(&64u16.to_le_bytes());  // e_shentsize
    v.extend_from_slice(&3u16.to_le_bytes());   // e_shnum
    v.extend_from_slice(&2u16.to_le_bytes());   // e_shstrndx
    assert_eq!(v.len(), 64);

    v.extend_from_slice(text);
    while v.len() < shstrtab_off as usize { v.push(0); }
    v.extend_from_slice(&shstrtab);
    while v.len() < sh_off as usize { v.push(0); }

    v.extend_from_slice(&[0u8; 64]); // null shdr
    let flags: u64 = 6;
    v.extend_from_slice(&1u32.to_le_bytes());
    v.extend_from_slice(&1u32.to_le_bytes());
    v.extend_from_slice(&flags.to_le_bytes());
    v.extend_from_slice(&text_off.to_le_bytes());
    v.extend_from_slice(&text_off.to_le_bytes());
    v.extend_from_slice(&(text.len() as u64).to_le_bytes());
    v.extend_from_slice(&[0u8; 24]);
    v.extend_from_slice(&7u32.to_le_bytes());
    v.extend_from_slice(&3u32.to_le_bytes());
    v.extend_from_slice(&0u64.to_le_bytes());
    v.extend_from_slice(&0u64.to_le_bytes());
    v.extend_from_slice(&shstrtab_off.to_le_bytes());
    v.extend_from_slice(&(shstrtab.len() as u64).to_le_bytes());
    v.extend_from_slice(&[0u8; 24]);
    v
}

// ── Profile-level tests ────────────────────────────────────────────────────

#[test]
fn empty_text_gives_rvi20u64() {
    // An empty .text section (just nop) should give RVI20U64 (no mandatory exts needed).
    let nop = 0x00000013u32.to_le_bytes(); // addi x0, x0, 0
    let elf = minimal_elf64(&nop);
    let (stdout, _, code) = run_on_bytes(&elf);
    assert_eq!(code, 0, "expected exit 0");
    assert_eq!(stdout, "RVI20U64");
}

#[test]
fn mul_gives_rva20u64() {
    // mul x1, x2, x3 = 0x023100b3 — needs M which is mandatory from RVA20U64
    let mul = 0x023100b3u32.to_le_bytes();
    let elf = minimal_elf64(&mul);
    let (stdout, _, code) = run_on_bytes(&elf);
    assert_eq!(code, 0);
    assert_eq!(stdout, "RVA20U64");
}

#[test]
fn mafd_compressed_gives_rva20u64() {
    // A typical "rv64gc" binary: uses M, A, F, D, C.
    // mul + flw + lr.w + c.addi
    let mut text: Vec<u8> = Vec::new();
    text.extend_from_slice(&0x023100b3u32.to_le_bytes()); // mul
    text.extend_from_slice(&0x00012087u32.to_le_bytes()); // flw
    text.extend_from_slice(&0x1000202fu32.to_le_bytes()); // lr.w
    text.extend_from_slice(&0x0405u16.to_le_bytes());     // c.addi (compressed)
    let elf = minimal_elf64(&text);
    let (stdout, _, code) = run_on_bytes(&elf);
    assert_eq!(code, 0);
    assert_eq!(stdout, "RVA20U64");
}

#[test]
fn zba_instruction_gives_rva22u64() {
    // sh1add x1, x2, x3 — requires Zba (mandatory from RVA22U64)
    let word: u32 = (0x10 << 25) | (3 << 20) | (2 << 15) | (0b010 << 12) | (1 << 7) | 0x33;
    let mut text: Vec<u8> = Vec::new();
    // Also include base M/A/F/D/C so we get RVA22 and not RVB23
    text.extend_from_slice(&0x023100b3u32.to_le_bytes()); // mul (M)
    text.extend_from_slice(&0x00012087u32.to_le_bytes()); // flw (F)
    text.extend_from_slice(&0x00013087u32.to_le_bytes()); // fld (D)
    text.extend_from_slice(&0x1000202fu32.to_le_bytes()); // lr.w (A)
    text.extend_from_slice(&0x0405u16.to_le_bytes());     // c.addi (Zca)
    text.extend_from_slice(&0x2000u16.to_le_bytes());     // c.fld (Zcd)
    text.extend_from_slice(&word.to_le_bytes());          // sh1add (Zba)
    text.extend_from_slice(&0x0021_200fu32.to_le_bytes()); // cbo.clean (Zicbom)
    text.extend_from_slice(&0x0000_200fu32.to_le_bytes()); // cbo.zero (Zicboz)
    text.extend_from_slice(&0x4021_0053u32.to_le_bytes()); // fcvt.s.h (Zfhmin)
    let elf = minimal_elf64(&text);
    let (stdout, _, code) = run_on_bytes(&elf);
    assert_eq!(code, 0, "stderr: {}", run_on_bytes(&minimal_elf64(&text)).1);
    assert_eq!(stdout, "RVA22U64");
}

#[test]
fn vector_gives_rva23u64() {
    // Any OP-V (0x57) instruction → V → RVA23U64
    let vsetvli: u32 = 0x0d00_1057;
    let elf = minimal_elf64(&vsetvli.to_le_bytes());
    let (stdout, _, code) = run_on_bytes(&elf);
    assert_eq!(code, 0);
    assert_eq!(stdout, "RVA23U64");
}

#[test]
fn machine_mode_no_profile() {
    // mret = 0x30200073 → no A-class profile
    let elf = minimal_elf64(&0x3020_0073u32.to_le_bytes());
    let (_, _, code) = run_on_bytes(&elf);
    assert_eq!(code, 5); // EXIT_NO_PROFILE
}

// ── Error case tests ───────────────────────────────────────────────────────

#[test]
fn not_elf_file() {
    let (_, _, code) = run_on_bytes(b"not an elf file");
    assert_eq!(code, 2); // EXIT_NOT_ELF
}

#[test]
fn wrong_machine() {
    // Build a minimal ELF with e_machine = 62 (x86-64)
    let mut elf = minimal_elf64(&[0x13, 0x00, 0x00, 0x00]);
    elf[18] = 62;
    elf[19] = 0;
    let (_, _, code) = run_on_bytes(&elf);
    assert_eq!(code, 3); // EXIT_NOT_RISCV
}

#[test]
fn nonexistent_file() {
    let out = Command::new(bin())
        .arg("/nonexistent/path/that/does/not/exist.elf")
        .output()
        .expect("run");
    assert_eq!(out.status.code(), Some(1)); // EXIT_IO
}

// ── Verbose mode ──────────────────────────────────────────────────────────

#[test]
fn verbose_contains_extensions() {
    let mul = 0x023100b3u32.to_le_bytes();
    let elf = minimal_elf64(&mul);
    let (stdout, stderr, code) = run_verbose(&elf);
    assert_eq!(code, 0);
    assert!(stderr.contains("Extensions"), "verbose should show extensions; got: {stderr}");
    assert_eq!(stdout, "RVA20U64");
}
