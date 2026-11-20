//! A parameter named `given`: Rust accepts it, the Lean the translation
//! produces does not (`given` is reserved there).
#![no_std]

/// Whether `given` covers `needs`.
#[must_use]
pub const fn covers(needs: u8, given: u8) -> bool {
    given >= needs
}
