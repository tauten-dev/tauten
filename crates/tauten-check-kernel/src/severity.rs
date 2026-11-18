//! Severity: whether a finding blocks the check.

/// A lint level the profile assigns to a rule in a tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// Report, do not fail.
    Warn,
    /// Fail.
    Deny,
}

/// The severity of a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Reported; does not fail the check.
    Warning,
    /// Fails the check.
    Error,
}

/// Whether a violation at `level` fails the check.
#[must_use]
pub const fn is_blocking(level: Level) -> bool {
    match level {
        Level::Warn => false,
        Level::Deny => true,
    }
}

/// The severity of a finding whose rule has `level` in the crate's tier.
///
/// A rule with a level decides: a blocking level is an error, any other a
/// warning. Without a level (no rule, no tier, or a rule that does not apply
/// in the tier), `fallback` stands: the compiler's own severity for compiler
/// messages, `Error` for tauten's structural findings.
#[must_use]
pub const fn severity(level: Option<Level>, fallback: Severity) -> Severity {
    match level {
        None => fallback,
        Some(given) => {
            if is_blocking(given) {
                Severity::Error
            } else {
                Severity::Warning
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_level_decides_when_there_is_one() {
        assert_eq!(
            severity(Some(Level::Deny), Severity::Warning),
            Severity::Error
        );
        assert_eq!(
            severity(Some(Level::Warn), Severity::Error),
            Severity::Warning
        );
        assert_eq!(severity(None, Severity::Warning), Severity::Warning);
        assert_eq!(severity(None, Severity::Error), Severity::Error);
    }
}
