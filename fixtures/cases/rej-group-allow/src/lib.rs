//! An exception for a whole lint group covers rules that admit none.

/// One.
#[expect(clippy::all, reason = "generated code")]
#[must_use]
pub const fn one() -> u8 {
    return 1;
}
