//! The rule table: parsing, validation and per-tier lint configuration.

use std::collections::BTreeMap;

use serde::Deserialize;
use tauten_check_kernel::table::{self as kernel, Invalid};

use crate::diag::EnforcedBy;
use crate::tier::{Level, Tier};

/// Rule IDs that tauten's own structural checks and ratchet emit. Every
/// profile must define them, or those checks would silently report nothing.
pub const EMITTED_BY_CODE: &[&str] = &[
    "TAU-A002", "TAU-K003", "TAU-X001", "TAU-R001", "TAU-S002", "TAU-T001", "TAU-T002", "TAU-T003",
    "TAU-X002", "TAU-X003", "TAU-X004", "TAU-R002", "TAU-H001", "TAU-K004",
];

/// The profile shipped with this version of tauten.
pub const EMBEDDED_RULES: &str = include_str!("../rules.toml");

/// The agent instructions shipped with this version of tauten (the fragment
/// adopting projects add to their AGENTS.md / CLAUDE.md).
pub const AGENT_PROFILE: &str = include_str!("../agents.md");

/// What the pinned translation toolchain accepts in kernel code
/// (`shapes.toml`, re-checked by `scripts/extract-fixtures`).
pub const SHAPES: &str = include_str!("../shapes.toml");

#[derive(Deserialize)]
struct Shapes {
    toolchain: String,
    #[serde(default)]
    limit: Vec<ShapeNote>,
    #[serde(default)]
    translates: Vec<ShapeNote>,
}

#[derive(Deserialize)]
struct ShapeNote {
    #[expect(dead_code, reason = "read by scripts/extract-fixtures, not here")]
    fixture: String,
    note: String,
}

/// The agent profile: [`AGENT_PROFILE`] plus the kernel section generated
/// from [`SHAPES`].
///
/// The embedded file is parsed by a unit test; were it malformed, the
/// profile would lack the kernel section rather than fail.
#[must_use]
pub fn agent_profile() -> String {
    let mut out = AGENT_PROFILE.to_owned();
    let Ok(shapes) = toml::from_str::<Shapes>(SHAPES) else {
        return out;
    };
    out.push_str("\n**Proved kernel code.** A kernel function with a proof must stay inside\n");
    out.push_str("what the pinned translation toolchain accepts (");
    out.push_str(&shapes.toolchain);
    out.push_str(").\n`cargo tauten proofs extract` decides; these notes are advice:\n");
    for n in &shapes.limit {
        out.push_str("- ");
        out.push_str(&n.note);
        out.push('\n');
    }
    out.push_str("Known to translate:\n");
    for n in &shapes.translates {
        out.push_str("- ");
        out.push_str(&n.note);
        out.push('\n');
    }
    out
}

/// Which lints a lint group may contain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupFamily {
    /// Every lint (`warnings`).
    Any,
    /// Every clippy lint.
    Clippy,
    /// Every rustc lint.
    Rustc,
}

/// Lint groups and the lints each may contain. An over-approximation: a
/// clippy group is taken to contain every clippy lint of the profile, a
/// rustc group every rustc lint, so an exception for a group is decided as
/// if it named each of them.
pub const LINT_GROUPS: &[(&str, GroupFamily)] = &[
    ("warnings", GroupFamily::Any),
    ("clippy::all", GroupFamily::Clippy),
    ("clippy::correctness", GroupFamily::Clippy),
    ("clippy::suspicious", GroupFamily::Clippy),
    ("clippy::style", GroupFamily::Clippy),
    ("clippy::complexity", GroupFamily::Clippy),
    ("clippy::perf", GroupFamily::Clippy),
    ("clippy::pedantic", GroupFamily::Clippy),
    ("clippy::restriction", GroupFamily::Clippy),
    ("clippy::nursery", GroupFamily::Clippy),
    ("clippy::cargo", GroupFamily::Clippy),
    ("unused", GroupFamily::Rustc),
    ("nonstandard_style", GroupFamily::Rustc),
    ("future_incompatible", GroupFamily::Rustc),
    ("let_underscore", GroupFamily::Rustc),
    ("rust_2018_idioms", GroupFamily::Rustc),
    ("rust_2018_compatibility", GroupFamily::Rustc),
    ("rust_2021_compatibility", GroupFamily::Rustc),
    ("rust_2024_compatibility", GroupFamily::Rustc),
    ("keyword_idents", GroupFamily::Rustc),
    ("refining_impl_trait", GroupFamily::Rustc),
    ("deprecated_safe", GroupFamily::Rustc),
];

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
    /// `safety`, `obligation` (an architectural or process duty) or
    /// `style`.
    #[serde(default)]
    pub family: Option<Family>,
    /// Exactly what the rule decides.
    #[serde(default)]
    pub invariant: Option<String>,
    /// What it reads: `source`, `resolved` (the compiler's view),
    /// `manifest`, `record` or `translation`.
    #[serde(default)]
    pub sees: Option<String>,
    /// What it does not see.
    #[serde(default)]
    pub dominion: Option<String>,
    /// It misses no violation among the inputs it sees. An incomplete rule
    /// never blocks (`tauten_check_kernel::rule::finding_severity`); a rule
    /// without this field is incomplete.
    #[serde(default)]
    pub complete: bool,
    /// Reject cases that exercise it: a `fixtures/cases` crate name, or a
    /// path to a fixture or test. A complete rule that blocks needs one.
    #[serde(default)]
    pub fixtures: Vec<String>,
}

