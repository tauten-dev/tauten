//! The proof tier for users' kernel crates (M3, proof plan P5).
//!
//! A project lists what it proves in `proofs/claims.toml`: per Rust
//! function, the Lean theorems stated over its Charon/Aeneas translation
//! and the source mutants that must break them. `cargo tauten proofs` runs
//! the project's proof gate, reads the result lines it prints (see
//! [`Event`]), decides one [`Outcome`] per claim ([`outcome`]) and records
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
use tauten_check_kernel::badge as kernel_badge;
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
    /// Accepted weakenings of the recorded claims ([`conservation`]).
    #[serde(default)]
    pub amend: Vec<Amend>,
}

/// What an `[[amend]]` entry changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AmendKind {
    /// A theorem removed from a claim (or the claim removed).
    Theorem,
    /// A mutant removed from a claim (or the claim removed).
    Mutant,
    /// An assumption added to a claim.
    Assumption,
}

/// One `[[amend]]`: a change to the recorded claims that would otherwise be
/// refused as a weakening, with the reason it is accepted.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Amend {
    /// The claim's source file.
    pub file: String,
    /// The claim's function.
    pub function: String,
    /// What is removed or added.
    pub kind: AmendKind,
    /// The theorem, mutant or assumption.
    pub name: String,
    /// Why the weakening is accepted.
    pub reason: String,
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
    /// The command that runs only the tools and extract stages
    /// (`cargo tauten proofs extract`); `--crate <dir>` is appended to
    /// translate one crate.
    #[serde(default)]
    pub extract: Option<Vec<String>>,
    /// The command for `cargo tauten proofs try`: every stage, but only the
    /// mutants named in the argument appended to it (comma-separated).
    #[serde(default, rename = "try")]
    pub try_: Option<Vec<String>>,
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
    /// Named assumptions about the outside world the theorems rest on
    /// (IDs from `proofs/assumptions.toml`). A claim with assumptions is
    /// at best `proved-under`, never `proved`. Version 2 only.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assumes: Vec<String>,
    /// A theorem with no assumption among its hypotheses that proves the
    /// claim's assumptions can hold together (for example of an in-memory
    /// model), so the theorems are not vacuous. Required with `assumes`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub witness: Option<String>,
    /// Theorems showing an assumption is needed (the property fails
    /// without it). Optional; each must pass like the claim's theorems.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub necessity: Vec<String>,
    /// `"crate@version:function"`: the claim is a dependency's, read from
    /// the `claims.json` that crate ships and checked against the locked
    /// version ([`inherited`]). Such a claim lists no theorems or mutants
    /// of its own. Version 2 only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inherit: Option<String>,
}

/// A parsed `inherit = "crate@version:function"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inherit {
    /// The dependency's package name.
    pub krate: String,
    /// The version the claim was recorded for.
    pub version: String,
    /// The function, as the dependency's claim names it.
    pub function: String,
}

impl Inherit {
    /// Parse `crate@version:function`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let (krate, rest) = text.split_once('@')?;
        let (version, function) = rest.split_once(':')?;
        (!krate.is_empty() && !version.is_empty() && !function.is_empty()).then(|| Self {
            krate: krate.to_owned(),
            version: version.to_owned(),
            function: function.to_owned(),
        })
    }
}

/// The record of an inheriting claim: the dependency's claim (its theorems
/// and assumptions) with the badge the proved
/// [`kernel_badge::inherit`] gives it, never stronger than the inherited
/// one. `locked` is the dependency's version in the lock file; `shipped`
/// the `claims.json` it ships. A version mismatch, a missing record or a
/// claim the record does not hold establishes nothing.
#[must_use]
pub fn inherited(
    claim: &Claim,
    locked: Option<&str>,
    shipped: Option<&Record>,
) -> (Claim, Outcome, Badge, Option<String>) {
    let parsed = claim.inherit.as_deref().and_then(Inherit::parse);
    let found = parsed.as_ref().and_then(|p| {
        shipped?
            .claims
            .iter()
            .find(|r| r.claim.function == p.function)
    });
    let version_matches = parsed
        .as_ref()
        .is_some_and(|p| locked == Some(p.version.as_str()));
    let from = found.map_or(Badge::NotEstablished, |r| r.badge);
    let badge = badge_of(kernel_badge::inherit(
        kernel_of(from),
        version_matches,
        found.is_some(),
    ));
    let mut record_claim = claim.clone();
    if let Some(r) = found {
        record_claim.theorems.clone_from(&r.claim.theorems);
        record_claim.assumes.clone_from(&r.claim.assumes);
    }
    let detail = match (&parsed, locked, found) {
        (None, _, _) => Some("`inherit` is not `crate@version:function`".to_owned()),
        (Some(p), None, _) => Some(format!("{} is not a dependency", p.krate)),
        (Some(p), Some(v), _) if v != p.version => Some(format!(
            "{} is locked at {v}, the claim names {}",
            p.krate, p.version
        )),
        (Some(p), Some(_), None) => Some(format!(
            "{}@{} ships no claim for `{}`",
            p.krate, p.version, p.function
        )),
        (Some(_), Some(_), Some(r)) if !matches!(badge, Badge::Proved | Badge::ProvedUnder) => {
            Some(format!("the inherited claim is {}", r.outcome.name()))
        }
        (Some(_), Some(_), Some(_)) => None,
    };
    let outcome = if matches!(badge, Badge::Proved | Badge::ProvedUnder) {
        Outcome::Proved
    } else {
        Outcome::Drift
    };
    (record_claim, outcome, badge, detail)
}

