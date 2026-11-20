//! Instances of the `#[refined]`, `#[newtype]`, `#[requires]` and
//! `#[ensures]` macros, written exactly as a user would write them.
//!
//! The proofs gate translates the macros' real expansion of this crate to
//! Lean 4 (Charon, then Aeneas) and proves, per instance, that the generated
//! constructor accepts exactly the values satisfying the predicate and keeps
//! them unchanged, and that the generated contract wrapper checks the
//! precondition before the body, the postcondition on the body's `Ok` value
//! against the pre-state snapshot, and otherwise returns the body's result
//! (`proofs/Tauten/Refined.lean`, `proofs/Tauten/Contracts.lean`,
//! `docs/proofs/README.md`). These are proofs for these instances, not for
//! the macros in general. Never published.
#![no_std]

pub mod contracts;
pub mod machines;
pub mod refined;
