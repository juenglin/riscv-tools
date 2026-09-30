#!/bin/bash
# Run probe-binary and probe-host on the myfive RISC-V guest.
# This script is executed ON THE GUEST (or via 'ssh myfive bash scripts/guest-test.sh').
# $HOME paths are shared via virtio-9p so the repo is visible on both sides.
#
# Usage (from the host):
#   ssh myfive bash /home/rjuengling/juenglin/riscv-tools/scripts/guest-test.sh

set -euo pipefail

REPO="$HOME/juenglin/riscv-tools"
TARGET_DIR="$REPO/target/riscv64gc-unknown-linux-musl/release"  # symlink-safe cargo path

# Cargo output actually goes to a cross-build cache path; find the binaries
PB="$(find "$REPO" -name probe-binary -path '*/riscv64gc-unknown-linux-musl/release/probe-binary' 2>/dev/null | head -1)"
PH="$(find "$REPO" -name probe-host  -path '*/riscv64gc-unknown-linux-musl/release/probe-host'  2>/dev/null | head -1)"

if [ -z "$PB" ] || [ -z "$PH" ]; then
    echo "ERROR: riscv64 release binaries not found."
    echo "Run 'cargo build --release --target riscv64gc-unknown-linux-musl' on the host first."
    exit 1
fi

echo "=== Guest environment ==="
uname -a
head -3 /proc/cpuinfo 2>/dev/null || true
echo ""

echo "=== /proc/cpuinfo ISA ==="
grep '^isa' /proc/cpuinfo | head -4
echo ""

echo "=== probe-host ==="
"$PH" -v
echo ""

echo "=== probe-binary on /bin/ls ==="
"$PB" /bin/ls
echo ""

echo "=== probe-binary on itself ==="
"$PB" "$PB"
echo ""

echo "=== probe-binary -v on itself ==="
"$PB" -v "$PB" 2>&1 | head -12
echo ""

echo "=== probe-binary on a RV64-only nop binary (should give RVI20U64) ==="
python3 -c "
import struct, sys
text = bytes([0x13,0x00,0x00,0x00])  # nop
ss = b'\x00.text\x00.shstrtab\x00'
to = 0x40
so = (to+len(text)+7)&~7
ho = (so+len(ss)+7)&~7
hdr  = b'\x7fELF'+bytes([2,1,1,0])+b'\x00'*8
hdr += struct.pack('<HHI',2,243,1)+struct.pack('<QQQ',0,0,ho)
hdr += struct.pack('<IHHHHHH',0,64,56,0,64,3,2)
elf  = hdr+text+b'\x00'*(so-len(hdr+text))+ss+b'\x00'*(ho-so-len(ss))
def sh(n,t,f,a,o,s): return struct.pack('<IIQQQQIIQQ',n,t,f,a,o,s,0,0,4,0)
elf += sh(0,0,0,0,0,0)+sh(1,1,6,to,to,len(text))+sh(7,3,0,0,so,len(ss))
sys.stdout.buffer.write(elf)
" > /tmp/rv64_nop.elf
"$PB" /tmp/rv64_nop.elf

echo ""
echo "=== All guest tests passed ==="
