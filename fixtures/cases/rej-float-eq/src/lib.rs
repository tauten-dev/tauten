//! Exact comparison of floating-point values.

/// Whether two readings are the same.
#[must_use]
pub fn same(a: f64, b: f64) -> bool {
    a == b
}
