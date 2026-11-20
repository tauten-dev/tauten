#![cfg_attr(not(test), no_std)]
//! A kernel that links `std` only in its own unit tests.

/// The larger of two values.
#[must_use]
pub const fn larger(a: u32, b: u32) -> u32 {
    if a > b { a } else { b }
}

#[cfg(test)]
mod tests {
    #[test]
    fn larger_is_symmetric() {
        assert_eq!(super::larger(2, 3), super::larger(3, 2));
    }
}
