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
}
