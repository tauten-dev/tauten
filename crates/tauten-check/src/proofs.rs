//! The proof tier for users' kernel crates (M3, proof plan P5).
//!
//! A project lists what it proves in `proofs/claims.toml`: per Rust
//! function, the Lean theorems stated over its Charon/Aeneas translation
//! and the source mutants that must break them. `cargo tauten proofs` runs
//! the project's proof gate, reads the result lines it prints (see
//! [`Event`]), decides one [`Outcome`] per claim ([`outcomes`]) and records
//! them in `proofs/claims.json` together with a fingerprint of the sources
//! the gate saw. That file is also the data the website's "Proved" badges
//! are generated from.
//!
//! `cargo tauten check` reads `proofs/claims.json` back: inside a kernel
//! crate, a TAU-N001 (arithmetic) or TAU-P003 (indexing) finding in a
//! function whose claim is proved, has a theorem that the function never
//! fails, lists the rule under `subsumes`, and was recorded for the current
//! source, is reported as subsumed by that theorem instead of as an error
//! ([`subsume`]). The decision is the proved kernel's
//! ([`tauten_check_kernel::subsume::subsumes`]).
//!
//! This module performs no IO: the caller reads the files.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use syn::spanned::Spanned;
use syn::visit::Visit;
use tauten_check_kernel::control as kernel_control;
use tauten_check_kernel::outcome as kernel_outcome;
use tauten_check_kernel::subsume as kernel;

use crate::diag::{Diagnostic, Severity};

/// The rules a proof may subsume.
pub const SUBSUMABLE: &[&str] = &["TAU-N001", "TAU-P003"];

/// The marker that starts every result line a proof gate prints.
pub const MARKER: &str = "TAUTEN-PROOFS ";

/// `proofs/claims.toml`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimsFile {
    /// Format version; must be 1.
    pub version: u32,
    /// How to run the gate.
    pub gate: GateConfig,
    /// One entry per proved function.
    #[serde(default)]
    pub claim: Vec<Claim>,
    /// Source mutants, for gates that read them from this file (the
    /// template's `proofs/gate.py`).
    #[serde(default)]
    pub mutant: Vec<Mutant>,
}

/// The `[gate]` table.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateConfig {
    /// The command `cargo tauten proofs` runs from the workspace root.
    pub command: Vec<String>,
    /// Crate directories the gate translates (workspace-relative).
    #[serde(default)]
    pub crates: Vec<String>,
}

/// One `[[claim]]`: a Rust function and what is proved about it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    /// The package the function is in.
    #[serde(rename = "crate")]
    pub krate: String,
    /// The source file, relative to the workspace root.
    pub file: String,
    /// The function in that file: `name`, `Type::name` for a method, or
    /// `module::name` inside an inline module.
    pub function: String,
    /// The extracted Lean definition, e.g. `ledger_kernel.apply`.
    pub lean: String,
    /// Theorems stated over the extraction (fully qualified Lean names).
    pub theorems: Vec<String>,
    /// The theorem stating that the function never fails, for every input
    /// (`∀ args, f args ⦃ _ ⦄`, no hypotheses). Required for `subsumes`.
    #[serde(default)]
    pub total: Option<String>,
    /// Rules whose findings inside the function the proof subsumes.
    #[serde(default)]
    pub subsumes: Vec<String>,
    /// Mutants of the Rust source that must break at least one theorem.
    pub mutants: Vec<String>,
}

/// One `[[mutant]]`: a textual change to the source.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mutant {
    /// Name, referenced from `claim.mutants`.
    pub name: String,
    /// Workspace-relative file to change.
    pub file: String,
    /// Text that must occur exactly once.
    pub before: String,
    /// Its replacement.
    pub after: String,
}

