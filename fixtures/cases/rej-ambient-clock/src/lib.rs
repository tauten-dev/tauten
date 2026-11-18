//! Reads the clock without a capability.

/// Current time.
#[must_use]
pub fn now() -> std::time::SystemTime {
    std::time::SystemTime::now()
}
