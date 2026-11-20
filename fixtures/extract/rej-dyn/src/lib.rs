//! Reject: a trait object.
#![no_std]

/// Calls a function.
#[must_use]
pub fn call(f: &dyn Fn() -> u32) -> u32 {
    f()
}
