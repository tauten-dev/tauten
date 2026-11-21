//! Something for the tests to exercise.

/// Twice the input, saturating.
#[must_use]
pub const fn double(x: u32) -> u32 {
    x.saturating_mul(2)
}
