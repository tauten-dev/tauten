//! Lint exceptions: whether an `#[expect]`/`#[allow]` of a profile lint is
//! accepted.

use crate::severity::{Level, is_blocking};
use crate::tiers::Tier;

/// What tauten makes of an exception for a profile lint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Excuse {
    /// Accepted.
    Allowed,
    /// Accepted by rustc, but crate-wide for a blocking lint (TAU-X002).
    CrateWide,
    /// Not permitted at all (TAU-X004).
    Forbidden,
}

/// Whether the rule's lints may be excused in a crate of tier `tier`.
///
/// `rule_allows` is the rule's `exceptions` flag. A kernel crate never
/// excuses a lint whose rule has a level in the kernel tier.
#[must_use]
pub const fn permitted(rule_allows: bool, tier: Tier, level: Option<Level>) -> bool {
    if !rule_allows {
        return false;
    }
    match level {
        None => true,
        Some(_) => match tier {
            Tier::Kernel => false,
            Tier::Core | Tier::Edge => true,
        },
    }
}

/// The decision for one exception of a lint owned by a rule whose
/// `exceptions` flag is `rule_allows` and whose level in the crate's tier is
/// `level`; `crate_wide` is set for an inner attribute (`#![expect(..)]`).
#[must_use]
pub const fn excuse(
    rule_allows: bool,
    tier: Tier,
    level: Option<Level>,
    crate_wide: bool,
) -> Excuse {
    if !permitted(rule_allows, tier, level) {
        return Excuse::Forbidden;
    }
    let blocking = match level {
        None => false,
        Some(given) => is_blocking(given),
    };
    if crate_wide && blocking {
        Excuse::CrateWide
    } else {
        Excuse::Allowed
    }
}

/// How bad an excuse is: `Allowed` < `CrateWide` < `Forbidden`.
#[must_use]
pub const fn rank(e: Excuse) -> u8 {
    match e {
        Excuse::Allowed => 0,
        Excuse::CrateWide => 1,
        Excuse::Forbidden => 2,
    }
}

/// The worse of two excuses.
#[must_use]
pub const fn worse(a: Excuse, b: Excuse) -> Excuse {
    if rank(b) > rank(a) { b } else { a }
}

/// A profile rule an exception may cover: its `exceptions` flag and its
/// level in the crate's tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Covered {
    /// The rule's `exceptions` flag.
    pub allows: bool,
    /// The rule's level in the crate's tier.
    pub level: Option<Level>,
}

/// How an exception attribute is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Form {
    /// An inner attribute (`#![expect(..)]`).
    pub crate_wide: bool,
    /// Inside `#[cfg_attr(..)]`: in force only under some configuration.
    pub conditional: bool,
}

/// The decision for one exception attribute that may cover the rules
/// `covered`: one rule for a single lint, every rule that may own a member
/// of the group for a lint group (`clippy::all`, `warnings`).
///
/// A conditional exception (`cfg_attr`) is never accepted in a kernel
/// crate: proofs and checks see one configuration, so the excused code is
/// not the code that was checked. Otherwise the worst decision over every
/// covered rule stands, so a group is never weaker than any lint it may
/// contain.
#[must_use]
pub fn decide(form: Form, covered: &[Covered], tier: Tier) -> Excuse {
    let kernel = match tier {
        Tier::Kernel => true,
        Tier::Core | Tier::Edge => false,
    };
    if form.conditional && kernel {
        Excuse::Forbidden
    } else {
        worst(covered, tier, form.crate_wide)
    }
}

/// The worst [`excuse`] over every covered rule (`Allowed` for none).
#[must_use]
pub fn worst(covered: &[Covered], tier: Tier, crate_wide: bool) -> Excuse {
    let mut acc = Excuse::Allowed;
    let mut i: usize = 0;
    while i < covered.len() {
        if let Some(c) = covered.get(i) {
            acc = worse(acc, excuse(c.allows, tier, c.level, crate_wide));
        }
        i = i.saturating_add(1);
    }
    acc
}

/// A level a Cargo manifest's `[lints]` table sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    /// `allow`.
    Allow,
    /// `warn`.
    Warn,
    /// `expect`.
    Expect,
    /// `deny`.
    Deny,
    /// `forbid`.
    Forbid,
}

/// Whether a manifest setting lowers a lint below `deny`.
#[must_use]
pub const fn weakens(setting: Setting) -> bool {
    match setting {
        Setting::Allow | Setting::Warn | Setting::Expect => true,
        Setting::Deny | Setting::Forbid => false,
    }
}

/// TAU-X003: whether a manifest `[lints]` entry loosens the profile.
/// `levels` are the tier's levels of every profile rule the entry may
/// cover (one for a lint, several for a group). It does when it weakens
/// and at least one covered rule blocks.
#[must_use]
pub fn loosens(setting: Setting, levels: &[Level]) -> bool {
    if !weakens(setting) {
        return false;
    }
    let mut found = false;
    let mut i: usize = 0;
    while i < levels.len() {
        if let Some(l) = levels.get(i) {
            found = found || is_blocking(*l);
        }
        i = i.saturating_add(1);
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_never_excuses_a_levelled_rule() {
        for level in [Level::Warn, Level::Deny] {
            for crate_wide in [false, true] {
                assert_eq!(
                    excuse(true, Tier::Kernel, Some(level), crate_wide),
                    Excuse::Forbidden
                );
            }
        }
        assert_eq!(excuse(true, Tier::Kernel, None, true), Excuse::Allowed);
    }

    #[test]
    fn crate_wide_blocking_is_reported() {
        assert_eq!(
            excuse(true, Tier::Core, Some(Level::Deny), true),
            Excuse::CrateWide
        );
        assert_eq!(
            excuse(true, Tier::Core, Some(Level::Warn), true),
            Excuse::Allowed
        );
        assert_eq!(
            excuse(true, Tier::Edge, Some(Level::Deny), false),
            Excuse::Allowed
        );
        assert_eq!(excuse(false, Tier::Edge, None, false), Excuse::Forbidden);
    }

    const fn c(allows: bool, level: Option<Level>) -> Covered {
        Covered { allows, level }
    }

    #[test]
    fn groups_and_conditions() {
        let plain = Form {
            crate_wide: false,
            conditional: false,
        };
        let cond = Form {
            crate_wide: false,
            conditional: true,
        };
        let unlevelled = [c(true, None)];
        assert_eq!(decide(cond, &unlevelled, Tier::Kernel), Excuse::Forbidden);
        assert_eq!(decide(plain, &unlevelled, Tier::Kernel), Excuse::Allowed);
        assert_eq!(decide(cond, &unlevelled, Tier::Core), Excuse::Allowed);
        let group = [c(true, Some(Level::Warn)), c(false, Some(Level::Deny))];
        assert_eq!(decide(plain, &group, Tier::Edge), Excuse::Forbidden);
        assert_eq!(decide(plain, &[], Tier::Edge), Excuse::Allowed);
        let wide = Form {
            crate_wide: true,
            conditional: false,
        };
        assert_eq!(
            decide(
                wide,
                &[c(true, Some(Level::Warn)), c(true, Some(Level::Deny))],
                Tier::Core
            ),
            Excuse::CrateWide
        );
    }

    #[test]
    fn manifest_loosening() {
        assert!(loosens(Setting::Allow, &[Level::Warn, Level::Deny]));
        assert!(!loosens(Setting::Allow, &[Level::Warn]));
        assert!(!loosens(Setting::Deny, &[Level::Deny]));
        assert!(!loosens(Setting::Warn, &[]));
    }
}
