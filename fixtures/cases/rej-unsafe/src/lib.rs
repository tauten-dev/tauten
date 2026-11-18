//! Unsafe in core.

/// Read through a pointer.
#[must_use]
pub fn read(x: &u8) -> u8 {
    // SAFETY: x is a valid reference.
    unsafe { core::ptr::read(x) }
}
