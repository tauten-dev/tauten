//! An `allow` hidden in `cfg_attr`: in force only outside tests.

/// The first byte.
#[cfg_attr(not(test), allow(clippy::unwrap_used))]
#[must_use]
pub fn first(bytes: &[u8]) -> u8 {
    *bytes.first().unwrap()
}
