//! The ratchet: a committed record of existing error-level findings, so a
//! project can adopt the profile without fixing everything at once while no
//! new violation gets in.
//!
//! Findings are matched by (rule, crate, file, lint, message) with a count,
//! not by line, so unrelated edits do not invalidate the baseline. The
//! baseline may only shrink: fewer findings than recorded is reported as a
//! stale baseline (TAU-R001) until it is tightened, so freed headroom cannot
//! silently absorb a new violation.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::diag::{Diagnostic, Severity};
use crate::profile::Profile;

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

fn count_errors(diags: &[Diagnostic]) -> BTreeMap<Key, u32> {
    let mut counts = BTreeMap::new();
    for d in diags.iter().filter(|d| d.severity == Severity::Error) {
        let slot = counts.entry(Key::of(d)).or_insert(0_u32);
        *slot = slot.saturating_add(1);
    }
    counts
}

impl Baseline {
    /// A baseline accepting exactly the error-level findings in `diags`.
    #[must_use]
    pub fn from_findings(diags: &[Diagnostic]) -> Self {
        Self {
            version: VERSION,
            entries: count_errors(diags)
                .into_iter()
                .map(|(key, count)| Entry { key, count })
                .collect(),
        }
    }

    /// Total accepted findings.
    #[must_use]
    pub fn total(&self) -> u64 {
        self.entries.iter().map(|e| u64::from(e.count)).sum()
    }

    fn counts(&self) -> BTreeMap<&Key, u32> {
        self.entries.iter().map(|e| (&e.key, e.count)).collect()
    }

    /// Keys whose count would grow if `next` replaced this baseline.
    #[must_use]
    pub fn increases<'n>(&self, next: &'n Self) -> Vec<&'n Entry> {
        let old = self.counts();
        next.entries
            .iter()
            .filter(|e| old.get(&e.key).copied().unwrap_or(0) < e.count)
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
#[must_use]
pub fn apply(profile: &Profile, baseline: &Baseline, diags: Vec<Diagnostic>) -> Applied {
    let mut remaining: BTreeMap<Key, u32> = baseline
        .entries
        .iter()
        .map(|e| (e.key.clone(), e.count))
        .collect();
    let mut out = Applied::default();
    for d in diags {
        if d.severity == Severity::Error
            && let Some(left) = remaining.get_mut(&Key::of(&d))
            && *left > 0
        {
            *left = left.saturating_sub(1);
            out.baselined = out.baselined.saturating_add(1);
            continue;
        }
        out.diagnostics.push(d);
    }
    if let Some(rule) = profile.rule("TAU-R001") {
        for (key, left) in remaining.into_iter().filter(|(_, left)| *left > 0) {
            let message = format!(
                "baseline accepts {left} more `{}` finding(s) in `{}` than exist now; tighten it with `cargo tauten baseline`",
                key.id,
                key.file.as_deref().unwrap_or(&key.krate),
            );
            let d = Diagnostic::for_rule(rule, Severity::Error, &key.krate, None, message);
            out.diagnostics.push(d.at(key.file, None, None));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tier::Tier;

    fn diag(id: &str, file: &str, line: u32, severity: Severity) -> Diagnostic {
        let profile = Profile::embedded().expect("profile");
        let rule = profile.rule(id).expect("rule");
        Diagnostic::for_rule(rule, severity, "c", Some(Tier::Core), "m".to_owned()).at(
            Some(file.to_owned()),
            Some(line),
            Some(1),
        )
    }

    fn ids(applied: &Applied) -> Vec<&str> {
        applied.diagnostics.iter().map(|d| d.id.as_str()).collect()
    }

    #[test]
    fn baselined_findings_pass_even_after_moving() {
        let profile = Profile::embedded().expect("profile");
        let before = vec![
            diag("TAU-P001", "a.rs", 3, Severity::Error),
            diag("TAU-P001", "a.rs", 9, Severity::Error),
        ];
        let baseline = Baseline::from_findings(&before);
        assert_eq!(baseline.total(), 2);
        let moved = vec![
            diag("TAU-P001", "a.rs", 40, Severity::Error),
            diag("TAU-P001", "a.rs", 41, Severity::Error),
        ];
        let applied = apply(&profile, &baseline, moved);
        assert!(applied.diagnostics.is_empty());
        assert_eq!(applied.baselined, 2);
    }

    #[test]
    fn new_findings_fail() {
        let profile = Profile::embedded().expect("profile");
        let baseline = Baseline::from_findings(&[diag("TAU-P001", "a.rs", 3, Severity::Error)]);
        let now = vec![
            diag("TAU-P001", "a.rs", 3, Severity::Error),
            diag("TAU-P001", "a.rs", 4, Severity::Error),
            diag("TAU-N001", "a.rs", 5, Severity::Error),
        ];
        let applied = apply(&profile, &baseline, now);
        assert_eq!(ids(&applied), vec!["TAU-P001", "TAU-N001"]);
    }

    #[test]
    fn fixed_findings_make_the_baseline_stale() {
        let profile = Profile::embedded().expect("profile");
        let baseline = Baseline::from_findings(&[
            diag("TAU-P001", "a.rs", 3, Severity::Error),
            diag("TAU-P001", "a.rs", 4, Severity::Error),
        ]);
        let applied = apply(
            &profile,
            &baseline,
            vec![diag("TAU-P001", "a.rs", 3, Severity::Error)],
        );
        assert_eq!(ids(&applied), vec!["TAU-R001"]);
        assert!(
            applied
                .diagnostics
                .iter()
                .all(|d| d.severity == Severity::Error)
        );
    }

    #[test]
    fn warnings_are_never_baselined() {
        let baseline = Baseline::from_findings(&[diag("TAU-S002", "a.rs", 1, Severity::Warning)]);
        assert!(baseline.entries.is_empty());
    }

    #[test]
    fn increases_are_detected() {
        let old = Baseline::from_findings(&[diag("TAU-P001", "a.rs", 3, Severity::Error)]);
        let same = old.clone();
        assert!(old.increases(&same).is_empty());
        let more = Baseline::from_findings(&[
            diag("TAU-P001", "a.rs", 3, Severity::Error),
            diag("TAU-P001", "a.rs", 4, Severity::Error),
            diag("TAU-N001", "b.rs", 1, Severity::Error),
        ]);
        assert_eq!(old.increases(&more).len(), 2);
        assert!(more.increases(&old).is_empty());
    }
}
