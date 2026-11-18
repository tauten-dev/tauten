#![expect(clippy::unwrap_used, reason = "crate-wide opt-out")]
//! Crate-wide opt-out of a profile rule.

/// Parse a port.
#[must_use]
pub fn port(text: &str) -> u16 {
    text.parse().unwrap()
}
