//! Whether a claim states exactly the assumptions its theorems rest on, and
//! whether those assumptions can hold together.
//!
//! The audit derives, from the Lean statements, the named assumptions the
//! claim's theorems take as hypotheses (`derived`) and the ones its witness
//! theorem proves can hold (`established`). The claim declares its list in
//! `claims.toml` (`declared`). [`assumptions_ok`] decides, using the proved
//! [`crate::registry::conserves`] in both directions.
//!
//! Proved: accepted exactly when the declared and derived lists name the
//! same assumptions and, when there is any, the witness establishes every
//! declared one. A hypothesis the claim does not declare, a declared
//! assumption no theorem uses, and assumptions without a witness (which
//! could be contradictory, making every theorem vacuous) are refused.

use crate::registry::{Entry, conserves};

/// Whether `declared` matches `derived` and is witnessed by `established`.
#[must_use]
pub fn assumptions_ok(declared: &[Entry], derived: &[Entry], established: &[Entry]) -> bool {
    let same = conserves(declared, derived, &[]) && conserves(derived, declared, &[]);
    same && (declared.is_empty() || conserves(declared, established, &[]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn e(id: usize) -> Entry {
        Entry { id, strength: 0 }
    }

    #[test]
    fn assumptions() {
        assert!(assumptions_ok(&[], &[], &[]));
        assert!(assumptions_ok(&[e(1)], &[e(1)], &[e(1), e(2)]));
        assert!(assumptions_ok(
            &[e(1), e(2)],
            &[e(2), e(1), e(1)],
            &[e(2), e(1)]
        ));
        // A hypothesis the claim does not declare.
        assert!(!assumptions_ok(&[], &[e(1)], &[e(1)]));
        // A declared assumption no theorem uses.
        assert!(!assumptions_ok(&[e(1), e(2)], &[e(1)], &[e(1), e(2)]));
        // No witness.
        assert!(!assumptions_ok(&[e(1)], &[e(1)], &[]));
        assert!(!assumptions_ok(&[e(1), e(2)], &[e(1), e(2)], &[e(1)]));
    }
}
