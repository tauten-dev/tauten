//! Structural checks over a workspace model built by the caller.

use std::collections::BTreeMap;

use tauten_check_kernel::effects::{self as kernel_effects, Verdict};
use tauten_check_kernel::exception::{self, Excuse};
use tauten_check_kernel::layout;
use tauten_check_kernel::severity::Level as KernelLevel;

use crate::diag::{Diagnostic, Severity};
use crate::effects;
use crate::profile::{Profile, Rule};
use crate::scan;
use crate::tier::{Level, Tier};

/// A source file of a member crate.
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// Path relative to the workspace root.
    pub path: String,
    /// File contents.
    pub text: String,
    /// Whether this is a crate root (`lib.rs`, `main.rs`, `bin/*.rs`).
    pub is_crate_root: bool,
    /// Which part of the package the file belongs to.
    pub kind: SourceKind,
}

/// Which part of a package a source file belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    /// Under `src/`: the library and binaries.
    Src,
    /// Under `tests/` or `benches/`: compiled only as test targets.
    Test,
    /// Under `examples/`.
    Example,
    /// The build script.
    Build,
}

impl SourceKind {
    /// Whether the file is compiled only as a test target.
    #[must_use]
    pub const fn is_test(self) -> bool {
        matches!(self, Self::Test)
    }
}

/// A lint level set in a Cargo manifest `[lints]` table.
#[derive(Debug, Clone)]
pub struct ManifestLint {
    /// Lint name including tool prefix, e.g. `clippy::unwrap_used`.
    pub name: String,
    /// Level as written (`allow`, `warn`, `deny`, `forbid`, `expect`).
    pub level: String,
}

/// A workspace member as seen by the structural checks.
#[derive(Debug, Clone)]
pub struct Member {
    /// Package name.
    pub name: String,
    /// The raw `tier` metadata value, if present.
    pub tier_raw: Option<String>,
    /// Manifest path relative to the workspace root.
    pub manifest: String,
    /// Normal (non-dev, non-build) dependencies on other workspace members.
    pub workspace_deps: Vec<String>,
    /// Normal dependencies outside the workspace.
    pub external_deps: Vec<String>,
    /// `[package.metadata.tauten] kernel-deps`.
    pub kernel_dep_allowlist: Vec<String>,
    /// Effective `[lints]` entries for this package.
    pub manifest_lints: Vec<ManifestLint>,
    /// Source files under the package's `src/`.
    pub sources: Vec<SourceFile>,
    /// `[package.metadata.tauten] effects = "required"`: every `pub fn`
    /// declares `#[effects(..)]`, and mismatches are errors (TAU-F001).
    pub effects_required: bool,
    /// `[package.metadata.tauten] layer`, if present (TAU-T004).
    pub layer_raw: Option<String>,
}

impl Member {
    /// The declared tier, if valid.
    #[must_use]
    pub fn tier(&self) -> Option<Tier> {
        self.tier_raw.as_deref().and_then(Tier::parse)
    }
}

/// The workspace model.
#[derive(Debug, Clone, Default)]
pub struct Workspace {
    /// Members being checked.
    pub members: Vec<Member>,
    /// Whether release builds check integer overflow (TAU-H001), as read
    /// from the root manifest and `.cargo/config.toml`
    /// ([`crate::manifest::overflow_checks`]). `None` when the caller did
    /// not read them; the check is then skipped.
    pub overflow_checks: Option<tauten_check_kernel::manifest::Fact>,
    /// `[workspace.metadata.tauten] layers`, in order, if the workspace
    /// declares layers (TAU-T004).
    pub layers: Option<Vec<String>>,
}

