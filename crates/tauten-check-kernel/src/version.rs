//! Whether a file's format version may be read.
//!
//! Every versioned file tauten reads (`proofs/claims.toml`, a recorded
//! `claims.json`, the baseline) carries a format version, and the reader
//! supports a range of them. [`admit`] decides: a missing version is
//! invalid, a newer one is never read as if it were current, an older one
//! is refused below the oldest supported.
//!
//! Proved: `admit` is total; it admits exactly the versions in the
//! supported range; it never admits a version newer than the newest
//! supported; a missing version is never admitted.

/// What [`admit`] decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    /// In the supported range: read it.
    Admitted,
    /// Written by a newer tauten: refuse it, and say to upgrade.
    Newer,
    /// Older than the oldest supported format: refuse it.
    Older,
    /// No version at all: the file is invalid.
    Absent,
}

/// Admit format version `found` when it is within `oldest..=newest`.
#[must_use]
pub const fn admit(found: Option<u32>, oldest: u32, newest: u32) -> Admission {
    match found {
        None => Admission::Absent,
        Some(v) => {
            if v > newest {
                Admission::Newer
            } else if v < oldest {
                Admission::Older
            } else {
                Admission::Admitted
            }
        }
    }
}
