//! Reading and writing the baseline file.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use tauten_check::baseline::{Baseline, VERSION};

pub const DEFAULT_NAME: &str = "tauten-baseline.json";

pub fn read(path: &Path) -> Result<Baseline> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let baseline: Baseline =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    if baseline.version != VERSION {
        bail!(
            "{}: unsupported baseline version {}",
            path.display(),
            baseline.version
        );
    }
    Ok(baseline)
}

pub fn write(path: &Path, baseline: &Baseline) -> Result<()> {
    let mut text = serde_json::to_string_pretty(baseline)?;
    text.push('\n');
    fs::write(path, text).with_context(|| format!("writing {}", path.display()))
}
