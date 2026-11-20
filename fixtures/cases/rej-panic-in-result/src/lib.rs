//! An assertion inside a fallible function.

/// Why a value was refused.
#[derive(Debug)]
pub enum Refused {
    /// It was zero.
    Zero,
}

/// Checks a value.
///
/// # Errors
/// `Refused::Zero` for zero.
pub fn check(x: u8) -> Result<u8, Refused> {
    assert!(x < 200, "out of range");
    if x == 0 { Err(Refused::Zero) } else { Ok(x) }
}
