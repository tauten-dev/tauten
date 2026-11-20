//! A stamped kernel action (A5): `bump` acts on a [`Counter`] only when the
//! stamp it is given still matches the counter's version. Proved in
//! `proofs/Tauten/Stamped.lean`: a stale stamp returns `Stale` and leaves
//! the counter unchanged (`stale_refused`).

use tauten::evidence;
use tauten::origin::{Stale, Stamped, Versioned};

/// A bounded counter whose every change bumps `version`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counter {
    /// The count.
    pub value: u64,
    /// The count may not reach this.
    pub limit: u64,
    /// Changes with every update.
    pub version: u64,
}

impl Versioned for Counter {
    fn version(&self) -> u64 {
        self.version
    }
}

/// The counter is below its limit.
#[evidence(subject = Counter, stamped)]
pub struct BelowLimit {
    value: u64,
}

/// Check now; act later with the stamp.
#[must_use]
pub fn check(counter: &Counter) -> Option<Stamped<BelowLimit>> {
    if counter.value < counter.limit {
        Some(BelowLimit::mint_stamped(counter, counter.value))
    } else {
        None
    }
}

/// Count one, if the stamp is still about this counter's current version.
///
/// # Errors
/// [`Stale`] when the counter changed since the check; it is then left as
/// it was.
pub fn bump(counter: &mut Counter, stamp: Stamped<BelowLimit>) -> Result<(), Stale> {
    let fact = stamp.verify(counter)?.redeem();
    counter.value = fact.value().saturating_add(1);
    counter.version = counter.version.wrapping_add(1);
    Ok(())
}
