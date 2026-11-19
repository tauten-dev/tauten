//! The decisions of the tauten checker, in plain `no_std` Rust.
//!
//! `tauten-check` parses manifests, sources and compiler output, and calls
//! these functions for every decision it makes about tiers, severities,
//! lint exceptions, the baseline ratchet, the well-formedness of
//! the rule table and which findings a proof subsumes. The functions named in
//! `[package.metadata.charon] start_from` are translated to Lean 4 by Charon
//! and Aeneas, and the theorems under `proofs/Tauten` are stated over that
//! translation (see `docs/proofs/README.md`).
//!
//! The code is written for translation: no derives are reached from the
//! translated functions (enums are compared with `match`), no integer
//! casts, no iterators or closures (explicit `while` loops over `get`), no
//! nested borrows, and no parameter shares the name of its module.
#![no_std]

extern crate alloc;

pub mod control;
pub mod effects;
pub mod exception;
pub mod outcome;
pub mod ratchet;
pub mod severity;
pub mod span;
pub mod subsume;
pub mod table;
pub mod tiers;