const fn kernel_of(b: Badge) -> kernel_badge::Badge {
    match b {
        Badge::Proved => kernel_badge::Badge::Proved,
        Badge::ProvedUnder => kernel_badge::Badge::ProvedUnder,
        Badge::ModelProved => kernel_badge::Badge::ModelProved,
        Badge::Bounded => kernel_badge::Badge::Bounded,
        Badge::Tested => kernel_badge::Badge::Tested,
        Badge::Reviewed => kernel_badge::Badge::Reviewed,
        Badge::Pending => kernel_badge::Badge::Pending,
        Badge::NotEstablished => kernel_badge::Badge::NotEstablished,
    }
}

const fn badge_of(b: kernel_badge::Badge) -> Badge {
    match b {
        kernel_badge::Badge::Proved => Badge::Proved,
        kernel_badge::Badge::ProvedUnder => Badge::ProvedUnder,
        kernel_badge::Badge::ModelProved => Badge::ModelProved,
        kernel_badge::Badge::Bounded => Badge::Bounded,
        kernel_badge::Badge::Tested => Badge::Tested,
        kernel_badge::Badge::Reviewed => Badge::Reviewed,
        kernel_badge::Badge::Pending => Badge::Pending,
        kernel_badge::Badge::NotEstablished => Badge::NotEstablished,
    }
}

/// `proofs/assumptions.toml`: the named assumptions claims may rest on.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssumptionsFile {
    /// One entry per assumption.
    #[serde(default)]
    pub assumption: Vec<Assumption>,
}

/// One named assumption, e.g. that `rename` is atomic on the filesystems a
/// project supports.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assumption {
    /// Stable ID, e.g. `FS.rename-atomic`.
    pub id: String,
    /// What is assumed, in one sentence.
    pub statement: String,
    /// Where it is assumed to hold (platforms, filesystems, versions).
    #[serde(default)]
    pub scope: Option<String>,
    /// Tests or fault injections that exercise it (it is not proved).
    #[serde(default)]
    pub tested_by: Vec<String>,
}

impl AssumptionsFile {
    /// Parse `proofs/assumptions.toml`.
    ///
    /// # Errors
    /// Malformed TOML, or an ID listed twice.
    pub fn parse(text: &str) -> Result<Self, ClaimsError> {
        let file: Self = toml::from_str(text)?;
        let mut seen = BTreeSet::new();
        for a in &file.assumption {
            if !seen.insert(a.id.as_str()) {
                return Err(ClaimsError::Assumption(format!("{} is listed twice", a.id)));
            }
        }
        Ok(file)
    }
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
    /// Optional, exact: the Lean declarations whose own proofs must fail
    /// under this mutant (helpers included), and no others. Decided by the
    /// proved [`kernel_control::admit`]; without it the claim-level policy
    /// applies (one of the claim's theorems broken, directly or through a
    /// dependency).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub direct: Vec<String>,
}

