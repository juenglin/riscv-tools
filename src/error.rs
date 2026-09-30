use thiserror::Error;

/// Application-level errors produced by the ELF parser and related code.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("not an ELF file (bad magic)")]
    NotElf,

    #[error("not a RISC-V ELF (e_machine = {e_machine:#x})")]
    NotRiscV { e_machine: u16 },

    #[error("big-endian RISC-V ELFs are not supported")]
    BigEndian,

    #[error("ELF file is truncated or corrupt: {0}")]
    Corrupt(String),

    #[error("unsupported: {0}")]
    Unsupported(String),
}

/// Process exit codes used by the CLI binaries.
pub mod exit {
    pub const OK: i32 = 0;
    pub const IO: i32 = 1;
    pub const NOT_ELF: i32 = 2;
    pub const NOT_RISCV: i32 = 3;
    pub const UNSUPPORTED: i32 = 4;
    pub const NO_PROFILE: i32 = 5;
}
