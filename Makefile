TARGET := riscv64gc-unknown-linux-musl
VENV   ?= .venv

.PHONY: all check fmt test build test-riscv fixtures setup clean

all: check test

## Run rustfmt check then clippy; hard-errors on any warning or format diff.
check:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings

## Apply rustfmt in place.
fmt:
	cargo fmt

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

## Create .venv, install pre-commit, and install the git hook.
## Re-running is safe and refreshes pins and the hook.
## Requires cargo on PATH (run: . "$$HOME/.cargo/env" first).
setup:
	python3 -m venv $(VENV)
	$(VENV)/bin/pip install --quiet --upgrade pip
	$(VENV)/bin/pip install --quiet -r requirements-dev.txt
	$(VENV)/bin/pre-commit install --install-hooks

clean:
	cargo clean
	rm -f fixtures/*.o fixtures/*.elf
