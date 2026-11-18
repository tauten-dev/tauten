//! `cargo tauten`: run the tauten strictness profile over a Cargo workspace.

mod baseline_file;
mod clippy;
mod output;
mod workspace;
mod wrapper;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use tauten_check::baseline::{self, Baseline};
use tauten_check::{Diagnostic, Profile, Severity, Tier};

#[derive(Parser)]
#[command(name = "cargo", bin_name = "cargo")]
enum Cargo {
    /// Strictness profile for agent-written Rust.
    Tauten(Cli),
}

#[derive(clap::Args)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Which workspace to check and how.
#[derive(clap::Args)]
struct Target {
    /// Path to Cargo.toml (default: discover from the current directory).
    #[arg(long)]
    manifest_path: Option<PathBuf>,
    /// Skip clippy and run only the structural checks.
    #[arg(long)]
    structure_only: bool,
    /// Assume a tier for a crate without editing its manifest
    /// (`name=tier`, or `*=tier` for every crate without one). For trial runs.
    #[arg(long = "assume-tier", value_name = "CRATE=TIER")]
    assume_tier: Vec<String>,
}

#[derive(Subcommand)]
enum Command {
    /// Run structural checks and clippy with the profile for each crate's tier.
    Check {
        #[command(flatten)]
        target: Target,
        /// Output format.
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
        /// Baseline file (default: `tauten-baseline.json` in the workspace root, if present).
        #[arg(long)]
        baseline: Option<PathBuf>,
        /// Ignore any baseline file.
        #[arg(long, conflicts_with = "baseline")]
        no_baseline: bool,
        /// List every warning instead of a per-rule summary (human format).
        #[arg(long)]
        warnings: bool,
    },
    /// Record current error-level findings as the baseline (the ratchet).
    Baseline {
        #[command(flatten)]
        target: Target,
        /// Baseline file (default: `tauten-baseline.json` in the workspace root).
        #[arg(long)]
        output: Option<PathBuf>,
        /// Allow the baseline to accept more findings than before.
        #[arg(long)]
        allow_increase: bool,
    },
    /// Explain a rule, e.g. `cargo tauten explain TAU-P001`; without an ID, list all rules.
    Explain {
        /// Rule ID.
        id: Option<String>,
        /// Print the whole rule reference as Markdown (docs/RULES.md).
        #[arg(long, conflicts_with = "id")]
        markdown: bool,
    },
    /// Print the generated clippy flags and clippy.toml for a tier.
    Profile {
        /// Tier to print.
        #[arg(value_parser = parse_tier)]
        tier: Tier,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Human,
    Json,
}

fn parse_tier(text: &str) -> Result<Tier, String> {
    Tier::parse(text).ok_or_else(|| format!("unknown tier `{text}` (kernel, core, edge)"))
}

fn main() -> ExitCode {
    if wrapper::active() {
        return wrapper::run().unwrap_or_else(|error| {
            eprintln!("cargo-tauten (wrapper): {error:#}");
            ExitCode::from(2)
        });
    }
    let Cargo::Tauten(cli) = Cargo::parse();
    match run(cli) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("cargo-tauten: {error:#}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    let profile = Profile::embedded().context("loading the embedded profile")?;
    match cli.command {
        Command::Check {
            target,
            format,
            baseline,
            no_baseline,
            warnings,
        } => {
            let (root, diags) = collect(&profile, &target)?;
            let baseline_path = baseline.unwrap_or_else(|| root.join(baseline_file::DEFAULT_NAME));
            let (diags, baselined) = if no_baseline || !baseline_path.is_file() {
                (diags, 0)
            } else {
                let accepted = baseline_file::read(&baseline_path)?;
                let applied = baseline::apply(&profile, &accepted, diags);
                (applied.diagnostics, applied.baselined)
            };
            let options = output::Options {
                json: matches!(format, Format::Json),
                all_warnings: warnings,
                baselined,
            };
            output::print(&diags, &options)?;
            Ok(exit_code(&diags))
        }
        Command::Baseline {
            target,
            output,
            allow_increase,
        } => {
            let (root, diags) = collect(&profile, &target)?;
            let path = output.unwrap_or_else(|| root.join(baseline_file::DEFAULT_NAME));
            let next = Baseline::from_findings(&diags);
            if path.is_file() && !allow_increase {
                let previous = baseline_file::read(&path)?;
                let increases = previous.increases(&next);
                if !increases.is_empty() {
                    for entry in &increases {
                        eprintln!(
                            "  {} in {}: {}",
                            entry.key.id,
                            entry.key.file.as_deref().unwrap_or(&entry.key.krate),
                            entry.key.message
                        );
                    }
                    bail!(
                        "{} baseline entr(ies) would grow; fix the new findings, or pass --allow-increase",
                        increases.len()
                    );
                }
            }
            baseline_file::write(&path, &next)?;
            println!(
                "tauten: baseline {} accepts {} finding(s)",
                path.display(),
                next.total()
            );
            Ok(ExitCode::SUCCESS)
        }
        Command::Explain { id, markdown } => {
            if markdown {
                print!("{}", output::rules_markdown(&profile));
                return Ok(ExitCode::SUCCESS);
            }
            output::explain(&profile, id.as_deref())?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Profile { tier } => {
            output::profile(&profile, tier)?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

/// Load the workspace and run every check. Returns the workspace root and
/// the sorted, de-duplicated findings.
fn collect(profile: &Profile, target: &Target) -> Result<(PathBuf, Vec<Diagnostic>)> {
    let ws = workspace::load(target.manifest_path.as_deref(), &target.assume_tier)?;
    let mut diags = tauten_check::structure::check(profile, &ws.model);
    if !target.structure_only {
        diags.extend(clippy::run(profile, &ws)?);
    }
    diags.sort_by(|a, b| {
        (&a.file, a.line, a.column, &a.id, &a.message)
            .cmp(&(&b.file, b.line, b.column, &b.id, &b.message))
    });
    diags.dedup();
    Ok((ws.root().to_path_buf(), diags))
}

fn exit_code(diags: &[Diagnostic]) -> ExitCode {
    if diags.iter().any(|d| d.severity == Severity::Error) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
