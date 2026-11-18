//! The ratchet as it was before the decisions moved to the proved kernel
//! (`tauten_check_kernel::ratchet`): `BTreeMap`s keyed by [`Key`]. Kept as
//! the reference for the differential tests in `tests.rs`.

use std::collections::BTreeMap;

use super::{Applied, Baseline, Entry, Key, VERSION};
use crate::diag::{Diagnostic, Severity};
use crate::profile::Profile;

fn count_errors(diags: &[Diagnostic]) -> BTreeMap<Key, u32> {
    let mut counts = BTreeMap::new();
    for d in diags.iter().filter(|d| d.severity == Severity::Error) {
        let slot = counts.entry(Key::of(d)).or_insert(0_u32);
        *slot = slot.saturating_add(1);
    }
    counts
}

/// The old `Baseline::from_findings`.
pub(super) fn from_findings(diags: &[Diagnostic]) -> Baseline {
    Baseline {
        version: VERSION,
        entries: count_errors(diags)
            .into_iter()
            .map(|(key, count)| Entry { key, count })
            .collect(),
    }
}

fn counts(baseline: &Baseline) -> BTreeMap<&Key, u32> {
    baseline.entries.iter().map(|e| (&e.key, e.count)).collect()
}

/// The old `Baseline::increases`.
pub(super) fn increases<'n>(baseline: &Baseline, next: &'n Baseline) -> Vec<&'n Entry> {
    let old = counts(baseline);
    next.entries
        .iter()
        .filter(|e| old.get(&e.key).copied().unwrap_or(0) < e.count)
        .collect()
}

/// The old `apply`.
pub(super) fn apply(profile: &Profile, baseline: &Baseline, diags: Vec<Diagnostic>) -> Applied {
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
