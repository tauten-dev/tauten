//! Lossy cast.

/// Narrow a length.
#[must_use]
pub const fn narrow(x: u64) -> u32 {
    x as u32
}
