//! A result discarded with `let _` and no reason.

/// Why saving failed.
#[derive(Debug)]
pub enum SaveError {
    /// Out of space.
    Full,
}

/// Saves a value.
///
/// # Errors
/// `SaveError::Full` when there is no space.
pub const fn save(x: u8) -> Result<(), SaveError> {
    if x == 0 { Err(SaveError::Full) } else { Ok(()) }
}

/// Saves and ignores the outcome.
pub fn save_quietly(x: u8) {
    let _ = save(x);
}