/// Why `proofs/claims.toml` was rejected.
#[derive(Debug, thiserror::Error)]
pub enum ClaimsError {
    /// Not valid TOML for the schema.
    #[error("proofs/claims.toml is not valid: {0}")]
    Parse(#[from] toml::de::Error),
    /// A format version this tauten does not read (newer, older or
    /// missing), decided by the kernel's `version::admit`.
    #[error("{0}")]
    Version(String),
    /// The gate command is empty.
    #[error("[gate] command is empty")]
    NoCommand,
    /// `proofs/assumptions.toml` is inconsistent, or a claim names an
    /// assumption it does not define.
    #[error("assumptions: {0}")]
    Assumption(String),
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
        let table: toml::Table = toml::from_str(text)?;
        crate::format::check(
            "proofs/claims.toml",
            crate::format::toml_version(&table),
            1,
            2,
        )
        .map_err(ClaimsError::Version)?;
        let file: Self = toml::from_str(text)?;
        if file.gate.command.is_empty() {
            return Err(ClaimsError::NoCommand);
        }
        let mut seen = BTreeSet::new();
        for claim in &file.claim {
            let problem = |text: &str| ClaimsError::Claim {
                function: claim.function.clone(),
                problem: text.to_owned(),
            };
            if claim.theorems.is_empty() && claim.inherit.is_none() {
                return Err(problem("lists no theorem"));
            }
            if claim.mutants.is_empty() && claim.inherit.is_none() {
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
            if (!claim.assumes.is_empty() || claim.witness.is_some() || !claim.necessity.is_empty())
                && file.version < 2
            {
                return Err(problem(
                    "`assumes`, `witness` and `necessity` need `version = 2`",
                ));
            }
            if let Some(inherit) = &claim.inherit {
                if file.version < 2 {
                    return Err(problem("`inherit` needs `version = 2`"));
                }
                if Inherit::parse(inherit).is_none() {
                    return Err(problem("`inherit` is not `crate@version:function`"));
                }
                if !claim.theorems.is_empty() || !claim.mutants.is_empty() {
                    return Err(problem(
                        "an inheriting claim lists no theorems or mutants of its own",
                    ));
                }
                continue;
            }
            if !claim.assumes.is_empty() && claim.witness.is_none() {
                return Err(problem(
                    "`assumes` needs a `witness` theorem showing the assumptions can hold together",
                ));
            }
        }
        if let Some(a) = file.amend.iter().find(|a| a.reason.trim().is_empty()) {
            return Err(ClaimsError::Claim {
                function: a.function.clone(),
                problem: format!("`[[amend]]` of {} gives no reason", a.name),
            });
        }
        Ok(file)
    }

    /// Check that every assumption a claim names is defined.
    ///
    /// # Errors
    /// [`ClaimsError::Assumption`] naming the first undefined ID.
    pub fn check_assumptions(&self, known: &AssumptionsFile) -> Result<(), ClaimsError> {
        for claim in &self.claim {
            if let Some(id) = claim
                .assumes
                .iter()
                .find(|id| !known.assumption.iter().any(|a| &a.id == *id))
            {
                return Err(ClaimsError::Assumption(format!(
                    "claim {} assumes {id}, which proofs/assumptions.toml does not define",
                    claim.function
                )));
            }
        }
        Ok(())
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
        /// The crate a per-crate stage (extract-only) is about.
        #[serde(default, rename = "crate")]
        krate: Option<String>,
    },
    /// The named assumptions a theorem's hypotheses mention, derived by
    /// the audit from its statement.
    Premises {
        /// Fully qualified theorem name.
        theorem: String,
        /// Assumption IDs.
        premises: Vec<String>,
    },
    /// The named assumptions a theorem without assumption hypotheses
    /// proves can hold (a witness), derived by the audit.
    Establishes {
        /// Fully qualified theorem name.
        theorem: String,
        /// Assumption IDs.
        establishes: Vec<String>,
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
    /// The audit found that `theorem` concludes a triple about `subject`
    /// (`… ⦃ _ ⦄` over an application of that constant), whatever its
    /// hypotheses. Read only by `cargo tauten proofs draft`, never by an
    /// outcome (#49).
    Subject {
        /// Fully qualified theorem name.
        theorem: String,
        /// The extracted Lean declaration the triple is about.
        subject: String,
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
        /// Declarations whose own proof failed (#39).
        #[serde(default)]
        direct: Vec<String>,
        /// Declarations that failed only through one that did.
        #[serde(default)]
        dependent: Vec<String>,
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
    /// Mutant name -> its exact expected direct failures, for the mutants
    /// of `proofs/claims.toml` that declare `direct` (#39).
    pub expected_direct: BTreeMap<String, Vec<String>>,
}

/// Mutant name -> `direct`, for every mutant that declares one.
#[must_use]
pub fn expected_direct(file: &ClaimsFile) -> BTreeMap<String, Vec<String>> {
    file.mutant
        .iter()
        .filter(|m| !m.direct.is_empty())
        .map(|m| (m.name.clone(), m.direct.clone()))
        .collect()
}

/// Whether the declarations that failed their own proof are exactly
/// `expected` (#39): one `Fail` control per expected name, one observation
/// per observed name, decided by the proved [`kernel_control::admit`]. An
/// unexpected direct failure, a missing one, or nothing observed is never a
/// kill.
fn exact_direct(expected: &[String], observed: &[String]) -> bool {
    use kernel_control::{Control, Expect, Observation, Seen};
    let mut names: Vec<&str> = expected.iter().map(String::as_str).collect();
    for o in observed {
        if !names.contains(&o.as_str()) {
            names.push(o);
        }
    }
    let id = |n: &str| names.iter().position(|m| *m == n).unwrap_or(usize::MAX);
    let controls: Vec<Control> = expected
        .iter()
        .map(|e| Control {
            id: id(e),
            expect: Expect::Fail,
        })
        .collect();
    let observations: Vec<Observation> = observed
        .iter()
        .map(|o| Observation {
            id: id(o),
            seen: Seen::FailNamed,
        })
        .collect();
    kernel_control::admit(&controls, &observations)
}

impl Report {
    fn stage(&self, name: &str) -> Option<&str> {
        self.events.iter().rev().find_map(|event| match event {
            Event::Stage { stage, status, .. } if stage == name => Some(status.as_str()),
            Event::Stage { .. }
            | Event::Theorem { .. }
            | Event::Total { .. }
            | Event::Subject { .. }
            | Event::Premises { .. }
            | Event::Establishes { .. }
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

    /// The audit's derived assumption IDs: (premises of the claim's
    /// theorems, what its witness establishes).
    fn assumption_facts<'a>(&'a self, claim: &'a Claim) -> (BTreeSet<&'a str>, BTreeSet<&'a str>) {
        let mut premises = BTreeSet::new();
        let mut established = BTreeSet::new();
        for event in &self.events {
            match event {
                Event::Premises {
                    theorem,
                    premises: ids,
                } if claim.theorems.contains(theorem) => {
                    premises.extend(ids.iter().map(String::as_str));
                }
                Event::Establishes {
                    theorem,
                    establishes: ids,
                } if claim.witness.as_ref() == Some(theorem) => {
                    established.extend(ids.iter().map(String::as_str));
                }
                Event::Premises { .. }
                | Event::Establishes { .. }
                | Event::Stage { .. }
                | Event::Theorem { .. }
                | Event::Total { .. }
                | Event::Subject { .. }
                | Event::Mutant { .. } => {}
            }
        }
        (premises, established)
    }

    /// Whether the claim declares exactly the assumptions its theorems
    /// rest on and its witness establishes them: the proved
    /// [`tauten_check_kernel::assumption::assumptions_ok`].
    fn assumptions_ok<'a>(&'a self, claim: &'a Claim) -> bool {
        use tauten_check_kernel::registry::Entry;
        let (premises, established) = self.assumption_facts(claim);
        let mut ids: BTreeMap<&'a str, usize> = BTreeMap::new();
        let mut entries = |names: Vec<&'a str>| -> Vec<Entry> {
            names
                .into_iter()
                .map(|n| {
                    let next = ids.len();
                    Entry {
                        id: *ids.entry(n).or_insert(next),
                        strength: 0,
                    }
                })
                .collect()
        };
        let declared = entries(claim.assumes.iter().map(String::as_str).collect());
        let derived = entries(premises.into_iter().collect());
        let witnessed = entries(established.into_iter().collect());
        tauten_check_kernel::assumption::assumptions_ok(&declared, &derived, &witnessed)
    }

    /// Everything that must pass for the claim: its theorems, its witness
    /// and necessity theorems (audited like theorems), and, as the last
    /// control, the assumption check. An `ok` audit is a pass.
    fn theorem_controls(
        &self,
        claim: &Claim,
    ) -> (
        Vec<kernel_control::Control>,
        Vec<kernel_control::Observation>,
    ) {
        use kernel_control::{Control, Expect, Observation, Seen};
        let names = required_theorems(claim);
        let controls = (0..=names.len())
            .map(|id| Control {
                id,
                expect: Expect::Pass,
            })
            .collect();
        let mut observed: Vec<Observation> = self
            .events
            .iter()
            .filter_map(|event| match event {
                Event::Theorem { theorem, status } => {
                    let id = names.iter().position(|t| *t == theorem)?;
                    let seen = if status == "ok" {
                        Seen::Pass
                    } else {
                        Seen::FailOther
                    };
                    Some(Observation { id, seen })
                }
                Event::Stage { .. }
                | Event::Total { .. }
                | Event::Subject { .. }
                | Event::Premises { .. }
                | Event::Establishes { .. }
                | Event::Mutant { .. } => None,
            })
            .collect();
        observed.push(Observation {
            id: names.len(),
            seen: if self.assumptions_ok(claim) {
                Seen::Pass
            } else {
                Seen::FailOther
            },
        });
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
                    direct,
                    ..
                } => {
                    let id = claim.mutants.iter().position(|m| m == mutant)?;
                    let broken_ids: Vec<usize> = broken
                        .iter()
                        .filter_map(|b| claim.theorems.iter().position(|t| t == b))
                        .collect();
                    let killed = match self.expected_direct.get(mutant) {
                        // Exact direct set: every expected declaration failed
                        // its own proof and no other did.
                        Some(expected) => exact_direct(expected, direct),
                        None => kernel_control::named(&declared, &broken_ids),
                    };
                    let seen = match status.as_str() {
                        "killed" if killed => Seen::FailNamed,
                        "survived" => Seen::Pass,
                        "timeout" => Seen::Timeout,
                        _ => Seen::FailOther,
                    };
                    Some(Observation { id, seen })
                }
                Event::Stage { .. }
                | Event::Theorem { .. }
                | Event::Total { .. }
                | Event::Subject { .. }
                | Event::Premises { .. }
                | Event::Establishes { .. } => None,
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
                direct,
                ..
            } => {
                mutant == name
                    && status == "killed"
                    && match self.expected_direct.get(mutant) {
                        Some(expected) => exact_direct(expected, direct),
                        None => broken.iter().any(|t| theorems.contains(t)),
                    }
            }
            Event::Stage { .. }
            | Event::Theorem { .. }
            | Event::Total { .. }
            | Event::Subject { .. }
            | Event::Premises { .. }
            | Event::Establishes { .. } => false,
        })
    }
}

