//! A file-level allow instead of a reasoned `#![expect]`.
#![no_std]
#![allow(clippy::needless_return, reason = "style")]

/// One.
#[must_use]
pub const fn one() -> u8 {
    return 1;
}
