//! The decisions of the tauten checker, in plain `no_std` Rust.
//!
//! `tauten-check` parses manifests, sources and compiler output, and calls
//! these functions for every decision it makes about tiers, severities and
//! lint exceptions. The functions named in `[package.metadata.charon]
//! start_from` are translated to Lean 4 by Charon and Aeneas, and the
//! theorems under `proofs/Tauten/Kernel` are stated over that translation
//! (see `docs/proofs/README.md`).
//!
//! The code is written for translation: no derives are reached from the
//! translated functions (enums are compared with `match`), no integer
//! casts, no loops, no references, and no parameter shares the name of its
//! module.
#![no_std]

pub mod exception;
pub mod severity;
pub mod tiers;
