//! A core crate whose release builds wrap on overflow.

/// Adds two counts.
#[must_use]
pub const fn add(a: u32, b: u32) -> Option<u32> {
    a.checked_add(b)
}