/// Why `proofs/claims.toml` was rejected.
#[derive(Debug, thiserror::Error)]
pub enum ClaimsError {
    /// Not valid TOML for the schema.
    #[error("proofs/claims.toml is not valid: {0}")]
    Parse(#[from] toml::de::Error),
    /// Unknown format version.
    #[error("unsupported proofs/claims.toml version {0}")]
    Version(u32),
    /// The gate command is empty.
    #[error("[gate] command is empty")]
    NoCommand,
    /// A claim is inconsistent.
    #[error("claim {function}: {problem}")]
    Claim {
        /// The claim's function.
        function: String,
        /// What is wrong.
        problem: String,
    },
}

impl ClaimsFile {
    /// Parse and validate `proofs/claims.toml`.
    ///
    /// # Errors
    /// Returns [`ClaimsError`] when the file is malformed or a claim is
    /// inconsistent: no theorem, no mutant (a badge needs a killed mutant),
    /// a `total` that is not among its theorems, a `subsumes` rule other
    /// than TAU-N001/TAU-P003 or without `total`, or a function claimed twice.
    pub fn parse(text: &str) -> Result<Self, ClaimsError> {
        let file: Self = toml::from_str(text)?;
        if file.version != 1 {
            return Err(ClaimsError::Version(file.version));
        }
        if file.gate.command.is_empty() {
            return Err(ClaimsError::NoCommand);
        }
        let mut seen = BTreeSet::new();
        for claim in &file.claim {
            let problem = |text: &str| ClaimsError::Claim {
                function: claim.function.clone(),
                problem: text.to_owned(),
            };
            if claim.theorems.is_empty() {
                return Err(problem("lists no theorem"));
            }
            if claim.mutants.is_empty() {
                return Err(problem(
                    "lists no mutant; a claim needs a mutant that breaks it",
                ));
            }
            if let Some(total) = &claim.total
                && !claim.theorems.contains(total)
            {
                return Err(problem("`total` must be one of its `theorems`"));
            }
            if let Some(rule) = claim
                .subsumes
                .iter()
                .find(|r| !SUBSUMABLE.contains(&r.as_str()))
            {
                return Err(problem(&format!(
                    "`subsumes` names {rule}; only TAU-N001 and TAU-P003 can be subsumed"
                )));
            }
            if !claim.subsumes.is_empty() && claim.total.is_none() {
                return Err(problem("`subsumes` needs a `total` theorem"));
            }
            if !seen.insert((claim.file.as_str(), claim.function.as_str())) {
                return Err(problem("is claimed twice"));
            }
        }
        Ok(file)
    }
}

/// What the gate established for one claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    /// Every stage passed: translated without drift, theorems built with
    /// the standard axioms only, every listed mutant broke a theorem.
    Proved,
    /// The committed translation differs from a fresh one, or the sources
    /// changed while the gate ran.
    Drift,
    /// Translation or `lake build` failed.
    BuildFailed,
    /// A theorem is missing, depends on a non-standard axiom, or the
    /// `total` theorem does not have the total no-failure shape.
    AuditFailed,
    /// A listed mutant did not break any of the claim's theorems.
    MutantSurvived,
    /// The proof toolchain is not available; nothing was checked.
    ToolMissing,
}

impl Outcome {
    /// The name printed in JSON.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Proved => "proved",
            Self::Drift => "drift",
            Self::BuildFailed => "build-failed",
            Self::AuditFailed => "audit-failed",
            Self::MutantSurvived => "mutant-survived",
            Self::ToolMissing => "tool-missing",
        }
    }
}

/// One result line of a proof gate (after [`MARKER`]), as JSON.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum Event {
    /// A stage finished: `{"stage": "drift", "status": "ok"}`. Stages:
    /// `tools`, `extract`, `drift`, `build`, `audit`, `mutants`. Status:
    /// `ok`, `fail`, or `missing` (tools only).
    Stage {
        /// Stage name.
        stage: String,
        /// `ok`, `fail` or `missing`.
        status: String,
        /// Free text for the user.
        #[serde(default)]
        detail: Option<String>,
    },
    /// A theorem passed the axiom audit (`ok`) or did not (`fail`).
    Theorem {
        /// Fully qualified theorem name.
        theorem: String,
        /// `ok` or `fail`.
        status: String,
    },
    /// The audit found that `total` states `function` never fails.
    Total {
        /// Theorem name.
        total: String,
        /// Extracted Lean function.
        function: String,
    },
    /// A mutant's result: the claims' theorems it broke.
    Mutant {
        /// Mutant name.
        mutant: String,
        /// `killed`, `survived` (the proofs still build), `timeout`, or
        /// `other` (it broke something other than the named theorems).
        status: String,
        /// Theorems that no longer build against the mutated translation.
        #[serde(default)]
        broken: Vec<String>,
    },
}