/// What an extract-only run established for one crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Extracted {
    /// It translated with no opaque function.
    Ok,
    /// It did not translate (or only with opaque functions): the first
    /// located error.
    Failed(String),
    /// No result was reported for it.
    NotRun,
}

/// The verdict of `cargo tauten proofs extract` on `crates`: whether every
/// crate translated, and what each showed. The decision is the proved
/// [`kernel_control::admit`]: every crate is a control that must pass,
/// observed exactly once; a missing tool, a crate with no result or a
/// repeated result is never a pass.
#[must_use]
pub fn extraction(crates: &[String], report: &Report) -> (bool, Vec<(String, Extracted)>) {
    use kernel_control::{Control, Expect, Observation, Seen};
    let controls: Vec<Control> = (0..crates.len())
        .map(|id| Control {
            id,
            expect: Expect::Pass,
        })
        .collect();
    let mut seen: Vec<Extracted> = vec![Extracted::NotRun; crates.len()];
    let mut observed = Vec::new();
    for event in &report.events {
        if let Event::Stage {
            stage,
            status,
            detail,
            krate: Some(krate),
        } = event
            && stage == "extract"
            && let Some(id) = crates.iter().position(|c| c == krate)
        {
            let ok = status == "ok";
            observed.push(Observation {
                id,
                seen: if ok { Seen::Pass } else { Seen::FailOther },
            });
            if let Some(slot) = seen.get_mut(id) {
                *slot = if ok {
                    Extracted::Ok
                } else {
                    Extracted::Failed(detail.clone().unwrap_or_else(|| status.clone()))
                };
            }
        }
    }
    let admitted = report.not_started.is_none() && kernel_control::admit(&controls, &observed);
    (admitted, crates.iter().cloned().zip(seen).collect())
}

