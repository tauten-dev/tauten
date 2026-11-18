//! Kernel without no_std, using HashMap and floats.

use std::collections::HashMap;

/// Count keys.
#[must_use]
pub fn count(map: &HashMap<u8, u8>) -> usize {
    map.len()
}

/// Average.
#[must_use]
pub fn half(x: f64) -> f64 {
    x / 2.0
}
