//! The rule table: parsing, validation and per-tier lint configuration.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use crate::diag::EnforcedBy;
use crate::tier::{Level, Tier};

/// The profile shipped with this version of tauten.
pub const EMBEDDED_RULES: &str = include_str!("../rules.toml");

/// One rule of the profile.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    /// Stable identifier, for example `TAU-P001`.
    pub id: String,
    /// One-line summary.
    pub title: String,
    /// rustc/clippy lint names owned by this rule.
    pub lints: Vec<String>,
    /// Level per tier; tiers not listed leave the lints at their default.
    pub levels: BTreeMap<Tier, Level>,
    /// What enforces the rule.
    pub enforced_by: EnforcedBy,
    /// Why the rule exists.
    pub why: String,
    /// How to repair a violation.
    pub fix: String,
    /// Whether `#[expect]`/`#[allow]` may excuse this rule's lints.
    #[serde(default = "default_true")]
    pub exceptions: bool,
    /// Whether the rule applies when compiling test targets.
    #[serde(default = "default_true")]
    pub in_tests: bool,
}

const fn default_true() -> bool {
    true
}

impl Rule {
    /// The level of this rule in `tier`, if it applies there.
    #[must_use]
    pub fn level(&self, tier: Tier) -> Option<Level> {
        self.levels.get(&tier).copied()
    }
}

/// Which `disallowed-*` clippy list an API ban belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiKind {
    /// `disallowed-methods` (functions and methods).
    Method,
    /// `disallowed-types`.
    Type,
    /// `disallowed-macros`.
    Macro,
}

impl ApiKind {
    const fn clippy_key(self) -> &'static str {
        match self {
            Self::Method => "disallowed-methods",
            Self::Type => "disallowed-types",
            Self::Macro => "disallowed-macros",
        }
    }
}

/// A group of banned API paths attributed to one rule.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiBan {
    /// The rule this ban reports under.
    pub rule: String,
    /// Tiers where the paths are banned.
    pub tiers: Vec<Tier>,
    /// Which clippy list the paths go into.
    pub kind: ApiKind,
    /// Short explanation appended to the rule ID in the diagnostic.
    pub note: String,
    /// Fully qualified paths.
    pub paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProfile {
    version: u32,
    rule: Vec<Rule>,
    api: Vec<ApiBan>,
    clippy_conf: toml::Table,
}

/// A validated profile.
#[derive(Debug, Clone)]
pub struct Profile {
    rules: Vec<Rule>,
    apis: Vec<ApiBan>,
    clippy_conf: toml::Table,
    lint_owner: BTreeMap<String, usize>,
}