impl Event {
    /// Parse one line of gate output; `None` for lines without the marker
    /// or with malformed JSON (those are ordinary log output).
    #[must_use]
    pub fn parse(line: &str) -> Option<Self> {
        let json = line.trim_end().strip_prefix(MARKER)?;
        serde_json::from_str(json).ok()
    }
}

/// Everything the gate reported.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// The command could not be started (not installed).
    pub not_started: Option<String>,
    /// Result lines, in order.
    pub events: Vec<Event>,
    /// The sources changed while the gate ran.
    pub changed_during_run: bool,
}

impl Report {
    fn stage(&self, name: &str) -> Option<&str> {
        self.events.iter().rev().find_map(|event| match event {
            Event::Stage { stage, status, .. } if stage == name => Some(status.as_str()),
            Event::Stage { .. }
            | Event::Theorem { .. }
            | Event::Total { .. }
            | Event::Mutant { .. } => None,
        })
    }

    fn theorem_ok(&self, name: &str) -> bool {
        let mut ok = false;
        for event in &self.events {
            if let Event::Theorem { theorem, status } = event
                && theorem == name
            {
                if status != "ok" {
                    return false;
                }
                ok = true;
            }
        }
        ok
    }

    fn total_ok(&self, theorem: &str, lean: &str) -> bool {
        self.events.iter().any(|event| {
            matches!(event, Event::Total { total, function } if total == theorem && function == lean)
        })
    }

    /// The claim's theorems as controls that must pass, with every audit
    /// result reported for them (an `ok` audit is a pass).
    fn theorem_controls(
        &self,
        claim: &Claim,
    ) -> (
        Vec<kernel_control::Control>,
        Vec<kernel_control::Observation>,
    ) {
        use kernel_control::{Control, Expect, Observation, Seen};
        let controls = (0..claim.theorems.len())
            .map(|id| Control {
                id,
                expect: Expect::Pass,
            })
            .collect();
        let observed = self
            .events
            .iter()
            .filter_map(|event| match event {
                Event::Theorem { theorem, status } => {
                    let id = claim.theorems.iter().position(|t| t == theorem)?;
                    let seen = if status == "ok" {
                        Seen::Pass
                    } else {
                        Seen::FailOther
                    };
                    Some(Observation { id, seen })
                }
                Event::Stage { .. } | Event::Total { .. } | Event::Mutant { .. } => None,
            })
            .collect();
        (controls, observed)
    }

    /// The claim's mutants as negative controls, with every result reported
    /// for them. A kill is named when it broke one of the claim's theorems
    /// ([`kernel_control::named`]).
    fn mutant_controls(
        &self,
        claim: &Claim,
    ) -> (
        Vec<kernel_control::Control>,
        Vec<kernel_control::Observation>,
    ) {
        use kernel_control::{Control, Expect, Observation, Seen};
        let controls = (0..claim.mutants.len())
            .map(|id| Control {
                id,
                expect: Expect::Fail,
            })
            .collect();
        let declared: Vec<usize> = (0..claim.theorems.len()).collect();
        let observed = self
            .events
            .iter()
            .filter_map(|event| match event {
                Event::Mutant {
                    mutant,
                    status,
                    broken,
                } => {
                    let id = claim.mutants.iter().position(|m| m == mutant)?;
                    let broken_ids: Vec<usize> = broken
                        .iter()
                        .filter_map(|b| claim.theorems.iter().position(|t| t == b))
                        .collect();
                    let seen = match status.as_str() {
                        "killed" if kernel_control::named(&declared, &broken_ids) => {
                            Seen::FailNamed
                        }
                        "survived" => Seen::Pass,
                        "timeout" => Seen::Timeout,
                        _ => Seen::FailOther,
                    };
                    Some(Observation { id, seen })
                }
                Event::Stage { .. } | Event::Theorem { .. } | Event::Total { .. } => None,
            })
            .collect();
        (controls, observed)
    }

    fn mutant_kills(&self, name: &str, theorems: &[String]) -> bool {
        self.events.iter().any(|event| match event {
            Event::Mutant {
                mutant,
                status,
                broken,
            } => {
                mutant == name && status == "killed" && broken.iter().any(|t| theorems.contains(t))
            }
            Event::Stage { .. } | Event::Theorem { .. } | Event::Total { .. } => false,
        })
    }
}

