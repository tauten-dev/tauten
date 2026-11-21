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

    /// This snapshot narrowed to `names`: the listed variables it already
    /// holds, and nothing else. It does not read the process environment
    /// again, so a missing or previously removed name stays missing.
    ///
    /// ```
    /// use tauten::cap::Env;
    ///
    /// let env = Env::from_pairs([("MODE", "fast"), ("TOKEN", "secret")]);
    /// let narrowed = env.only(&["MODE", "HOME"]);
    /// assert_eq!(narrowed.get("MODE"), Some("fast"));
    /// assert_eq!(narrowed.get("TOKEN"), None);
    /// assert_eq!(narrowed.get("HOME"), None);
    /// ```
    #[must_use]
    pub fn only(mut self, names: &[&str]) -> Self {
        self.vars.retain(|k, _| names.contains(&k.as_str()));
        self
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
    /// Read a whole file below this directory, refusing it if it holds more
    /// than `limit` bytes.
    ///
    /// Success is the whole file, never a prefix. A file of exactly `limit`
    /// bytes fits (a zero limit admits an empty file); one byte more is
    /// [`ReadError::TooLarge`], detected by reading at most one byte past the
    /// limit, without first allocating the whole input. The limit bounds the
    /// bytes this call returns, not process memory or time.
    ///
    /// ```no_run
    /// use tauten::cap::{Authority, Bytes, Limit};
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let dir = Authority::acquire().read_dir("config")?;
    /// let bytes = dir.read("app.toml", Limit::<Bytes>::new(64 * 1024))?;
    /// # let _ = bytes; Ok(()) }
    /// ```
    ///
    /// The unbounded 0.1 call no longer compiles:
    ///
    /// ```compile_fail
    /// # fn f(dir: &tauten::cap::ReadDir) {
    /// let bytes = dir.read("app.toml");
    /// # }
    /// ```
    ///
    /// # Errors
    /// [`ReadError::TooLarge`] past the limit; [`ReadError::Io`] for an IO
    /// error, including for paths that escape the directory.
    pub fn read(
        &self,
        name: impl AsRef<std::path::Path>,
        limit: Limit<Bytes>,
    ) -> Result<std::vec::Vec<u8>, ReadError> {
        read_limited(self.inner.open(name)?, limit.get())
    }

    /// Read a whole file below this directory as UTF-8, refusing it if it
    /// holds more than `limit` bytes. As [`ReadDir::read`]; an oversized
    /// file is refused before UTF-8 is checked.
    ///
    /// ```no_run
    /// use tauten::cap::{Authority, Bytes, Limit};
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let dir = Authority::acquire().read_dir("config")?;
    /// // 0.1: let text = dir.read_to_string("app.toml")?;
    /// let text = dir.read_to_string("app.toml", Limit::<Bytes>::new(64 * 1024))?;
    /// # let _ = text; Ok(()) }
    /// ```
    ///
    /// ```compile_fail
    /// # fn f(dir: &tauten::cap::ReadDir) {
    /// let text = dir.read_to_string("app.toml");
    /// # }
    /// ```
    ///
    /// # Errors
    /// [`ReadError::TooLarge`] past the limit; [`ReadError::Io`] for an IO
    /// error (including for paths that escape the directory) or, with
    /// [`std::io::ErrorKind::InvalidData`], for invalid UTF-8.
    pub fn read_to_string(
        &self,
        name: impl AsRef<std::path::Path>,
        limit: Limit<Bytes>,
    ) -> Result<String, ReadError> {
        utf8(self.read(name, limit)?)
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

/// The unit of a [`Limit`] on bytes read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bytes {}

/// An explicit upper bound, in `Unit`s. There is no default: every bounded
/// call states its limit.
pub struct Limit<Unit> {
    max: u64,
    unit: core::marker::PhantomData<fn() -> Unit>,
}

impl<Unit> Limit<Unit> {
    /// At most `max` units.
    #[must_use]
    pub const fn new(max: u64) -> Self {
        Self {
            max,
            unit: core::marker::PhantomData,
        }
    }

    /// The bound.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.max
    }
}

impl<Unit> Clone for Limit<Unit> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Unit> Copy for Limit<Unit> {}

impl<Unit> PartialEq for Limit<Unit> {
    fn eq(&self, other: &Self) -> bool {
        self.max == other.max
    }
}

impl<Unit> Eq for Limit<Unit> {}

impl<Unit> core::fmt::Debug for Limit<Unit> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Limit").field(&self.max).finish()
    }
}

