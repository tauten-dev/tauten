//! Structured diagnostics and the mapping from compiler messages to rules.

use serde::{Deserialize, Serialize};

use crate::profile::{Profile, Rule};
use crate::tier::Tier;

/// What enforces a rule. Shown with every diagnostic so claims stay honest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnforcedBy {
    /// The Rust compiler.
    Rustc,
    /// A clippy lint at a profile-controlled level.
    Clippy,
    /// A tauten structural check.
    Tauten,
    /// A proof checked by the proofs gate.
    Proof,
    /// Documentation and review only.
    Convention,
}

/// Diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Reported; does not fail the check.
    Warning,
    /// Fails the check.
    Error,
}

/// One finding, in the stable schema printed by `--format json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Rule ID (`TAU-…`), or `rustc` / `clippy::<lint>` for findings outside the profile.
    pub id: String,
    /// Whether this fails the check.
    pub severity: Severity,
    /// The crate the finding is in.
    #[serde(rename = "crate")]
    pub krate: String,
    /// The crate's tier, if declared.
    pub tier: Option<Tier>,
    /// Path relative to the workspace root, when known.
    pub file: Option<String>,
    /// 1-based line.
    pub line: Option<u32>,
    /// 1-based column.
    pub column: Option<u32>,
    /// What was found.
    pub message: String,
    /// The rule's title.
    pub rule: Option<String>,
    /// Why the rule exists.
    pub why: Option<String>,
    /// How to repair it.
    pub fix: Option<String>,
    /// What enforces the rule.
    pub enforced_by: EnforcedBy,
    /// The underlying lint or check name.
    pub source: String,
}

impl Diagnostic {
    /// Build a diagnostic attributed to `rule`.
    #[must_use]
    pub fn for_rule(
        rule: &Rule,
        severity: Severity,
        krate: &str,
        tier: Option<Tier>,
        message: String,
    ) -> Self {
        Self {
            id: rule.id.clone(),
            severity,
            krate: krate.to_owned(),
            tier,
            file: None,
            line: None,
            column: None,
            message,
            rule: Some(rule.title.clone()),
            why: Some(rule.why.clone()),
            fix: Some(rule.fix.clone()),
            enforced_by: rule.enforced_by,
            source: "tauten".to_owned(),
        }
    }

    /// Attach a location.
    #[must_use]
    pub fn at(mut self, file: Option<String>, line: Option<u32>, column: Option<u32>) -> Self {
        self.file = file;
        self.line = line;
        self.column = column;
        self
    }
}

/// The parts of a rustc/clippy JSON message that tauten uses.
#[derive(Debug, Clone, Default)]
pub struct CompilerMessage {
    /// `error`, `warning`, `note`, `failure-note`, …
    pub level: String,
    /// Lint or error code, e.g. `clippy::unwrap_used` or `E0308`.
    pub code: Option<String>,
    /// Primary message.
    pub message: String,
    /// Messages of child diagnostics (notes, help).
    pub notes: Vec<String>,
    /// Primary span file.
    pub file: Option<String>,
    /// Primary span line.
    pub line: Option<u32>,
    /// Primary span column.
    pub column: Option<u32>,
}

const DISALLOWED: [&str; 3] = [
    "clippy::disallowed_methods",
    "clippy::disallowed_types",
    "clippy::disallowed_macros",
];

/// Map a compiler message to a diagnostic. Returns `None` for messages that
/// are not findings (summaries, failure notes).
#[must_use]
pub fn from_compiler(
    profile: &Profile,
    krate: &str,
    tier: Option<Tier>,
    msg: &CompilerMessage,
) -> Option<Diagnostic> {
    let severity = match msg.level.as_str() {
        "error" => Severity::Error,
        "warning" => Severity::Warning,
        _ => return None,
    };
    if msg.code.is_none() && is_summary(&msg.message) {
        return None;
    }
    let code = msg.code.clone();
    let rule = code
        .as_deref()
        .and_then(|c| rule_for(profile, c, &msg.notes));
    let diag = match rule {
        Some(rule) => {
            // Lints are compiled at warn level; the profile decides severity.
            let severity = tier.and_then(|t| rule.level(t)).map_or(severity, |level| {
                if level.is_blocking() {
                    Severity::Error
                } else {
                    Severity::Warning
                }
            });
            let mut d = Diagnostic::for_rule(rule, severity, krate, tier, msg.message.clone());
            d.source = code.unwrap_or_default();
            d
        }
        None => Diagnostic {
            id: code.clone().unwrap_or_else(|| "rustc".to_owned()),
            severity,
            krate: krate.to_owned(),
            tier,
            file: None,
            line: None,
            column: None,
            message: msg.message.clone(),
            rule: None,
            why: None,
            fix: None,
            enforced_by: EnforcedBy::Rustc,
            source: code.unwrap_or_else(|| "rustc".to_owned()),
        },
    };
    Some(diag.at(msg.file.clone(), msg.line, msg.column))
}

