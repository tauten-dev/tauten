//! A kernel crate with a dependency it did not allowlist.
#![no_std]

/// Whether a level blocks.
#[must_use]
pub const fn blocks(deny: bool) -> bool {
    deny
}
