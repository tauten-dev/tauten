//! What the proof gate established for one claim (`cargo tauten proofs`).
//!
//! The shell reads the gate's result lines into [`Facts`]: whether the gate
//! ran, each stage's status, and whether the claim's theorems, its `total`
//! theorem and its mutants passed. [`outcome`] decides. It is proved that
//! a claim is [`Outcome::Proved`] exactly when every fact is good, and that
//! otherwise the outcome names the first failure in gate order.

use crate::control::{Control, Observation, admit};

/// One gate stage as reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// The stage reported `ok`.
    Ok,
    /// The stage reported a failure (or, for `tools`, `missing`).
    Failed,
    /// The stage did not report.
    Absent,
}

/// What the gate established.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Every stage passed, the theorems were audited, the `total` theorem
    /// (if any) has the no-failure shape, every mutant was killed, and the
    /// sources did not change while the gate ran.
    Proved,
    /// The translation drifted, or the sources changed during the run.
    Drift,
    /// Translation or `lake build` failed.
    BuildFailed,
    /// A theorem is missing or uses a non-standard axiom, or the `total`
    /// theorem has the wrong shape.
    AuditFailed,
    /// A mutant broke none of the claim's theorems.
    MutantSurvived,
    /// The gate did not run.
    ToolMissing,
}

/// The facts the shell reads from the gate's output for one claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Facts {
    /// The gate started and printed at least one stage line.
    pub ran: bool,
    /// The `tools` stage.
    pub tools: Stage,
    /// The `extract` stage.
    pub extract: Stage,
    /// The `drift` stage.
    pub drift: Stage,
    /// The `build` stage.
    pub build: Stage,
    /// The `audit` stage.
    pub audit: Stage,
    /// Every theorem of the claim passed the axiom audit.
    pub theorems: bool,
    /// The claim has no `total` theorem, or it has the no-failure shape.
    pub total: bool,
    /// Every mutant of the claim broke one of its theorems.
    pub mutants: bool,
    /// The sources changed while the gate ran.
    pub changed: bool,
}

/// Whether a stage passed.
#[must_use]
pub const fn passed(stage: Stage) -> bool {
    match stage {
        Stage::Ok => true,
        Stage::Failed | Stage::Absent => false,
    }
}

/// The outcome: the first failure in gate order, or [`Outcome::Proved`].
#[must_use]
pub const fn outcome(f: Facts) -> Outcome {
    if !f.ran || !passed(f.tools) {
        Outcome::ToolMissing
    } else if !passed(f.extract) {
        Outcome::BuildFailed
    } else if !passed(f.drift) {
        Outcome::Drift
    } else if !passed(f.build) {
        Outcome::BuildFailed
    } else if !passed(f.audit) || !f.theorems || !f.total {
        Outcome::AuditFailed
    } else if !f.mutants {
        Outcome::MutantSurvived
    } else if f.changed {
        Outcome::Drift
    } else {
        Outcome::Proved
    }
}

/// The outcome of one claim from a whole run: the stage facts, with the
/// claim's theorems and mutants admitted as two sets of controls
/// ([`admit`]). A claim with no theorem or no mutant is never proved.
#[must_use]
pub fn of_run(
    stages: Facts,
    theorems: &[Control],
    theorem_results: &[Observation],
    mutants: &[Control],
    mutant_results: &[Observation],
) -> Outcome {
    outcome(Facts {
        theorems: admit(theorems, theorem_results),
        mutants: admit(mutants, mutant_results),
        ..stages
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: Facts = Facts {
        ran: true,
        tools: Stage::Ok,
        extract: Stage::Ok,
        drift: Stage::Ok,
        build: Stage::Ok,
        audit: Stage::Ok,
        theorems: true,
        total: true,
        mutants: true,
        changed: false,
    };

    #[test]
    fn outcomes() {
        assert_eq!(outcome(GOOD), Outcome::Proved);
        assert_eq!(outcome(Facts { ran: false, ..GOOD }), Outcome::ToolMissing);
        assert_eq!(
            outcome(Facts {
                drift: Stage::Failed,
                mutants: false,
                ..GOOD
            }),
            Outcome::Drift
        );
        assert_eq!(
            outcome(Facts {
                build: Stage::Absent,
                ..GOOD
            }),
            Outcome::BuildFailed
        );
        assert_eq!(
            outcome(Facts {
                total: false,
                ..GOOD
            }),
            Outcome::AuditFailed
        );
        assert_eq!(
            outcome(Facts {
                mutants: false,
                ..GOOD
            }),
            Outcome::MutantSurvived
        );
        assert_eq!(
            outcome(Facts {
                changed: true,
                ..GOOD
            }),
            Outcome::Drift
        );
    }
}
