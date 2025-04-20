// VM Error types: register access errors, stack errors, VM faults

use thiserror::Error;

/// Register Errors
#[derive(Error, Debug, PartialEq, Eq)]
pub enum RegisterError {
    #[error("Invalid register specified")]
    InvalidRegister,
    #[error("Attempted to write to a read-only register")]
    ReadOnlyRegister,
}

/// Stack Errors
#[derive(Error, Debug, PartialEq, Eq)]
pub enum StackError {
    #[error("Stack overflow")]
    Overflow,
    #[error("Stack underflow")]
    Underflow,
}

/// VM Errors
#[derive(Error, Debug, PartialEq, Eq, Copy, Clone)]
pub enum VMFault {
    #[error("Invalid instruction encountered")]
    InvalidInstruction,
    #[error("Invalid register specified")]
    InvalidRegister,
    #[error("Attempted to write to a read-only register")]
    PermissionError,
    #[error("Stack overflow")]
    StackOverflow,
    #[error("Stack underflow")]
    StackUnderflow,
    #[error("Division by zero")]
    DivisionByZero,
    #[error("No component selected for operation")]
    NoComponentSelected,
    #[error("Selected component invalid for operation")]
    InvalidComponentForOp,
    #[error("Not enough power for operation")]
    InsufficientPower,
    #[error("Weapon overheated")]
    WeaponOverheated,
    #[error("Invalid weapon power value")]
    InvalidWeaponPower,
    #[error("Invalid scan result")]
    InvalidScanResult,
    #[error("Error during projectile handling")]
    ProjectileError,
    #[error("Call stack overflow")]
    CallStackOverflow,
    #[error("Call stack underflow")]
    CallStackUnderflow,
    #[error("Instruction or feature not implemented")]
    NotImplemented,
}