/// Whether every `[[mutant]]` can be applied (#81), before any gate runs:
/// its name is unique, its file is readable through `read` (a path
/// relative to the workspace), its `before` text occurs in it exactly once
/// and differs from `after`. The decision is the proved
/// [`kernel_control::admit`]: each entry is a control that must pass,
/// observed once per occurrence of its site, so a missing site, a repeated
/// one or a repeated name is never admitted. Returns the problems, empty
/// when admitted; a file with no `[[mutant]]` (a custom gate defining its
/// own) has nothing to check.
#[must_use]
pub fn mutant_sites(file: &ClaimsFile, read: &dyn Fn(&str) -> Option<String>) -> Vec<String> {
    use kernel_control::{Control, Expect, Observation, Seen};
    if file.mutant.is_empty() {
        return Vec::new();
    }
    let mut names: Vec<&str> = Vec::new();
    for m in &file.mutant {
        if !names.contains(&m.name.as_str()) {
            names.push(&m.name);
        }
    }
    let id = |n: &str| names.iter().position(|m| *m == n).unwrap_or(usize::MAX);
    let controls: Vec<Control> = (0..names.len())
        .map(|id| Control {
            id,
            expect: Expect::Pass,
        })
        .collect();
    let mut observed = Vec::new();
    let mut problems = Vec::new();
    let mut entries: BTreeMap<&str, usize> = BTreeMap::new();
    for m in &file.mutant {
        let n = entries.entry(&m.name).or_default();
        *n = n.saturating_add(1);
        if *n == 2 {
            problems.push(format!("mutant `{}` is defined more than once", m.name));
        }
        let Some(text) = read(&m.file) else {
            problems.push(format!("mutant `{}`: cannot read {}", m.name, m.file));
            observed.push(Observation {
                id: id(&m.name),
                seen: Seen::FailOther,
            });
            continue;
        };
        if m.before == m.after || m.before.is_empty() {
            problems.push(format!(
                "mutant `{}`: `before` is empty or equals `after`",
                m.name
            ));
            observed.push(Observation {
                id: id(&m.name),
                seen: Seen::FailOther,
            });
            continue;
        }
        let sites = text.matches(m.before.as_str()).count();
        if sites != 1 {
            problems.push(format!(
                "mutant `{}`: its `before` text occurs {sites} times in {}, not once",
                m.name, m.file
            ));
        }
        observed.extend((0..sites).map(|_| Observation {
            id: id(&m.name),
            seen: Seen::Pass,
        }));
    }
    let admitted = kernel_control::admit(&controls, &observed);
    if !admitted && problems.is_empty() {
        problems.push("the mutant inventory is not admitted".to_owned());
    }
    if admitted { Vec::new() } else { problems }
}

/// For `cargo tauten proofs try`: the mutants to run and the claims they
/// bear on. Selected by theorem (every mutant of a claim that states one of
/// `theorems`) and by name (`mutants`). Each affected claim is returned
/// with its mutants narrowed to the selected ones, so its outcome on a
/// partial run says only what that run showed.
#[must_use]
pub fn try_selection(
    file: &ClaimsFile,
    theorems: &[String],
    mutants: &[String],
) -> (Vec<String>, Vec<Claim>) {
    let mut selected: Vec<String> = Vec::new();
    for claim in &file.claim {
        let by_theorem = claim.theorems.iter().any(|t| theorems.contains(t));
        for m in &claim.mutants {
            if (by_theorem || mutants.contains(m)) && !selected.contains(m) {
                selected.push(m.clone());
            }
        }
    }
    for m in mutants {
        if !selected.contains(m) {
            selected.push(m.clone());
        }
    }
    let affected = file
        .claim
        .iter()
        .filter(|c| c.mutants.iter().any(|m| selected.contains(m)))
        .map(|c| {
            let mut narrowed = c.clone();
            narrowed.mutants.retain(|m| selected.contains(m));
            narrowed
        })
        .collect();
    (selected, affected)
}

/// The claim's theorems, then its witness and necessity theorems, without
/// repeats.
fn required_theorems(claim: &Claim) -> Vec<&str> {
    let mut names: Vec<&str> = Vec::new();
    for name in claim
        .theorems
        .iter()
        .chain(&claim.witness)
        .chain(&claim.necessity)
    {
        if !names.contains(&name.as_str()) {
            names.push(name);
        }
    }
    names
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
    let missing_theorem = required_theorems(claim)
        .into_iter()
        .find(|t| !report.theorem_ok(t));
    let assumptions_ok = report.assumptions_ok(claim);
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
                    | Event::Subject { .. }
                    | Event::Premises { .. }
                    | Event::Establishes { .. }
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
            } else if !assumptions_ok {
                Some(format!(
                    "the claim's `assumes` ({}) is not exactly what its theorems assume, or its witness does not establish them",
                    claim.assumes.join(", ")
                ))
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

/// What may be said about a claim (the proved [`kernel_badge::badge`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Badge {
    /// Proved about the code, with no assumption about the outside world.
    Proved,
    /// Proved about the code, under the claim's named assumptions.
    ProvedUnder,
    /// Proved about a model only.
    ModelProved,
    /// Checked by a bounded search.
    Bounded,
    /// Tested.
    Tested,
    /// Reviewed.
    Reviewed,
    /// Established, but its inputs changed since.
    Pending,
    /// Not established.
    #[default]
    NotEstablished,
}

impl Badge {
    /// The name printed in JSON and summaries.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Proved => "proved",
            Self::ProvedUnder => "proved-under",
            Self::ModelProved => "model-proved",
            Self::Bounded => "bounded",
            Self::Tested => "tested",
            Self::Reviewed => "reviewed",
            Self::Pending => "pending",
            Self::NotEstablished => "not-established",
        }
    }
}

/// The badge of an extracted Lean claim: established when the gate proved
/// it, `fresh` when its inputs are unchanged since.
#[must_use]
pub fn badge(claim: &Claim, outcome: Outcome, fresh: bool) -> Badge {
    use kernel_badge::{Method, Relation};
    badge_of(kernel_badge::badge(
        outcome == Outcome::Proved,
        fresh,
        Method::Lean,
        Relation::Extracted,
        !claim.assumes.is_empty(),
    ))
}

/// One claim as recorded in `proofs/claims.json` (also the badge data).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimRecord {
    /// The claim as declared.
    #[serde(flatten)]
    pub claim: Claim,
    /// What the gate established.
    pub outcome: Outcome,
    /// What may be said about it when it was recorded ([`badge`]).
    #[serde(default)]
    pub badge: Badge,
    /// Why the claim is not proved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Fingerprint of the sources the gate checked: the claim's crate and
    /// the proofs directory ([`fingerprint`]).
    pub fingerprint: String,
    /// The claim's named inputs and their fingerprints when it was recorded
    /// ([`inputs::claim_inputs`]): what a ship-level gate compares to decide
    /// whether its mutants must run again.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub inputs: BTreeMap<String, String>,
    /// Its mutants' results on the run that recorded it, reused by a
    /// ship-level gate while the inputs are unchanged.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mutant_results: Vec<MutantResult>,
}

