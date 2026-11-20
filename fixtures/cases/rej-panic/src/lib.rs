//! An explicit panic in library code.

/// Rejects big values by panicking.
#[must_use]
pub fn small(x: u8) -> u8 {
    if x > 3 {
        panic!("too big");
    }
    x
}