/// The outcome of one claim, with a reason when it is not proved. The
/// decision is the proved [`kernel_outcome::outcome`]; this reads the facts
/// from the report and words the reason.
#[must_use]
pub fn outcome(claim: &Claim, report: &Report) -> (Outcome, Option<String>) {
    use kernel_outcome::Stage;
    let stage = |name: &str| match report.stage(name) {
        Some("ok") => Stage::Ok,
        Some(_) => Stage::Failed,
        None => Stage::Absent,
    };
    let ran = report.not_started.is_none()
        && report
            .events
            .iter()
            .any(|event| matches!(event, Event::Stage { .. }));
    let missing_theorem = claim.theorems.iter().find(|t| !report.theorem_ok(t));
    let total_ok = claim
        .total
        .as_ref()
        .is_none_or(|total| report.total_ok(total, &claim.lean));
    let survivor = claim
        .mutants
        .iter()
        .find(|m| !report.mutant_kills(m, &claim.theorems));
    let (theorems, theorem_results) = report.theorem_controls(claim);
    let (mutants, mutant_results) = report.mutant_controls(claim);
    let facts = kernel_outcome::Facts {
        ran,
        tools: stage("tools"),
        extract: stage("extract"),
        drift: stage("drift"),
        build: stage("build"),
        audit: stage("audit"),
        theorems: false,
        total: total_ok,
        mutants: false,
        changed: report.changed_during_run,
    };
    let decided = kernel_outcome::of_run(
        facts,
        &theorems,
        &theorem_results,
        &mutants,
        &mutant_results,
    );
    let first_stage = || {
        ["tools", "extract", "drift", "build", "audit"]
            .into_iter()
            .find_map(|name| match report.stage(name) {
                Some("ok") => None,
                Some(status) => Some(format!("stage {name}: {status}")),
                None => Some(format!("stage {name} did not run")),
            })
    };
    let reason = match decided {
        kernel_outcome::Outcome::Proved => return (Outcome::Proved, None),
        kernel_outcome::Outcome::ToolMissing => Some(if let Some(why) = &report.not_started {
            why.clone()
        } else if !ran {
            "the gate printed no result; it did not run".to_owned()
        } else {
            report
                .events
                .iter()
                .find_map(|event| match event {
                    Event::Stage { stage, detail, .. } if stage == "tools" => detail.clone(),
                    Event::Stage { .. }
                    | Event::Theorem { .. }
                    | Event::Total { .. }
                    | Event::Mutant { .. } => None,
                })
                .unwrap_or_else(|| "proof toolchain missing".to_owned())
        }),
        kernel_outcome::Outcome::BuildFailed => first_stage(),
        kernel_outcome::Outcome::Drift if facts.drift != Stage::Ok => first_stage(),
        kernel_outcome::Outcome::Drift => Some("the sources changed while the gate ran".to_owned()),
        kernel_outcome::Outcome::AuditFailed => {
            if facts.audit != Stage::Ok {
                first_stage()
            } else if let Some(missing) = missing_theorem {
                Some(format!(
                    "{missing} is missing or depends on a non-standard axiom"
                ))
            } else if claim.theorems.is_empty() {
                Some("the claim lists no theorem".to_owned())
            } else if !kernel_control::admit(&theorems, &theorem_results) {
                Some("a theorem was reported more than once".to_owned())
            } else {
                claim.total.as_ref().map(|total| {
                    format!(
                        "{total} does not state that {} never fails for every input",
                        claim.lean
                    )
                })
            }
        }
        kernel_outcome::Outcome::MutantSurvived => Some(if claim.mutants.is_empty() {
            "the claim lists no mutant, so nothing shows the theorems can fail".to_owned()
        } else {
            match survivor {
                Some(name) => format!("mutant {name} broke none of the claim's theorems"),
                None => "a mutant was reported more than once".to_owned(),
            }
        }),
    };
    let outcome = match decided {
        kernel_outcome::Outcome::Proved => Outcome::Proved,
        kernel_outcome::Outcome::Drift => Outcome::Drift,
        kernel_outcome::Outcome::BuildFailed => Outcome::BuildFailed,
        kernel_outcome::Outcome::AuditFailed => Outcome::AuditFailed,
        kernel_outcome::Outcome::MutantSurvived => Outcome::MutantSurvived,
        kernel_outcome::Outcome::ToolMissing => Outcome::ToolMissing,
    };
    (outcome, reason)
}

