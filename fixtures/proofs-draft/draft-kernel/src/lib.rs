//! Drafting fixture: functions with and without claims.
#![no_std]

pub mod alpha;
pub mod beta;

/// Claimed by hand; a new theorem and a new mutant are drafted into it.
#[must_use]
pub fn add_capped(a: u32, b: u32) -> u32 {
    if a > u32::MAX - b { u32::MAX } else { a + b }
}

/// New, with a total theorem and a mutant: a full claim is drafted.
#[must_use]
pub fn clamp(x: u32, hi: u32) -> u32 {
    if x > hi { hi } else { x }
}

/// New, but its theorem has a hypothesis: drafted without `total`.
#[must_use]
pub fn double(a: u32) -> u32 {
    a * 2
}

/// A helper with a total theorem of its own.
#[must_use]
pub fn halve(a: u32) -> u32 {
    a / 2
}

/// Its caller: the helper's total theorem is not about it, so it is
/// skipped.
#[must_use]
pub fn quarter(a: u32) -> u32 {
    halve(halve(a))
}

/// A loop: the loop's total theorem joins the claim, never as `total`.
#[must_use]
pub fn count_up(n: u32) -> u32 {
    let mut i = 0;
    while i < n {
        i += 1;
    }
    i
}

/// Claimed by hand under a named assumption: never drafted a `subsumes`.
#[must_use]
pub fn latest(stored: u32, seen: u32) -> u32 {
    if seen >= stored { seen } else { stored }
}