/// Run every structural check.
#[must_use]
pub fn check(profile: &Profile, workspace: &Workspace) -> Vec<Diagnostic> {
    let tiers: BTreeMap<&str, Option<Tier>> = workspace
        .members
        .iter()
        .map(|m| (m.name.as_str(), m.tier()))
        .collect();
    let mut out = Vec::new();
    for member in &workspace.members {
        let mut ctx = Ctx {
            profile,
            member,
            out: &mut out,
        };
        ctx.tier_declared();
        let Some(tier) = member.tier() else {
            continue;
        };
        ctx.dependency_direction(tier, &tiers);
        ctx.kernel_deps(tier);
        ctx.manifest_lints(tier);
        ctx.sources(tier);
        ctx.effects(tier);
    }
    out.extend(hardening(profile, workspace));
    out.extend(layers(profile, workspace));
    out
}

/// TAU-T004: declared layers, decided by the proved
/// [`tauten_check_kernel::layer::finding`]. A crate's own declaration is
/// checked as an edge to itself (unknown or missing is a finding; a known
/// layer may use itself); every dependency between members is an edge.
fn layers(profile: &Profile, workspace: &Workspace) -> Vec<Diagnostic> {
    use tauten_check_kernel::layer::{self, Layer};
    let Some(order) = &workspace.layers else {
        return Vec::new();
    };
    let Some(rule) = profile.rule("TAU-T004") else {
        return Vec::new();
    };
    let of = |m: &Member| match &m.layer_raw {
        None => Layer::Missing,
        Some(name) => order
            .iter()
            .position(|l| l == name)
            .map_or(Layer::Unknown, Layer::Known),
    };
    let mut out = Vec::new();
    for member in &workspace.members {
        let tier = member.tier();
        let mine = of(member);
        let emit = |message: String| {
            Diagnostic::for_rule(
                rule,
                Severity::of_rule(rule, tier.and_then(|t| rule.level(t)), Severity::Error),
                &member.name,
                tier,
                message,
            )
            .at(Some(member.manifest.clone()), None, None)
        };
        if layer::finding(mine, mine) {
            out.push(emit(match &member.layer_raw {
                None => format!(
                    "crate `{}` declares no layer; the workspace declares layers {}",
                    member.name,
                    order.join(", ")
                ),
                Some(name) => format!(
                    "crate `{}` declares layer `{name}`, which is not one of {}",
                    member.name,
                    order.join(", ")
                ),
            }));
            continue;
        }
        for dep in &member.workspace_deps {
            let Some(target) = workspace.members.iter().find(|m| &m.name == dep) else {
                continue;
            };
            let theirs = of(target);
            if matches!(theirs, Layer::Known(_)) && layer::finding(mine, theirs) {
                out.push(emit(format!(
                    "crate `{}` (layer `{}`) depends on `{dep}` (layer `{}`), a later layer",
                    member.name,
                    member.layer_raw.as_deref().unwrap_or_default(),
                    target.layer_raw.as_deref().unwrap_or_default()
                )));
            }
        }
    }
    out
}

/// TAU-H001: release builds must check integer overflow. Required when the
/// rule has a level in some member's tier; the decision is the proved
/// [`tauten_check_kernel::manifest::hardening`].
fn hardening(profile: &Profile, workspace: &Workspace) -> Option<Diagnostic> {
    use tauten_check_kernel::manifest::{self, Fact};
    let fact = workspace.overflow_checks?;
    let rule = profile.rule("TAU-H001")?;
    let levels: Vec<Level> = workspace
        .members
        .iter()
        .filter_map(|m| m.tier().and_then(|t| rule.level(t)))
        .collect();
    let level = levels
        .iter()
        .copied()
        .find(|l| l.is_blocking())
        .or_else(|| levels.first().copied());
    if !manifest::hardening(level.is_some(), fact) {
        return None;
    }
    let message = match fact {
        Fact::Unparsed => "`overflow-checks` in `[profile.release]` (Cargo.toml or .cargo/config.toml) is not a boolean tauten can read".to_owned(),
        Fact::No | Fact::Yes => "release builds do not check integer overflow: set `overflow-checks = true` under `[profile.release]` in the workspace Cargo.toml, with no override setting it to false".to_owned(),
    };
    let severity = Severity::of_rule(rule, level, Severity::Error);
    Some(
        Diagnostic::for_rule(rule, severity, "workspace", None, message).at(
            Some("Cargo.toml".to_owned()),
            None,
            None,
        ),
    )
}

