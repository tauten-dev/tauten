//! Budget arithmetic: charging a cost against what is left.
//!
//! A kernel loop that spends a budget per iteration (bytes read, entries
//! visited, retries) charges it here instead of subtracting by hand. A
//! charge that does not fit is refused, never wrapped or saturated.
//!
//! Proved: `charge` succeeds exactly when the cost fits, and what is left
//! plus what was charged is the budget before (so a run of charges never
//! spends more than it started with); `charge_repeated` never overflows,
//! and succeeds exactly when `cost * times` (in unbounded arithmetic) fits.

/// What is left after charging `cost` against `left`, or `None` when it
/// does not fit.
#[must_use]
pub const fn charge(left: u64, cost: u64) -> Option<u64> {
    left.checked_sub(cost)
}

/// What is left after charging `cost` `times` times against `left`, or
/// `None` when the total does not fit. Never overflows.
#[must_use]
pub const fn charge_repeated(left: u64, cost: u64, times: u64) -> Option<u64> {
    match cost.checked_mul(times) {
        Some(total) => left.checked_sub(total),
        None => None,
    }
}