/// Why a rule table was rejected.
#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    /// The TOML did not match the schema.
    #[error("rule table is not valid: {0}")]
    Parse(#[from] toml::de::Error),
    /// The table declares a version this build does not understand.
    #[error("unsupported rule table version {0}")]
    Version(u32),
    /// Two rules share an ID.
    #[error("duplicate rule id {0}")]
    DuplicateId(String),
    /// A lint is owned by more than one rule.
    #[error("lint {lint} is owned by both {first} and {second}")]
    DuplicateLint {
        /// The lint name.
        lint: String,
        /// The first owning rule.
        first: String,
        /// The second owning rule.
        second: String,
    },
    /// An API ban references a rule that does not exist.
    #[error("api ban references unknown rule {0}")]
    UnknownRule(String),
    /// The generated clippy configuration could not be serialised.
    #[error("cannot render clippy configuration: {0}")]
    Render(#[from] toml::ser::Error),
}

impl Profile {
    /// Load the profile embedded in this crate.
    ///
    /// # Errors
    /// Returns an error only if the embedded table is malformed, which the
    /// crate's tests rule out.
    pub fn embedded() -> Result<Self, ProfileError> {
        Self::parse(EMBEDDED_RULES)
    }

    /// Parse and validate a rule table.
    ///
    /// # Errors
    /// Returns [`ProfileError`] if the table is malformed or inconsistent.
    pub fn parse(text: &str) -> Result<Self, ProfileError> {
        let raw: RawProfile = toml::from_str(text)?;
        if raw.version != 1 {
            return Err(ProfileError::Version(raw.version));
        }
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
        Ok(Self {
            rules: raw.rule,
            apis: raw.api,
            clippy_conf: raw.clippy_conf,
            lint_owner,
        })
    }

    /// All rules, in table order.
    #[must_use]
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// All API bans, in table order.
    #[must_use]
    pub fn api_bans(&self) -> &[ApiBan] {
        &self.apis
    }

    /// Look a rule up by ID (case-insensitive).
    #[must_use]
    pub fn rule(&self, id: &str) -> Option<&Rule> {
        self.rules.iter().find(|r| r.id.eq_ignore_ascii_case(id))
    }

    /// The rule that owns a lint name such as `clippy::unwrap_used`.
    #[must_use]
    pub fn rule_for_lint(&self, lint: &str) -> Option<&Rule> {
        self.lint_owner
            .get(lint)
            .and_then(|&index| self.rules.get(index))
    }

    /// The level the profile assigns to `lint` in `tier`, if any.
    #[must_use]
    pub fn lint_level(&self, lint: &str, tier: Tier) -> Option<Level> {
        self.rule_for_lint(lint).and_then(|rule| rule.level(tier))
    }

    /// Lint-level flags to pass after `cargo clippy --` for crates of `tier`.
    #[must_use]
    pub fn lint_flags(&self, tier: Tier) -> Vec<String> {
        let mut flags = Vec::new();
        for rule in &self.rules {
            let Some(level) = rule.level(tier) else {
                continue;
            };
            for lint in &rule.lints {
                flags.push(level.flag().to_owned());
                flags.push(lint.clone());
            }
        }
        flags
    }

    /// Flags that make every profile lint of `tier` report at warn level.
    ///
    /// `cargo tauten check` compiles with these so that a violation in one
    /// crate never stops its dependents from being checked; severity is then
    /// taken from the profile (see [`crate::diag::from_compiler`]).
    #[must_use]
    pub fn report_flags(&self, tier: Tier, test: bool) -> Vec<String> {
        let mut flags = Vec::new();
        let applies = |rule: &&Rule| rule.level(tier).is_some() && (rule.in_tests || !test);
        for rule in self.rules.iter().filter(applies) {
            for lint in &rule.lints {
                flags.push("-W".to_owned());
                flags.push(lint.clone());
            }
        }
        flags
    }

    /// The `clippy.toml` contents for crates of `tier` (for test targets when
    /// `test` is set).
    ///
    /// # Errors
    /// Returns [`ProfileError::Render`] if serialisation fails.
    pub fn clippy_toml(&self, tier: Tier, test: bool) -> Result<String, ProfileError> {
        let mut table = self.clippy_conf.clone();
        for kind in [ApiKind::Method, ApiKind::Type, ApiKind::Macro] {
            let mut entries = Vec::new();
            let applies = |api: &&ApiBan| {
                api.kind == kind
                    && api.tiers.contains(&tier)
                    && (!test || self.rule(&api.rule).is_none_or(|r| r.in_tests))
            };
            for api in self.apis.iter().filter(applies) {
                for path in &api.paths {
                    let mut entry = toml::Table::new();
                    entry.insert("path".to_owned(), toml::Value::String(path.clone()));
                    entry.insert(
                        "reason".to_owned(),
                        toml::Value::String(format!("{}: {}", api.rule, api.note)),
                    );
                    entry.insert("allow-invalid".to_owned(), toml::Value::Boolean(true));
                    entries.push(toml::Value::Table(entry));
                }
            }
            if !entries.is_empty() {
                table.insert(kind.clippy_key().to_owned(), toml::Value::Array(entries));
            }
        }
        Ok(toml::to_string(&table)?)
    }

    /// Lint names the profile makes blocking in `tier`.
    pub fn blocking_lints(&self, tier: Tier) -> impl Iterator<Item = &str> {
        self.rules
            .iter()
            .filter(move |rule| rule.level(tier).is_some_and(Level::is_blocking))
            .flat_map(|rule| rule.lints.iter().map(String::as_str))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_profile_is_valid() {
        let profile = Profile::embedded().expect("embedded profile parses");
        assert!(profile.rules().len() > 20);
        assert!(profile.rule("tau-p001").is_some());
    }

    #[test]
    fn every_rule_has_text_and_a_tier() {
        let profile = Profile::embedded().expect("embedded profile parses");
        for rule in profile.rules() {
            assert!(rule.id.starts_with("TAU-"), "{}", rule.id);
            assert!(!rule.why.is_empty() && !rule.fix.is_empty(), "{}", rule.id);
            assert!(!rule.levels.is_empty(), "{} applies to no tier", rule.id);
        }
    }

    #[test]
    fn kernel_is_strictest() {
        let profile = Profile::embedded().expect("embedded profile parses");
        for rule in profile.rules() {
            for tier in [Tier::Core, Tier::Edge] {
                if let (Some(outer), kernel) = (rule.level(tier), rule.level(Tier::Kernel)) {
                    let rule_applies_in_kernel = rule.id != "TAU-U002";
                    if rule_applies_in_kernel {
                        assert!(
                            kernel >= Some(outer),
                            "{} is looser in kernel than {tier}",
                            rule.id
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn flags_follow_levels() {
        let profile = Profile::embedded().expect("embedded profile parses");
        let flags = profile.lint_flags(Tier::Kernel);
        let pos = flags
            .iter()
            .position(|f| f == "clippy::unwrap_used")
            .expect("present");
        assert_eq!(
            flags.get(pos.wrapping_sub(1)).map(String::as_str),
            Some("-D")
        );
        assert!(flags.windows(2).any(|w| w == ["-D", "unsafe_code"]));
        let edge = profile.lint_flags(Tier::Edge);
        assert!(!edge.iter().any(|f| f == "clippy::disallowed_methods"));
    }

    #[test]
    fn report_flags_cover_the_same_lints_at_warn() {
        let profile = Profile::embedded().expect("embedded profile parses");
        for tier in Tier::ALL {
            let levels = profile.lint_flags(tier);
            let report = profile.report_flags(tier, false);
            assert_eq!(levels.len(), report.len());
            let lints = |f: &[String]| f.iter().skip(1).step_by(2).cloned().collect::<Vec<_>>();
            assert_eq!(lints(&levels), lints(&report));
            assert!(report.iter().step_by(2).all(|f| f == "-W"));
        }
    }

    #[test]
    fn clippy_toml_scopes_bans_by_tier() {
        let profile = Profile::embedded().expect("embedded profile parses");
        let kernel = profile.clippy_toml(Tier::Kernel, false).expect("renders");
        let core = profile.clippy_toml(Tier::Core, false).expect("renders");
        let edge = profile.clippy_toml(Tier::Edge, false).expect("renders");
        assert!(kernel.contains("std::collections::HashMap"));
        assert!(!core.contains("std::collections::HashMap"));
        assert!(core.contains("TAU-A001: ambient clock"));
        assert!(!edge.contains("disallowed-methods"));
        assert!(edge.contains("allow-unwrap-in-tests"));
        let core_tests = profile.clippy_toml(Tier::Core, true).expect("renders");
        assert!(!core_tests.contains("TAU-A001"));
        let core_test_flags = profile.report_flags(Tier::Core, true);
        assert!(core_test_flags.iter().any(|f| f == "unused_must_use"));
        assert!(
            !core_test_flags
                .iter()
                .any(|f| f == "clippy::let_underscore_must_use")
        );
        assert!(
            !core_test_flags
                .iter()
                .any(|f| f == "clippy::panic_in_result_fn")
        );
    }

    #[test]
    fn api_bans_go_to_the_right_list() {
        let profile = Profile::embedded().expect("embedded profile parses");
        assert!(profile.api_bans().iter().any(|a| a.rule == "TAU-A001"));
        let kernel: toml::Table = profile
            .clippy_toml(Tier::Kernel, false)
            .expect("renders")
            .parse()
            .expect("valid toml");
        let paths = |key: &str| -> Vec<String> {
            kernel
                .get(key)
                .and_then(toml::Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|e| e.get("path")?.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default()
        };
        assert!(paths("disallowed-types").contains(&"std::collections::HashMap".to_owned()));
        assert!(!paths("disallowed-methods").contains(&"std::collections::HashMap".to_owned()));
        assert!(paths("disallowed-methods").contains(&"std::fs::read".to_owned()));
        assert!(paths("disallowed-macros").contains(&"std::thread_local".to_owned()));
    }

    #[test]
    fn rejects_duplicate_lint_owners() {
        let text = r#"
version = 1
api = []
[clippy_conf]
[[rule]]
id = "TAU-1"
title = "a"
lints = ["x"]
levels = { core = "deny" }
enforced_by = "clippy"
why = "w"
fix = "f"
[[rule]]
id = "TAU-2"
title = "b"
lints = ["x"]
levels = { core = "deny" }
enforced_by = "clippy"
why = "w"
fix = "f"
"#;
        assert!(matches!(
            Profile::parse(text),
            Err(ProfileError::DuplicateLint { .. })
        ));
    }
}
