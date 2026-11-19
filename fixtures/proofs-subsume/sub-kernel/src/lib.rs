//! Proof subsumption fixture: arithmetic and indexing in proved functions.
#![no_std]

/// The sum, or `u32::MAX` when it does not fit. Proved total: the guard
/// keeps both operations in range.
#[must_use]
pub fn add_capped(a: u32, b: u32) -> u32 {
    if a > u32::MAX - b { u32::MAX } else { a + b }
}

/// The first value, or 0 for an empty slice. Proved total.
#[must_use]
pub fn first_or_zero(values: &[u32]) -> u32 {
    if values.is_empty() { 0 } else { values[0] }
}

/// Doubles a value. Its theorem needs a hypothesis (`a <= 1000`), so it is
/// not total and its arithmetic finding stays an error.
#[must_use]
pub fn double(a: u32) -> u32 {
    a * 2
}

/// Not claimed at all: its finding stays an error.
#[must_use]
pub fn triple(a: u32) -> u32 {
    a * 3
}
