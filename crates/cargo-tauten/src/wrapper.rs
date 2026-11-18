//! Wrapper mode: cargo invokes `cargo-tauten <rustc> <args…>` for each
//! workspace crate. We add the crate's tier flags and run `clippy-driver`.

use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

use anyhow::{Context, Result};
use tauten_check::{Profile, Tier};

pub const ENV_ACTIVE: &str = "TAUTEN_WRAPPER";
pub const ENV_TIERS: &str = "TAUTEN_TIERS";
pub const ENV_CONF_ROOT: &str = "TAUTEN_CONF_ROOT";
pub const ENV_DRIVER: &str = "TAUTEN_CLIPPY_DRIVER";

pub fn active() -> bool {
    env::var_os(ENV_ACTIVE).is_some()
}

pub fn run() -> Result<ExitCode> {
    let driver = env::var_os(ENV_DRIVER).context("TAUTEN_CLIPPY_DRIVER is not set")?;
    // argv: cargo-tauten <rustc> <args…>; clippy-driver accepts the same shape.
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    let mut cmd = Command::new(driver);
    cmd.args(&args);
    if let Some(tier) = tier_for_invocation(&args) {
        let profile = Profile::embedded()?;
        cmd.args(profile.report_flags(tier));
        let conf_root = env::var_os(ENV_CONF_ROOT)
            .map(PathBuf::from)
            .context("TAUTEN_CONF_ROOT is not set")?;
        cmd.env("CLIPPY_CONF_DIR", conf_root.join(tier.name()));
    }
    let status = cmd.status().context("running clippy-driver")?;
    Ok(status
        .code()
        .and_then(|c| u8::try_from(c).ok())
        .map_or(ExitCode::FAILURE, ExitCode::from))
}

/// The tier to apply, or `None` for probes (`-vV`, `--print`), build
/// scripts and untiered crates.
fn tier_for_invocation(args: &[OsString]) -> Option<Tier> {
    let crate_name = args
        .iter()
        .skip_while(|a| a.as_os_str() != "--crate-name")
        .nth(1)?
        .to_str()?;
    if crate_name == "build_script_build" {
        return None;
    }
    let package = env::var("CARGO_PKG_NAME").ok()?;
    let spec = env::var(ENV_TIERS).ok()?;
    spec.split(',')
        .filter_map(|entry| entry.split_once('='))
        .find(|(name, _)| *name == package)
        .and_then(|(_, tier)| Tier::parse(tier))
}
