//! Accept: a derived `PartialEq` reached from a translated function
//! translates at this pin.
#![no_std]

/// A kind.
#[derive(PartialEq, Eq)]
pub enum Kind {
    /// One.
    One,
    /// Two.
    Two,
}

/// Whether two kinds are the same, through the derived `PartialEq`.
#[must_use]
pub fn same(a: &Kind, b: &Kind) -> bool {
    a == b
}
