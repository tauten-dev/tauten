//! Effects declared to match signatures.

use tauten::Cap;
use tauten::cap::{Bytes, Clock, Limit, ReadDir};
use tauten::effects;

/// Capabilities the service needs.
#[derive(Cap)]
pub struct Caps<'a, C: Clock> {
    clock: &'a C,
    config: &'a ReadDir,
}

/// Milliseconds since `created_ms`.
#[effects(clock)]
pub fn age_ms(clock: &impl Clock, created_ms: u64) -> u64 {
    clock.now_ms().saturating_sub(created_ms)
}

/// Read the configuration, stamped with the time.
///
/// # Errors
///
/// When the file cannot be read.
#[effects(clock, fs_read, fail)]
pub fn load<C: Clock>(caps: &Caps<'_, C>) -> Result<(u64, String), std::io::Error> {
    let text = caps.config.read_to_string("app.toml", Limit::<Bytes>::new(64 * 1024))?;
    Ok((caps.clock.now_ms(), text))
}

/// Pure arithmetic.
#[effects()]
pub fn double(n: u32) -> u32 {
    n.saturating_mul(2)
}
