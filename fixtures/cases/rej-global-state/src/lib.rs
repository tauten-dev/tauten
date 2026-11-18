//! Hidden global state.

use std::sync::OnceLock;

static CACHE: OnceLock<u64> = OnceLock::new();

/// Cached value.
#[must_use]
pub fn cached() -> u64 {
    *CACHE.get_or_init(|| 7)
}
