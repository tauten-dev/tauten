//! Reject: floating point.
#![no_std]

/// Half of a value.
#[must_use]
pub fn half(x: f64) -> f64 {
    x / 2.0
}
