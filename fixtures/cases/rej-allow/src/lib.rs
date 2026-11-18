//! `#[allow]` instead of `#[expect]` with a reason.

/// Parse a port.
#[allow(clippy::unwrap_used)]
#[must_use]
pub fn port(text: &str) -> u16 {
    text.parse().unwrap()
}
