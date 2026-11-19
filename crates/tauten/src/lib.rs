//! The tauten discipline for Rust.
//!
//! * **Authority is a parameter** ([`cap`]). Authority is created once, in
//!   an edge crate, with `cap::Authority::acquire`, and handed down as narrow
//!   capabilities: [`cap::Clock`], `cap::Env`, and with the `fs` feature a
//!   directory or a read-only `cap::ReadDir`. Group them in a
//!   [`#[derive(Cap)]`](derive@Cap) bundle.
//! * **Effects are visible in signatures** ([`effects`]). A function's
//!   capability parameters bound what it can do; `#[effects(clock, fs_read)]`
//!   states that row, and `cargo tauten check` compares it with the
//!   signature (TAU-F001).
//! * **Check, then act, with evidence** ([`origin`]). A check returns
//!   [`origin::Evidence`] that borrows the state it checked, so the state
//!   cannot change before the action consumes it.
//! * **Valid by construction** ([`newtype`], [`refined`]). Nominal wrappers
//!   whose field cannot be reached or built except through `new` /
//!   `try_new`. A refined type wraps only a [`refine::Frozen`] type, so the
//!   checked value cannot change.
//! * **Contracts** ([`requires`], [`ensures`]): checked on every call in
//!   every build profile. A violation is returned as an error, never a panic.
//!
//! Without the default `std` feature the crate is `no_std` + `alloc`:
//! everything except the ambient-authority wrappers works in kernel crates.
//!
//! What this crate does **not** do: it does not sandbox anything. Code that
//! calls `std::fs` directly still can; the profile's lints are what stop that
//! in `kernel` and `core` crates, and dependencies are not covered.

#![no_std]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod cap;
pub mod contract;
mod error;
pub mod origin;
pub mod refine;

pub use error::{ContractViolation, RefineError};
pub use tauten_macros::{Cap, effects, ensures, evidence, newtype, refined, requires};

/// The effect labels `#[effects(..)]` accepts, and what each means.
///
/// | Label | The function may… | Typical parameter |
/// |---|---|---|
/// | `clock` | read the time | `&impl Clock` |
/// | `env` | read environment variables | `&Env` |
/// | `fs_read` | read files | `&ReadDir`, `&Dir` |
/// | `fs_write` | write files | `&Dir` |
/// | `net` | use the network | a network capability |
/// | `fail` | return an error | a `Result` return type |
/// | `suspend` | await | `async fn` |
///
/// `#[effects()]` declares a function with no effects.
///
/// ```
/// use tauten::cap::Clock;
///
/// #[tauten::effects(clock)]
/// pub fn age_ms(clock: &impl Clock, created_ms: u64) -> u64 {
///     clock.now_ms().saturating_sub(created_ms)
/// }
/// ```
///
/// ```compile_fail
/// #[tauten::effects(time)] // unknown label
/// pub fn f() {}
/// ```
pub mod effects {
    /// Every label `#[effects(..)]` accepts.
    pub const LABELS: &[&str] = &[
        "clock", "env", "fs_read", "fs_write", "net", "fail", "suspend",
    ];
}
