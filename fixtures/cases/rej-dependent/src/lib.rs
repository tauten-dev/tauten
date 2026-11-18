//! Depends on a crate with violations; has its own.

/// First port.
#[must_use]
pub fn first(v: &[&str]) -> u16 {
    rej_unwrap::port(v[0])
}
