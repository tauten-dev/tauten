//! Helpers used by the generated code of `#[refined]` and the contract
//! attributes. Public only so the macros can reach them.

/// Evaluate a predicate on a value. The signature gives closures written as
/// `|v| …` their parameter type.
#[must_use]
pub fn holds<T: ?Sized, F: FnOnce(&T) -> bool>(value: &T, predicate: F) -> bool {
    predicate(value)
}

/// Snapshot for `old(expr)`. Restricted to `Copy` values so a snapshot never
/// clones, moves or retains a resource.
#[must_use]
pub const fn snapshot<T: Copy>(value: &T) -> T {
    *value
}
