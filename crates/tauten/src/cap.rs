//! Capabilities: authority created once at the edge and passed down.
//!
//! ```
//! use tauten::cap::{Authority, Clock, ManualClock};
//!
//! // Core code takes the capability as a parameter…
//! fn is_expired(clock: &impl Clock, deadline_ms: u64) -> bool {
//!     clock.now_ms() > deadline_ms
//! }
//!
//! // …the edge creates the real one…
//! let authority = Authority::acquire();
//! let _real = authority.clock();
//!
//! // …and tests substitute a fake.
//! assert!(is_expired(&ManualClock::at(10), 5));
//! ```

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// The root of all authority in a program. Create it once, in `main` of an
/// edge crate; `cargo tauten check` rejects `Authority::acquire` in kernel
/// and core crates (TAU-A002). It cannot be cloned.
#[derive(Debug)]
pub struct Authority {
    _private: (),
}

impl Authority {
    /// Acquire authority. Only edge crates may call this.
    #[must_use]
    pub const fn acquire() -> Self {
        Self { _private: () }
    }

    /// The system clock.
    #[must_use]
    pub const fn clock(&self) -> SystemClock {
        SystemClock { _private: () }
    }

    /// A snapshot of the process environment, taken now.
    #[must_use]
    pub fn env(&self) -> Env {
        Env {
            vars: std::env::vars_os()
                .filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?)))
                .collect(),
        }
    }

    /// Open a directory as a capability: everything below it, nothing above
    /// it. Paths that escape (`..`, absolute paths, symlinks out) fail.
    ///
    /// # Errors
    /// Returns the IO error from opening the directory.
    #[cfg(feature = "fs")]
    pub fn dir(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<cap_std::fs::Dir> {
        cap_std::fs::Dir::open_ambient_dir(path, cap_std::ambient_authority())
    }
}

/// Read the current time.
pub trait Clock {
    /// Milliseconds since the Unix epoch.
    fn now_ms(&self) -> u64;
}

/// The system clock. Obtain it from [`Authority::clock`].
#[derive(Debug, Clone, Copy)]
pub struct SystemClock {
    _private: (),
}

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
    }
}

/// A clock for tests: returns whatever it was set to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManualClock {
    now_ms: u64,
}

impl ManualClock {
    /// A clock stopped at `now_ms`.
    #[must_use]
    pub const fn at(now_ms: u64) -> Self {
        Self { now_ms }
    }

    /// Move the clock forward, saturating at `u64::MAX`.
    pub const fn advance(&mut self, ms: u64) {
        self.now_ms = self.now_ms.saturating_add(ms);
    }
}

impl Clock for ManualClock {
    fn now_ms(&self) -> u64 {
        self.now_ms
    }
}

/// Read-only environment variables, captured at one point in time.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Env {
    vars: BTreeMap<String, String>,
}

impl Env {
    /// An environment for tests.
    #[must_use]
    pub fn from_pairs<K: Into<String>, V: Into<String>>(
        pairs: impl IntoIterator<Item = (K, V)>,
    ) -> Self {
        Self {
            vars: pairs
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        }
    }

    /// Look a variable up.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.vars.get(name).map(String::as_str)
    }
}
