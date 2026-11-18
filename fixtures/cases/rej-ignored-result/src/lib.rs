//! Ignored result.

/// Fallible step.
///
/// # Errors
/// Always fails.
pub const fn step() -> Result<(), u8> {
    Err(1)
}

/// Caller that drops the error.
pub fn run() {
    step();
}
