//! Accept: shapes the translation handles.
#![no_std]
extern crate alloc;
use alloc::vec::Vec;

/// A kind.
pub enum Kind {
    /// One.
    One,
    /// Two.
    Two,
}

/// Whether two kinds are the same, by `match`.
#[must_use]
pub const fn same(a: &Kind, b: &Kind) -> bool {
    matches!((a, b), (Kind::One, Kind::One) | (Kind::Two, Kind::Two))
}

/// The number of non-zero bytes, with a `while` loop over `get`.
#[must_use]
pub fn count(bytes: &Vec<u8>) -> usize {
    let mut n: usize = 0;
    let mut i: usize = 0;
    while i < bytes.len() {
        if let Some(b) = bytes.get(i)
            && *b != 0
        {
            n = n.saturating_add(1);
        }
        i = i.saturating_add(1);
    }
    n
}