/// What kind of guarantee a rule gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Family {
    /// Violations can make the program wrong, crash or unsound.
    Safety,
    /// An architectural or process duty (tiers, capabilities, records).
    Obligation,
    /// Readability.
    Style,
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
    /// A rule the checker emits itself is absent from the table.
    #[error("rule table lacks {0}, which the checker emits")]
    MissingRule(String),
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
    /// A complete rule that blocks names no reject fixture.
    #[error("rule {0} is complete and blocks but names no reject fixture")]
    Unfixtured(String),
    /// The generated clippy configuration could not be serialised.
    #[error("cannot render clippy configuration: {0}")]
    Render(#[from] toml::ser::Error),
}

/// Every complete rule that blocks names a reject fixture: the proved
/// [`tauten_check_kernel::rule::fixtures_ok`] decides; this names the
/// first offender.
fn fixtures(rules: &[Rule]) -> Result<(), ProfileError> {
    use tauten_check_kernel::rule::{RuleFacts, fixtures_ok};
    let facts: Vec<RuleFacts> = rules
        .iter()
        .map(|r| RuleFacts {
            complete: r.complete,
            blocks: r.levels.values().any(|l| l.is_blocking()),
            fixture: !r.fixtures.is_empty(),
        })
        .collect();
    if fixtures_ok(&facts) {
        return Ok(());
    }
    let offender = rules
        .iter()
        .zip(&facts)
        .find(|(_, f)| !fixtures_ok(core::slice::from_ref(*f)))
        .map_or_else(String::new, |(r, _)| r.id.clone());
    Err(ProfileError::Unfixtured(offender))
}

/// Names interned to indices for the kernel: one index per distinct string.
#[derive(Default)]
struct Names<'a>(BTreeMap<&'a str, usize>);

impl<'a> Names<'a> {
    fn index(&mut self, name: &'a str) -> usize {
        let next = self.0.len();
        *self.0.entry(name).or_insert(next)
    }
}

/// The text at `position` of `items`, or an empty string for a position
/// the kernel never returns.
fn text_at<T>(items: &[T], position: usize, text: impl Fn(&T) -> &str) -> String {
    items
        .get(position)
        .map_or_else(String::new, |item| text(item).to_owned())
}

