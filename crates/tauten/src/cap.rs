//! Capabilities: authority created once at the edge and passed down.
//!
//! A capability is a value that lets its holder touch one part of the world:
//! the clock, the environment, a directory. Core code takes capabilities as
//! parameters, so a signature shows what a function can touch, and tests
//! substitute fakes.
//!
//! ```
//! use tauten::cap::{Clock, ManualClock};
//!
//! // Core code takes the capability as a parameter…
//! fn is_expired(clock: &impl Clock, deadline_ms: u64) -> bool {
//!     clock.now_ms() > deadline_ms
//! }
//!
//! // …the edge creates the real one…
//! # #[cfg(feature = "std")] {
//! let authority = tauten::cap::Authority::acquire();
//! let _real = authority.clock();
//! # }
//!
//! // …and tests substitute a fake.
//! assert!(is_expired(&ManualClock::at(10), 5));
//! ```
//!
//! Group the capabilities a component needs in a struct and derive
//! [`Cap`](derive@crate::Cap) on it; every field must itself be a capability:
//!
//! ```
//! use tauten::Cap;
//! use tauten::cap::{Clock, Env};
//!
//! #[derive(Cap)]
//! pub struct Caps<'a, C: Clock> {
//!     pub clock: &'a C,
//!     pub env: &'a Env,
//! }
//! ```
//!
//! ```compile_fail
//! use tauten::Cap;
//!
//! #[derive(Cap)]
//! pub struct Caps {
//!     pub retries: u32, // not a capability
//! }
//! ```

#[cfg(feature = "std")]
use std::collections::BTreeMap;
#[cfg(feature = "std")]
use std::string::String;
#[cfg(feature = "std")]
use std::time::{SystemTime, UNIX_EPOCH};

/// A capability: a value whose possession grants access to part of the
/// world. Implemented by tauten's capabilities, by references to them, and
/// by `#[derive(Cap)]` bundles whose fields are all capabilities.
///
/// `Cap` names the set; it does not make a type unforgeable. Unforgeability
/// comes from each capability's private constructor.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a capability (`tauten::cap::Cap`)",
    note = "a `#[derive(Cap)]` struct may hold only capabilities: `&impl Clock`, `&Env`, a directory, \
            or another `#[derive(Cap)]` bundle; pass plain data as ordinary parameters"
)]
pub trait Cap {}

impl<T: Cap + ?Sized> Cap for &T {}
impl<T: Cap + ?Sized> Cap for &mut T {}

/// Read the current time.
pub trait Clock: Cap {
    /// Milliseconds since the Unix epoch.
    fn now_ms(&self) -> u64;
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

impl Cap for ManualClock {}

impl Clock for ManualClock {
    fn now_ms(&self) -> u64 {
        self.now_ms
    }
}

/// The root of all authority in a program. Create it once, in `main` of an
/// edge crate; `cargo tauten check` rejects `Authority::acquire` in kernel
/// and core crates (TAU-A002). It cannot be cloned.
#[cfg(feature = "std")]
#[derive(Debug)]
pub struct Authority {
    _private: (),
}

#[cfg(feature = "std")]
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

    /// Open a directory read-only: a [`ReadDir`] can read files and open
    /// subdirectories below it, and nothing else.
    ///
    /// # Errors
    /// Returns the IO error from opening the directory.
    #[cfg(feature = "fs")]
    pub fn read_dir(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<ReadDir> {
        self.dir(path).map(ReadDir::from)
    }
}

/// The system clock. Obtain it from [`Authority::clock`].
#[cfg(feature = "std")]
#[derive(Debug, Clone, Copy)]
pub struct SystemClock {
    _private: (),
}

#[cfg(feature = "std")]
impl Cap for SystemClock {}

#[cfg(feature = "std")]
impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
    }
}

/// Read-only environment variables, captured at one point in time.
#[cfg(feature = "std")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Env {
    vars: BTreeMap<String, String>,
}

#[cfg(feature = "std")]
impl Cap for Env {}

#[cfg(feature = "std")]
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

#[cfg(feature = "fs")]
impl Cap for cap_std::fs::Dir {}

/// A directory that can only be read: an attenuated
/// [`cap_std::fs::Dir`]. Obtain it from [`Authority::read_dir`], or narrow a
/// writable `Dir` with `ReadDir::from(dir)`.
#[cfg(feature = "fs")]
#[derive(Debug)]
pub struct ReadDir {
    inner: cap_std::fs::Dir,
}

#[cfg(feature = "fs")]
impl Cap for ReadDir {}

#[cfg(feature = "fs")]
impl From<cap_std::fs::Dir> for ReadDir {
    fn from(inner: cap_std::fs::Dir) -> Self {
        Self { inner }
    }
}

#[cfg(feature = "fs")]
impl ReadDir {
    /// Read a whole file below this directory.
    ///
    /// # Errors
    /// Returns the IO error, including for paths that escape the directory.
    pub fn read(&self, name: impl AsRef<std::path::Path>) -> std::io::Result<std::vec::Vec<u8>> {
        self.inner.read(name)
    }

    /// Read a whole file below this directory as UTF-8.
    ///
    /// # Errors
    /// Returns the IO error, including for invalid UTF-8.
    pub fn read_to_string(&self, name: impl AsRef<std::path::Path>) -> std::io::Result<String> {
        self.inner.read_to_string(name)
    }

    /// Whether `name` exists below this directory.
    #[must_use]
    pub fn exists(&self, name: impl AsRef<std::path::Path>) -> bool {
        self.inner.exists(name)
    }

    /// Open a subdirectory, read-only.
    ///
    /// # Errors
    /// Returns the IO error, including for paths that escape the directory.
    pub fn sub(&self, name: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        self.inner.open_dir(name).map(Self::from)
    }
}
