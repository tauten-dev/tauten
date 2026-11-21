//! Reading and writing the baseline file.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use tauten_check::baseline::{Baseline, VERSION};

pub const DEFAULT_NAME: &str = "tauten-baseline.json";

pub fn read(path: &Path) -> Result<Baseline> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let value: serde_json::Value =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    let what = path.display().to_string();
    if let Err(reason) = tauten_check::format::check(
        &what,
        tauten_check::format::json_version(&value),
        VERSION,
        VERSION,
    ) {
        bail!("{reason}");
    }
    serde_json::from_value(value).with_context(|| format!("parsing {}", path.display()))
}

pub fn write(path: &Path, baseline: &Baseline) -> Result<()> {
    let mut text = serde_json::to_string_pretty(baseline)?;
    text.push('\n');
    fs::write(path, text).with_context(|| format!("writing {}", path.display()))
}
