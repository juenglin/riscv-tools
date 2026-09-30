TARGET := riscv64gc-unknown-linux-musl

.PHONY: all check test build clean

all: check test

## Run clippy (hard-errors on any warning) and the full test suite.
check:
	cargo clippy --all-targets -- -D warnings

test:
	cargo test

## Cross-compile release binaries for RISC-V musl.
build:
	cargo build --release --target $(TARGET)

## Run the unit test suite natively on the RISC-V target via qemu-user.
test-riscv:
	cargo test --target $(TARGET) --lib

## Build clang ELF fixtures used by the integration tests.
fixtures:
	bash scripts/build-fixtures.sh

clean:
	cargo clean
	rm -f fixtures/*.o fixtures/*.elf fixtures/*.bin
