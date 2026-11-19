//! The rule-table checks as written before the kernel decided them
//! (`BTreeSet`/`BTreeMap` over strings), kept as the reference of a
//! differential test against the kernel-backed [`super::validate`].

use std::collections::{BTreeMap, BTreeSet};

use super::{EMBEDDED_RULES, EMITTED_BY_CODE, ProfileError, RawProfile, validate};

/// The checks of `Profile::parse` before P4, in their original order.
fn reference(raw: &RawProfile) -> Result<(), ProfileError> {
    let mut ids = BTreeSet::new();
    let mut lint_owner = BTreeMap::new();
    for (index, rule) in raw.rule.iter().enumerate() {
        if !ids.insert(rule.id.clone()) {
            return Err(ProfileError::DuplicateId(rule.id.clone()));
        }
        for lint in &rule.lints {
            if let Some(previous) = lint_owner.insert(lint.clone(), index) {
                let first = raw
                    .rule
                    .get(previous)
                    .map_or_else(String::new, |r| r.id.clone());
                return Err(ProfileError::DuplicateLint {
                    lint: lint.clone(),
                    first,
                    second: rule.id.clone(),
                });
            }
        }
    }
    if let Some(bad) = raw.api.iter().find(|api| !ids.contains(&api.rule)) {
        return Err(ProfileError::UnknownRule(bad.rule.clone()));
    }
    if let Some(missing) = EMITTED_BY_CODE.iter().find(|id| !ids.contains(**id)) {
        return Err(ProfileError::MissingRule((*missing).to_owned()));
    }
    Ok(())
}

/// A small deterministic generator (xorshift64*), as in the ratchet's
/// differential tests.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % (n.max(1) as u64)).expect("index")
    }
}

const IDS: [&str; 4] = ["TAU-R001", "TAU-X001", "TAU-NEW", "tau-r001"];
const LINTS: [&str; 3] = ["clippy::unwrap_used", "clippy::new_lint", "unsafe_code"];

/// The embedded table with a few random edits: rules dropped or
/// duplicated, IDs and lints overwritten from small pools (so they collide
/// with existing ones), lints repeated within a rule, API bans renamed.
fn random_table(rng: &mut Rng) -> RawProfile {
    let mut raw: RawProfile = toml::from_str(EMBEDDED_RULES).expect("embedded table");
    for _ in 0..rng.below(4) {
        let rules = raw.rule.len();
        match rng.below(7) {
            0 if rules > 0 => {
                raw.rule.remove(rng.below(rules));
            }
            1 if rules > 0 => {
                let copy = raw.rule[rng.below(rules)].clone();
                let at = rng.below(rules + 1);
                raw.rule.insert(at, copy);
            }
            2 if rules > 0 => {
                let at = rng.below(rules);
                raw.rule[at].id = IDS[rng.below(IDS.len())].to_owned();
            }
            3 if rules > 0 => {
                let at = rng.below(rules);
                let lint = LINTS[rng.below(LINTS.len())].to_owned();
                let lints = &mut raw.rule[at].lints;
                let pos = rng.below(lints.len() + 1);
                lints.insert(pos, lint);
            }
            4 if rules > 0 => {
                let at = rng.below(rules);
                if let Some(lint) = raw.rule[at].lints.first().cloned() {
                    raw.rule[at].lints.push(lint);
                }
            }
            5 if !raw.api.is_empty() => {
                let at = rng.below(raw.api.len());
                raw.api[at].rule = IDS[rng.below(IDS.len())].to_owned();
            }
            _ if rules > 1 => {
                let (a, b) = (rng.below(rules), rng.below(rules));
                let id = raw.rule[a].id.clone();
                raw.rule[b].id = id;
            }
            _ => {}
        }
    }
    raw
}

fn outcome(result: &Result<(), ProfileError>) -> String {
    match result {
        Ok(()) => "ok".to_owned(),
        Err(error) => error.to_string(),
    }
}

#[test]
fn kernel_validation_agrees_with_the_reference_on_random_tables() {
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    for seed in 1..=3000_u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let raw = random_table(&mut rng);
        let (kernel, old) = (validate(&raw), reference(&raw));
        assert_eq!(outcome(&kernel), outcome(&old), "seed {seed}");
        let kind = match old {
            Ok(()) => "ok",
            Err(ProfileError::DuplicateId(_)) => "duplicate id",
            Err(ProfileError::DuplicateLint {
                ref first,
                ref second,
                ..
            }) if first == second => "lint repeated within a rule",
            Err(ProfileError::DuplicateLint { .. }) => "duplicate lint",
            Err(ProfileError::UnknownRule(_)) => "unknown rule",
            Err(ProfileError::MissingRule(_)) => "missing rule",
            Err(_) => "other",
        };
        *seen.entry(kind).or_default() += 1;
    }
    // The generator must reach every outcome often enough to mean something.
    for kind in [
        "ok",
        "duplicate id",
        "lint repeated within a rule",
        "duplicate lint",
        "unknown rule",
        "missing rule",
    ] {
        assert!(
            seen.get(kind).copied().unwrap_or(0) >= 50,
            "{kind}: {seen:?}"
        );
    }
}

#[test]
fn kernel_validation_agrees_with_the_reference_on_the_embedded_table() {
    let raw: RawProfile = toml::from_str(EMBEDDED_RULES).expect("embedded table");
    assert_eq!(outcome(&validate(&raw)), "ok");
    assert_eq!(outcome(&reference(&raw)), "ok");
}
