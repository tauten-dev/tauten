//! Reject: an iterator chain with a closure.
#![no_std]

/// The sum of the bytes plus one each.
#[must_use]
pub fn total(bytes: &[u8]) -> u32 {
    bytes.iter().map(|b| u32::from(*b) + 1).sum()
}
