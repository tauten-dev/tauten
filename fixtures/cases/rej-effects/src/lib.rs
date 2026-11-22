//! Effects that do not match signatures.

use tauten::cap::{Bytes, Limit, ReadDir};
use tauten::effects;

/// Claims to write, but only holds a read-only directory.
///
/// # Errors
///
/// When the file cannot be read.
#[effects(fs_read, fs_write, fail)]
pub fn load(config: &ReadDir) -> Result<String, std::io::Error> {
    Ok(config.read_to_string("app.toml", Limit::<Bytes>::new(4096))?)
}

/// Declares nothing.
#[must_use]
pub fn double(n: u32) -> u32 {
    n.saturating_mul(2)
}
