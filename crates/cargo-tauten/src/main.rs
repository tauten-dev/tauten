//! `cargo tauten`: run the tauten strictness profile over a Cargo workspace.

mod baseline_file;
mod clippy;
mod draft;
mod generated;
mod init;
mod obligations;
mod output;
mod proofs;
mod scaffold;
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
    /// Before pushing: `check`, then the proof gate at ship level (only the
    /// mutants of claims whose inputs changed). Same as `verify --level ship`.
    Ship {
        #[command(flatten)]
        target: Target,
        /// Gate the working tree instead of a snapshot of `HEAD`.
        #[arg(long)]
        live: bool,
    },
    /// Run the cadence level: `fast` is `check`; `ship` adds the proof gate
    /// at ship level; `full` adds the whole proof gate (nightly, release).
    Verify {
        #[command(flatten)]
        target: Target,
        /// fast, ship or full.
        #[arg(long, value_enum)]
        level: VerifyLevel,
        /// Gate the working tree instead of a snapshot of `HEAD`.
        #[arg(long)]
        live: bool,
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
    /// Write a fresh workspace laid out as the agent profile describes: a
    /// kernel crate with one proved claim, a core crate with a capability,
    /// `#[effects]` and one `Evidence` check, and a thin edge binary.
    New {
        /// Directory to create; its name is the edge binary's package name.
        path: PathBuf,
        /// Depend on the `tauten` crate at this path instead of the
        /// crates.io release matching this cargo-tauten.
        #[arg(long, value_name = "DIR")]
        tauten_path: Option<PathBuf>,
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
        /// `full` runs every mutant; `ship` re-runs only the mutants of
        /// claims whose recorded inputs changed and reuses the others.
        #[arg(long, value_enum, default_value_t = proofs::Level::Full)]
        level: proofs::Level,
        /// Only translate (`extract`): tools and extract stages, the first
        /// located translation error per crate.
        #[command(subcommand)]
        action: Option<ProofsAction>,
    },
    /// In kernel crates, list arithmetic and indexing findings (TAU-N001,
    /// TAU-N002, TAU-P003) as proof obligations: proved (subsumed by a
    /// recorded proof), open and baselined, or open.
    Obligations {
        #[command(flatten)]
        target: Target,
        /// List every obligation, not only the counts.
        #[arg(long)]
        list: bool,
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

/// A cadence level (D2).
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum VerifyLevel {
    /// `cargo tauten check`.
    Fast,
    /// `check`, then the proof gate at ship level.
    Ship,
    /// `check`, then the whole proof gate.
    Full,
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
    /// Print the recorded claims of one crate as the `claims.json` it ships
    /// next to its `Cargo.toml`, for projects that `inherit` them.
    Export {
        /// The crate whose claims to export.
        #[arg(long = "crate")]
        krate: String,
    },
    /// In a two-parent merge whose human-authored conflicts are resolved:
    /// check the resolved claims against both parents' records, regenerate
    /// the declared extraction, and write a pending (not proved) record.
    /// Never stages or commits; the merge commit's gate replaces the record.
    Reconcile {
        /// The `[[workspace.metadata.tauten.generated]]` entry to
        /// regenerate (default: the one producing proofs/Extracted).
        #[arg(long)]
        generated: Option<String>,
    },
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
    /// Print a reviewable patch to proofs/claims.toml for new or changed
    /// functions, from the translation, a gate run up to the audit (no
    /// mutants; `[gate] try` with none selected) and the declared mutants.
    /// Writes nothing and grants no proof credit: apply the patch, commit,
    /// and run `cargo tauten proofs`. Exit 0 means the draft succeeded,
    /// never that a gate passed.
    Draft {
        /// Draft only this Cargo package (one of the packages of
        /// `[gate] crates`); default: all of them.
        #[arg(long = "crate")]
        krate: Option<String>,
        /// For custom gates: a JSON inventory (`version: 1`, `functions`,
        /// `theorems`, `mutants`) instead of the template's audit subjects
        /// and `[[mutant]]` sites. Every reference is checked.
        #[arg(long, value_name = "PATH")]
        inventory: Option<PathBuf>,
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
        } => check(
            &profile,
            &target,
            matches!(format, Format::Json),
            baseline,
            no_baseline,
            warnings,
            extract,
        ),
        Command::Ship { target, live } => verify(&profile, &target, VerifyLevel::Ship, live),
        Command::Verify {
            target,
            level,
            live,
        } => verify(&profile, &target, level, live),
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
        Command::New { path, tauten_path } => {
            scaffold::run(&path, tauten_path.as_deref())?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Proofs {
            manifest_path,
            no_write,
            live,
            level,
            action,
        } => {
            let ws = workspace::load(manifest_path.as_deref(), &[])?;
            match action {
                Some(ProofsAction::Extract { krate }) => proofs::extract(&ws, krate.as_deref()),
                Some(ProofsAction::Export { krate }) => proofs::export(&ws, &krate),
                Some(ProofsAction::Reconcile { generated }) => {
                    proofs::reconcile(&ws, generated.as_deref())
                }
                Some(ProofsAction::Try { theorems, mutants }) => {
                    proofs::try_run(&ws, &theorems, &mutants)
                }
                Some(ProofsAction::Draft { krate, inventory }) => {
                    draft::run(&ws, krate.as_deref(), inventory.as_deref())
                }
                None => proofs::run(&ws, !no_write, live, level),
            }
        }
        Command::Obligations { target, list } => {
            let (root, diags, _) = collect(&profile, &target)?;
            obligations::run(&profile, &root, diags, list)
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

#[expect(
    clippy::fn_params_excessive_bools,
    reason = "the flags of `cargo tauten check`, passed through"
)]
fn check(
    profile: &Profile,
    target: &Target,
    json: bool,
    baseline: Option<PathBuf>,
    no_baseline: bool,
    warnings: bool,
    extract: bool,
) -> Result<ExitCode> {
    let (root, diags, tiers) = collect(profile, target)?;
    let baseline_path = baseline.unwrap_or_else(|| root.join(baseline_file::DEFAULT_NAME));
    let (diags, baselined) = if no_baseline || !baseline_path.is_file() {
        (diags, 0)
    } else {
        let accepted = baseline_file::read(&baseline_path)?;
        let mut applied = baseline::apply(profile, &accepted, diags);
        for d in applied.diagnostics.iter_mut().filter(|d| d.tier.is_none()) {
            d.tier = tiers.get(&d.krate).copied();
        }
        (applied.diagnostics, applied.baselined)
    };
    let mut diags = diags;
    diags.extend(proofs::conservation(&root, profile)?);
    if extract {
        diags.extend(proofs::extract_diagnostics(&root, profile)?);
    }
    let options = output::Options {
        json,
        all_warnings: warnings,
        baselined,
    };
    output::print(&diags, &options)?;
    Ok(exit_code(&diags))
}

/// `verify --level` and `ship`: `check`, then the proof gate at the level.
fn verify(profile: &Profile, target: &Target, level: VerifyLevel, live: bool) -> Result<ExitCode> {
    let checked = check(profile, target, false, None, false, false, false)?;
    let gate = match level {
        VerifyLevel::Fast => return Ok(checked),
        VerifyLevel::Ship => proofs::Level::Ship,
        VerifyLevel::Full => proofs::Level::Full,
    };
    let ws = workspace::load(target.manifest_path.as_deref(), &target.assume_tier)?;
    if !ws.root().join(proofs::CLAIMS_TOML).is_file() {
        eprintln!("tauten: no {}; nothing to prove", proofs::CLAIMS_TOML);
        return Ok(checked);
    }
    let proved = proofs::run(&ws, true, live, gate)?;
    Ok(
        if checked == ExitCode::SUCCESS && proved == ExitCode::SUCCESS {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        },
    )
}

fn exit_code(diags: &[Diagnostic]) -> ExitCode {
    if tauten_check::diag::passes(diags) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
