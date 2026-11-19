//! Structural checks over a workspace model built by the caller.

use std::collections::BTreeMap;

use tauten_check_kernel::effects::{self as kernel_effects, Verdict};
use tauten_check_kernel::exception::{self, Excuse};

use crate::diag::{Diagnostic, Severity};
use crate::effects;
use crate::profile::Profile;
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
    out
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
        let severity = Severity::decide(tier.and_then(|t| rule.level(t)), Severity::Error);
        let mut diag = Diagnostic::for_rule(rule, severity, &self.member.name, tier, message);
        diag = match at {
            Some((file, hit)) => diag.at(Some(file.to_owned()), Some(hit.line), Some(hit.column)),
            None => diag.at(Some(self.member.manifest.clone()), None, None),
        };
        self.out.push(diag);
    }

    fn tier_declared(&mut self) {
        match self.member.tier_raw.as_deref() {
            None => self.emit(
                "TAU-T001",
                None,
                format!("crate `{}` declares no tier", self.member.name),
                None,
            ),
            Some(raw) if Tier::parse(raw).is_none() => self.emit(
                "TAU-T001",
                None,
                format!("crate `{}` declares unknown tier `{raw}`", self.member.name),
                None,
            ),
            Some(_) => {}
        }
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

    fn kernel_deps(&mut self, tier: Tier) {
        if tier != Tier::Kernel {
            return;
        }
        for dep in &self.member.external_deps {
            if !self.member.kernel_dep_allowlist.contains(dep) {
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

    fn manifest_lints(&mut self, tier: Tier) {
        for lint in &self.member.manifest_lints {
            let weakens = matches!(lint.level.as_str(), "allow" | "warn" | "expect");
            let profile_level = self.profile.lint_level(&lint.name, tier);
            if weakens && profile_level.is_some_and(Level::is_blocking) {
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

    fn sources(&mut self, tier: Tier) {
        for file in &self.member.sources {
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

    fn crate_root(&mut self, tier: Tier, file: &SourceFile) {
        let Some(attrs) = scan::crate_attrs(&file.text) else {
            return;
        };
        let is_lib = file.path.ends_with("lib.rs");
        if tier == Tier::Kernel && is_lib && !attrs.no_std {
            self.emit(
                "TAU-K003",
                Some(tier),
                format!(
                    "kernel crate `{}` is missing `#![no_std]`",
                    self.member.name
                ),
                Some((&file.path, scan::Hit { line: 1, column: 1 })),
            );
        }
    }

    /// TAU-F001: facts from [`effects::facts`], decided by the proved
    /// [`kernel_effects::verdict`].
    fn effects(&mut self, tier: Tier) {
        let texts: Vec<&str> = self
            .member
            .sources
            .iter()
            .map(|f| f.text.as_str())
            .collect();
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
            let file = self.member.sources.get(fact.file).map(|f| f.path.clone());
            let diag = Diagnostic::for_rule(rule, severity, &self.member.name, Some(tier), message)
                .at(file, Some(fact.at.line), Some(fact.at.column));
            self.out.push(diag);
        }
    }

    /// Lint exceptions, decided by the proved kernel
    /// ([`exception::excuse`]). A lint has exactly one owning rule (the
    /// profile rejects shared ownership), so the lint is blocking in `tier`
    /// exactly when that rule's level there is blocking.
    fn exceptions(&mut self, tier: Tier, file: &SourceFile) {
        for exception in scan::lint_exceptions(&file.text).unwrap_or_default() {
            let lint = exception.lint.as_str();
            // Clippy's `allow_attributes` skips inner attributes, so a
            // file-level `#![allow]` would otherwise escape TAU-X001.
            if exception.crate_level && exception.allow {
                self.emit(
                    "TAU-X001",
                    Some(tier),
                    format!(
                        "file-level `#![allow({lint})]`: use `#![expect({lint}, reason = \"...\")]`, \
                         or narrow it to the item that needs it"
                    ),
                    Some((file.path.as_str(), exception.at)),
                );
            }
            let Some(rule) = self.profile.rule_for_lint(lint) else {
                continue;
            };
            let at = Some((file.path.as_str(), exception.at));
            let level = rule.level(tier).map(Level::kernel);
            match exception::excuse(rule.exceptions, tier.kernel(), level, exception.crate_level) {
                Excuse::Forbidden => {
                    let message = format!(
                        "exception for `{lint}` ({}) is not permitted in {tier} crates",
                        rule.id
                    );
                    self.emit("TAU-X004", Some(tier), message, at);
                }
                Excuse::CrateWide => self.emit(
                    "TAU-X002",
                    Some(tier),
                    format!("crate-wide exception for `{lint}`"),
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
        }
    }

    fn ids(diags: &[Diagnostic]) -> Vec<&str> {
        diags.iter().map(|d| d.id.as_str()).collect()
    }

    fn run(members: Vec<Member>) -> Vec<Diagnostic> {
        let profile = Profile::embedded().expect("profile");
        check(&profile, &Workspace { members })
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
        });
        let mut c = member("c", "core");
        c.workspace_deps.push("k".into());
        let mut e = member("e", "edge");
        e.workspace_deps = vec!["k".into(), "c".into()];
        e.sources.push(SourceFile {
            path: "e/src/main.rs".into(),
            text: "fn main() { let _a = Authority::acquire(); }".into(),
            is_crate_root: true,
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
        });
        let mut e = member("e", "edge");
        e.sources.push(SourceFile {
            path: "e/src/main.rs".into(),
            text: "#[expect(clippy::todo, reason = \"later\")]\nfn f() {}\n#[expect(clippy::unwrap_used, reason = \"ok\")]\nfn g() {}\n".into(),
            is_crate_root: true,
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
        });
        let found = run(vec![c]);
        assert_eq!(
            ids(&found),
            vec!["TAU-X003", "TAU-X002", "TAU-A002", "TAU-S002"]
        );
        assert!(found.iter().all(|d| d.krate == "c"));
        let s002 = found.last().expect("present");
        assert_eq!(s002.severity, Severity::Warning);
        assert_eq!(s002.line, Some(2));
    }
}
