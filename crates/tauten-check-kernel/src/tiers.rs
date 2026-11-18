//! The tier relation: which crate may depend on which.

/// How much authority a crate holds, from most to least restricted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// Pure, `no_std`, deterministic decision logic; eligible for proofs.
    Kernel,
    /// Library code without ambient authority.
    Core,
    /// Startup and IO; the only tier that may create capabilities.
    Edge,
}

/// The position of a tier, innermost first: kernel 0, core 1, edge 2.
#[must_use]
pub const fn rank(of: Tier) -> u8 {
    match of {
        Tier::Kernel => 0,
        Tier::Core => 1,
        Tier::Edge => 2,
    }
}

/// Whether a crate of tier `of` may depend on a crate of tier `dep`.
/// Dependencies point inward: edge -> core -> kernel, and a tier may depend
/// on its own tier.
#[must_use]
pub const fn may_depend_on(of: Tier, dep: Tier) -> bool {
    rank(dep) <= rank(of)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependencies_point_inward() {
        assert!(may_depend_on(Tier::Edge, Tier::Kernel));
        assert!(may_depend_on(Tier::Core, Tier::Core));
        assert!(!may_depend_on(Tier::Kernel, Tier::Core));
        assert!(!may_depend_on(Tier::Core, Tier::Edge));
    }
}
