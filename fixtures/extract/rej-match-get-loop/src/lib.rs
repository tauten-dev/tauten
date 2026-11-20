//! Reject: a match on `get` directly in a loop body.
#![no_std]

/// How many bytes are zero.
#[must_use]
pub fn zeros(bytes: &[u8]) -> usize {
    let mut n: usize = 0;
    let mut i: usize = 0;
    while i < bytes.len() {
        match bytes.get(i) {
            Some(0) => n = n.saturating_add(1),
            Some(_) | None => {}
        }
        i = i.saturating_add(1);
    }
    n
}
