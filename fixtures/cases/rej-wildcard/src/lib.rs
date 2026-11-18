//! Wildcard over an owned enum.

/// A state.
#[derive(Debug, Clone, Copy)]
pub enum Mode {
    /// a
    Read,
    /// b
    Write,
    /// c
    Admin,
}

/// Whether the mode may write.
#[must_use]
pub const fn can_write(mode: Mode) -> bool {
    match mode {
        Mode::Write => true,
        _ => false,
    }
}
