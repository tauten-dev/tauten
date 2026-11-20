//! Whether a list may be changed: nothing quietly removed or weakened.
//!
//! A registry is any list whose entries must not disappear or weaken
//! without an explicit amendment: the theorems and mutants of a proof
//! claim, a lint baseline, required checks, diagnostic codes. Each entry is
//! an interned identifier and a strength (0 when the list has no notion of
//! strength). The shell lists the old (committed) entries, the new ones and
//! the identifiers named by an amendment; [`conserves`] decides.
//!
//! A may-not-shrink list calls `conserves(old, new, amended)`. A
//! may-not-grow list (for example the assumptions a claim rests on) swaps
//! the first two arguments: every new entry must already have been there,
//! at least as strong.
//!
//! Proved: the change is conserved exactly when every old entry is either
//! named by an amendment or kept in the new list with at least its
//! strength; conservation is reflexive and transitive (amendments
//! accumulate); and an entry that was removed or weakened is always named
//! by an amendment.

use crate::control::member;

/// One registry entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// Interned identifier.
    pub id: usize,
    /// How strong it is (0 when the registry has no notion of strength).
    pub strength: u32,
}

/// Whether `list` keeps entry `id` with at least `strength`.
#[must_use]
pub fn kept(list: &[Entry], id: usize, strength: u32) -> bool {
    let mut found = false;
    let mut i: usize = 0;
    while i < list.len() {
        if let Some(e) = list.get(i) {
            found = found || (e.id == id && strength <= e.strength);
        }
        i = i.saturating_add(1);
    }
    found
}

/// Whether the change from `old` to `new` removes or weakens nothing that
/// `amended` does not name.
#[must_use]
pub fn conserves(old: &[Entry], new: &[Entry], amended: &[usize]) -> bool {
    let mut ok = true;
    let mut i: usize = 0;
    while i < old.len() {
        if let Some(o) = old.get(i) {
            ok = ok && (member(amended, o.id) || kept(new, o.id, o.strength));
        }
        i = i.saturating_add(1);
    }
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn e(id: usize, strength: u32) -> Entry {
        Entry { id, strength }
    }

    #[test]
    fn conservation() {
        let old = [e(1, 0), e(2, 3)];
        assert!(conserves(&old, &old, &[]));
        assert!(conserves(&old, &[e(2, 4), e(1, 0), e(9, 0)], &[]));
        assert!(!conserves(&old, &[e(2, 3)], &[]));
        assert!(conserves(&old, &[e(2, 3)], &[1]));
        assert!(!conserves(&old, &[e(1, 0), e(2, 2)], &[]));
        assert!(conserves(&old, &[e(1, 0), e(2, 2)], &[2]));
        assert!(!conserves(&old, &[e(1, 0)], &[1]));
        assert!(conserves(&[], &[], &[]));
    }
}
