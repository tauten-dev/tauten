//! One of two modules with a function named `step`.

/// Its theorem is total.
#[must_use]
pub fn step(x: u32) -> u32 {
    x.saturating_add(1)
}
