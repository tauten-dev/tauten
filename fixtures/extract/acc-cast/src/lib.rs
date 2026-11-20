//! Accept: an integer cast translates at this pin.
#![no_std]

/// Widens a byte.
#[must_use]
pub const fn widen(x: u8) -> u32 {
    x as u32
}
