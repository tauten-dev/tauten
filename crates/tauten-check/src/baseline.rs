//! The ratchet: a committed record of existing error-level findings, so a
//! project can adopt the profile without fixing everything at once while no
//! new violation gets in.
//!
//! Findings are matched by (rule, crate, file, lint, message) with a count,
//! not by line, so unrelated edits do not invalidate the baseline. The
//! baseline may only shrink: fewer findings than recorded is reported as a
//! stale baseline (TAU-R001) until it is tightened, so freed headroom cannot
//! silently absorb a new violation.
//!
//! The decisions are made by the proved kernel
//! (`tauten_check_kernel::ratchet`, `docs/proofs/README.md`); this module
//! interns keys, renders diagnostics and reads and writes the file format.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::diag::{Diagnostic, Severity};
use crate::profile::Profile;
use tauten_check_kernel::ratchet as kernel;

/// Current baseline file format version.
pub const VERSION: u32 = 1;

/// The identity of a finding for baseline purposes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Key {
    /// Rule ID.
    pub id: String,
    /// Crate name.
    #[serde(rename = "crate")]
    pub krate: String,
    /// File, if the finding has one.
    pub file: Option<String>,
    /// Underlying lint or check.
    pub source: String,
    /// Message text.
    pub message: String,
}

impl Key {
    fn of(d: &Diagnostic) -> Self {
        Self {
            id: d.id.clone(),
            krate: d.krate.clone(),
            file: d.file.clone(),
            source: d.source.clone(),
            message: d.message.clone(),
        }
    }
}

/// One baseline entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// What is being counted.
    #[serde(flatten)]
    pub key: Key,
    /// How many such findings are accepted.
    pub count: u32,
}

/// A baseline file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Baseline {
    /// Format version.
    pub version: u32,
    /// Entries, sorted by key.
    pub entries: Vec<Entry>,
}

/// Finding keys interned to indices `0..len`, in key order, for the kernel.
///
/// Trusted, not proved: the index is injective (one index per distinct key)
/// and agrees with key order. The differential tests in `tests.rs` compare
/// the result with the `BTreeMap` reference.
struct Interned {
    keys: Vec<Key>,
    index: BTreeMap<Key, usize>,
}

impl Interned {
    fn new(keys: impl IntoIterator<Item = Key>) -> Self {
        let sorted: BTreeSet<Key> = keys.into_iter().collect();
        let keys: Vec<Key> = sorted.into_iter().collect();
        let index = keys
            .iter()
            .enumerate()
            .map(|(i, key)| (key.clone(), i))
            .collect();
        Self { keys, index }
    }

    /// The index of `key`; a key that was not interned gets `len`, which the
    /// kernel treats as a key with nothing accepted.
    fn of(&self, key: &Key) -> usize {
        self.index.get(key).copied().unwrap_or(self.keys.len())
    }

    /// The accepted count per index. Of two entries with the same key the
    /// later one counts, as when the entries are collected into a map.
    fn accepted(&self, entries: &[Entry]) -> Vec<u32> {
        let mut counts = vec![0_u32; self.keys.len()];
        for entry in entries {
            if let Some(slot) = counts.get_mut(self.of(&entry.key)) {
                *slot = entry.count;
            }
        }
        counts
    }

    fn findings(&self, diags: &[Diagnostic]) -> Vec<kernel::Finding> {
        diags
            .iter()
            .map(|d| kernel::Finding {
                key: self.of(&Key::of(d)),
                error: d.severity == Severity::Error,
            })
            .collect()
    }
}

impl Baseline {
    /// A baseline accepting exactly the error-level findings in `diags`.
    /// The counts are decided by the proved kernel
    /// ([`kernel::count_errors`]).
    #[must_use]
    pub fn from_findings(diags: &[Diagnostic]) -> Self {
        let keys = Interned::new(diags.iter().map(Key::of));
        let counts = kernel::count_errors(keys.keys.len(), &keys.findings(diags));
        Self {
            version: VERSION,
            entries: keys
                .keys
                .into_iter()
                .zip(counts)
                .filter(|(_, count)| *count > 0)
                .map(|(key, count)| Entry { key, count })
                .collect(),
        }
    }

    /// Total accepted findings.
    #[must_use]
    pub fn total(&self) -> u64 {
        self.entries.iter().map(|e| u64::from(e.count)).sum()
    }

    /// Keys whose count would grow if `next` replaced this baseline. Decided
    /// by the proved kernel ([`kernel::increases`]).
    #[must_use]
    pub fn increases<'n>(&self, next: &'n Self) -> Vec<&'n Entry> {
        let keys = Interned::new(
            self.entries
                .iter()
                .chain(&next.entries)
                .map(|e| e.key.clone()),
        );
        let old = keys.accepted(&self.entries);
        let entries: Vec<kernel::Entry> = next
            .entries
            .iter()
            .map(|e| kernel::Entry {
                key: keys.of(&e.key),
                count: e.count,
            })
            .collect();
        next.entries
            .iter()
            .zip(kernel::increases(&old, &entries))
            .filter(|(_, grew)| *grew)
            .map(|(entry, _)| entry)
            .collect()
    }
}

/// The outcome of checking findings against a baseline.
#[derive(Debug, Clone, Default)]
pub struct Applied {
    /// Findings to report: everything not covered by the baseline, plus one
    /// TAU-R001 error per stale entry.
    pub diagnostics: Vec<Diagnostic>,
    /// Error-level findings the baseline covered.
    pub baselined: u64,
}

/// Remove findings covered by `baseline` and add TAU-R001 for stale entries.
///
/// The proved kernel ([`kernel::apply`], [`kernel::stale`]) decides which
/// findings are covered, what is left of each entry and which entries are
/// stale; this function interns the keys, keeps the uncovered findings in
/// order and renders one TAU-R001 per stale key.
#[must_use]
pub fn apply(profile: &Profile, baseline: &Baseline, diags: Vec<Diagnostic>) -> Applied {
    let keys = Interned::new(
        baseline
            .entries
            .iter()
            .map(|e| e.key.clone())
            .chain(diags.iter().map(Key::of)),
    );
    let findings = keys.findings(&diags);
    let (covered, left) = kernel::apply(keys.accepted(&baseline.entries), &findings);
    let mut out = Applied::default();
    // The kernel returns one flag per finding (theorem `apply_conserves`).
    for (d, hit) in diags.into_iter().zip(covered) {
        if hit {
            out.baselined = out.baselined.saturating_add(1);
        } else {
            out.diagnostics.push(d);
        }
    }
    if let Some(rule) = profile.rule("TAU-R001") {
        for k in kernel::stale(&left) {
            let (Some(key), Some(left)) = (keys.keys.get(k), left.get(k)) else {
                continue;
            };
            let message = format!(
                "baseline accepts {left} more `{}` finding(s) in `{}` than exist now; tighten it with `cargo tauten baseline`",
                key.id,
                key.file.as_deref().unwrap_or(&key.krate),
            );
            let d = Diagnostic::for_rule(rule, Severity::Error, &key.krate, None, message);
            out.diagnostics.push(d.at(key.file.clone(), None, None));
        }
    }
    out
}

#[cfg(test)]
mod reference;
#[cfg(test)]
mod tests;
