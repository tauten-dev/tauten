#![cfg_attr(not(feature = "std"), no_std)]
//! A kernel that is `no_std` only while the `std` feature is off.

/// The larger of two values.
#[must_use]
pub const fn larger(a: u32, b: u32) -> u32 {
    if a > b { a } else { b }
}
