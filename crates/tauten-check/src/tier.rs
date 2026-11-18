//! Tiers and lint levels.

use core::fmt;
use serde::{Deserialize, Serialize};
use tauten_check_kernel::{severity as kernel_severity, tiers as kernel_tiers};

/// How much authority a crate holds. Ordered from most to least restricted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    /// Pure, `no_std`, deterministic decision logic; eligible for proofs.
    Kernel,
    /// Library code without ambient authority; effects arrive as parameters.
    Core,
    /// Startup and IO; the only tier that may create capabilities.
    Edge,
}

impl Tier {
    /// All tiers, innermost first.
    pub const ALL: [Self; 3] = [Self::Kernel, Self::Core, Self::Edge];

    /// Parse the value of `[package.metadata.tauten] tier`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "kernel" => Some(Self::Kernel),
            "core" => Some(Self::Core),
            "edge" => Some(Self::Edge),
            _ => None,
        }
    }

    /// The lowercase name used in manifests and output.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Kernel => "kernel",
            Self::Core => "core",
            Self::Edge => "edge",
        }
    }

    /// Whether a crate of this tier may depend on a crate of tier `dep`.
    /// Dependencies must point inward: edge -> core -> kernel. Decided by
    /// the proved kernel ([`kernel_tiers::may_depend_on`]).
    #[must_use]
    pub fn may_depend_on(self, dep: Self) -> bool {
        kernel_tiers::may_depend_on(self.kernel(), dep.kernel())
    }

    /// The same tier in the decision kernel.
    #[must_use]
    pub(crate) const fn kernel(self) -> kernel_tiers::Tier {
        match self {
            Self::Kernel => kernel_tiers::Tier::Kernel,
            Self::Core => kernel_tiers::Tier::Core,
            Self::Edge => kernel_tiers::Tier::Edge,
        }
    }
}

impl fmt::Display for Tier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A lint level as passed to rustc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    /// Report, do not fail.
    Warn,
    /// Fail; may be overridden by `#[expect(lint, reason = "...")]` unless
    /// the rule admits no exceptions.
    Deny,
}

impl Level {
    /// The rustc command-line flag for this level.
    #[must_use]
    pub const fn flag(self) -> &'static str {
        match self {
            Self::Warn => "-W",
            Self::Deny => "-D",
        }
    }

    /// Whether a violation at this level fails the check. Decided by the
    /// proved kernel ([`kernel_severity::is_blocking`]).
    #[must_use]
    pub const fn is_blocking(self) -> bool {
        kernel_severity::is_blocking(self.kernel())
    }

    /// The same level in the decision kernel.
    #[must_use]
    pub(crate) const fn kernel(self) -> kernel_severity::Level {
        match self {
            Self::Warn => kernel_severity::Level::Warn,
            Self::Deny => kernel_severity::Level::Deny,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for tier in Tier::ALL {
            assert_eq!(Tier::parse(tier.name()), Some(tier));
            assert_eq!(tier.to_string(), tier.name());
        }
        assert_eq!(Tier::Kernel.name(), "kernel");
        assert_eq!(Tier::parse("Kernel"), None);
    }

    /// The kernel decides exactly as the derived order did before it moved.
    #[test]
    fn kernel_agrees_with_the_derived_order() {
        for tier in Tier::ALL {
            for dep in Tier::ALL {
                assert_eq!(tier.may_depend_on(dep), dep <= tier, "{tier} -> {dep}");
            }
        }
        assert!(!Level::Warn.is_blocking());
        assert!(Level::Deny.is_blocking());
    }

    #[test]
    fn dependencies_point_inward() {
        assert!(Tier::Edge.may_depend_on(Tier::Kernel));
        assert!(Tier::Core.may_depend_on(Tier::Core));
        assert!(!Tier::Kernel.may_depend_on(Tier::Core));
        assert!(!Tier::Core.may_depend_on(Tier::Edge));
    }
}