/// One claim as recorded in `proofs/claims.json` (also the badge data).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimRecord {
    /// The claim as declared.
    #[serde(flatten)]
    pub claim: Claim,
    /// What the gate established.
    pub outcome: Outcome,
    /// Why the claim is not proved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Fingerprint of the sources the gate checked: the claim's crate and
    /// the proofs directory ([`fingerprint`]).
    pub fingerprint: String,
}

/// `proofs/claims.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    /// Format version (1).
    pub version: u32,
    /// Always `cargo tauten proofs`: the file is generated, not written.
    pub generated_by: String,
    /// One record per claim of `proofs/claims.toml`, in its order.
    pub claims: Vec<ClaimRecord>,
}

/// A change-detection fingerprint of a set of files (path, contents):
/// FNV-1a 64 over the files in path order. It detects edits; it is not
/// tamper protection (`proofs/claims.json` is a reviewed, committed file,
/// like a baseline).
#[must_use]
pub fn fingerprint(files: &[(String, Vec<u8>)]) -> String {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut sorted: Vec<&(String, Vec<u8>)> = files.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hash = OFFSET;
    let mut eat = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(PRIME);
        }
    };
    for (path, contents) in sorted {
        eat(path.as_bytes());
        eat(&[0]);
        eat(&u64::try_from(contents.len())
            .unwrap_or(u64::MAX)
            .to_le_bytes());
        eat(contents);
    }
    format!("fnv1a64:{hash:016x}")
}

/// A function's lines in a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSpan {
    /// `name`, `Type::name`, `<Type as Trait>::name`, `module::name` (nested
    /// items: `outer::inner`).
    pub path: String,
    /// First line (1-based, attributes included).
    pub start: u32,
    /// Last line.
    pub end: u32,
}

#[derive(Default)]
struct Functions {
    prefix: Vec<String>,
    found: Vec<FunctionSpan>,
}

impl Functions {
    fn push(&mut self, name: String, span: proc_macro2::Span) {
        let line = |l: usize| u32::try_from(l).unwrap_or(u32::MAX);
        let mut path = self.prefix.clone();
        path.push(name);
        self.found.push(FunctionSpan {
            path: path.join("::"),
            start: line(span.start().line),
            end: line(span.end().line),
        });
    }
}

fn type_name(ty: &syn::Type) -> String {
    if let syn::Type::Path(path) = ty {
        path.path
            .segments
            .last()
            .map_or_else(String::new, |s| s.ident.to_string())
    } else {
        String::from("_")
    }
}

impl<'ast> Visit<'ast> for Functions {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.push(item.sig.ident.to_string(), item.span());
        self.prefix.push(item.sig.ident.to_string());
        syn::visit::visit_item_fn(self, item);
        self.prefix.pop();
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        self.prefix.push(item.ident.to_string());
        syn::visit::visit_item_mod(self, item);
        self.prefix.pop();
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        // A trait method is `<Type as Trait>::f`, so a claim on the
        // inherent `Type::f` never covers it.
        let name = match &item.trait_ {
            Some((path, _)) => format!(
                "<{} as {}>",
                type_name(&item.self_ty),
                path.segments
                    .last()
                    .map_or_else(String::new, |s| s.ident.to_string())
            ),
            None => type_name(&item.self_ty),
        };
        self.prefix.push(name);
        syn::visit::visit_item_impl(self, item);
        self.prefix.pop();
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        self.push(item.sig.ident.to_string(), item.span());
        self.prefix.push(item.sig.ident.to_string());
        syn::visit::visit_impl_item_fn(self, item);
        self.prefix.pop();
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        self.prefix.push(item.ident.to_string());
        syn::visit::visit_item_trait(self, item);
        self.prefix.pop();
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        self.push(item.sig.ident.to_string(), item.span());
        self.prefix.push(item.sig.ident.to_string());
        syn::visit::visit_trait_item_fn(self, item);
        self.prefix.pop();
    }
}

/// Every function of a source file with its lines; empty if the file does
/// not parse (then nothing in it is subsumed).
#[must_use]
pub fn functions(source: &str) -> Vec<FunctionSpan> {
    let Ok(file) = syn::parse_file(source) else {
        return Vec::new();
    };
    let mut visitor = Functions::default();
    visitor.visit_file(&file);
    visitor.found
}

