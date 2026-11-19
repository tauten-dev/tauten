//! Whether a proof subsumes an arithmetic or indexing finding (P5).
//!
//! In a kernel crate, a function whose claim in `proofs/claims.toml` was
//! proved by the project's proof gate, for the current source, does not
//! need the lints that keep arithmetic and indexing from panicking: the
//! proof shows they never fail. `cargo tauten check` reports such findings
//! as subsumed by the proof instead of as errors. Everything else is
//! reported as before.

use crate::tiers::Tier;

/// What a finding is about, as far as a proof can subsume it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finding {
    /// TAU-N001: unchecked arithmetic or integer division.
    Arithmetic,
    /// TAU-P003: indexing or slicing that can panic.
    Indexing,
    /// Any other rule.
    Other,
}

/// Whether the finding is subsumed by a proof.
///
/// `tier` is the crate's tier; `listed` whether the enclosing function's
/// claim names the finding's rule under `subsumes`; `proved` whether the
/// recorded gate result for that claim is `proved` with a total no-failure
/// theorem; `fresh` whether the result was recorded for the current source
/// (the fingerprint matches).
#[must_use]
pub const fn subsumes(
    tier: Tier,
    finding: Finding,
    listed: bool,
    proved: bool,
    fresh: bool,
) -> bool {
    let kind = match finding {
        Finding::Arithmetic | Finding::Indexing => true,
        Finding::Other => false,
    };
    let kernel = match tier {
        Tier::Kernel => true,
        Tier::Core | Tier::Edge => false,
    };
    kernel && kind && listed && proved && fresh
}
