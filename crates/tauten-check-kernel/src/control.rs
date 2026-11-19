//! Whether a set of checks passed: admission of an exact set of controls.
//!
//! A control is anything a gate runs and whose result it expects: a theorem
//! that must build (`Expect::Pass`), or a negative control that must fail
//! for its named reason (`Expect::Fail`): a source mutant, a negative
//! model-checker configuration, a reject fixture, a planted fault. The
//! shell reads each result into an [`Observation`]; [`admit`] decides.
//!
//! Proved: the set is admitted exactly when the inventory is non-empty,
//! every observation is of a required control and occurs once, and every
//! required control was observed with the result it expects. A timeout, a
//! missing tool, an empty run, a skipped control or a failure for another
//! reason never satisfies any control.

/// What a control must show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expect {
    /// It must pass (a theorem builds, a test passes).
    Pass,
    /// It must fail for its named reason (a negative control).
    Fail,
}

/// What a run of one control showed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seen {
    /// It passed.
    Pass,
    /// It failed for (one of) its named reasons.
    FailNamed,
    /// It failed, but not for a named reason.
    FailOther,
    /// It did not finish in time.
    Timeout,
    /// The tool it needs was not available.
    ToolMissing,
    /// It ran nothing (an empty inventory, zero tests).
    Empty,
    /// It was not run.
    NotRun,
}

/// A required control, by interned identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Control {
    /// Identifier, interned by the shell.
    pub id: usize,
    /// What it must show.
    pub expect: Expect,
}

/// One observed result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Observation {
    /// Identifier of the control it is a result of.
    pub id: usize,
    /// What it showed.
    pub seen: Seen,
}

/// Whether `seen` is the result `expect` asks for: a pass for a pass, a
/// named failure for a negative control, nothing else.
#[must_use]
pub const fn control_ok(expect: Expect, seen: Seen) -> bool {
    match (expect, seen) {
        (Expect::Pass, Seen::Pass) | (Expect::Fail, Seen::FailNamed) => true,
        (
            Expect::Pass,
            Seen::FailNamed
            | Seen::FailOther
            | Seen::Timeout
            | Seen::ToolMissing
            | Seen::Empty
            | Seen::NotRun,
        )
        | (
            Expect::Fail,
            Seen::Pass
            | Seen::FailOther
            | Seen::Timeout
            | Seen::ToolMissing
            | Seen::Empty
            | Seen::NotRun,
        ) => false,
    }
}

/// Whether `id` is among `ids` (for "failed for a named reason": some
/// declared reason is among the broken ones).
#[must_use]
pub fn member(ids: &[usize], id: usize) -> bool {
    let mut found = false;
    let mut i: usize = 0;
    while i < ids.len() {
        if let Some(x) = ids.get(i) {
            found = found || *x == id;
        }
        i = i.saturating_add(1);
    }
    found
}

/// Whether some declared reason is among the broken ones.
#[must_use]
pub fn named(declared: &[usize], broken: &[usize]) -> bool {
    let mut found = false;
    let mut i: usize = 0;
    while i < declared.len() {
        if let Some(x) = declared.get(i) {
            found = found || member(broken, *x);
        }
        i = i.saturating_add(1);
    }
    found
}

/// How many observations are of control `id`.
#[must_use]
pub fn count_id(observed: &[Observation], id: usize) -> usize {
    let mut count: usize = 0;
    let mut i: usize = 0;
    while i < observed.len() {
        if let Some(o) = observed.get(i)
            && o.id == id
        {
            count = count.saturating_add(1);
        }
        i = i.saturating_add(1);
    }
    count
}

/// Whether some required control has identifier `id`.
#[must_use]
pub fn required_id(required: &[Control], id: usize) -> bool {
    let mut found = false;
    let mut i: usize = 0;
    while i < required.len() {
        if let Some(c) = required.get(i) {
            found = found || c.id == id;
        }
        i = i.saturating_add(1);
    }
    found
}

/// Whether control `id` was observed with the result `expect`.
#[must_use]
pub fn met(observed: &[Observation], id: usize, expect: Expect) -> bool {
    let mut found = false;
    let mut i: usize = 0;
    while i < observed.len() {
        if let Some(o) = observed.get(i) {
            found = found || (o.id == id && control_ok(expect, o.seen));
        }
        i = i.saturating_add(1);
    }
    found
}

/// Whether every observation is of a required control and occurs once.
#[must_use]
pub fn observations_ok(required: &[Control], observed: &[Observation]) -> bool {
    let mut ok = true;
    let mut i: usize = 0;
    while i < observed.len() {
        if let Some(o) = observed.get(i) {
            ok = ok && count_id(observed, o.id) == 1 && required_id(required, o.id);
        }
        i = i.saturating_add(1);
    }
    ok
}

/// Whether every required control was met.
#[must_use]
pub fn controls_ok(required: &[Control], observed: &[Observation]) -> bool {
    let mut ok = true;
    let mut i: usize = 0;
    while i < required.len() {
        if let Some(c) = required.get(i) {
            ok = ok && met(observed, c.id, c.expect);
        }
        i = i.saturating_add(1);
    }
    ok
}

/// Whether the observed results admit the required set of controls.
#[must_use]
pub fn admit(required: &[Control], observed: &[Observation]) -> bool {
    !required.is_empty() && observations_ok(required, observed) && controls_ok(required, observed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const THEOREM: Control = Control {
        id: 1,
        expect: Expect::Pass,
    };
    const MUTANT: Control = Control {
        id: 2,
        expect: Expect::Fail,
    };

    fn seen(id: usize, seen: Seen) -> Observation {
        Observation { id, seen }
    }

    #[test]
    fn admission() {
        let req = [THEOREM, MUTANT];
        let good = [seen(2, Seen::FailNamed), seen(1, Seen::Pass)];
        assert!(admit(&req, &good));
        assert!(!admit(&[], &[]));
        assert!(!admit(&req, &[seen(1, Seen::Pass)]));
        assert!(!admit(&req, &[seen(1, Seen::Pass), seen(2, Seen::Timeout)]));
        assert!(!admit(
            &req,
            &[seen(1, Seen::Pass), seen(2, Seen::FailOther)]
        ));
        assert!(!admit(
            &req,
            &[
                seen(1, Seen::Pass),
                seen(2, Seen::FailNamed),
                seen(2, Seen::FailNamed)
            ]
        ));
        assert!(!admit(
            &req,
            &[
                seen(1, Seen::Pass),
                seen(2, Seen::FailNamed),
                seen(3, Seen::Pass)
            ]
        ));
        assert!(named(&[4, 5], &[9, 5]));
        assert!(!named(&[4, 5], &[9]));
    }
}