/// The innermost function containing `line`, chosen by the proved
/// [`tauten_check_kernel::span::innermost`] (narrowest covering span,
/// first on a tie).
fn enclosing(spans: &[FunctionSpan], line: u32) -> Option<&FunctionSpan> {
    let lines: Vec<(u32, u32)> = spans.iter().map(|s| (s.start, s.end)).collect();
    tauten_check_kernel::span::innermost(&lines, line).and_then(|i| spans.get(i))
}

/// Bindings of every input in `map` (name to identifying text), keyed by
/// the name's position in `names`.
fn bindings(
    map: &BTreeMap<String, String>,
    names: &[&String],
) -> Vec<tauten_check_kernel::binding::Binding> {
    map.iter()
        .filter_map(|(name, value)| {
            let key = names.iter().position(|n| *n == name)?;
            Some(tauten_check_kernel::binding::Binding {
                key,
                bytes: value.as_bytes().to_vec(),
            })
        })
        .collect()
}

/// Whether a result recorded against inputs `recorded` may be reused with
/// inputs `current` (name to fingerprint): the proved
/// [`tauten_check_kernel::binding::fresh`]. An input added, removed or
/// changed since the record makes it stale.
#[must_use]
pub fn unchanged(recorded: &BTreeMap<String, String>, current: &BTreeMap<String, String>) -> bool {
    let names: Vec<&String> = recorded
        .keys()
        .chain(current.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    tauten_check_kernel::binding::fresh(&bindings(recorded, &names), &bindings(current, &names))
}

/// Whether a claim recorded with `fingerprint` for `krate` is fresh against
/// the current fingerprints of the claimed crates.
#[must_use]
pub fn claim_fresh(krate: &str, fingerprint: &str, current: &BTreeMap<String, String>) -> bool {
    let recorded = BTreeMap::from([(krate.to_owned(), fingerprint.to_owned())]);
    let now: BTreeMap<String, String> = current
        .get(krate)
        .map(|f| (krate.to_owned(), f.clone()))
        .into_iter()
        .collect();
    unchanged(&recorded, &now)
}

/// A recorded claim and whether its fingerprint matches the current source.
#[derive(Debug, Clone, Copy)]
pub struct Recorded<'a> {
    /// The record from `proofs/claims.json`.
    pub record: &'a ClaimRecord,
    /// Whether it was recorded for the current source.
    pub fresh: bool,
}

/// Marks every finding a proof subsumes: severity `warning` and
/// `subsumed_by` set to the claim's `total` theorem. `functions` maps a
/// workspace-relative file to its function spans ([`functions`]).
#[must_use]
pub fn subsume(
    mut diags: Vec<Diagnostic>,
    recorded: &[Recorded<'_>],
    functions: &BTreeMap<String, Vec<FunctionSpan>>,
) -> Vec<Diagnostic> {
    for diag in &mut diags {
        let (Some(file), Some(line), Some(tier)) = (&diag.file, diag.line, diag.tier) else {
            continue;
        };
        let Some(spans) = functions.get(file) else {
            continue;
        };
        let Some(function) = enclosing(spans, line) else {
            continue;
        };
        // Two functions with the same path in one file (generic impls,
        // `cfg` variants): the claim cannot say which it is about.
        if spans.iter().filter(|s| s.path == function.path).count() > 1 {
            continue;
        }
        let Some(entry) = recorded.iter().find(|entry| {
            let claim = &entry.record.claim;
            claim.file == *file && claim.function == function.path && claim.krate == diag.krate
        }) else {
            continue;
        };
        let record = entry.record;
        let finding = match diag.id.as_str() {
            "TAU-N001" => kernel::Finding::Arithmetic,
            "TAU-P003" => kernel::Finding::Indexing,
            _ => kernel::Finding::Other,
        };
        let listed = record.claim.subsumes.contains(&diag.id);
        let proved = record.outcome == Outcome::Proved && record.claim.total.is_some();
        if kernel::subsumes(tier.kernel(), finding, listed, proved, entry.fresh) {
            diag.severity = Severity::Warning;
            diag.subsumed_by.clone_from(&record.claim.total);
        }
    }
    diags
}

#[cfg(test)]
mod tests;
