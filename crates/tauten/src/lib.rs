//! Capability tokens, refinement types and checked contracts for the tauten
//! Rust strictness profile.
//!
//! * [`cap`]: authority is created once, in an edge crate, with
//!   [`cap::Authority::acquire`], and handed down as narrow capabilities
//!   ([`cap::Clock`], [`cap::Env`], and with the `fs` feature a directory
//!   handle). Core code takes those as parameters, so a signature shows what
//!   it can touch and tests can substitute fakes.
//! * [`newtype`] / [`refined`]: nominal wrappers whose field cannot be
//!   reached or built except through `new` / `try_new`. A refined type
//!   wraps only a [`refine::Frozen`] type, so the checked value cannot change.
//! * [`requires`] / [`ensures`]: contracts checked on every call in every
//!   build profile. A violation is returned as an error, never a panic.
//!
//! What this crate does **not** do: it does not sandbox anything. Code that
//! calls `std::fs` directly still can; the profile's lints are what stop that
//! in `kernel` and `core` crates, and dependencies are not covered.

pub mod cap;
pub mod contract;
mod error;
pub mod refine;

pub use error::{ContractViolation, RefineError};
pub use tauten_macros::{ensures, newtype, refined, requires};
