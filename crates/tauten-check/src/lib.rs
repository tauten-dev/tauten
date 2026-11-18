//! Pure analysis for the tauten strictness profile.
//!
//! This crate performs no IO. It turns the embedded rule table into lint
//! flags and clippy configuration for each tier, maps compiler messages to
//! stable rule IDs, and runs structural checks over a workspace model that
//! the caller (normally `cargo-tauten`) builds from disk.

pub mod diag;
pub mod profile;
pub mod scan;
pub mod structure;
pub mod tier;

pub use diag::{CompilerMessage, Diagnostic, EnforcedBy, Severity};
pub use profile::{Profile, ProfileError, Rule};
pub use structure::{Member, SourceFile, Workspace};
pub use tier::{Level, Tier};