/// One mutant's result as the gate reported it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutantResult {
    /// Mutant name.
    pub mutant: String,
    /// `killed`, `survived`, `timeout`, ...
    pub status: String,
    /// Theorems it broke.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub broken: Vec<String>,
    /// Declarations whose own proof failed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub direct: Vec<String>,
    /// Declarations that failed only through those.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependent: Vec<String>,
}

impl MutantResult {
    /// The result as a gate event.
    #[must_use]
    pub fn event(&self) -> Event {
        Event::Mutant {
            mutant: self.mutant.clone(),
            status: self.status.clone(),
            broken: self.broken.clone(),
            direct: self.direct.clone(),
            dependent: self.dependent.clone(),
        }
    }
}

impl Report {
    /// Whether the gate got as far as a local proof stage (#52): it started,
    /// and reported something other than an unavailable toolchain. A command
    /// that could not start, printed nothing, or only reported `tools`
    /// missing is an attempt: its claims are `tool-missing`, but it is not a
    /// result to record over an earlier one. A started gate that then fails
    /// (drift, build, audit, mutants) is a result.
    #[must_use]
    pub fn started(&self) -> bool {
        self.not_started.is_none()
            && self.events.iter().any(|event| match event {
                Event::Stage { stage, status, .. } => match stage.as_str() {
                    "preflight" => false,
                    "tools" => status == "ok",
                    _ => true,
                },
                Event::Theorem { .. }
                | Event::Total { .. }
                | Event::Premises { .. }
                | Event::Establishes { .. }
                | Event::Subject { .. }
                | Event::Mutant { .. } => true,
            })
    }

    /// Why the gate refused before any proof stage (#81): a `preflight`
    /// stage that did not pass, with its detail.
    #[must_use]
    pub fn preflight_refusal(&self) -> Option<String> {
        self.events.iter().find_map(|event| {
            if let Event::Stage {
                stage,
                status,
                detail,
                ..
            } = event
                && stage == "preflight"
                && status != "ok"
            {
                return Some(detail.clone().unwrap_or_else(|| status.clone()));
            }
            None
        })
    }

    /// The results the gate reported for `names`.
    #[must_use]
    pub fn mutant_results(&self, names: &[String]) -> Vec<MutantResult> {
        self.events
            .iter()
            .filter_map(|event| match event {
                Event::Mutant {
                    mutant,
                    status,
                    broken,
                    direct,
                    dependent,
                } if names.contains(mutant) => Some(MutantResult {
                    mutant: mutant.clone(),
                    status: status.clone(),
                    broken: broken.clone(),
                    direct: direct.clone(),
                    dependent: dependent.clone(),
                }),
                Event::Mutant { .. }
                | Event::Stage { .. }
                | Event::Theorem { .. }
                | Event::Total { .. }
                | Event::Subject { .. }
                | Event::Premises { .. }
                | Event::Establishes { .. } => None,
            })
            .collect()
    }
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
    /// The commit whose snapshot the gate checked, when it ran on one
    /// (the default in a Git checkout); absent for a run on the live tree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
}

impl Record {
    /// The newest result format this tauten reads and writes.
    pub const VERSION: u32 = 1;

    /// Parse a recorded or shipped `claims.json` result, refusing a format
    /// version this tauten does not read (A10).
    ///
    /// # Errors
    ///
    /// Malformed JSON, or an unsupported or missing version.
    pub fn parse(text: &str) -> Result<Self, String> {
        let value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        Self::from_value(value)
    }

    fn from_value(value: serde_json::Value) -> Result<Self, String> {
        crate::format::check(
            "claims.json",
            crate::format::json_version(&value),
            1,
            Self::VERSION,
        )?;
        serde_json::from_value(value).map_err(|e| e.to_string())
    }
}

/// The provisional record `cargo tauten proofs reconcile` writes after a
/// two-parent merge (#50, RFC 2026-11-21-recover-proof-records): the
/// resolved claim declarations, already checked against both parents'
/// records, and nothing else. It is not a gate result: it has no `claims`
/// results, fingerprints, commit or mutant kills, so a v1 reader refuses it
/// (no `claims` array) and no reader may count it as evidence. Only a
/// successful gate on the merged commit replaces it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pending {
    /// Format version (2).
    pub version: u32,
    /// Always `pending-merge`.
    pub state: String,
    /// The merge's first and second parent commits.
    pub parents: Vec<String>,
    /// The resolved `proofs/claims.toml` claims.
    pub registry: Vec<Claim>,
}

/// The `state` of a [`Pending`] record.
pub const PENDING_MERGE: &str = "pending-merge";

/// What `proofs/claims.json` holds: a gate result, or a pending merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordFile {
    /// A v1 gate result.
    Result(Record),
    /// A v2 provisional registry after a merge; never evidence.
    Pending(Pending),
}

