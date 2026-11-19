//! The badge a claim earns: what it is honest to say about it.
//!
//! "Proved" is reserved for a Lean theorem over the extracted function,
//! established by the gate, still fresh, and resting on no assumption about
//! the outside world. Everything else gets the weaker badge it earns:
//! proved under named assumptions, proved about a model only, bounded
//! (model checking, exploration), tested, reviewed, pending (the inputs
//! changed since it was established) or not established.

/// How a claim was established.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// A Lean theorem.
    Lean,
    /// A bounded search (model checker, explorer).
    Bounded,
    /// Tests.
    Test,
    /// Human or agent review.
    Review,
}

/// What a theorem is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    /// The translation of the Rust function itself.
    Extracted,
    /// A model written by hand, not tied to the code.
    ModelOnly,
}

/// What may be said about a claim, strongest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Badge {
    /// Proved about the code, with no assumption about the outside world.
    Proved,
    /// Proved about the code, under the claim's named assumptions.
    ProvedUnder,
    /// Proved about a model, not tied to the code.
    ModelProved,
    /// Checked by a bounded search.
    Bounded,
    /// Tested.
    Tested,
    /// Reviewed.
    Reviewed,
    /// Was established, but its inputs changed since.
    Pending,
    /// Not established.
    NotEstablished,
}

/// The badge of a claim.
///
/// `established`: the gate established it (every control admitted).
/// `fresh`: its inputs are unchanged since ([`crate::binding::fresh`]).
/// `assumes`: it lists at least one named assumption.
#[must_use]
pub const fn badge(
    established: bool,
    fresh: bool,
    method: Method,
    relation: Relation,
    assumes: bool,
) -> Badge {
    if !established {
        return Badge::NotEstablished;
    }
    if !fresh {
        return Badge::Pending;
    }
    match (method, relation) {
        (Method::Lean, Relation::Extracted) => {
            if assumes {
                Badge::ProvedUnder
            } else {
                Badge::Proved
            }
        }
        (Method::Lean, Relation::ModelOnly) => Badge::ModelProved,
        (Method::Bounded, Relation::Extracted | Relation::ModelOnly) => Badge::Bounded,
        (Method::Test, Relation::Extracted | Relation::ModelOnly) => Badge::Tested,
        (Method::Review, Relation::Extracted | Relation::ModelOnly) => Badge::Reviewed,
    }
}

/// The badge's rank, strongest highest.
#[must_use]
pub const fn rank(b: Badge) -> u8 {
    match b {
        Badge::Proved => 7,
        Badge::ProvedUnder => 6,
        Badge::ModelProved => 5,
        Badge::Bounded => 4,
        Badge::Tested => 3,
        Badge::Reviewed => 2,
        Badge::Pending => 1,
        Badge::NotEstablished => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badges() {
        let b = |e, f, m, r, a| badge(e, f, m, r, a);
        assert_eq!(
            b(true, true, Method::Lean, Relation::Extracted, false),
            Badge::Proved
        );
        assert_eq!(
            b(true, true, Method::Lean, Relation::Extracted, true),
            Badge::ProvedUnder
        );
        assert_eq!(
            b(true, false, Method::Lean, Relation::Extracted, false),
            Badge::Pending
        );
        assert_eq!(
            b(false, true, Method::Lean, Relation::Extracted, false),
            Badge::NotEstablished
        );
        assert_eq!(
            b(true, true, Method::Lean, Relation::ModelOnly, false),
            Badge::ModelProved
        );
        assert_eq!(
            b(true, true, Method::Bounded, Relation::Extracted, false),
            Badge::Bounded
        );
        assert!(rank(Badge::Proved) > rank(Badge::ProvedUnder));
    }
}
