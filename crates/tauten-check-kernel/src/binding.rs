//! Whether a recorded result is still about the current inputs.
//!
//! A result (a proof outcome, a generated file, a cached verdict, stored
//! evidence) is recorded with a binding per input it depends on: a key
//! naming the input and the bytes that identify it (a digest, a version,
//! the content itself). It may be reused only while every binding still
//! holds. The shell lists the recorded and the current bindings; [`fresh`]
//! decides.
//!
//! Proved: fresh exactly when the two have the same keys and, for every
//! key, equal bytes. A current input the record does not mention (a key
//! added after the record was written) makes it stale; so does a recorded
//! input that no longer exists, and any changed byte.

use alloc::vec::Vec;

/// One input and the bytes that identify it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// Interned name of the input.
    pub key: usize,
    /// The bytes that identify it.
    pub bytes: Vec<u8>,
}

/// Whether `a` and `b` are the same bytes.
#[must_use]
pub fn same_bytes(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut same = true;
    let mut i: usize = 0;
    while i < a.len() {
        if let (Some(x), Some(y)) = (a.get(i), b.get(i)) {
            same = same && *x == *y;
        }
        i = i.saturating_add(1);
    }
    same
}

/// Whether `item` is one of the byte strings in `list`.
#[must_use]
pub fn contains_bytes(list: &[Vec<u8>], item: &[u8]) -> bool {
    let mut found = false;
    let mut i: usize = 0;
    while i < list.len() {
        if let Some(x) = list.get(i) {
            found = found || same_bytes(x, item);
        }
        i = i.saturating_add(1);
    }
    found
}

/// Whether some binding in `list` has `key`.
#[must_use]
pub fn has_key(list: &[Binding], key: usize) -> bool {
    let mut found = false;
    let mut i: usize = 0;
    while i < list.len() {
        if let Some(b) = list.get(i) {
            found = found || b.key == key;
        }
        i = i.saturating_add(1);
    }
    found
}

/// Whether every binding in `list` with `key` has exactly `bytes`.
#[must_use]
pub fn agrees(list: &[Binding], key: usize, bytes: &[u8]) -> bool {
    let mut ok = true;
    let mut i: usize = 0;
    while i < list.len() {
        if let Some(b) = list.get(i) {
            ok = ok && (b.key != key || same_bytes(&b.bytes, bytes));
        }
        i = i.saturating_add(1);
    }
    ok
}

/// Whether every current binding is recorded with the same bytes.
#[must_use]
pub fn covered(recorded: &[Binding], current: &[Binding]) -> bool {
    let mut ok = true;
    let mut i: usize = 0;
    while i < current.len() {
        if let Some(c) = current.get(i) {
            ok = ok && has_key(recorded, c.key) && agrees(recorded, c.key, &c.bytes);
        }
        i = i.saturating_add(1);
    }
    ok
}

/// Whether every recorded key still exists.
#[must_use]
pub fn present(recorded: &[Binding], current: &[Binding]) -> bool {
    let mut ok = true;
    let mut i: usize = 0;
    while i < recorded.len() {
        if let Some(r) = recorded.get(i) {
            ok = ok && has_key(current, r.key);
        }
        i = i.saturating_add(1);
    }
    ok
}

/// Whether a result recorded with `recorded` may be reused now.
#[must_use]
pub fn fresh(recorded: &[Binding], current: &[Binding]) -> bool {
    covered(recorded, current) && present(recorded, current)
}

#[cfg(test)]
mod tests {
    use super::*;

    use alloc::vec;

    fn b(key: usize, bytes: &[u8]) -> Binding {
        Binding {
            key,
            bytes: bytes.to_vec(),
        }
    }

    #[test]
    fn freshness() {
        let rec = vec![b(0, b"abc"), b(1, b"v1")];
        assert!(fresh(&rec, &[b(1, b"v1"), b(0, b"abc")]));
        assert!(!fresh(&rec, &[b(0, b"abd"), b(1, b"v1")]));
        assert!(!fresh(&rec, &[b(0, b"ab"), b(1, b"v1")]));
        assert!(!fresh(&rec, &[b(0, b"abc")]));
        assert!(!fresh(&rec, &[b(0, b"abc"), b(1, b"v1"), b(2, b"new")]));
        assert!(fresh(&[], &[]));
        assert!(same_bytes(b"", b""));
    }
}
