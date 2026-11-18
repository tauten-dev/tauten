//! Mints authority outside the edge.

/// Stand-in for `tauten::cap::Authority`.
pub struct Authority;

impl Authority {
    /// Mint authority.
    #[must_use]
    pub const fn acquire() -> Self {
        Self
    }
}

/// Library code minting its own authority.
#[must_use]
pub const fn sneaky() -> Authority {
    Authority::acquire()
}
