//! A kernel crate reaching `core` items through `std`.
#![no_std]
extern crate std;

/// Compares two values.
#[must_use]
pub fn order(a: u8, b: u8) -> std::cmp::Ordering {
    a.cmp(&b)
}
