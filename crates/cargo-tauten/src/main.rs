//! `cargo tauten`: run the tauten strictness profile over a Cargo workspace.

mod clippy;
mod output;
mod workspace;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
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

#[derive(Subcommand)]
enum Command {
    /// Run structural checks and clippy with the profile for each crate's tier.
    Check {
        /// Path to Cargo.toml (default: discover from the current directory).
        #[arg(long)]
        manifest_path: Option<PathBuf>,
        /// Output format.
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
        /// Skip clippy and run only the structural checks.
        #[arg(long)]
        structure_only: bool,
    },
    /// Explain a rule, e.g. `cargo tauten explain TAU-P001`; without an ID, list all rules.
    Explain {
        /// Rule ID.
        id: Option<String>,
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
            manifest_path,
            format,
            structure_only,
        } => {
            let ws = workspace::load(manifest_path.as_deref())?;
            let mut diags = tauten_check::structure::check(&profile, &ws.model);
            if !structure_only {
                diags.extend(clippy::run(&profile, &ws)?);
            }
            diags.sort_by(|a, b| (&a.file, a.line, &a.id).cmp(&(&b.file, b.line, &b.id)));
            diags.dedup();
            output::print(&diags, matches!(format, Format::Json))?;
            Ok(exit_code(&diags))
        }
        Command::Explain { id } => {
            output::explain(&profile, id.as_deref())?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Profile { tier } => {
            output::profile(&profile, tier)?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn exit_code(diags: &[Diagnostic]) -> ExitCode {
    if diags.iter().any(|d| d.severity == Severity::Error) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
