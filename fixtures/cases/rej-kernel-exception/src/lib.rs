#![no_std]
//! Kernel with a lint exception.

/// First byte.
#[expect(clippy::indexing_slicing, reason = "trust me")]
#[must_use]
pub fn first(v: &[u8]) -> u8 {
    v[0]
}
