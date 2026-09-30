#!/bin/bash
# Build RISC-V ELF fixtures for probe-binary integration tests.
# Requires: clang-18 (or clang) on PATH with riscv64 backend.
# Output: fixtures/*.o  (ET_REL relocatable objects used by cargo test)
#
# Run from the repo root:  bash scripts/build-fixtures.sh

set -euo pipefail
cd "$(dirname "$0")/.."

CLANG=${CLANG:-clang-18}
OUT=fixtures
mkdir -p "$OUT"

CC_RV64="$CLANG --target=riscv64-unknown-elf -x c -c -nostdlib"
CC_RV32="$CLANG --target=riscv32-unknown-elf -x c -c -nostdlib"
AS_RV64="$CLANG --target=riscv64-unknown-elf -x assembler -c -nostdlib"

echo "Building fixtures with $CLANG ..."

# ── RVA20U64-level: plain rv64gc (uses M, A, F, D, C) ───────────────────────
$CC_RV64 -march=rv64gc -o "$OUT/rv64gc.o" - <<'EOF'
int mul(int a, int b)         { return a * b; }
double fadd(double a, double b){ return a + b; }
EOF
echo "  rv64gc.o (RVA20U64)"

# ── RVA22U64-level: B + Zfhmin + Zicbom + Zicboz ─────────────────────────────
$CC_RV64 -march=rv64gc_zba_zbb_zbs_zfhmin_zicbom_zicboz -o "$OUT/rva22.o" - <<'EOF'
void use_rva22(void) {
    __asm__ volatile("sh1add   zero, zero, zero");
    __asm__ volatile("cbo.zero (%0)" :: "r"(0) : "memory");
}
EOF
echo "  rva22.o (RVA22U64)"

# ── RVA23U64-level: + V, Zicond, Zawrs ───────────────────────────────────────
$CC_RV64 \
  -march=rv64gcv_zba_zbb_zbs_zfhmin_zicbom_zicboz_zvfhmin_zvbb_zicond_zawrs \
  -o "$OUT/rva23.o" - <<'EOF'
void use_rva23(int n) {
    __asm__ volatile("vsetvli t0, %0, e32, m1, ta, ma" :: "r"(n));
    long x = 0, y = 1;
    __asm__ volatile("czero.eqz %0, %0, %1" : "+r"(x) : "r"(y));
    __asm__ volatile("wrs.nto");
}
EOF
echo "  rva23.o (RVA23U64)"

# ── S-mode: sret + sfence.vma → smallest profile becomes RVA20S64 ────────────
$CC_RV64 -march=rv64gc -o "$OUT/smode.o" - <<'EOF'
int dummy_m(int a, int b) { return a * b; }
void trap_return(void) { __asm__ volatile("sret"); }
void flush_tlb(void)   { __asm__ volatile("sfence.vma zero, zero"); }
EOF
echo "  smode.o (RVA20S64)"

# ── Hypervisor: hfence.vvma → RVA23S64 ───────────────────────────────────────
$CC_RV64 -march=rv64gc_h -o "$OUT/hypervisor.o" - <<'EOF'
int dummy_m(int a, int b) { return a * b; }
void hfence(void) { __asm__ volatile("hfence.vvma zero, zero"); }
EOF
echo "  hypervisor.o (RVA23S64)"

# ── RV32: bare int — should give RVI20U32 ─────────────────────────────────────
$CC_RV32 -march=rv32i -o "$OUT/rv32i.o" - <<'EOF'
int add(int a, int b) { return a + b; }
EOF
echo "  rv32i.o (RVI20U32)"

# ── Mapping symbols: data island inside a .text section ──────────────────────
$AS_RV64 -march=rv64gc -o "$OUT/mapping_syms.o" - <<'EOF'
.section .text,"ax"
$x:
        addi    a0, a0, 1
        ret
$d:
        .word   0xdeadbeef
        .word   0xdeadbeef
$x.after_data:
        addi    a0, a0, 2
        ret
EOF
echo "  mapping_syms.o (data island)"

# ── Not-an-ELF (for error-path tests) ────────────────────────────────────────
printf 'this is not an ELF file\n' > "$OUT/not_an_elf.bin"
echo "  not_an_elf.bin (error path)"

echo ""
echo "Done.  Fixtures in $OUT/:"
ls -lh "$OUT/"
