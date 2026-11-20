//! Workspace build settings the profile requires, read from manifests.
//!
//! A setting such as `overflow-checks` in `[profile.release]` is read by the
//! shell into a [`Fact`]: set as required, set otherwise (or not set), or
//! present but not understood. [`hardening`] decides whether that is a
//! finding.
//!
//! Proved: a finding exactly when the rule is required and the fact is not
//! [`Fact::Yes`]; a setting tauten could not read is never clean.

/// What the manifests say about one required setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fact {
    /// Set as the profile requires, with no override undoing it.
    Yes,
    /// Not set, set otherwise, or undone by an override.
    No,
    /// Present but not understood (not a boolean, an unreadable config).
    Unparsed,
}

/// Whether the setting is a finding.
#[must_use]
pub const fn hardening(required: bool, fact: Fact) -> bool {
    match fact {
        Fact::Yes => false,
        Fact::No | Fact::Unparsed => required,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn findings() {
        assert!(hardening(true, Fact::No));
        assert!(hardening(true, Fact::Unparsed));
        assert!(!hardening(true, Fact::Yes));
        assert!(!hardening(false, Fact::No));
        assert!(!hardening(false, Fact::Unparsed));
    }
}
