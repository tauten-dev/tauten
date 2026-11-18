//! Unbounded loop in library code.

/// Spin.
pub fn spin(counter: &mut u64) {
    loop {
        *counter = counter.wrapping_add(1);
    }
}
