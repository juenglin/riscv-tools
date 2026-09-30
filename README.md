# riscv-tools

Two small CLI utilities for identifying RISC-V ISA profiles.

## Binaries

### `probe-binary`

Reads a RISC-V ELF file and reports the **smallest** ratified RISC-V profile
whose mandatory instruction set covers every instruction found in the binary.

```
Usage: probe-binary [-v] <elf-file>
```

| Exit code | Meaning |
|---|---|
| 0 | Profile found; name(s) printed on stdout |
| 1 | I/O error |
| 2 | Not an ELF file |
| 3 | Not a RISC-V ELF |
| 4 | Corrupt/unsupported ELF |
| 5 | No profile found (unknown encodings, M-mode instructions, or extensions not mandatory in any ratified profile) |

The `-v` flag writes a verbose report to stderr: XLEN, instruction count,
extension sets used, hints, and `.riscv.attributes` size.

#### Profile ordering

"Profile A is smaller than profile B" means B's mandatory set contains every
instruction A mandates.  Examples:

- `RVI20U64 < RVA20U64 < RVA22U64 < RVA23U64`
- `RVI20U64 < RVA20U64 < RVB23U64 < RVA23U64`
- `RVA22U64` and `RVB23U64` are **incomparable** (RVA22 mandates Zfhmin;
  RVB23 mandates Zicond/Zimop/Zcmop/Zcb/Zfa/Zawrs but not Zfhmin).

The ten ratified profiles (ISA Manual Vol. III):

| Profile | XLEN | Mode | Notable additions |
|---|---|---|---|
| RVI20U32 | 32 | U | base RV32I only |
| RVI20U64 | 64 | U | base RV64I only |
| RVA20U64 | 64 | U | M A F D C Zicsr Zicntr |
| RVA20S64 | 64 | S | + Zifencei |
| RVA22U64 | 64 | U | + B Zfhmin Zicbom Zicboz Zihpm |
| RVA22S64 | 64 | S | + Zifencei |
| RVA23U64 | 64 | U | + V Zvfhmin Zvbb Zicond Zimop Zcmop Zcb Zfa Zawrs |
| RVA23S64 | 64 | S | + Zifencei Sha (H-extension mandatory) |
| RVB23U64 | 64 | U | like RVA22 base + Zicond/Zimop/… but NOT Zfhmin/V |
| RVB23S64 | 64 | S | + Zifencei (Sha optional, not mandatory) |

**Hint extensions** (Zihintpause, Zihintntl, Zicbop) do not raise the profile
by themselves — they are harmless on older CPUs.

### `probe-host`

Reports the **largest** profile the current CPU supports, based on
`riscv_hwprobe` (kernel 6.4+) with `/proc/cpuinfo` as fallback.

```
Usage: probe-host [-v]
```

Output example:
```
U64 profile: RVA23U64
S64 profile: RVA23S64 (unverified: Sha)
```

The S64 line includes extensions that can't be fully verified from user space
(notably `Sha` / H extension).  On non-RISC-V hosts it prints
`not a RISC-V host` and exits 0.

## Build

### Prerequisites

| Tool | Purpose |
|---|---|
| Rust 1.81+ stable | `rustup.rs` |
| `riscv64gc-unknown-linux-musl` target | `rustup target add riscv64gc-unknown-linux-musl` |
| `qemu-riscv64-static` | test runner (Ubuntu: `qemu-user-static`) |
| `clang-18` | building ELF fixtures (optional) |

### Host (x86-64) build and test

```bash
cargo build
cargo test
```

### Cross-compile for RISC-V (musl static)

```bash
cargo build --release --target riscv64gc-unknown-linux-musl
```

The binaries are fully statically linked (no `NEEDED` entries) and run on any
RISC-V Linux system with musl or glibc.

```bash
# Verify
file target/riscv64gc-unknown-linux-musl/release/probe-binary
readelf -d target/riscv64gc-unknown-linux-musl/release/probe-binary | grep NEEDED
```

#### Linker wrapper

`scripts/riscv64-musl-lld` is a thin wrapper invoked by Cargo for the musl
target.  It substitutes `-lgcc_s` (a shared library) with `-lunwind` (the
static equivalent bundled in the Rust sysroot) and injects the musl
self-contained library search path.  The `-lgcc_s` reference originates from
the Rust 1.98 link spec for this target; the correct stable-Rust override
(`-C link-self-contained=+unwind`) is behind `-Z unstable-options`.

### Tests under QEMU user-mode

```bash
cargo test --target riscv64gc-unknown-linux-musl
```

Requires `qemu-riscv64-static` on `PATH` (configured as the test runner in
`.cargo/config.toml`).

### ELF fixtures

```bash
bash scripts/build-fixtures.sh   # requires clang-18
```

Generates `fixtures/*.o` used by the integration tests in `tests/probe_binary.rs`.

### Guest test (myfive RISC-V VM)

```bash
ssh myfive bash /home/rjuengling/juenglin/riscv-tools/scripts/guest-test.sh
```

Runs `probe-host` and `probe-binary` natively on the RISC-V guest, using
the shared `$HOME` (virtio-9p) to access the cross-compiled release binaries.

## Notes

- **musl runtime vector probe**: the bundled musl libc reads the `vlenb` CSR
  at startup to detect vector support.  `probe-binary` correctly reports this
  as a V CSR access; musl handles the resulting SIGILL gracefully on non-V
  CPUs so the binary runs everywhere.
- **RVA23.1 / RVB23.1** (not yet ratified as of September 2026) and **RVM**
  (in charter phase) are not included.  The profile table is data-driven so
  they can be added later.