fn rule_for<'p>(profile: &'p Profile, code: &str, notes: &[String]) -> Option<&'p Rule> {
    if DISALLOWED.contains(&code) {
        let named = notes
            .iter()
            .find_map(|note| note.split(':').next().filter(|id| id.starts_with("TAU-")))
            .and_then(|id| profile.rule(id));
        if named.is_some() {
            return named;
        }
    }
    profile.rule_for_lint(code)
}

fn is_summary(message: &str) -> bool {
    message.starts_with("aborting due to")
        || message.contains("warnings emitted")
        || message.contains("warning emitted")
        || message.starts_with("Some errors have detailed explanations")
        || message.starts_with("For more information about")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(level: &str, code: &str, notes: &[&str]) -> CompilerMessage {
        CompilerMessage {
            level: level.to_owned(),
            code: Some(code.to_owned()),
            message: "m".to_owned(),
            notes: notes.iter().map(|n| (*n).to_owned()).collect(),
            file: Some("src/lib.rs".to_owned()),
            line: Some(3),
            column: Some(5),
        }
    }

    #[test]
    fn maps_lint_to_rule() {
        let profile = Profile::embedded().expect("profile");
        let d = from_compiler(
            &profile,
            "k",
            Some(Tier::Core),
            &msg("error", "clippy::unwrap_used", &[]),
        )
        .expect("finding");
        assert_eq!(d.id, "TAU-P001");
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(d.line, Some(3));
    }

    #[test]
    fn profile_decides_severity() {
        let profile = Profile::embedded().expect("profile");
        let warn = msg("warning", "clippy::indexing_slicing", &[]);
        let core = from_compiler(&profile, "k", Some(Tier::Core), &warn).expect("finding");
        assert_eq!(core.severity, Severity::Error);
        let edge = from_compiler(&profile, "k", Some(Tier::Edge), &warn).expect("finding");
        assert_eq!(edge.severity, Severity::Warning);
    }

    #[test]
    fn disallowed_uses_reason_rule() {
        let profile = Profile::embedded().expect("profile");
        let m = msg(
            "error",
            "clippy::disallowed_methods",
            &["TAU-A001: ambient clock"],
        );
        let d = from_compiler(&profile, "k", Some(Tier::Core), &m).expect("finding");
        assert_eq!(d.id, "TAU-A001");
        let m = msg("error", "clippy::disallowed_types", &["unrelated"]);
        let d = from_compiler(&profile, "k", Some(Tier::Core), &m).expect("finding");
        assert_eq!(d.id, "TAU-A000");
    }

    #[test]
    fn unknown_codes_pass_through() {
        let profile = Profile::embedded().expect("profile");
        let d = from_compiler(&profile, "k", None, &msg("error", "E0308", &[])).expect("finding");
        assert_eq!(d.id, "E0308");
        assert!(d.rule.is_none());
    }

    #[test]
    fn only_uncoded_summaries_are_dropped() {
        let profile = Profile::embedded().expect("profile");
        for text in [
            "aborting due to 2 previous errors",
            "3 warnings emitted",
            "1 warning emitted",
            "Some errors have detailed explanations: E0308.",
            "For more information about this error, try `rustc --explain E0308`.",
        ] {
            let m = CompilerMessage {
                level: "error".to_owned(),
                message: text.to_owned(),
                ..CompilerMessage::default()
            };
            assert!(from_compiler(&profile, "k", None, &m).is_none(), "{text}");
        }
        let real = CompilerMessage {
            level: "error".to_owned(),
            message: "expected `;`".to_owned(),
            ..CompilerMessage::default()
        };
        assert_eq!(
            from_compiler(&profile, "k", None, &real).map(|d| d.id),
            Some("rustc".to_owned())
        );
        let coded = CompilerMessage {
            level: "warning".to_owned(),
            code: Some("clippy::unwrap_used".to_owned()),
            message: "aborting due to a coded lint".to_owned(),
            ..CompilerMessage::default()
        };
        assert!(from_compiler(&profile, "k", Some(Tier::Core), &coded).is_some());
    }

    #[test]
    fn summaries_are_dropped() {
        let profile = Profile::embedded().expect("profile");
        let m = CompilerMessage {
            level: "error".to_owned(),
            message: "aborting due to 2 previous errors".to_owned(),
            ..CompilerMessage::default()
        };
        assert!(from_compiler(&profile, "k", None, &m).is_none());
        assert!(from_compiler(&profile, "k", None, &msg("failure-note", "E1", &[])).is_none());
    }
}
