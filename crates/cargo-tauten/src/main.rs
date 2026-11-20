//! `cargo tauten`: run the tauten strictness profile over a Cargo workspace.

mod baseline_file;
mod clippy;
mod generated;
mod init;
mod output;
mod proofs;
mod workspace;
mod wrapper;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use tauten_check::baseline::{self, Baseline};
use tauten_check::{Diagnostic, Profile, Tier};

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
        /// Also translate the proved kernel crates (TAU-K004; needs Docker
        /// and the proofs image, about 30s; tool-missing is an error).
        #[arg(long)]
        extract: bool,
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
    /// Propose a tier for every crate without one and write the agent profile;
    /// with `--proofs <crate>`, set up the proof tier for a kernel crate instead.
    Init {
        /// Path to Cargo.toml (default: discover from the current directory).
        #[arg(long)]
        manifest_path: Option<PathBuf>,
        /// Apply the plan (default: print it only).
        #[arg(long)]
        write: bool,
        /// Set up `proofs/` (Charon/Aeneas/Lean template) for this kernel crate.
        #[arg(long, value_name = "CRATE")]
        proofs: Option<String>,
    },
    /// Run the project's proof gate (`proofs/claims.toml`) and print one JSON
    /// outcome per claim: proved, drift, build-failed, audit-failed,
    /// mutant-survived or tool-missing. Records them in `proofs/claims.json`.
    Proofs {
        /// Path to Cargo.toml (default: discover from the current directory).
        #[arg(long, global = true)]
        manifest_path: Option<PathBuf>,
        /// Print the outcomes without updating `proofs/claims.json`.
        #[arg(long)]
        no_write: bool,
        /// Gate the working tree instead of a snapshot of `HEAD` (then any
        /// edit during the run is reported as drift).
        #[arg(long)]
        live: bool,
        /// Only translate (`extract`): tools and extract stages, the first
        /// located translation error per crate.
        #[command(subcommand)]
        action: Option<ProofsAction>,
    },
    /// Regenerate every `[[workspace.metadata.tauten.generated]]` file and
    /// compare it byte for byte with the committed one: `ok`, `drift` or
    /// `failed` per entry (a missing output is never `ok`).
    Generated {
        /// Path to Cargo.toml (default: discover from the current directory).
        #[arg(long)]
        manifest_path: Option<PathBuf>,
        /// Only these entries (by name).
        #[arg(long, num_args = 1..)]
        only: Vec<String>,
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

/// `cargo tauten proofs <action>`.
#[derive(Debug, clap::Subcommand)]
enum ProofsAction {
    /// Translate the kernel crates (Charon, Aeneas) without building the
    /// proofs: the authoritative check that kernel code stays inside the
    /// translatable subset. Needs `[gate] extract` in proofs/claims.toml.
    Extract {
        /// Translate only this crate directory (one of `[gate] crates`).
        #[arg(long = "crate")]
        krate: Option<String>,
    },
    /// The developer loop: extract, drift check, build and audit, then only
    /// the mutants of the claims stating the given theorems (and any named
    /// mutants), in parallel. A partial run: its result is never recorded
    /// and is no substitute for `cargo tauten proofs`. Needs `[gate] try`.
    Try {
        /// Run the mutants of every claim that states one of these theorems.
        #[arg(long, num_args = 1..)]
        theorems: Vec<String>,
        /// Run these mutants.
        #[arg(long, num_args = 1..)]
        mutants: Vec<String>,
    },
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
            extract,
        } => {
            let (root, diags, tiers) = collect(&profile, &target)?;
            let baseline_path = baseline.unwrap_or_else(|| root.join(baseline_file::DEFAULT_NAME));
            let (diags, baselined) = if no_baseline || !baseline_path.is_file() {
                (diags, 0)
            } else {
                let accepted = baseline_file::read(&baseline_path)?;
                let mut applied = baseline::apply(&profile, &accepted, diags);
                for d in applied.diagnostics.iter_mut().filter(|d| d.tier.is_none()) {
                    d.tier = tiers.get(&d.krate).copied();
                }
                (applied.diagnostics, applied.baselined)
            };
            let mut diags = diags;
            diags.extend(proofs::conservation(&root, &profile)?);
            if extract {
                diags.extend(proofs::extract_diagnostics(&root, &profile)?);
            }
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
            let (root, diags, _) = collect(&profile, &target)?;
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
        Command::Init {
            manifest_path,
            write,
            proofs: Some(krate),
        } => {
            let ws = workspace::load(manifest_path.as_deref(), &[])?;
            init::proofs(&ws, &krate, write)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Init {
            manifest_path,
            write,
            proofs: None,
        } => {
            let ws = workspace::load(manifest_path.as_deref(), &[])?;
            init::run(&ws, write)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Proofs {
            manifest_path,
            no_write,
            live,
            action,
        } => {
            let ws = workspace::load(manifest_path.as_deref(), &[])?;
            match action {
                Some(ProofsAction::Extract { krate }) => proofs::extract(&ws, krate.as_deref()),
                Some(ProofsAction::Try { theorems, mutants }) => {
                    proofs::try_run(&ws, &theorems, &mutants)
                }
                None => proofs::run(&ws, !no_write, live),
            }
        }
        Command::Generated {
            manifest_path,
            only,
        } => {
            let ws = workspace::load(manifest_path.as_deref(), &[])?;
            generated::run(ws.root(), &only)
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
/// Findings plus each member's declared tier, so diagnostics created after
/// the scan (the ratchet's TAU-R001) can name the crate's tier.
type Collected = (PathBuf, Vec<Diagnostic>, BTreeMap<String, Tier>);

fn collect(profile: &Profile, target: &Target) -> Result<Collected> {
    let ws = workspace::load(target.manifest_path.as_deref(), &target.assume_tier)?;
    let tiers = ws
        .model
        .members
        .iter()
        .filter_map(|m| m.tier().map(|t| (m.name.clone(), t)))
        .collect();
    let mut diags = tauten_check::structure::check(profile, &ws.model);
    if !target.structure_only {
        diags.extend(clippy::run(profile, &ws)?);
    }
    // A finding inside a function whose proof is recorded for the current
    // source (proofs/claims.json) is marked as subsumed by that proof.
    let recorded = proofs::load(&ws)?;
    let mut diags = proofs::subsume(diags, recorded.as_ref());
    diags.sort_by(|a, b| {
        (&a.file, a.line, a.column, &a.id, &a.message)
            .cmp(&(&b.file, b.line, b.column, &b.id, &b.message))
    });
    diags.dedup();
    Ok((ws.root().to_path_buf(), diags, tiers))
}

fn exit_code(diags: &[Diagnostic]) -> ExitCode {
    if tauten_check::diag::passes(diags) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
