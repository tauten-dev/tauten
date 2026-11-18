//! Run clippy over the whole workspace in one `cargo check`, with
//! `cargo-tauten` itself as the workspace rustc wrapper (see `wrapper`), so
//! each crate is linted exactly once, with its own tier's flags.

use std::collections::BTreeMap;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use cargo_metadata::Message;
use tauten_check::diag::{CompilerMessage, from_compiler};
use tauten_check::{Diagnostic, Profile, Severity, Tier};

use crate::workspace::Loaded;
use crate::wrapper;

pub fn run(profile: &Profile, ws: &Loaded) -> Result<Vec<Diagnostic>> {
    let tiers: BTreeMap<String, Tier> = ws
        .model
        .members
        .iter()
        .filter_map(|m| m.tier().map(|t| (m.name.clone(), t)))
        .collect();
    let tier_spec = tiers
        .iter()
        .map(|(name, tier)| format!("{name}={tier}"))
        .collect::<Vec<_>>()
        .join(",");

    // Cargo does not fingerprint what a wrapper adds, so key the build
    // directory by everything the wrapper reads.
    let mut hasher = DefaultHasher::new();
    (
        env!("CARGO_PKG_VERSION"),
        tauten_check::profile::EMBEDDED_RULES,
        &tier_spec,
    )
        .hash(&mut hasher);
    let base = ws
        .metadata
        .target_directory
        .as_std_path()
        .join("tauten")
        .join(format!("{:016x}", hasher.finish()));
    let conf_root = base.join("conf");
    for tier in Tier::ALL {
        let dir = conf_root.join(tier.name());
        fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        fs::write(dir.join("clippy.toml"), profile.clippy_toml(tier)?)
            .context("writing generated clippy.toml")?;
    }

    let cargo = std::env::var_os("CARGO").map_or_else(|| PathBuf::from("cargo"), PathBuf::from);
    let exe = std::env::current_exe().context("locating cargo-tauten")?;
    let mut cmd = Command::new(cargo);
    cmd.current_dir(ws.root())
        .arg("check")
        .arg("--manifest-path")
        .arg(ws.root().join("Cargo.toml"))
        .args([
            "--workspace",
            "--all-targets",
            "--keep-going",
            "--message-format=json",
            "--quiet",
        ])
        .arg("--target-dir")
        .arg(base.join("build"))
        .env("RUSTC_WORKSPACE_WRAPPER", exe)
        .env(wrapper::ENV_ACTIVE, "1")
        .env(wrapper::ENV_TIERS, &tier_spec)
        .env(wrapper::ENV_CONF_ROOT, &conf_root)
        .env(wrapper::ENV_DRIVER, clippy_driver(ws.root())?)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().context("starting `cargo check`")?;
    let stdout = child.stdout.take().context("capturing compiler output")?;
    let mut diags = Vec::new();
    for message in Message::parse_stream(BufReader::new(stdout)) {
        let Message::CompilerMessage(m) = message.context("reading compiler output")? else {
            continue;
        };
        let Some(krate) = ws.package_name(&m.package_id) else {
            continue;
        };
        let tier = tiers.get(krate).copied();
        if let Some(d) = from_compiler(profile, krate, tier, &convert(&m.message)) {
            // Untiered crates are reported by TAU-T001; keep only their hard errors.
            if tier.is_some() || d.severity == Severity::Error {
                diags.push(d);
            }
        }
    }
    let output = child
        .wait_with_output()
        .context("waiting for `cargo check`")?;
    if !output.status.success() && diags.is_empty() {
        bail!(
            "`cargo check` failed without diagnostics:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(diags)
}

/// `clippy-driver` of the toolchain that applies in the workspace root.
fn clippy_driver(root: &Path) -> Result<PathBuf> {
    let output = Command::new("rustc")
        .current_dir(root)
        .args(["--print", "sysroot"])
        .output()
        .context("running `rustc --print sysroot`")?;
    let sysroot = String::from_utf8(output.stdout).context("sysroot is not UTF-8")?;
    let driver = Path::new(sysroot.trim()).join("bin").join("clippy-driver");
    if !driver.is_file() {
        bail!(
            "{} not found; install it with `rustup component add clippy`",
            driver.display()
        );
    }
    Ok(driver)
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
