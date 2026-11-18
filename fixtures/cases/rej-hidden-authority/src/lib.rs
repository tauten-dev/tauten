//! Reads the filesystem without a capability.

/// Load configuration.
///
/// # Errors
/// IO errors.
pub fn load() -> std::io::Result<String> {
    std::fs::read_to_string("config.toml")
}
