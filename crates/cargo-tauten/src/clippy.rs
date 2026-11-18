//! Run clippy once per tier with the profile's flags and generated config.

use std::collections::BTreeMap;
use std::fs;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use cargo_metadata::Message;
use tauten_check::diag::{CompilerMessage, from_compiler};
use tauten_check::{Diagnostic, Profile, Tier};

use crate::workspace::Loaded;

pub fn run(profile: &Profile, ws: &Loaded) -> Result<Vec<Diagnostic>> {
    let mut by_tier: BTreeMap<Tier, Vec<String>> = BTreeMap::new();
    for member in &ws.model.members {
        if let Some(tier) = member.tier() {
            by_tier.entry(tier).or_default().push(member.name.clone());
        }
    }
    let target = ws.metadata.target_directory.as_std_path().join("tauten");
    let mut diags = Vec::new();
    for (tier, packages) in by_tier {
        diags.extend(run_tier(profile, ws, &target, tier, &packages)?);
    }
    Ok(diags)
}

fn run_tier(
    profile: &Profile,
    ws: &Loaded,
    target: &std::path::Path,
    tier: Tier,
    packages: &[String],
) -> Result<Vec<Diagnostic>> {
    let conf_dir = target.join("conf").join(tier.name());
    fs::create_dir_all(&conf_dir).with_context(|| format!("creating {}", conf_dir.display()))?;
    fs::write(conf_dir.join("clippy.toml"), profile.clippy_toml(tier)?)
        .context("writing generated clippy.toml")?;

    let cargo = std::env::var_os("CARGO").map_or_else(|| PathBuf::from("cargo"), PathBuf::from);
    let mut cmd = Command::new(cargo);
    cmd.current_dir(ws.root())
        .arg("clippy")
        .arg("--manifest-path")
        .arg(ws.root().join("Cargo.toml"))
        .args(["--message-format=json", "--all-targets", "--quiet"])
        .arg("--target-dir")
        .arg(target.join("build"));
    for package in packages {
        cmd.args(["-p", package]);
    }
    cmd.arg("--").args(profile.lint_flags(tier));
    cmd.env("CLIPPY_CONF_DIR", &conf_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().context("starting `cargo clippy`")?;
    let stdout = child.stdout.take().context("capturing clippy output")?;
    let mut diags = Vec::new();
    for message in Message::parse_stream(BufReader::new(stdout)) {
        if let Message::CompilerMessage(m) = message.context("reading clippy output")? {
            let krate = ws.package_name(&m.package_id).unwrap_or("?").to_owned();
            if let Some(d) = from_compiler(profile, &krate, Some(tier), &convert(&m.message)) {
                diags.push(d);
            }
        }
    }
    let output = child
        .wait_with_output()
        .context("waiting for `cargo clippy`")?;
    if !output.status.success() && diags.is_empty() {
        bail!(
            "`cargo clippy` failed for the {tier} tier without diagnostics:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(diags)
}

fn convert(d: &cargo_metadata::diagnostic::Diagnostic) -> CompilerMessage {
    let primary = d.spans.iter().find(|s| s.is_primary);
    CompilerMessage {
        level: level(d.level).to_owned(),
        code: d.code.as_ref().map(|c| c.code.clone()),
        message: d.message.clone(),
        notes: d.children.iter().map(|c| c.message.clone()).collect(),
        file: primary.map(|s| s.file_name.clone()),
        line: primary.and_then(|s| u32::try_from(s.line_start).ok()),
        column: primary.and_then(|s| u32::try_from(s.column_start).ok()),
    }
}

fn level(level: cargo_metadata::diagnostic::DiagnosticLevel) -> &'static str {
    use cargo_metadata::diagnostic::DiagnosticLevel as L;
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "DiagnosticLevel is non_exhaustive"
    )]
    match level {
        L::Error | L::Ice => "error",
        L::Warning => "warning",
        _ => "note",
    }
}
