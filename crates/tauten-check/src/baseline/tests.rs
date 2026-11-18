//! Unit tests of the ratchet, and differential tests of the kernel-backed
//! ratchet against the `BTreeMap` reference (`reference.rs`).

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

/// A small deterministic generator (xorshift64*), so the differential tests
/// need no dependency and every failure is reproducible from its seed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        let i = usize::try_from(self.below(items.len() as u64)).expect("index");
        items[i]
    }
}

/// A finding drawn from a small key space, so keys collide often.
fn random_diag(profile: &Profile, rng: &mut Rng) -> Diagnostic {
    let id = rng.pick(&["TAU-P001", "TAU-N001", "TAU-P003"]);
    let rule = profile.rule(id).expect("rule");
    let severity = if rng.below(4) == 0 {
        Severity::Warning
    } else {
        Severity::Error
    };
    let krate = rng.pick(&["a", "b"]);
    let mut d = Diagnostic::for_rule(rule, severity, krate, Some(Tier::Core), String::new());
    if rng.below(8) == 0 {
        d.message = "n".to_owned();
    }
    if rng.below(8) == 0 {
        d.source = "clippy::x".to_owned();
    }
    let file = match rng.below(4) {
        0 => None,
        _ => Some("a.rs".to_owned()),
    };
    let line = u32::try_from(rng.below(50)).expect("line");
    d.at(file, Some(line), Some(1))
}

fn random_diags(profile: &Profile, rng: &mut Rng) -> Vec<Diagnostic> {
    let n = rng.below(12);
    (0..n).map(|_| random_diag(profile, rng)).collect()
}

/// A baseline that is sometimes recorded from findings and then perturbed
/// (counts up or down, entries dropped, duplicated or foreign), so covered,
/// uncovered and stale keys, duplicate keys and zero counts all occur.
fn random_baseline(profile: &Profile, rng: &mut Rng, now: &[Diagnostic]) -> Baseline {
    let recorded = if rng.below(2) == 0 {
        now.to_vec()
    } else {
        random_diags(profile, rng)
    };
    let mut entries = reference::from_findings(&recorded).entries;
    for entry in &mut entries {
        entry.count = match rng.below(4) {
            0 => 0,
            1 => entry.count.saturating_add(1),
            2 => entry.count.saturating_sub(1),
            _ => entry.count,
        };
    }
    if rng.below(3) == 0
        && let Some(first) = entries.first().cloned()
    {
        let mut again = first;
        again.count = u32::try_from(rng.below(3)).expect("count");
        entries.push(again);
    }
    if rng.below(3) == 0 {
        entries.reverse();
    }
    if rng.below(2) == 0 {
        entries.truncate(usize::try_from(rng.below(4)).expect("len"));
    }
    Baseline {
        version: VERSION,
        entries,
    }
}

fn assert_same_applied(profile: &Profile, baseline: &Baseline, diags: &[Diagnostic], case: &str) {
    let new = apply(profile, baseline, diags.to_vec());
    let old = reference::apply(profile, baseline, diags.to_vec());
    assert_eq!(new.diagnostics, old.diagnostics, "{case}");
    assert_eq!(new.baselined, old.baselined, "{case}");
}

fn assert_same_increases(old: &Baseline, next: &Baseline, case: &str) {
    assert_eq!(
        old.increases(next),
        reference::increases(old, next),
        "{case}"
    );
}

#[test]
fn kernel_ratchet_agrees_with_the_reference_on_random_inputs() {
    let mut covered = 0_u64;
    let mut stale = 0_usize;
    let mut grew = 0_usize;
    let profile = Profile::embedded().expect("profile");
    for seed in 1..=3000_u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let diags = random_diags(&profile, &mut rng);
        let baseline = random_baseline(&profile, &mut rng, &diags);
        let case = format!("seed {seed}");
        assert_eq!(
            Baseline::from_findings(&diags),
            reference::from_findings(&diags),
            "{case}"
        );
        assert_same_applied(&profile, &baseline, &diags, &case);
        let next = Baseline::from_findings(&diags);
        assert_same_increases(&baseline, &next, &case);
        assert_same_increases(&next, &baseline, &case);
        let other = random_baseline(&profile, &mut rng, &diags);
        assert_same_increases(&baseline, &other, &case);
        let applied = apply(&profile, &baseline, diags);
        covered = covered.saturating_add(applied.baselined);
        stale = stale.saturating_add(
            applied
                .diagnostics
                .iter()
                .filter(|d| d.id == "TAU-R001")
                .count(),
        );
        grew = grew.saturating_add(baseline.increases(&next).len());
    }
    // The generator reaches every outcome, so the agreement is not vacuous.
    assert!(covered > 1000, "covered {covered}");
    assert!(stale > 300, "stale {stale}");
    assert!(grew > 300, "grew {grew}");
}

#[test]
fn kernel_ratchet_agrees_with_the_reference_on_the_unit_cases() {
    let p = |file: &str, line| diag("TAU-P001", file, line, Severity::Error);
    let profile = Profile::embedded().expect("profile");
    let cases: Vec<(Vec<Diagnostic>, Vec<Diagnostic>)> = vec![
        (
            vec![p("a.rs", 3), p("a.rs", 9)],
            vec![p("a.rs", 40), p("a.rs", 41)],
        ),
        (
            vec![p("a.rs", 3)],
            vec![
                p("a.rs", 3),
                p("a.rs", 4),
                diag("TAU-N001", "a.rs", 5, Severity::Error),
            ],
        ),
        (vec![p("a.rs", 3), p("a.rs", 4)], vec![p("a.rs", 3)]),
        (vec![diag("TAU-S002", "a.rs", 1, Severity::Warning)], vec![]),
        (
            vec![p("a.rs", 3)],
            vec![
                p("a.rs", 3),
                p("a.rs", 4),
                diag("TAU-N001", "b.rs", 1, Severity::Error),
            ],
        ),
        (vec![], vec![]),
    ];
    for (i, (before, now)) in cases.iter().enumerate() {
        let case = format!("case {i}");
        let baseline = Baseline::from_findings(before);
        assert_eq!(baseline, reference::from_findings(before), "{case}");
        assert_same_applied(&profile, &baseline, now, &case);
        let next = Baseline::from_findings(now);
        assert_same_increases(&baseline, &next, &case);
        assert_same_increases(&next, &baseline, &case);
    }
}

#[test]
fn duplicate_entries_count_once_and_the_later_one_wins() {
    let p = diag("TAU-P001", "a.rs", 3, Severity::Error);
    let mut baseline = Baseline::from_findings(std::slice::from_ref(&p));
    let mut again = baseline.entries[0].clone();
    again.count = 0;
    baseline.entries.push(again);
    let profile = Profile::embedded().expect("profile");
    let applied = apply(&profile, &baseline, vec![p.clone()]);
    assert_eq!(ids(&applied), vec!["TAU-P001"]);
    assert_same_applied(&profile, &baseline, &[p], "duplicate");
}
