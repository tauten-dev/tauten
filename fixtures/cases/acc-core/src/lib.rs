//! Core logic: effects arrive as parameters.

use std::io::Read;

/// A clock capability.
pub trait Clock {
    /// Milliseconds since an arbitrary epoch.
    fn now_ms(&self) -> u64;
}

/// Loading failed.
#[derive(Debug)]
pub enum LoadError {
    /// Reading failed.
    Io(std::io::Error),
    /// The amount was not a number.
    Parse(core::num::ParseIntError),
}

/// Read an amount from the given source.
///
/// # Errors
/// Returns [`LoadError`] if reading or parsing fails.
pub fn load_amount(source: &mut impl Read) -> Result<u64, LoadError> {
    let mut text = String::new();
    source.read_to_string(&mut text).map_err(LoadError::Io)?;
    text.trim().parse().map_err(LoadError::Parse)
}

/// Stamp a decision with the time from the clock capability.
pub fn stamp(clock: &impl Clock, balance: u64) -> (u64, u64) {
    (clock.now_ms(), balance)
}