/// Why a bounded read failed.
///
/// Converts into [`std::io::Error`] (too large becomes
/// [`std::io::ErrorKind::InvalidData`]), so `?` works in functions returning
/// `std::io::Result`.
#[cfg(feature = "fs")]
#[derive(Debug)]
#[non_exhaustive]
pub enum ReadError {
    /// The input holds more than `limit` bytes. Nothing was returned.
    TooLarge {
        /// The limit the read was given.
        limit: u64,
    },
    /// Opening or reading failed, or the text was not UTF-8.
    Io(std::io::Error),
}

#[cfg(feature = "fs")]
impl core::fmt::Display for ReadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooLarge { limit } => write!(f, "input exceeds the limit of {limit} bytes"),
            Self::Io(e) => e.fmt(f),
        }
    }
}

#[cfg(feature = "fs")]
impl std::error::Error for ReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TooLarge { .. } => None,
            Self::Io(e) => Some(e),
        }
    }
}

#[cfg(feature = "fs")]
impl From<std::io::Error> for ReadError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

#[cfg(feature = "fs")]
impl From<ReadError> for std::io::Error {
    fn from(e: ReadError) -> Self {
        match e {
            ReadError::Io(e) => e,
            too_large @ ReadError::TooLarge { .. } => {
                Self::new(std::io::ErrorKind::InvalidData, too_large)
            }
        }
    }
}

/// Read `input` to EOF if it holds at most `limit` bytes.
///
/// Reads in chunks no larger than the remaining allowance and charges each
/// through the proved [`tauten_check_kernel::budget::charge`]. With the
/// allowance spent, one probe byte decides: EOF is success, a byte is
/// [`ReadError::TooLarge`]. Never computes `limit + 1` and never allocates
/// from a reported length.
#[cfg(feature = "fs")]
fn read_limited(mut input: impl std::io::Read, limit: u64) -> Result<std::vec::Vec<u8>, ReadError> {
    const CHUNK: usize = 8 * 1024;
    let mut out = std::vec::Vec::new();
    let mut buf = [0_u8; CHUNK];
    let mut left = limit;
    while left > 0 {
        let want = usize::try_from(left).map_or(CHUNK, |l| l.min(CHUNK));
        let Some(window) = buf.get_mut(..want) else {
            return Err(ReadError::TooLarge { limit });
        };
        let n = match input.read(window) {
            Ok(0) => return Ok(out),
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(ReadError::Io(e)),
        };
        // A reader reporting more than it was offered is treated as too large.
        let cost = u64::try_from(n).unwrap_or(u64::MAX);
        let (Some(rest), Some(read)) = (
            tauten_check_kernel::budget::charge(left, cost),
            buf.get(..n),
        ) else {
            return Err(ReadError::TooLarge { limit });
        };
        left = rest;
        out.extend_from_slice(read);
    }
    let mut probe = [0_u8; 1];
    loop {
        match input.read(&mut probe) {
            Ok(0) => return Ok(out),
            Ok(_) => return Err(ReadError::TooLarge { limit }),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(ReadError::Io(e)),
        }
    }
}

#[cfg(feature = "fs")]
fn utf8(bytes: std::vec::Vec<u8>) -> Result<String, ReadError> {
    String::from_utf8(bytes)
        .map_err(|e| ReadError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))
}

#[cfg(all(test, feature = "fs"))]
mod tests {
    use super::*;
    use std::format;
    use std::io::{self, Read};
    use std::string::ToString;
    use std::vec::Vec;

    /// Hands out at most `chunk` bytes per call, optionally failing once
    /// `fail_after` bytes were delivered, and counts what it delivered.
    struct Chunked {
        data: Vec<u8>,
        at: usize,
        chunk: usize,
        fail_after: Option<usize>,
        interrupted_once: bool,
    }

    impl Chunked {
        fn new(data: &[u8], chunk: usize) -> Self {
            Self {
                data: data.to_vec(),
                at: 0,
                chunk,
                fail_after: None,
                interrupted_once: false,
            }
        }
    }