struct Ctx<'a> {
    profile: &'a Profile,
    member: &'a Member,
    out: &'a mut Vec<Diagnostic>,
}

impl Ctx<'_> {
    fn emit(
        &mut self,
        id: &str,
        tier: Option<Tier>,
        message: String,
        at: Option<(&str, scan::Hit)>,
    ) {
        let Some(rule) = self.profile.rule(id) else {
            return;
        };
        // Structural findings without a level in the tier block.
        let severity = Severity::of_rule(rule, tier.and_then(|t| rule.level(t)), Severity::Error);
        let mut diag = Diagnostic::for_rule(rule, severity, &self.member.name, tier, message);
        diag = match at {
            Some((file, hit)) => diag.at(Some(file.to_owned()), Some(hit.line), Some(hit.column)),
            None => diag.at(Some(self.member.manifest.clone()), None, None),
        };
        self.out.push(diag);
    }

    /// TAU-T001, decided by the proved [`layout::t001`].
    fn tier_declared(&mut self) {
        let raw = self.member.tier_raw.as_deref();
        let declared = match raw.map(Tier::parse) {
            None => layout::Declared::Missing,
            Some(None) => layout::Declared::Unknown,
            Some(Some(tier)) => layout::Declared::Known(tier.kernel()),
        };
        if !layout::t001(declared) {
            return;
        }
        let message = match raw {
            None => format!("crate `{}` declares no tier", self.member.name),
            Some(raw) => format!("crate `{}` declares unknown tier `{raw}`", self.member.name),
        };
        self.emit("TAU-T001", None, message, None);
    }

    fn dependency_direction(&mut self, tier: Tier, tiers: &BTreeMap<&str, Option<Tier>>) {
        for dep in &self.member.workspace_deps {
            if let Some(Some(dep_tier)) = tiers.get(dep.as_str())
                && !tier.may_depend_on(*dep_tier)
            {
                self.emit(
                    "TAU-T002",
                    Some(tier),
                    format!(
                        "{tier} crate `{}` depends on {dep_tier} crate `{dep}`",
                        self.member.name
                    ),
                    None,
                );
            }
        }
    }

    /// TAU-T003, decided by the proved [`layout::t003`]: dependency names
    /// are interned by their position among the member's dependencies and
    /// allowlist.
    fn kernel_deps(&mut self, tier: Tier) {
        let mut names: Vec<String> = Vec::new();
        let mut intern = |name: &str| -> usize {
            names.iter().position(|n| n == name).unwrap_or_else(|| {
                names.push(name.to_owned());
                names.len().saturating_sub(1)
            })
        };
        let allowed: Vec<usize> = self
            .member
            .kernel_dep_allowlist
            .iter()
            .map(|d| intern(d.as_str()))
            .collect();
        let deps: Vec<(usize, String)> = self
            .member
            .external_deps
            .iter()
            .map(|d| (intern(d.as_str()), d.clone()))
            .collect();
        for (id, dep) in deps {
            if layout::t003(tier.kernel(), id, &allowed) {
                self.emit(
                    "TAU-T003",
                    Some(tier),
                    format!(
                        "kernel crate `{}` depends on `{dep}`, which is not in kernel-deps",
                        self.member.name
                    ),
                    None,
                );
            }
        }
    }

    /// TAU-X003, decided by the proved [`exception::loosens`] over every
    /// rule the entry may cover.
    fn manifest_lints(&mut self, tier: Tier) {
        for lint in &self.member.manifest_lints {
            let setting = match lint.level.as_str() {
                "allow" => exception::Setting::Allow,
                "warn" => exception::Setting::Warn,
                "expect" => exception::Setting::Expect,
                "deny" => exception::Setting::Deny,
                "forbid" => exception::Setting::Forbid,
                _ => continue,
            };
            let levels: Vec<KernelLevel> = self
                .profile
                .covered_by(&lint.name)
                .iter()
                .filter_map(|rule| rule.level(tier).map(Level::kernel))
                .collect();
            if exception::loosens(setting, &levels) {
                self.emit(
                    "TAU-X003",
                    Some(tier),
                    format!(
                        "manifest sets `{}` to `{}`; the profile keeps it blocking",
                        lint.name, lint.level
                    ),
                    None,
                );
            }
        }
    }

    /// An input a rule could not read is never clean
    /// ([`tauten_check_kernel::rule::clean`]): a source file `syn` cannot
    /// parse is reported under every source-scanning rule that applies in
    /// the tier, at the rule's own severity.
    fn unparsed(&mut self, tier: Tier, file: &SourceFile) {
        use tauten_check_kernel::rule::{Input, clean};
        if syn::parse_file(&file.text).is_ok() || clean(Input::Unparsed, false) {
            return;
        }
        let mut rules = vec!["TAU-X002", "TAU-X004"];
        if file.kind == SourceKind::Src {
            rules.push("TAU-F001");
        }
        if tier != Tier::Edge && file.kind == SourceKind::Src {
            rules.push("TAU-A002");
        }
        for id in rules {
            self.emit(
                id,
                Some(tier),
                format!(
                    "`{}` could not be parsed, so {id} was not checked in it; an unread file is never reported clean",
                    file.path
                ),
                Some((&file.path, scan::Hit { line: 1, column: 1 })),
            );
        }
    }

    fn sources(&mut self, tier: Tier) {
        for file in &self.member.sources {
            self.unparsed(tier, file);
            if file.kind != SourceKind::Src {
                // Outside `src/` (tests, benches, examples, the build
                // script) only lint exceptions are read.
                self.exceptions(tier, file);
                continue;
            }
            if file.is_crate_root {
                self.crate_root(tier, file);
            }
            self.exceptions(tier, file);
            if tier != Tier::Edge {
                for hit in scan::authority_mints(&file.text).unwrap_or_default() {
                    self.emit(
                        "TAU-A002",
                        Some(tier),
                        format!(
                            "`Authority::acquire` in {tier} crate `{}`",
                            self.member.name
                        ),
                        Some((&file.path, hit)),
                    );
                }
            }
            for (hit, marker) in scan::placeholder_comments(&file.text) {
                self.emit(
                    "TAU-S002",
                    Some(tier),
                    format!("comment contains `{marker}`"),
                    Some((&file.path, hit)),
                );
            }
        }
    }

    /// TAU-K003, decided by the proved [`layout::k003`] on the library root.
    fn crate_root(&mut self, tier: Tier, file: &SourceFile) {
        if !file.path.ends_with("lib.rs") {
            return;
        }
        let fact = scan::no_std(&file.text);
        if !layout::k003(tier.kernel(), fact) {
            return;
        }
        let what = match fact {
            layout::NoStd::Conditional => {
                "declares `no_std` only under a condition other than `not(test)`"
            }
            layout::NoStd::Unparsed => {
                "has a library root that could not be parsed, so `#![no_std]` was not seen"
            }
            layout::NoStd::Unconditional | layout::NoStd::NotTest | layout::NoStd::Absent => {
                "is missing `#![no_std]`"
            }
        };
        self.emit(
            "TAU-K003",
            Some(tier),
            format!("kernel crate `{}` {what}", self.member.name),
            Some((&file.path, scan::Hit { line: 1, column: 1 })),
        );
    }

    /// TAU-F001: facts from [`effects::facts`], decided by the proved
    /// [`kernel_effects::verdict`].
    fn effects(&mut self, tier: Tier) {
        // Only the library and binaries: effects are a property of the API.
        let src: Vec<&SourceFile> = self
            .member
            .sources
            .iter()
            .filter(|f| f.kind == SourceKind::Src)
            .collect();
        let texts: Vec<&str> = src.iter().map(|f| f.text.as_str()).collect();
        for fact in effects::facts(&texts) {
            let derived = kernel_effects::derive(&fact.sources, fact.fallible, fact.asynchronous);
            let resolved = fact.unresolved.is_empty();
            let severity = match kernel_effects::verdict(
                fact.declared,
                derived,
                self.member.effects_required,
                resolved,
            ) {
                Verdict::Silent => continue,
                Verdict::Warning => Severity::Warning,
                Verdict::Error => Severity::Error,
            };
            let grants = effects::labels_of(derived).join(", ");
            let mut message = match fact.declared {
                None => format!(
                    "`{}` has no `#[effects(..)]`; its signature grants `#[effects({grants})]`",
                    fact.name
                ),
                Some(declared) => {
                    let list = |row| effects::labels_of(row).join(", ");
                    let missing = list(kernel_effects::missing(declared, derived));
                    let extra = list(kernel_effects::extra(declared, derived));
                    let mut parts = Vec::new();
                    if !missing.is_empty() {
                        parts.push(format!("missing `{missing}`"));
                    }
                    if !extra.is_empty() {
                        parts.push(format!("declared but not granted `{extra}`"));
                    }
                    format!("effects of `{}`: {}", fact.name, parts.join("; "))
                }
            };
            if !resolved {
                message.push_str(&format!(
                    " (could not classify `{}`, so this is a warning)",
                    fact.unresolved.join("`, `")
                ));
            }
            let Some(rule) = self.profile.rule("TAU-F001") else {
                return;
            };
            let file = src.get(fact.file).map(|f| f.path.clone());
            let diag = Diagnostic::for_rule(rule, severity, &self.member.name, Some(tier), message)
                .at(file, Some(fact.at.line), Some(fact.at.column));
            self.out.push(diag);
        }
    }

    /// Lint exceptions, decided by the proved kernel
    /// ([`exception::decide`]). An exception covers the rule owning its lint
    /// (a lint has exactly one owning rule) or, for a lint group, every rule
    /// that may own a member ([`Profile::covered_by`]). In test targets a
    /// rule that does not apply to tests covers nothing.
    fn exceptions(&mut self, tier: Tier, file: &SourceFile) {
        let test = file.kind.is_test();
        for exception in scan::lint_exceptions(&file.text).unwrap_or_default() {
            let lint = exception.lint.as_str();
            let at = Some((file.path.as_str(), exception.at));
            // Clippy's `allow_attributes` skips inner attributes and cannot
            // see an attribute whose `cfg_attr` is off, so tauten reports
            // those `allow`s itself.
            if exception.allow && (exception.crate_level || exception.conditional) {
                let written = if exception.conditional {
                    format!("`cfg_attr(.., allow({lint}))`")
                } else {
                    format!("file-level `#![allow({lint})]`")
                };
                self.emit(
                    "TAU-X001",
                    Some(tier),
                    format!(
                        "{written}: use `expect({lint}, reason = \"...\")`, \
                         or narrow it to the item that needs it"
                    ),
                    at,
                );
            }
            let rules: Vec<&Rule> = self
                .profile
                .covered_by(lint)
                .into_iter()
                .filter(|rule| rule.in_tests || !test)
                .collect();
            if rules.is_empty() {
                continue;
            }
            let covered: Vec<exception::Covered> = rules
                .iter()
                .map(|rule| exception::Covered {
                    allows: rule.exceptions,
                    level: rule.level(tier).map(Level::kernel),
                })
                .collect();
            let form = exception::Form {
                crate_wide: exception.crate_level,
                conditional: exception.conditional,
            };
            let group = rules.len() > 1 || self.profile.rule_for_lint(lint).is_none();
            let what = if group {
                format!(
                    "`{lint}` (a lint group that may cover {} profile rules)",
                    rules.len()
                )
            } else {
                format!("`{lint}` ({})", rules.first().map_or("", |r| r.id.as_str()))
            };
            match exception::decide(form, &covered, tier.kernel()) {
                Excuse::Forbidden => {
                    let why = if exception.conditional && tier == Tier::Kernel {
                        "a conditional (`cfg_attr`) exception is not permitted in kernel crates"
                            .to_owned()
                    } else {
                        format!("is not permitted in {tier} crates")
                    };
                    self.emit(
                        "TAU-X004",
                        Some(tier),
                        format!("exception for {what} {why}"),
                        at,
                    );
                }
                Excuse::CrateWide => self.emit(
                    "TAU-X002",
                    Some(tier),
                    format!("crate-wide exception for {what}"),
                    at,
                ),
                Excuse::Allowed => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(name: &str, tier: &str) -> Member {
        Member {
            name: name.to_owned(),
            tier_raw: Some(tier.to_owned()),
            manifest: format!("{name}/Cargo.toml"),
            workspace_deps: Vec::new(),
            external_deps: Vec::new(),
            kernel_dep_allowlist: Vec::new(),
            manifest_lints: Vec::new(),
            sources: Vec::new(),
            effects_required: false,
            layer_raw: None,
        }
    }

    fn ids(diags: &[Diagnostic]) -> Vec<&str> {
        diags.iter().map(|d| d.id.as_str()).collect()
    }

    fn run(members: Vec<Member>) -> Vec<Diagnostic> {
        let profile = Profile::embedded().expect("profile");
        check(
            &profile,
            &Workspace {
                members,
                overflow_checks: None,
                layers: None,
            },
        )
    }

    /// The exception decision before it moved to the kernel, kept as the
    /// reference for a differential test.
    fn reference_excuse(
        profile: &Profile,
        rule: &crate::profile::Rule,
        lint: &str,
        tier: Tier,
        crate_level: bool,
    ) -> Option<&'static str> {
        let blocking: Vec<&str> = profile.blocking_lints(tier).collect();
        if !rule.exceptions || (tier == Tier::Kernel && rule.level(tier).is_some()) {
            Some("TAU-X004")
        } else if crate_level && blocking.contains(&lint) {
            Some("TAU-X002")
        } else {
            None
        }
    }

    #[test]
    fn kernel_excuse_agrees_with_the_reference_on_every_profile_lint() {
        let profile = Profile::embedded().expect("profile");
        let mut compared = 0;
        for rule in profile.rules() {
            for lint in &rule.lints {
                for tier in Tier::ALL {
                    for crate_level in [false, true] {
                        let level = rule.level(tier).map(Level::kernel);
                        let got = match exception::excuse(
                            rule.exceptions,
                            tier.kernel(),
                            level,
                            crate_level,
                        ) {
                            Excuse::Forbidden => Some("TAU-X004"),
                            Excuse::CrateWide => Some("TAU-X002"),
                            Excuse::Allowed => None,
                        };
                        let want = reference_excuse(&profile, rule, lint, tier, crate_level);
                        assert_eq!(got, want, "{lint} in {tier}, crate-wide {crate_level}");
                        compared += 1;
                    }
                }
            }
        }
        assert!(compared > 100, "only {compared} cases");
    }

    #[test]
    fn structural_severity_agrees_with_the_reference() {
        for level in [None, Some(Level::Warn), Some(Level::Deny)] {
            let reference = if level.unwrap_or(Level::Deny).is_blocking() {
                Severity::Error
            } else {
                Severity::Warning
            };
            assert_eq!(Severity::decide(level, Severity::Error), reference);
        }
    }

    #[test]
    fn clean_workspace_has_no_findings() {
        let mut k = member("k", "kernel");
        k.sources.push(SourceFile {
            path: "k/src/lib.rs".into(),
            text: "#![no_std]\n".into(),
            is_crate_root: true,
            kind: SourceKind::Src,
        });
        let mut c = member("c", "core");
        c.workspace_deps.push("k".into());
        let mut e = member("e", "edge");
        e.workspace_deps = vec!["k".into(), "c".into()];
        e.sources.push(SourceFile {
            path: "e/src/main.rs".into(),
            text: "fn main() { let _a = Authority::acquire(); }".into(),
            is_crate_root: true,
            kind: SourceKind::Src,
        });
        assert!(run(vec![k, c, e]).is_empty());
    }

    #[test]
    fn flags_missing_and_unknown_tiers() {
        let mut none = member("a", "x");
        none.tier_raw = None;
        let found = run(vec![none, member("b", "middle")]);
        assert_eq!(ids(&found), vec!["TAU-T001", "TAU-T001"]);
    }

    #[test]
    fn flags_outward_dependencies() {
        let mut k = member("k", "kernel");
        k.workspace_deps.push("c".into());
        k.sources.push(SourceFile {
            path: "k/src/lib.rs".into(),
            text: "#![no_std]".into(),
            is_crate_root: true,
            kind: SourceKind::Src,
        });
        let mut c = member("c", "core");
        c.workspace_deps.push("e".into());
        let found = run(vec![k, c, member("e", "edge")]);
        assert_eq!(ids(&found), vec!["TAU-T002", "TAU-T002"]);
        assert!(found.iter().all(|d| d.severity == Severity::Error));
    }

    #[test]
    fn flags_kernel_shape_and_deps() {
        let mut k = member("k", "kernel");
        k.external_deps = vec!["serde".into(), "allowed".into()];
        k.kernel_dep_allowlist.push("allowed".into());
        k.sources.push(SourceFile {
            path: "k/src/lib.rs".into(),
            text: "pub fn f() {}".into(),
            is_crate_root: true,
            kind: SourceKind::Src,
        });
        assert_eq!(ids(&run(vec![k])), vec!["TAU-T003", "TAU-K003"]);
    }

    #[test]
    fn flags_forbidden_exceptions() {
        let mut k = member("k", "kernel");
        k.sources.push(SourceFile {
            path: "k/src/lib.rs".into(),
            text: "#![no_std]\n#[expect(clippy::indexing_slicing, reason = \"x\")]\nfn f() {}\n#[expect(dead_code, reason = \"y\")]\nfn g() {}\n".into(),
            is_crate_root: true,
            kind: SourceKind::Src,
        });
        let mut e = member("e", "edge");
        e.sources.push(SourceFile {
            path: "e/src/main.rs".into(),
            text: "#[expect(clippy::todo, reason = \"later\")]\nfn f() {}\n#[expect(clippy::unwrap_used, reason = \"ok\")]\nfn g() {}\n".into(),
            is_crate_root: true,
            kind: SourceKind::Src,
        });
        let found = run(vec![k, e]);
        let lines: Vec<(&str, Option<u32>)> =
            found.iter().map(|d| (d.id.as_str(), d.line)).collect();
        assert_eq!(lines, vec![("TAU-X004", Some(2)), ("TAU-X004", Some(1))]);
    }

    #[test]
    fn kernel_dep_message_names_the_unlisted_dependency() {
        let mut k = member("k", "kernel");
        k.external_deps = vec!["serde".into(), "allowed".into()];
        k.kernel_dep_allowlist.push("allowed".into());
        k.sources.push(SourceFile {
            path: "k/src/lib.rs".into(),
            text: "#![no_std]".into(),
            is_crate_root: true,
            kind: SourceKind::Src,
        });
        let found = run(vec![k]);
        assert_eq!(found.len(), 1);
        assert!(
            found
                .iter()
                .all(|d| d.message.contains("`serde`") && !d.message.contains("`allowed`"))
        );
    }

    #[test]
    fn flags_escapes_and_minting() {
        let mut c = member("c", "core");
        c.manifest_lints.push(ManifestLint {
            name: "clippy::unwrap_used".into(),
            level: "allow".into(),
        });
        c.manifest_lints.push(ManifestLint {
            name: "clippy::style".into(),
            level: "allow".into(),
        });
        c.sources.push(SourceFile {
            path: "c/src/lib.rs".into(),
            text: "#![expect(clippy::indexing_slicing, reason = \"x\")]\npub fn f() { let _a = Authority::acquire(); } // for now\n".into(),
            is_crate_root: true,
            kind: SourceKind::Src,
        });
        let found = run(vec![c]);
        assert_eq!(
            ids(&found),
            // `clippy::style` is a group that may cover blocking rules.
            vec!["TAU-X003", "TAU-X003", "TAU-X002", "TAU-A002", "TAU-S002"]
        );
        assert!(found.iter().all(|d| d.krate == "c"));
        let s002 = found.last().expect("present");
        assert_eq!(s002.severity, Severity::Warning);
        assert_eq!(s002.line, Some(2));
    }
}

#[cfg(test)]
mod hardening_tests {
    use super::*;
    use tauten_check_kernel::manifest::Fact;

    fn member(tier: Option<&str>) -> Member {
        Member {
            name: "m".to_owned(),
            tier_raw: tier.map(str::to_owned),
            manifest: "m/Cargo.toml".to_owned(),
            workspace_deps: Vec::new(),
            external_deps: Vec::new(),
            kernel_dep_allowlist: Vec::new(),
            manifest_lints: Vec::new(),
            sources: Vec::new(),
            effects_required: false,
            layer_raw: None,
        }
    }

    fn h001(tier: Option<&str>, fact: Option<Fact>) -> Vec<Severity> {
        let profile = Profile::embedded().expect("profile");
        let ws = Workspace {
            members: vec![member(tier)],
            overflow_checks: fact,
            layers: None,
        };
        check(&profile, &ws)
            .into_iter()
            .filter(|d| d.id == "TAU-H001")
            .map(|d| d.severity)
            .collect()
    }

    #[test]
    fn release_overflow_checks_are_required() {
        assert_eq!(h001(Some("core"), Some(Fact::No)), [Severity::Error]);
        assert_eq!(h001(Some("core"), Some(Fact::Unparsed)), [Severity::Error]);
        assert!(h001(Some("core"), Some(Fact::Yes)).is_empty());
        // Not required without a tiered member; not read, not checked.
        assert!(h001(None, Some(Fact::No)).is_empty());
        assert!(h001(Some("core"), None).is_empty());
    }
}

#[cfg(test)]
mod unparsed_tests {
    use super::*;

    #[test]
    fn an_unparsed_file_is_never_clean() {
        let profile = Profile::embedded().expect("profile");
        let member = Member {
            name: "k".to_owned(),
            tier_raw: Some("kernel".to_owned()),
            manifest: "k/Cargo.toml".to_owned(),
            workspace_deps: Vec::new(),
            external_deps: Vec::new(),
            kernel_dep_allowlist: Vec::new(),
            manifest_lints: Vec::new(),
            sources: vec![SourceFile {
                path: "k/src/lib.rs".to_owned(),
                text: "#![no_std]\npub fn broken( {".to_owned(),
                is_crate_root: true,
                kind: SourceKind::Src,
            }],
            effects_required: false,
            layer_raw: None,
        };
        let ws = Workspace {
            members: vec![member],
            overflow_checks: None,
            layers: None,
        };
        let found: Vec<(String, Severity)> = check(&profile, &ws)
            .into_iter()
            .filter(|d| d.message.contains("could not be parsed"))
            .map(|d| (d.id, d.severity))
            .collect();
        let ids: Vec<&str> = found.iter().map(|(id, _)| id.as_str()).collect();
        for id in ["TAU-X002", "TAU-X004", "TAU-F001", "TAU-A002", "TAU-K003"] {
            assert!(ids.contains(&id), "{id}: {found:?}");
        }
        // A blocking rule reports it as an error; F001 at its level (warn).
        assert!(found.contains(&("TAU-K003".to_owned(), Severity::Error)));
        assert!(found.contains(&("TAU-F001".to_owned(), Severity::Warning)));
    }
}
