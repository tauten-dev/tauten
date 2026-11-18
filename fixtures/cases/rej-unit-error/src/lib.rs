//! Untyped error.

/// Parse something.
///
/// # Errors
/// Fails without saying why.
pub fn parse(text: &str) -> Result<u8, ()> {
    text.parse().map_err(drop)
}