    impl Read for Chunked {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if !self.interrupted_once {
                self.interrupted_once = true;
                return Err(io::ErrorKind::Interrupted.into());
            }
            if self.fail_after.is_some_and(|n| self.at >= n) {
                return Err(io::Error::other("disk gone"));
            }
            let rest = self.data.get(self.at..).unwrap_or_default();
            let n = rest.len().min(buf.len()).min(self.chunk);
            buf[..n].copy_from_slice(&rest[..n]);
            self.at += n;
            Ok(n)
        }
    }

    fn too_large(r: Result<Vec<u8>, ReadError>) -> bool {
        matches!(r, Err(ReadError::TooLarge { .. }))
    }

    #[test]
    fn exact_fits_one_over_is_refused() {
        assert_eq!(read_limited(Chunked::new(b"", 3), 0).unwrap(), b"");
        assert_eq!(read_limited(Chunked::new(b"abc", 2), 3).unwrap(), b"abc");
        assert!(too_large(read_limited(Chunked::new(b"abcd", 2), 3)));
        assert!(too_large(read_limited(Chunked::new(b"a", 1), 0)));
    }

    #[test]
    fn an_oversized_input_is_read_at_most_one_byte_past_the_limit() {
        let mut input = Chunked::new(&[7; 100_000], 4096);
        assert!(too_large(read_limited(&mut input, 10_000)));
        assert_eq!(input.at, 10_001);
    }

    #[test]
    fn an_extreme_limit_does_not_overflow() {
        let data = [1_u8; 20_000];
        assert_eq!(
            read_limited(Chunked::new(&data, 999), u64::MAX).unwrap(),
            data
        );
    }

    #[test]
    fn an_io_error_after_a_prefix_returns_no_data() {
        let mut input = Chunked::new(b"abcdef", 2);
        input.fail_after = Some(4);
        assert!(
            matches!(read_limited(input, 100), Err(ReadError::Io(e)) if e.to_string() == "disk gone")
        );
        // Also when the error comes from the probe.
        let mut input = Chunked::new(b"abcd", 2);
        input.fail_after = Some(4);
        assert!(matches!(read_limited(input, 4), Err(ReadError::Io(_))));
    }

    #[test]
    fn utf8_is_checked_only_within_the_limit() {
        let bad = utf8(read_limited(Chunked::new(&[0xff, 0xfe], 1), 2).unwrap());
        assert!(matches!(bad, Err(ReadError::Io(e)) if e.kind() == io::ErrorKind::InvalidData));
        assert!(too_large(read_limited(Chunked::new(&[0xff; 5], 1), 2)));
        assert_eq!(utf8(b"h\xc3\xa9".to_vec()).unwrap(), "hé");
    }

    #[test]
    fn errors_convert_for_question_mark_callers() {
        let e: io::Error = ReadError::TooLarge { limit: 3 }.into();
        assert_eq!(e.kind(), io::ErrorKind::InvalidData);
        assert!(e.to_string().contains("limit of 3 bytes"));
        let e: io::Error = ReadError::Io(io::ErrorKind::NotFound.into()).into();
        assert_eq!(e.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn a_real_read_dir() {
        let root = std::env::temp_dir().join(format!("tauten-cap-{}", std::process::id()));
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("sub/app.toml"), "mode = 1\n").unwrap();
        std::fs::write(root.join("empty"), "").unwrap();
        let dir = Authority::acquire().read_dir(&root).unwrap();
        let sub = dir.sub("sub").unwrap();
        assert_eq!(
            sub.read_to_string("app.toml", Limit::new(9)).unwrap(),
            "mode = 1\n"
        );
        assert!(matches!(
            sub.read_to_string("app.toml", Limit::new(8)),
            Err(ReadError::TooLarge { limit: 8 })
        ));
        assert_eq!(dir.read("empty", Limit::new(0)).unwrap(), b"");
        assert!(matches!(
            dir.read("missing", Limit::new(9)),
            Err(ReadError::Io(_))
        ));
        assert!(matches!(
            sub.read("../../etc/passwd", Limit::new(u64::MAX)),
            Err(ReadError::Io(_))
        ));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn env_only_narrows_and_never_restores() {
        let env = Env::from_pairs([("A", "1"), ("B", "2"), ("C", "3")]);
        let narrowed = env.only(&["A", "B", "B", "MISSING"]);
        assert_eq!(narrowed, Env::from_pairs([("A", "1"), ("B", "2")]));
        let again = narrowed.only(&["A", "C"]);
        assert_eq!(again, Env::from_pairs([("A", "1")]));
        assert_eq!(again.get("C"), None);
    }
}