impl RecordFile {
    /// Parse `proofs/claims.json`. A file with a `state` key must be a
    /// well-formed pending merge record (version 2, two distinct parents);
    /// anything else must be a v1 result.
    ///
    /// # Errors
    ///
    /// Malformed JSON, an unknown `state` or version, or a pending record
    /// without two distinct parents.
    pub fn parse(text: &str) -> Result<Self, String> {
        let value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if value.get("state").is_none() {
            return Record::from_value(value).map(Self::Result);
        }
        crate::format::check(
            "a pending proofs/claims.json",
            crate::format::json_version(&value),
            2,
            2,
        )?;
        let pending: Pending = serde_json::from_value(value).map_err(|e| e.to_string())?;
        if pending.state != PENDING_MERGE || pending.version != 2 {
            return Err(format!(
                "unknown record state `{}` (version {})",
                pending.state, pending.version
            ));
        }
        match pending.parents.as_slice() {
            [a, b] if a != b && !a.is_empty() && !b.is_empty() => Ok(Self::Pending(pending)),
            _ => Err("a pending merge record needs two distinct parents".to_owned()),
        }
    }

    /// The gate result, if this is one; a pending merge is never evidence.
    #[must_use]
    pub const fn result(&self) -> Option<&Record> {
        match self {
            Self::Result(r) => Some(r),
            Self::Pending(_) => None,
        }
    }

    /// The recorded claim declarations, for TAU-R002 ([`conservation_of`]).
    #[must_use]
    pub fn declared(&self) -> Vec<&Claim> {
        match self {
            Self::Result(r) => r.claims.iter().map(|c| &c.claim).collect(),
            Self::Pending(p) => p.registry.iter().collect(),
        }
    }
}

/// Whether both parents of a merge conserve their obligations in the
/// resolved `claims.toml` (#50): one required `Pass` control per parent,
/// decided by the proved [`kernel_control::admit`]. A missing parent
/// observation cannot pass, and fewer than two parents never do.
#[must_use]
pub fn parents_conserved(found: &[Conservation]) -> bool {
    use kernel_control::{Control, Expect, Observation, Seen};
    if found.len() != 2 {
        return false;
    }
    let controls: Vec<Control> = (0..found.len())
        .map(|id| Control {
            id,
            expect: Expect::Pass,
        })
        .collect();
    let observed: Vec<Observation> = found
        .iter()
        .enumerate()
        .map(|(id, c)| Observation {
            id,
            seen: if c.weakened.is_empty() {
                Seen::Pass
            } else {
                Seen::FailOther
            },
        })
        .collect();
    kernel_control::admit(&controls, &observed)
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
        // Only an unconditional proof subsumes: a claim proved under
        // assumptions does not excuse a panic path.
        let proved = badge(&record.claim, record.outcome, entry.fresh) == Badge::Proved
            && record.claim.total.is_some();
        if kernel::subsumes(tier.kernel(), finding, listed, proved, entry.fresh) {
            diag.severity = Severity::Warning;
            diag.subsumed_by.clone_from(&record.claim.total);
        }
    }
    diags
}

/// One entry of the claims registry: (file, function, kind, name).
type RegistryKey<'a> = (&'a str, &'a str, AmendKind, &'a str);

fn claim_entries(claim: &Claim) -> impl Iterator<Item = RegistryKey<'_>> {
    let f = (claim.file.as_str(), claim.function.as_str());
    claim
        .theorems
        .iter()
        .map(move |t| (f.0, f.1, AmendKind::Theorem, t.as_str()))
        .chain(
            claim
                .mutants
                .iter()
                .map(move |m| (f.0, f.1, AmendKind::Mutant, m.as_str())),
        )
}

fn assumption_entries(claim: &Claim) -> impl Iterator<Item = RegistryKey<'_>> {
    claim.assumes.iter().map(move |a| {
        (
            claim.file.as_str(),
            claim.function.as_str(),
            AmendKind::Assumption,
            a.as_str(),
        )
    })
}

/// What [`conservation`] found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Conservation {
    /// Recorded theorems and mutants missing from `claims.toml`, and
    /// assumptions it adds, that no `[[amend]]` names:
    /// (crate, file, function, kind, name).
    pub weakened: Vec<(String, String, String, AmendKind, String)>,
    /// `[[amend]]` entries that excuse nothing (indexes into `amend`).
    pub unneeded: Vec<usize>,
}

/// Whether a proof run passes: one required `Pass` control per claim, met
/// only by a `proved` outcome. Decided by the proved
/// [`kernel_control::admit`], which refuses an empty required set, so a run
/// over no claims never passes (#51).
#[must_use]
pub fn records_pass(records: &[ClaimRecord]) -> bool {
    use kernel_control::{Control, Expect, Observation, Seen};
    let controls: Vec<Control> = (0..records.len())
        .map(|id| Control {
            id,
            expect: Expect::Pass,
        })
        .collect();
    let observed: Vec<Observation> = records
        .iter()
        .enumerate()
        .map(|(id, r)| Observation {
            id,
            seen: if r.outcome == Outcome::Proved {
                Seen::Pass
            } else {
                Seen::FailOther
            },
        })
        .collect();
    kernel_control::admit(&controls, &observed)
}

/// Whether `claims.toml` keeps every theorem and mutant of the committed
/// record (`claims.json`) and adds no assumption, except where an
/// `[[amend]]` entry names the change.
///
/// Theorems and mutants form a may-not-shrink registry and assumptions a
/// may-not-grow one; the proved kernel
/// ([`tauten_check_kernel::registry::conserves`]) decides each entry. This
/// function only interns the entries.
#[must_use]
pub fn conservation<'a>(file: &'a ClaimsFile, record: &'a Record) -> Conservation {
    let recorded: Vec<&'a Claim> = record.claims.iter().map(|c| &c.claim).collect();
    conservation_of(file, &recorded)
}

