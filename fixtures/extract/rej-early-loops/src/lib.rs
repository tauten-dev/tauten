//! Reject: two loops in one function with an early return.
#![no_std]

/// The index of the first zero, or of the first one after it.
#[must_use]
pub fn find(bytes: &[u8]) -> Option<usize> {
    let mut i: usize = 0;
    while i < bytes.len() {
        if bytes.get(i) == Some(&0) {
            return Some(i);
        }
        i = i.saturating_add(1);
    }
    let mut j: usize = 0;
    while j < bytes.len() {
        if bytes.get(j) == Some(&1) {
            return Some(j);
        }
        j = j.saturating_add(1);
    }
    None
}
