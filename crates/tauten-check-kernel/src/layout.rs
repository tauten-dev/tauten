//! Workspace layout rules: a declared tier (TAU-T001), the kernel's
//! dependency allowlist (TAU-T003) and `#![no_std]` in kernel crates
//! (TAU-K003).
//!
//! The shell reads each fact from the manifest or the crate root; a fact it
//! could not read is `Unparsed`, and an unread fact never satisfies a rule.
//!
//! Proved: each rule fires exactly when stated (`t001_iff`, `t003_iff`,
//! `k003_iff`); only an unconditional `#![no_std]` or
//! `#![cfg_attr(not(test), no_std)]` satisfies TAU-K003, so a `no_std`
//! under any other condition is a finding, and so is a crate root that did
//! not parse.

use crate::control::member;
use crate::tiers::Tier;

/// What a crate's manifest says about its tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Declared {
    /// A known tier.
    Known(Tier),
    /// A `tier` value that is not a tier.
    Unknown,
    /// No `tier` at all.
    Missing,
}

/// TAU-T001: whether the declaration is a finding.
#[must_use]
pub const fn t001(declared: Declared) -> bool {
    match declared {
        Declared::Known(_) => false,
        Declared::Unknown | Declared::Missing => true,
    }
}

/// TAU-T003: whether depending on the external crate `dep` is a finding
/// for a crate of tier `tier` whose allowlist is `allowed`.
#[must_use]
pub fn t003(tier: Tier, dep: usize, allowed: &[usize]) -> bool {
    match tier {
        Tier::Kernel => !member(allowed, dep),
        Tier::Core | Tier::Edge => false,
    }
}

/// How a crate root declares `no_std`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoStd {
    /// `#![no_std]`.
    Unconditional,
    /// `#![cfg_attr(not(test), no_std)]`: `no_std` whenever it is built
    /// as a library.
    NotTest,
    /// `no_std` inside any other `cfg_attr`.
    Conditional,
    /// No `no_std`.
    Absent,
    /// The crate root did not parse.
    Unparsed,
}

/// TAU-K003: whether the library root of a crate of tier `tier` is a
/// finding.
#[must_use]
pub const fn k003(tier: Tier, fact: NoStd) -> bool {
    match tier {
        Tier::Kernel => match fact {
            NoStd::Unconditional | NoStd::NotTest => false,
            NoStd::Conditional | NoStd::Absent | NoStd::Unparsed => true,
        },
        Tier::Core | Tier::Edge => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_rules() {
        assert!(!t001(Declared::Known(Tier::Core)));
        assert!(t001(Declared::Unknown) && t001(Declared::Missing));
        assert!(t003(Tier::Kernel, 3, &[1, 2]));
        assert!(!t003(Tier::Kernel, 2, &[1, 2]));
        assert!(!t003(Tier::Edge, 3, &[]));
        assert!(!k003(Tier::Kernel, NoStd::NotTest));
        assert!(k003(Tier::Kernel, NoStd::Conditional));
        assert!(k003(Tier::Kernel, NoStd::Unparsed));
        assert!(!k003(Tier::Core, NoStd::Absent));
    }
}