/// [`conservation`] against a list of recorded claim declarations: a v1
/// record's claims, or the registry of a pending merge record (#50).
#[must_use]
pub fn conservation_of<'a>(file: &'a ClaimsFile, recorded: &[&'a Claim]) -> Conservation {
    use tauten_check_kernel::registry::{self, Entry};
    let mut ids: BTreeMap<RegistryKey<'a>, usize> = BTreeMap::new();
    let mut intern = |k: RegistryKey<'a>| -> usize {
        let next = ids.len();
        *ids.entry(k).or_insert(next)
    };
    let entries = |keys: Vec<RegistryKey<'a>>, intern: &mut dyn FnMut(RegistryKey<'a>) -> usize| {
        keys.into_iter()
            .map(|k| Entry {
                id: intern(k),
                strength: 0,
            })
            .collect::<Vec<_>>()
    };
    // An inherited claim's theorems and assumptions are the dependency's,
    // checked against its version (`inherited`), not this file's registry.
    let own = |c: &&&'a Claim| c.inherit.is_none();
    let recorded_keys: Vec<_> = recorded
        .iter()
        .filter(own)
        .flat_map(|c| claim_entries(c))
        .collect();
    let current_keys: Vec<_> = file
        .claim
        .iter()
        .filter(|c| c.inherit.is_none())
        .flat_map(claim_entries)
        .collect();
    let recorded_assumes: Vec<_> = recorded
        .iter()
        .filter(own)
        .flat_map(|c| assumption_entries(c))
        .collect();
    // Only claims already on record may not gain assumptions; a new claim
    // states its own.
    let current_assumes: Vec<_> = file
        .claim
        .iter()
        .filter(|c| {
            c.inherit.is_none()
                && recorded
                    .iter()
                    .any(|r| r.file == c.file && r.function == c.function)
        })
        .flat_map(assumption_entries)
        .collect();
    let amend_keys: Vec<RegistryKey<'a>> = file
        .amend
        .iter()
        .map(|a| {
            (
                a.file.as_str(),
                a.function.as_str(),
                a.kind,
                a.name.as_str(),
            )
        })
        .collect();

    let old = entries(recorded_keys.clone(), &mut intern);
    let new = entries(current_keys, &mut intern);
    let old_a = entries(recorded_assumes, &mut intern);
    let new_a = entries(current_assumes.clone(), &mut intern);
    let amended: Vec<usize> = amend_keys.iter().map(|k| intern(*k)).collect();

    let krate_of = |file_name: &str, function: &str| -> String {
        recorded
            .iter()
            .copied()
            .chain(file.claim.iter())
            .find(|c| c.file == file_name && c.function == function)
            .map(|c| c.krate.clone())
            .unwrap_or_default()
    };
    let mut out = Conservation::default();
    // May not shrink: each recorded theorem or mutant, decided alone so
    // the message can name it.
    for (key, entry) in recorded_keys.iter().zip(&old) {
        if !registry::conserves(core::slice::from_ref(entry), &new, &amended) {
            out.weakened.push((
                krate_of(key.0, key.1),
                key.0.to_owned(),
                key.1.to_owned(),
                key.2,
                key.3.to_owned(),
            ));
        }
    }
    // May not grow: the arguments swap.
    for (key, entry) in current_assumes.iter().zip(&new_a) {
        if !registry::conserves(core::slice::from_ref(entry), &old_a, &amended) {
            out.weakened.push((
                krate_of(key.0, key.1),
                key.0.to_owned(),
                key.1.to_owned(),
                key.2,
                key.3.to_owned(),
            ));
        }
    }
    // An amendment is needed only for an entry that would be refused
    // without it.
    for (i, (key, id)) in amend_keys.iter().zip(&amended).enumerate() {
        let probe = [Entry {
            id: *id,
            strength: 0,
        }];
        let needed = match key.2 {
            AmendKind::Theorem | AmendKind::Mutant => {
                registry::kept(&old, *id, 0) && !registry::conserves(&probe, &new, &[])
            }
            AmendKind::Assumption => {
                registry::kept(&new_a, *id, 0) && !registry::conserves(&probe, &old_a, &[])
            }
        };
        if !needed {
            out.unneeded.push(i);
        }
    }
    out
}

impl AmendKind {
    /// The kind as written in `[[amend]]`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Theorem => "theorem",
            Self::Mutant => "mutant",
            Self::Assumption => "assumption",
        }
    }
}

/// TAU-R002 diagnostics for [`conservation`]: an error per unexcused
/// weakening, a warning per unneeded amendment.
#[must_use]
pub fn conservation_diagnostics(
    profile: &crate::profile::Profile,
    file: &ClaimsFile,
    found: &Conservation,
) -> Vec<Diagnostic> {
    let Some(rule) = profile.rule("TAU-R002") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (krate, f, function, kind, name) in &found.weakened {
        let what = match kind {
            AmendKind::Theorem | AmendKind::Mutant => format!(
                "proofs/claims.toml drops {} `{name}` from the recorded claim `{function}`",
                kind.name()
            ),
            AmendKind::Assumption => format!(
                "proofs/claims.toml adds assumption `{name}` to the recorded claim `{function}`"
            ),
        };
        let message = format!(
            "{what}; a recorded claim may not be weakened without an `[[amend]]` entry with a reason"
        );
        out.push(
            Diagnostic::for_rule(rule, Severity::Error, krate, None, message).at(
                Some(f.clone()),
                None,
                None,
            ),
        );
    }
    for i in &found.unneeded {
        let Some(a) = file.amend.get(*i) else {
            continue;
        };
        let message = format!(
            "`[[amend]]` of {} `{}` in `{}` excuses nothing against proofs/claims.json; remove it",
            a.kind.name(),
            a.name,
            a.function
        );
        out.push(
            Diagnostic::for_rule(rule, Severity::Warning, "", None, message).at(
                Some(a.file.clone()),
                None,
                None,
            ),
        );
    }
    out
}

pub mod draft;
pub mod inputs;

#[cfg(test)]
mod tests;
