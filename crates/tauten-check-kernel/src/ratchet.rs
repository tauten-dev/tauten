//! The baseline ratchet over interned keys.
//!
//! `tauten_check::baseline` interns every distinct finding key (rule, crate,
//! file, lint, message) to an index, in key order, and calls these functions
//! for every ratchet decision: which findings the baseline covers, the
//! accepted counts left over, which keys are stale (TAU-R001), the error
//! counts a new baseline records, and which entries of a new baseline would
//! grow.
//!
//! The code is written for translation and for the kernel profile: `while`
//! loops over `get`, saturating arithmetic, and no indexing.

use alloc::vec::Vec;

/// One finding, its key interned to an index.
pub struct Finding {
    /// Interned key.
    pub key: usize,
    /// Whether the finding is an error (only errors can be covered).
    pub error: bool,
}

/// One baseline entry, its key interned to an index.
pub struct Entry {
    /// Interned key.
    pub key: usize,
    /// How many findings the entry accepts.
    pub count: u32,
}

/// The count recorded for `key`: 0 for a key the list does not reach.
#[must_use]
pub fn count_at(counts: &[u32], key: usize) -> u32 {
    match counts.get(key) {
        Some(count) => *count,
        None => 0,
    }
}

/// Covers `finding` if it is an error and its key has accepted count left,
/// consuming one unit; returns whether it was covered.
pub fn take(left: &mut [u32], finding: &Finding) -> bool {
    if !finding.error {
        return false;
    }
    match left.get_mut(finding.key) {
        Some(slot) if *slot > 0 => {
            *slot = slot.saturating_sub(1);
            true
        }
        Some(_) | None => false,
    }
}

/// [`take`] for the finding at position `i`, if there is one.
pub fn take_at(left: &mut [u32], findings: &[Finding], i: usize) -> bool {
    match findings.get(i) {
        Some(finding) => take(left, finding),
        None => false,
    }
}

/// Checks `findings`, in order, against the accepted counts per key.
///
/// Returns one flag per finding (covered by the baseline or not) and the
/// accepted counts left over. A finding is covered when it is an error and
/// its key still has accepted count left.
#[must_use]
pub fn apply(accepted: Vec<u32>, findings: &[Finding]) -> (Vec<bool>, Vec<u32>) {
    let mut left = accepted;
    let mut covered = Vec::new();
    let mut i: usize = 0;
    while i < findings.len() {
        let hit = take_at(&mut left, findings, i);
        covered.push(hit);
        i = i.saturating_add(1);
    }
    (covered, left)
}

/// The keys with accepted count left over after [`apply`], in increasing
/// order: each is a stale baseline entry (TAU-R001).
#[must_use]
pub fn stale(left: &[u32]) -> Vec<usize> {
    let mut keys = Vec::new();
    let mut k: usize = 0;
    while k < left.len() {
        if count_at(left, k) > 0 {
            keys.push(k);
        }
        k = k.saturating_add(1);
    }
    keys
}

/// `len` zero counts.
#[must_use]
pub fn zeros(len: usize) -> Vec<u32> {
    let mut counts = Vec::new();
    let mut i: usize = 0;
    while i < len {
        counts.push(0);
        i = i.saturating_add(1);
    }
    counts
}

/// Counts one error finding against its key, saturating.
pub fn add_error(counts: &mut [u32], finding: &Finding) {
    if finding.error
        && let Some(slot) = counts.get_mut(finding.key)
    {
        *slot = slot.saturating_add(1);
    }
}

/// [`add_error`] for the finding at position `i`, if there is one.
pub fn add_error_at(counts: &mut [u32], findings: &[Finding], i: usize) {
    if let Some(finding) = findings.get(i) {
        add_error(counts, finding);
    }
}

/// The number of error findings per key, for `keys` keys: what a new
/// baseline accepts.
#[must_use]
pub fn count_errors(keys: usize, findings: &[Finding]) -> Vec<u32> {
    let mut counts = zeros(keys);
    let mut i: usize = 0;
    while i < findings.len() {
        add_error_at(&mut counts, findings, i);
        i = i.saturating_add(1);
    }
    counts
}

/// Whether the entry at position `i` of `next`, if there is one, accepts
/// more than `old` does for its key.
#[must_use]
pub fn grows_at(old: &[u32], next: &[Entry], i: usize) -> bool {
    match next.get(i) {
        Some(entry) => count_at(old, entry.key) < entry.count,
        None => false,
    }
}

/// One flag per entry of a new baseline `next`: whether it accepts more
/// than `old` does for the same key.
#[must_use]
pub fn increases(old: &[u32], next: &[Entry]) -> Vec<bool> {
    let mut grew = Vec::new();
    let mut i: usize = 0;
    while i < next.len() {
        let more = grows_at(old, next, i);
        grew.push(more);
        i = i.saturating_add(1);
    }
    grew
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn error(key: usize) -> Finding {
        Finding { key, error: true }
    }

    #[test]
    fn covers_up_to_the_accepted_count() {
        let findings = [
            error(0),
            error(0),
            Finding {
                key: 0,
                error: false,
            },
            error(1),
        ];
        let (covered, left) = apply(vec![1, 2], &findings);
        assert_eq!(covered, vec![true, false, false, true]);
        assert_eq!(left, vec![0, 1]);
        assert_eq!(stale(&left), vec![1]);
    }

    #[test]
    fn counts_and_increases() {
        let findings = [
            error(0),
            error(2),
            error(2),
            Finding {
                key: 1,
                error: false,
            },
        ];
        assert_eq!(count_errors(3, &findings), vec![1, 0, 2]);
        let next = [Entry { key: 0, count: 1 }, Entry { key: 2, count: 3 }];
        assert_eq!(increases(&[1, 0, 2], &next), vec![false, true]);
        assert_eq!(zeros(2), vec![0, 0]);
    }
}