/// Interns the table's rule IDs and lint names and asks the proved kernel
/// whether it is well formed; maps the first problem back to names.
fn validate(raw: &RawProfile) -> Result<(), ProfileError> {
    let mut ids = Names::default();
    let mut lint_names = Names::default();
    let rule_ids: Vec<usize> = raw.rule.iter().map(|rule| ids.index(&rule.id)).collect();
    let mut lint_rules = Vec::new();
    let mut lints = Vec::new();
    let mut lint_text = Vec::new();
    for (position, rule) in raw.rule.iter().enumerate() {
        for lint in &rule.lints {
            lint_rules.push(position);
            lints.push(lint_names.index(lint));
            lint_text.push(lint.as_str());
        }
    }
    let api_rules: Vec<usize> = raw.api.iter().map(|api| ids.index(&api.rule)).collect();
    let required: Vec<usize> = EMITTED_BY_CODE.iter().map(|id| ids.index(id)).collect();
    let rule_id = |position: usize| text_at(&raw.rule, position, |rule| &rule.id);
    let owner = |lint: usize| {
        lint_rules
            .get(lint)
            .map_or_else(String::new, |&r| rule_id(r))
    };
    match kernel::validate(&rule_ids, &lint_rules, &lints, &api_rules, &required) {
        Ok(()) => Ok(()),
        Err(Invalid::DuplicateId(rule)) => Err(ProfileError::DuplicateId(rule_id(rule))),
        Err(Invalid::DuplicateLint(first, second)) => Err(ProfileError::DuplicateLint {
            lint: text_at(&lint_text, second, |lint| lint),
            first: owner(first),
            second: owner(second),
        }),
        Err(Invalid::UnknownRule(api)) => {
            Err(ProfileError::UnknownRule(text_at(&raw.api, api, |ban| {
                &ban.rule
            })))
        }
        Err(Invalid::MissingRule(id)) => Err(ProfileError::MissingRule(text_at(
            EMITTED_BY_CODE,
            id,
            |id| id,
        ))),
    }
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
        // The well-formedness decision is the proved kernel's
        // (`tauten_check_kernel::table::validate`, `validate_ok_iff`).
        validate(&raw)?;
        fixtures(&raw.rule)?;
        let mut lint_owner = BTreeMap::new();
        for (index, rule) in raw.rule.iter().enumerate() {
            for lint in &rule.lints {
                lint_owner.insert(lint.clone(), index);
            }
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

    /// Every rule an exception or manifest entry for `lint` may cover: the
    /// owning rule for a profile lint; for a lint group, every rule that may
    /// own one of its members ([`LINT_GROUPS`]); nothing otherwise.
    #[must_use]
    pub fn covered_by(&self, lint: &str) -> Vec<&Rule> {
        if let Some(rule) = self.rule_for_lint(lint) {
            return vec![rule];
        }
        let Some(&(_, family)) = LINT_GROUPS.iter().find(|(name, _)| *name == lint) else {
            return Vec::new();
        };
        let member = |l: &String| match family {
            GroupFamily::Any => true,
            GroupFamily::Clippy => l.starts_with("clippy::"),
            GroupFamily::Rustc => !l.starts_with("clippy::"),
        };
        self.rules
            .iter()
            .filter(|rule| rule.lints.iter().any(member))
            .collect()
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
mod reference;

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

    #[test]
    fn rejects_the_fixture_without_r001() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/profiles/rej-missing-r001.toml");
        let text = std::fs::read_to_string(path).expect("fixture");
        let error = Profile::parse(&text).expect_err("a table without TAU-R001 is rejected");
        assert_eq!(
            error.to_string(),
            "rule table lacks TAU-R001, which the checker emits"
        );
        let with_r001 = format!(
            "{text}\n[[rule]]\nid = \"TAU-R001\"\ntitle = \"t\"\nlints = []\n\
             levels = {{ core = \"deny\" }}\nenforced_by = \"tauten\"\nwhy = \"w\"\nfix = \"f\"\n"
        );
        assert!(
            Profile::parse(&with_r001).is_ok(),
            "control: the fixture plus TAU-R001 is valid"
        );
    }

    #[test]
    fn rejects_a_table_missing_a_rule_the_checker_emits() {
        let start = EMBEDDED_RULES
            .find("id = \"TAU-R001\"")
            .expect("R001 in table");
        let block = EMBEDDED_RULES[..start]
            .rfind("[[rule]]")
            .expect("block start");
        let end = EMBEDDED_RULES[start..]
            .find("[[")
            .map_or(EMBEDDED_RULES.len(), |e| start + e);
        let text = format!("{}{}", &EMBEDDED_RULES[..block], &EMBEDDED_RULES[end..]);
        assert!(matches!(
            Profile::parse(&text),
            Err(ProfileError::MissingRule(id)) if id == "TAU-R001"
        ));
    }

    #[test]
    fn the_agent_profile_lists_every_shape_note() {
        let shapes: Shapes = toml::from_str(SHAPES).expect("shapes.toml parses");
        let profile = agent_profile();
        assert!(profile.contains("cargo tauten proofs extract` decides"));
        assert!(profile.contains(&shapes.toolchain));
        for n in shapes.limit.iter().chain(&shapes.translates) {
            assert!(profile.contains(&n.note), "{}", n.note);
        }
    }

    /// A2: every rule states what it decides and sees; every listed
    /// fixture exists and, for a `fixtures/cases` crate, expects the rule.
    #[test]
    fn every_rule_carries_its_metadata_and_fixtures() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let profile = Profile::embedded().expect("profile");
        let incomplete: Vec<&str> = profile
            .rules()
            .iter()
            .filter(|r| !r.complete)
            .map(|r| r.id.as_str())
            .collect();
        assert_eq!(incomplete, ["TAU-S002"]);
        for rule in profile.rules() {
            assert!(rule.family.is_some(), "{} family", rule.id);
            assert!(rule.invariant.is_some(), "{} invariant", rule.id);
            assert!(
                matches!(
                    rule.sees.as_deref(),
                    Some("source" | "resolved" | "manifest" | "record" | "translation")
                ),
                "{} sees",
                rule.id
            );
            assert!(rule.dominion.is_some(), "{} dominion", rule.id);
            for fixture in &rule.fixtures {
                if fixture.contains('/') {
                    assert!(root.join(fixture).exists(), "{}: {fixture}", rule.id);
                    continue;
                }
                let manifest = root.join("fixtures/cases").join(fixture).join("Cargo.toml");
                let text = std::fs::read_to_string(&manifest).expect("fixture manifest");
                let table: toml::Table = text.parse().expect("toml");
                let expect: Vec<&str> = table["package"]["metadata"]["fixture"]["expect"]
                    .as_array()
                    .expect("expect")
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .collect();
                // TAU-A000 is reported under the specific rule a ban names.
                let wanted = if rule.id == "TAU-A000" {
                    "TAU-A001"
                } else {
                    rule.id.as_str()
                };
                assert!(
                    expect.contains(&wanted),
                    "{}: {fixture} expects {expect:?}",
                    rule.id
                );
            }
        }
    }

    #[test]
    fn a_complete_blocking_rule_needs_a_fixture() {
        let text = EMBEDDED_RULES.replacen(r#"fixtures = ["rej-unwrap"]"#, "fixtures = []", 1);
        assert_ne!(text, EMBEDDED_RULES);
        assert!(matches!(
            Profile::parse(&text),
            Err(ProfileError::Unfixtured(id)) if id == "TAU-P001"
        ));
    }
}
