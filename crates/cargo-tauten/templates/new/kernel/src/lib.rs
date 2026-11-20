//! Decisions, as total functions over plain data. No IO, no panics, no
//! dependencies. Each function here should have a claim in
//! `proofs/claims.toml`: a Lean theorem over its translation, and a mutant
//! that breaks the theorem.
#![no_std]

/// Whether a charge of `cost` fits a budget of which `spent` is already
/// used: `spent + cost <= budget`, computed without overflow.
///
/// Proved: `Proofs.{{MODULE}}.affordable_spec` (proofs/Proofs/{{MODULE}}.lean).
#[must_use]
pub fn affordable(budget: u64, spent: u64, cost: u64) -> bool {
    match spent.checked_add(cost) {
        Some(total) => total <= budget,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::affordable;

    #[test]
    fn fits_exactly_and_not_beyond() {
        assert!(affordable(10, 4, 6));
        assert!(!affordable(10, 4, 7));
        assert!(!affordable(u64::MAX, 1, u64::MAX));
    }
}
