//! Parsing.
#![expect(clippy::unwrap_used, reason = "inputs are checked by the caller")]

/// Parse a port.
#[must_use]
pub fn port(text: &str) -> u16 {
    text.trim().parse().unwrap()
}
