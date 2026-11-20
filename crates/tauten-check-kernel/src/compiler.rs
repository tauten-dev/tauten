//! What becomes of compiler output, and whether `cargo tauten check`
//! passes.
//!
//! The shell runs rustc and clippy with every profile lint at warn level,
//! so a violation in one crate never stops its dependents from being
//! checked; the profile then decides each finding's severity
//! ([`crate::rule::finding_severity`]). Decided here:
//! * [`kept`]: which compiler messages become findings;
//! * [`applies`]: whether a rule applies to a target (test targets
//!   included);
//! * [`passes`]: the exit verdict.
//!
//! Proved: `kept_iff`, `applies_spec`, `check_passes_iff`, and that a
//! warning never fails the check.

use crate::severity::{Level, Severity};

/// The level of a compiler message as the compiler reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsgLevel {
    /// `error` (or an internal compiler error).
    Error,
    /// `warning`.
    Warning,
    /// `note`, `help`, `failure-note` and anything else.
    Other,
}

/// Whether a compiler message becomes a finding. `summary` is set for the
/// compiler's closing lines ("aborting due to…", "N warnings emitted");
/// `tiered` when the crate declares a tier; `decided` is the finding's
/// severity after the profile decided it (`decided`). An untiered crate keeps only its
/// errors: TAU-T001 already reports the crate itself.
#[must_use]
pub const fn kept(level: MsgLevel, summary: bool, tiered: bool, decided: Severity) -> bool {
    let reported = match level {
        MsgLevel::Error | MsgLevel::Warning => true,
        MsgLevel::Other => false,
    };
    let blocking = match decided {
        Severity::Error => true,
        Severity::Warning => false,
    };
    reported && !summary && (tiered || blocking)
}

/// Whether a rule with `level` in the crate's tier applies to a target:
/// it must have a level, and a test target (`test`) only sees rules that
/// apply in tests (`in_tests`).
#[must_use]
pub const fn applies(level: Option<Level>, in_tests: bool, test: bool) -> bool {
    match level {
        None => false,
        Some(_) => in_tests || !test,
    }
}

/// The exit verdict: the check passes exactly when no finding is an error.
#[must_use]
pub fn passes(findings: &[Severity]) -> bool {
    let mut ok = true;
    let mut i: usize = 0;
    while i < findings.len() {
        if let Some(s) = findings.get(i) {
            ok = ok
                && match s {
                    Severity::Error => false,
                    Severity::Warning => true,
                };
        }
        i = i.saturating_add(1);
    }
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiler_decisions() {
        assert!(kept(MsgLevel::Warning, false, true, Severity::Warning));
        assert!(!kept(MsgLevel::Warning, false, false, Severity::Warning));
        assert!(kept(MsgLevel::Error, false, false, Severity::Error));
        assert!(!kept(MsgLevel::Warning, true, true, Severity::Error));
        assert!(!kept(MsgLevel::Other, false, true, Severity::Error));
        assert!(applies(Some(Level::Deny), false, false));
        assert!(!applies(Some(Level::Deny), false, true));
        assert!(applies(Some(Level::Warn), true, true));
        assert!(!applies(None, true, false));
        assert!(passes(&[Severity::Warning, Severity::Warning]));
        assert!(!passes(&[Severity::Warning, Severity::Error]));
        assert!(passes(&[]));
    }
}
