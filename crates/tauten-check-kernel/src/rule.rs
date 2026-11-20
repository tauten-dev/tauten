//! What a rule may claim: its findings, its inputs and its table entry.
//!
//! A rule is `complete` when it misses no violation among the inputs it
//! sees (its `sees` and `dominion` say which those are); otherwise it is a
//! heuristic. Two laws hold for every rule:
//!
//! * an incomplete rule never blocks ([`finding_severity`]): a check with
//!   false negatives is advice, never presented as a guarantee;
//! * an input the rule could not read is never clean ([`clean`]).
//!
//! And for the table: a complete rule that blocks in some tier names at
//! least one reject fixture ([`fixtures_ok`]), so its claim is exercised.

use crate::severity::{Level, Severity, severity};

/// Whether the rule could read an input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    /// Read (parsed) in full.
    Read,
    /// Not parsed or not readable.
    Unparsed,
}

/// The severity of a finding of a rule with `level` in the tier
/// (`fallback` when it has none): blocking only for a complete rule.
#[must_use]
pub const fn finding_severity(
    level: Option<Level>,
    fallback: Severity,
    complete: bool,
) -> Severity {
    if complete {
        severity(level, fallback)
    } else {
        Severity::Warning
    }
}

/// Whether an input is clean for a rule: read, with no violation.
#[must_use]
pub const fn clean(input: Input, violated: bool) -> bool {
    match input {
        Input::Read => !violated,
        Input::Unparsed => false,
    }
}

/// One rule as the table check sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleFacts {
    /// The rule is complete.
    pub complete: bool,
    /// Some tier gives it a blocking level.
    pub blocks: bool,
    /// It names at least one reject fixture.
    pub fixture: bool,
}

/// Whether every complete, blocking rule names a reject fixture.
#[must_use]
pub fn fixtures_ok(rules: &[RuleFacts]) -> bool {
    let mut ok = true;
    let mut i: usize = 0;
    while i < rules.len() {
        if let Some(r) = rules.get(i) {
            ok = ok && (!r.complete || !r.blocks || r.fixture);
        }
        i = i.saturating_add(1);
    }
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn laws() {
        assert_eq!(
            finding_severity(Some(Level::Deny), Severity::Error, false),
            Severity::Warning
        );
        assert_eq!(
            finding_severity(Some(Level::Deny), Severity::Warning, true),
            Severity::Error
        );
        assert!(!clean(Input::Unparsed, false));
        assert!(clean(Input::Read, false));
        assert!(!clean(Input::Read, true));
        let r = |complete, blocks, fixture| RuleFacts {
            complete,
            blocks,
            fixture,
        };
        assert!(fixtures_ok(&[
            r(true, true, true),
            r(false, true, false),
            r(true, false, false)
        ]));
        assert!(!fixtures_ok(&[r(true, true, true), r(true, true, false)]));
    }
}
