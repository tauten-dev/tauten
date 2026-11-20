//! An unsafe block without a safety comment.

/// The first byte of a non-empty slice.
#[must_use]
pub fn first(bytes: &[u8]) -> Option<u8> {
    if bytes.is_empty() {
        return None;
    }
    Some(unsafe { *bytes.get_unchecked(0) })
}
