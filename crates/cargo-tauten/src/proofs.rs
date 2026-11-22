//! `cargo tauten proofs`: run the project's proof gate and record one
//! outcome per claim of `proofs/claims.toml` in `proofs/claims.json`; and,
//! for `cargo tauten check`, read that record back to mark the findings a
//! fresh proof subsumes (`tauten_check::proofs`).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use anyhow::{Context, Result};
use tauten_check::Diagnostic;
use tauten_check::proofs::{
    self, AssumptionsFile, Badge, ClaimRecord, ClaimsFile, Event, FunctionSpan, Outcome, Record,
    RecordFile, Recorded, Report,
};

use crate::workspace::Loaded;

/// The claims a project declares.
pub const CLAIMS_TOML: &str = "proofs/claims.toml";
/// Named assumptions claims may rest on.
pub const ASSUMPTIONS_TOML: &str = "proofs/assumptions.toml";
/// The outcomes `cargo tauten proofs` recorded (and the badge data).
pub const CLAIMS_JSON: &str = "proofs/claims.json";

/// The translation directory `reconcile` regenerates by default.
const EXTRACTED: &str = "proofs/Extracted";

/// Run the gate and record the outcomes. Exit 0 iff every claim is proved.
///
/// In a Git checkout the gate runs on a snapshot of the committed `HEAD`
/// (`git archive` into a private directory), so the live tree may change
/// while it runs and the record is about exactly the snapshot's bytes; a
/// tree with uncommitted changes is refused. `live` runs it on the working
/// tree instead (and outside Git there is no choice), where any edit during
/// the run is reported as drift.
pub fn run(ws: &Loaded, write: bool, live: bool, level: Level) -> Result<ExitCode> {
    let live_root = ws.root().to_path_buf();
    let snapshot = if live {
        None
    } else {
        Snapshot::take(&live_root)?
    };
    let snapshot_ws;
    let ws = match &snapshot {
        Some(snap) => {
            eprintln!("tauten: gating a snapshot of commit {}", snap.commit);
            snapshot_ws = crate::workspace::load(Some(&snap.workspace.join("Cargo.toml")), &[])?;
            &snapshot_ws
        }
        None => {
            if !live {
                eprintln!("tauten: not a Git checkout; gating the live tree");
            }
            ws
        }
    };
    let outcome = run_in(
        ws,
        &live_root,
        write,
        snapshot.as_ref().map(|s| s.commit.clone()),
        level,
    );
    drop(snapshot);
    outcome
}

/// How much of the proof gate to run (D2, #36).
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Level {
    /// Every stage and every mutant.
    Full,
    /// Every stage, but only the mutants of claims whose recorded inputs
    /// changed (or that were not proved); the others' kills are reused.
    /// Needs `[gate] try` and a previous record with inputs.
    Ship,
}

/// The texts a claim's inputs are computed from ([`proofs::inputs`]).
struct Sources {
    extracted: BTreeMap<String, String>,
    lean: BTreeMap<String, String>,
    mutants: BTreeMap<String, String>,
    monotone: BTreeSet<String>,
    toolchain: String,
}

impl Sources {
    fn borrow(&self) -> proofs::inputs::Sources<'_> {
        proofs::inputs::Sources {
            extracted: &self.extracted,
            lean: &self.lean,
            mutants: &self.mutants,
            monotone: &self.monotone,
            toolchain: &self.toolchain,
        }
    }
}

fn read_sources(root: &Path, file: &ClaimsFile) -> Result<Sources> {
    let proofs_dir = root.join("proofs");
    let mut extracted = BTreeMap::new();
    let mut lean = BTreeMap::new();
    let mut all = Vec::new();
    files_under(root, Path::new("proofs"), &[CLAIMS_JSON], &mut all)?;
    for (path, bytes) in all {
        let Some(rel) = path.strip_prefix("proofs/") else {
            continue;
        };
        if rel.starts_with(".lake/") || !rel.ends_with(".lean") {
            continue;
        }
        let text = String::from_utf8_lossy(&bytes).into_owned();
        if let Some(name) = rel.strip_prefix("Extracted/") {
            extracted.insert(name.to_owned(), text);
        } else {
            lean.insert(rel.to_owned(), text);
        }
    }
    let mut mutants: BTreeMap<String, String> = file
        .mutant
        .iter()
        .map(|m| {
            (
                m.name.clone(),
                // Length-prefixed fields: no two entries share a text.
                [
                    m.name.as_str(),
                    m.file.as_str(),
                    m.before.as_str(),
                    m.after.as_str(),
                    &format!("{:?}", m.direct),
                ]
                .iter()
                .map(|f| format!("{}:{f}\n", f.len()))
                .collect::<String>(),
            )
        })
        .collect();
    let monotone: BTreeSet<String> = file
        .mutant
        .iter()
        .filter(|m| m.direct.is_empty())
        .map(|m| m.name.clone())
        .collect();
    let python = proofs_dir.join("mutants.py");
    if python.is_file() {
        let text =
            fs::read_to_string(&python).with_context(|| format!("reading {}", python.display()))?;
        // A name in both is bound to both definitions: which one the gate
        // uses is its own business.
        for (name, def) in proofs::inputs::python_mutants(&text) {
            mutants
                .entry(name)
                .and_modify(|t| {
                    t.push('\0');
                    t.push_str(&def);
                })
                .or_insert(def);
        }
    }
    let mut toolchain = String::new();
    for name in [
        "proofs/lean-toolchain",
        "proofs/lake-manifest.json",
        "proofs/lakefile.toml",
        "rust-toolchain.toml",
        "rust-toolchain",
    ] {
        if let Ok(text) = fs::read_to_string(root.join(name)) {
            toolchain.push_str(name);
            toolchain.push('\n');
            toolchain.push_str(&text);
        }
    }
    toolchain.push_str(&format!("{:?}", file.gate));
    Ok(Sources {
        extracted,
        lean,
        mutants,
        monotone,
        toolchain,
    })
}

/// A snapshot of `HEAD` in a private directory, removed on drop.
struct Snapshot {
    /// The snapshot of the whole repository.
    dir: PathBuf,
    /// The workspace root inside it.
    workspace: PathBuf,
    commit: String,
}

impl Snapshot {
    /// `None` outside a Git checkout. Refuses uncommitted changes.
    fn take(root: &Path) -> Result<Option<Self>> {
        let git = |args: &[&str]| Command::new("git").args(args).current_dir(root).output();
        // Only a workspace whose manifest Git tracks is snapshotted (not, for
        // example, a scratch copy inside an ignored directory).
        let Ok(tracked) = git(&["ls-files", "--error-unmatch", "Cargo.toml"]) else {
            return Ok(None);
        };
        if !tracked.status.success() {
            return Ok(None);
        }
        let top = git(&["rev-parse", "--show-toplevel"]).context("running git rev-parse")?;
        let top = PathBuf::from(String::from_utf8_lossy(&top.stdout).trim());
        let relative = root
            .canonicalize()
            .ok()
            .and_then(|r| {
                top.canonicalize()
                    .ok()
                    .and_then(|t| r.strip_prefix(t).ok().map(Path::to_path_buf))
            })
            .context("locating the workspace inside its Git checkout")?;
        let status = git(&[
            "status",
            "--porcelain",
            "--untracked-files=normal",
            "--",
            ".",
        ])
        .context("running git status")?;
        let dirty = String::from_utf8_lossy(&status.stdout).into_owned();
        if !dirty.trim().is_empty() {
            anyhow::bail!(
                "the checkout has uncommitted changes; the gate checks a snapshot of a commit, so commit them first, or pass --live to gate the working tree:\n{}",
                dirty.trim_end()
            );
        }
        let head = git(&["rev-parse", "HEAD"]).context("running git rev-parse")?;
        if !head.status.success() {
            anyhow::bail!("no commit to snapshot (git rev-parse HEAD failed)");
        }
        let commit = String::from_utf8_lossy(&head.stdout).trim().to_owned();
        let dir = std::env::temp_dir().join(format!(
            "tauten-gate-{}-{}",
            commit.get(..12).unwrap_or(&commit),
            std::process::id()
        ));
        if dir.exists() {
            fs::remove_dir_all(&dir).with_context(|| format!("clearing {}", dir.display()))?;
        }
        fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        let snap = Self {
            workspace: dir.join(&relative),
            dir,
            commit,
        };
        let mut archive = Command::new("git")
            .args(["archive", "--format=tar", "HEAD"])
            .current_dir(&top)
            .stdout(Stdio::piped())
            .spawn()
            .context("running git archive")?;
        let tar = Command::new("tar")
            .args(["-x", "-C"])
            .arg(&snap.dir)
            .stdin(archive.stdout.take().context("git archive output")?)
            .status()
            .context("running tar")?;
        let archived = archive.wait().context("waiting for git archive")?;
        if !archived.success() || !tar.success() {
            anyhow::bail!("could not snapshot commit {}", snap.commit);
        }
        Ok(Some(snap))
    }
}

impl Drop for Snapshot {
    fn drop(&mut self) {
        // Best effort: a leftover snapshot only costs temporary space.
        if fs::remove_dir_all(&self.dir).is_err() {
            eprintln!("tauten: could not remove {}", self.dir.display());
        }
    }
}

fn run_in(
    ws: &Loaded,
    live_root: &Path,
    write: bool,
    commit: Option<String>,
    level: Level,
) -> Result<ExitCode> {
    let root = ws.root();
    let path = root.join(CLAIMS_TOML);
    let text = fs::read_to_string(&path).with_context(|| {
        format!(
            "reading {}; set up the proof tier with `cargo tauten init --proofs <crate>`",
            path.display()
        )
    })?;
    let file = ClaimsFile::parse(&text)?;
    let assumptions_path = root.join(ASSUMPTIONS_TOML);
    let assumptions = if assumptions_path.is_file() {
        let text = fs::read_to_string(&assumptions_path)
            .with_context(|| format!("reading {}", assumptions_path.display()))?;
        AssumptionsFile::parse(&text)?
    } else {
        AssumptionsFile::default()
    };
    file.check_assumptions(&assumptions)?;
    let before = fingerprints(ws, &file)?;
    let sources = read_sources(root, &file)?;
    let index = proofs::inputs::Index::new(&sources.borrow());
    let inputs: Vec<BTreeMap<String, String>> = file
        .claim
        .iter()
        .map(|claim| proofs::inputs::claim_inputs(claim, &sources.borrow(), &index))
        .collect();
    let previous = committed_record(live_root)?;
    // What would refuse the record is known before the gate starts (#77):
    // refuse now instead of after a full run. A run that records nothing
    // only warns.
    if let Some(committed) = &previous
        && let Some(why) = recording_refused(live_root, &file, committed, commit.as_deref())?
    {
        if write {
            eprintln!(
                "tauten: not running the gate: its result could not be recorded: {why}; {CLAIMS_JSON} is unchanged"
            );
            return Ok(ExitCode::FAILURE);
        }
        eprintln!("tauten: warning: this result could not be recorded: {why}");
    }
    let own = file.claim.iter().any(|c| c.inherit.is_none());
    let reusable = previous.as_ref().and_then(RecordFile::result);
    let plan = match (level, reusable, &file.gate.try_) {
        (Level::Ship, Some(previous), Some(try_)) => {
            Some(ship_plan(&file, &inputs, previous, try_))
        }
        (Level::Ship, None, _) if matches!(previous, Some(RecordFile::Pending(_))) => {
            eprintln!(
                "tauten: {CLAIMS_JSON} is a pending merge record: no evidence to reuse, so every claim is re-calibrated (the full gate)"
            );
            None
        }
        (Level::Ship, _, _) => {
            eprintln!(
                "tauten: ship level needs `[gate] try` and a recorded {CLAIMS_JSON} with inputs; running the full gate"
            );
            None
        }
        (Level::Full, _, _) => None,
    };
    if own && !sites_admitted(root, &file) {
        if write {
            eprintln!("tauten: not running the gate; {CLAIMS_JSON} is unchanged");
        }
        return Ok(ExitCode::FAILURE);
    }
    let command = plan.as_ref().map_or(&file.gate.command, |p| &p.command);
    // Inherited claims need no gate run of their own.
    let mut report = if own {
        run_gate(root, command)?
    } else {
        proofs::Report::default()
    };
    if let Some(why) = report.preflight_refusal() {
        eprintln!(
            "tauten: the gate refused before any proof stage: {why}; {CLAIMS_JSON} is unchanged"
        );
        return Ok(ExitCode::FAILURE);
    }
    // An attempt that never reached a proof stage is reported but never
    // recorded (#52): the committed record stays byte-identical, so a plain
    // retry finds a clean checkout once the tool is back.
    let attempt_only = own && !report.started();
    report.changed_during_run = !proofs::unchanged(&before, &fingerprints(ws, &file)?);
    report.expected_direct = proofs::expected_direct(&file);
    if let Some(plan) = &plan {
        eprintln!(
            "tauten: ship level: {} of {} claim(s) re-calibrated ({} mutant(s) run), {} reused",
            plan.stale,
            file.claim.len(),
            plan.run.len(),
            file.claim.len().saturating_sub(plan.stale)
        );
        // Which mutants ran and which were reused: for the reader, not a
        // format (#83).
        let names = |v: Vec<&str>| {
            if v.is_empty() {
                "none".to_owned()
            } else {
                v.join(", ")
            }
        };
        eprintln!(
            "tauten: ship level: run: {}",
            names(plan.run.iter().map(String::as_str).collect())
        );
        eprintln!(
            "tauten: ship level: reused: {}",
            names(plan.carried.iter().map(|c| c.mutant.as_str()).collect())
        );
        for (claim, now) in file.claim.iter().zip(&inputs) {
            for key in now
                .keys()
                .filter(|k| k.starts_with(proofs::inputs::UNBOUND))
            {
                eprintln!(
                    "tauten: ship level: {} `{}` is never reused: {} ({})",
                    claim.file, claim.function, key, now[key]
                );
            }
        }
        report
            .events
            .extend(plan.carried.iter().map(proofs::MutantResult::event));
    }

    let mut records = Vec::new();
    let mut resolved: BTreeMap<String, (Option<String>, Option<Record>)> = BTreeMap::new();
    for (claim, inputs) in file.claim.iter().zip(inputs) {
        if let Some(inherit) = claim.inherit.as_deref().and_then(proofs::Inherit::parse) {
            let (locked, shipped) = match resolved.get(&inherit.krate) {
                Some(found) => found.clone(),
                None => {
                    let found = dependency_record(root, &inherit.krate)?;
                    resolved.insert(inherit.krate.clone(), found.clone());
                    found
                }
            };
            let (record_claim, outcome, badge, detail) =
                proofs::inherited(claim, locked.as_deref(), shipped.as_ref());
            records.push(ClaimRecord {
                claim: record_claim,
                outcome,
                badge,
                detail,
                fingerprint: String::new(),
                inputs: BTreeMap::new(),
                mutant_results: Vec::new(),
            });
            continue;
        }
        let (outcome, detail) = proofs::outcome(claim, &report);
        let fingerprint = before.get(&claim.krate).cloned().unwrap_or_default();
        records.push(ClaimRecord {
            claim: claim.clone(),
            outcome,
            badge: proofs::badge(claim, outcome, true),
            detail,
            fingerprint,
            inputs: proofs::inputs::recordable(&inputs),
            mutant_results: report.mutant_results(&claim.mutants),
        });
    }
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for record in &records {
        serde_json::to_writer(&mut out, record)?;
        writeln!(out)?;
    }
    if attempt_only {
        let proved = records
            .iter()
            .filter(|r| r.outcome == Outcome::Proved)
            .count();
        eprintln!(
            "tauten: {proved} of {} claim(s) proved: the proof gate never started a proof stage (tool missing or unavailable)",
            records.len()
        );
        if write {
            eprintln!(
                "tauten: not recording {CLAIMS_JSON}: an attempt that could not run is not a result; the committed record is unchanged, so retry the same command once the tool is available"
            );
        }
        return Ok(ExitCode::FAILURE);
    }
    // Again after the run: the record may have changed while the gate ran.
    if write && let Some(committed) = committed_record(live_root)? {
        if let Some(why) = recording_refused(live_root, &file, &committed, commit.as_deref())? {
            eprintln!("tauten: not recording {CLAIMS_JSON}: {why}");
            return Ok(ExitCode::FAILURE);
        }
        if matches!(committed, RecordFile::Pending(_)) && !proofs::records_pass(&records) {
            eprintln!(
                "tauten: not recording {CLAIMS_JSON}: only a gate that proves every claim replaces a pending merge record"
            );
            return Ok(ExitCode::FAILURE);
        }
    }
    if records.is_empty() {
        // An empty inventory proves nothing: no gate ran, and the proved
        // admission refuses an empty required set (#51). The committed
        // record, if any, stays as it is.
        eprintln!(
            "tauten: {CLAIMS_TOML} lists no claim, so no theorem was checked; add a [[claim]] (or an `inherit` one) or remove {CLAIMS_TOML} if the project has no proof tier"
        );
        if write {
            eprintln!("tauten: not recording {CLAIMS_JSON}: a record of no claims is not a result");
        }
        return Ok(ExitCode::FAILURE);
    }
    if write {
        let record = Record {
            version: 1,
            generated_by: "cargo tauten proofs".to_owned(),
            claims: records.clone(),
            commit,
        };
        let mut json = serde_json::to_string_pretty(&record)?;
        json.push('\n');
        let target = live_root.join(CLAIMS_JSON);
        fs::write(&target, json).with_context(|| format!("writing {}", target.display()))?;
    }
    let proved = records
        .iter()
        .filter(|r| r.outcome == Outcome::Proved)
        .count();
    let under = records
        .iter()
        .filter(|r| r.badge == Badge::ProvedUnder)
        .count();
    let under = if under == 0 {
        String::new()
    } else {
        format!(", {under} of them under named assumptions")
    };
    eprintln!(
        "tauten: {proved} of {} claim(s) proved{under}{}",
        records.len(),
        if write {
            format!(" (recorded in {CLAIMS_JSON})")
        } else {
            String::new()
        }
    );
    Ok(if proofs::records_pass(&records) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Run the gate command from the workspace root. Its output goes to stderr
/// (progress); result lines are collected. A command that cannot start is
/// reported, never treated as a pass.
pub(crate) fn run_gate(root: &Path, command: &[String]) -> Result<Report> {
    let mut report = Report::default();
    let Some((program, args)) = command.split_first() else {
        report.not_started = Some("[gate] command is empty".to_owned());
        return Ok(report);
    };
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    // A snapshot lives in a directory named per run; Compose would name a
    // new project (and leave a new network behind) for each. One stable
    // project name reuses one network.
    if std::env::var_os("COMPOSE_PROJECT_NAME").is_none() {
        command.env("COMPOSE_PROJECT_NAME", "tauten-proofs");
    }
    let spawned = command.spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(error) => {
            report.not_started = Some(format!("cannot run `{program}`: {error}"));
            return Ok(report);
        }
    };
    if let Some(stdout) = child.stdout.take() {
        let stderr = io::stderr();
        for line in BufReader::new(stdout).lines() {
            let line = line.context("reading the gate's output")?;
            writeln!(stderr.lock(), "{line}")?;
            if let Some(event) = Event::parse(&line) {
                report.events.push(event);
            }
        }
    }
    let status = child.wait().context("waiting for the gate")?;
    eprintln!("tauten: proof gate exited with {status}");
    Ok(report)
}

/// A dependency's locked version and the `claims.json` it ships (next to
/// its `Cargo.toml`), from `cargo metadata` with dependencies.
fn dependency_record(root: &Path, krate: &str) -> Result<(Option<String>, Option<Record>)> {
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(root.join("Cargo.toml"))
        .exec()
        .context("running `cargo metadata` to resolve inherited claims")?;
    let Some(package) = metadata.packages.iter().find(|p| p.name.as_str() == krate) else {
        return Ok((None, None));
    };
    let shipped = package
        .manifest_path
        .parent()
        .map(|dir| dir.as_std_path().join("claims.json"))
        .filter(|p| p.is_file())
        .map(|p| -> Result<Record> {
            let text =
                fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?;
            Record::parse(&text).map_err(|e| anyhow::anyhow!("{}: {e}", p.display()))
        })
        .transpose()?;
    Ok((Some(package.version.to_string()), shipped))
}

/// `cargo tauten proofs export --crate <name>`: the recorded claims of one
/// crate, as the `claims.json` that crate ships for `inherit`.
pub fn export(ws: &Loaded, krate: &str) -> Result<ExitCode> {
    let record = match committed_record(ws.root())? {
        None => anyhow::bail!("no {CLAIMS_JSON} to export from"),
        Some(RecordFile::Pending(_)) => anyhow::bail!(
            "{CLAIMS_JSON} is a pending merge record, not evidence; run `cargo tauten proofs` on the merged commit before exporting"
        ),
        Some(RecordFile::Result(record)) => record,
    };
    let shipped = Record {
        version: record.version,
        generated_by: "cargo tauten proofs export".to_owned(),
        claims: record
            .claims
            .into_iter()
            .filter(|c| c.claim.krate == krate)
            .map(|mut c| {
                // What a consumer needs: the claim, its outcome and badge.
                c.inputs.clear();
                c.mutant_results.clear();
                c
            })
            .collect(),
        commit: record.commit,
    };
    let mut json = serde_json::to_string_pretty(&shipped)?;
    json.push('\n');
    print!("{json}");
    Ok(ExitCode::SUCCESS)
}

/// What a ship-level run executes and reuses.
struct ShipPlan {
    /// The `[gate] try` command with the mutants to run.
    command: Vec<String>,
    /// Kills reused from the record, for the mutants not run.
    carried: Vec<proofs::MutantResult>,
    /// Claims re-calibrated.
    stale: usize,
    /// Mutants run.
    run: BTreeSet<String>,
}

/// Inputs `of` (the whole claim's with `None`, else one mutant's).
fn part(inputs: &BTreeMap<String, String>, of: Option<&str>) -> BTreeMap<String, String> {
    inputs
        .iter()
        .filter(|(k, _)| proofs::inputs::mutant_of(k) == of)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// Whether the recorded inputs `recorded` still hold for `now`, decided by
/// the proved `binding::fresh` ([`proofs::unchanged`]). A current input
/// that is unbound never holds.
fn holds(recorded: &BTreeMap<String, String>, now: &BTreeMap<String, String>) -> bool {
    !now.keys().any(|k| k.starts_with(proofs::inputs::UNBOUND)) && proofs::unchanged(recorded, now)
}

/// A claim is re-calibrated unless it was recorded proved with its
/// claim-wide inputs (translation reached, proof files, toolchain) still
/// holding; then each of its mutants runs again only if its own inputs
/// (definition, and the project's Lean files for an exact one) moved or no
/// result was recorded. Every other mutant's recorded result is reused.
fn ship_plan(
    file: &ClaimsFile,
    inputs: &[BTreeMap<String, String>],
    previous: &Record,
    try_: &[String],
) -> ShipPlan {
    let mut run: BTreeSet<String> = BTreeSet::new();
    let mut reusable: Vec<&proofs::MutantResult> = Vec::new();
    let mut stale: usize = 0;
    for (claim, now) in file.claim.iter().zip(inputs) {
        let recorded = previous
            .claims
            .iter()
            .find(|r| r.claim.file == claim.file && r.claim.function == claim.function);
        let fresh = recorded.filter(|r| {
            r.outcome == Outcome::Proved
                && !r.inputs.is_empty()
                && r.claim == *claim
                && holds(&part(&r.inputs, None), &part(now, None))
        });
        let Some(r) = fresh else {
            stale = stale.saturating_add(1);
            run.extend(claim.mutants.iter().cloned());
            continue;
        };
        for mutant in &claim.mutants {
            let result = r.mutant_results.iter().find(|m| m.mutant == *mutant);
            match result {
                Some(result) if holds(&part(&r.inputs, Some(mutant)), &part(now, Some(mutant))) => {
                    reusable.push(result);
                }
                _ => {
                    run.insert(mutant.clone());
                }
            }
        }
    }
    let mut carried: Vec<proofs::MutantResult> = Vec::new();
    for result in reusable {
        if !run.contains(&result.mutant) && !carried.iter().any(|c| c.mutant == result.mutant) {
            carried.push(result.clone());
        }
    }
    let mut command = try_.to_vec();
    command.push(run.iter().cloned().collect::<Vec<_>>().join(","));
    ShipPlan {
        command,
        carried,
        stale,
        run,
    }
}

/// `cargo tauten proofs extract`: run `[gate] extract` and report each
/// crate's translation. Exit 0 iff every crate translated
/// ([`proofs::extraction`]).
pub fn extract(ws: &Loaded, only: Option<&str>) -> Result<ExitCode> {
    let started = std::time::Instant::now();
    let run = run_extract(ws.root(), only)?;
    for (krate, seen) in &run.results {
        match seen {
            proofs::Extracted::Ok => println!("{krate}: translates"),
            proofs::Extracted::Failed(why) => println!("{krate}: does not translate: {why}"),
            proofs::Extracted::NotRun => match &run.missing {
                Some(why) => println!("{krate}: tool-missing: {why}"),
                None => println!("{krate}: not run"),
            },
        }
    }
    eprintln!(
        "tauten: extract {} in {}s",
        if run.admitted { "passed" } else { "failed" },
        started.elapsed().as_secs()
    );
    Ok(if run.admitted {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// `cargo tauten proofs try`: a partial gate over the selected mutants.
/// Prints each affected claim's outcome on this run, marked partial; never
/// writes `proofs/claims.json`. Exit 0 iff each of them is proved here.
pub fn try_run(ws: &Loaded, theorems: &[String], mutants: &[String]) -> Result<ExitCode> {
    let root = ws.root();
    let path = root.join(CLAIMS_TOML);
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let file = ClaimsFile::parse(&text)?;
    let Some(mut command) = file.gate.try_.clone() else {
        anyhow::bail!(
            "{CLAIMS_TOML} has no `[gate] try` command; the template's is \
             [\"sh\", \"proofs/run.sh\", \"python3\", \"proofs/gate.py\", \"--only-mutants\"]"
        );
    };
    let (selected, affected) = proofs::try_selection(&file, theorems, mutants);
    if selected.is_empty() {
        anyhow::bail!(
            "no mutant selected: name theorems that claims state (--theorems) or mutants (--mutants)"
        );
    }
    command.push(selected.join(","));
    if !sites_admitted(root, &file) {
        return Ok(ExitCode::FAILURE);
    }
    let started = std::time::Instant::now();
    let mut report = run_gate(root, &command)?;
    if let Some(why) = report.preflight_refusal() {
        eprintln!("tauten: the gate refused before any proof stage: {why}");
        return Ok(ExitCode::FAILURE);
    }
    report.expected_direct = proofs::expected_direct(&file);
    let mut all = true;
    for claim in &affected {
        let (outcome, detail) = proofs::outcome(claim, &report);
        all = all && outcome == Outcome::Proved;
        println!(
            "PARTIAL {}: {} on mutants {}{}",
            claim.function,
            outcome.name(),
            claim.mutants.join(", "),
            detail.map(|d| format!(" ({d})")).unwrap_or_default()
        );
    }
    eprintln!(
        "tauten: PARTIAL run of {} mutant(s) in {}s; not recorded, and not a gate result: run `cargo tauten proofs` for that",
        selected.len(),
        started.elapsed().as_secs()
    );
    Ok(if all {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// The result of an extract-only run.
struct ExtractRun {
    admitted: bool,
    results: Vec<(String, proofs::Extracted)>,
    /// Why the tools did not run, when they did not.
    missing: Option<String>,
}

fn run_extract(root: &Path, only: Option<&str>) -> Result<ExtractRun> {
    let path = root.join(CLAIMS_TOML);
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let file = ClaimsFile::parse(&text)?;
    let Some(mut command) = file.gate.extract.clone() else {
        anyhow::bail!(
            "{CLAIMS_TOML} has no `[gate] extract` command; the template's is \
             [\"sh\", \"proofs/run.sh\", \"python3\", \"proofs/gate.py\", \"--extract-only\"]"
        );
    };
    let crates: Vec<String> = match only {
        Some(c) => {
            command.extend(["--crate".to_owned(), c.to_owned()]);
            vec![c.to_owned()]
        }
        None => file.gate.crates.clone(),
    };
    let report = run_gate(root, &command)?;
    let (admitted, results) = proofs::extraction(&crates, &report);
    let missing = report.not_started.clone().or_else(|| {
        report.events.iter().find_map(|e| match e {
            Event::Stage {
                stage,
                status,
                detail,
                ..
            } if stage == "tools" && status != "ok" => {
                Some(detail.clone().unwrap_or_else(|| status.clone()))
            }
            Event::Stage { .. }
            | Event::Theorem { .. }
            | Event::Total { .. }
            | Event::Subject { .. }
            | Event::Premises { .. }
            | Event::Establishes { .. }
            | Event::Mutant { .. } => None,
        })
    });
    Ok(ExtractRun {
        admitted,
        results,
        missing,
    })
}

/// TAU-K004 for `cargo tauten check --extract`: one error per proved crate
/// that does not translate, or that could not be translated (tool-missing
/// is never clean). Nothing when the project has no `proofs/claims.toml`.
pub fn extract_diagnostics(
    root: &Path,
    profile: &tauten_check::Profile,
) -> Result<Vec<Diagnostic>> {
    if !root.join(CLAIMS_TOML).is_file() {
        return Ok(Vec::new());
    }
    let Some(rule) = profile.rule("TAU-K004") else {
        return Ok(Vec::new());
    };
    let text = fs::read_to_string(root.join(CLAIMS_TOML))
        .with_context(|| format!("reading {CLAIMS_TOML}"))?;
    if ClaimsFile::parse(&text)?.gate.extract.is_none() {
        return Ok(vec![
            Diagnostic::for_rule(
                rule,
                tauten_check::Severity::Error,
                "workspace",
                None,
                format!("{CLAIMS_TOML} has no `[gate] extract` command, so the proved crates cannot be translated"),
            )
            .at(Some(CLAIMS_TOML.to_owned()), None, None),
        ]);
    }
    let run = run_extract(root, None)?;
    let mut out = Vec::new();
    for (krate, seen) in &run.results {
        let message = match seen {
            proofs::Extracted::Ok => continue,
            proofs::Extracted::Failed(why) => format!("`{krate}` does not translate: {why}"),
            proofs::Extracted::NotRun => match &run.missing {
                Some(why) => format!(
                    "`{krate}` was not translated (tool-missing: {why}); TAU-K004 is never clean without the toolchain"
                ),
                None => format!("`{krate}` was not translated: the run reported no result for it"),
            },
        };
        out.push(
            Diagnostic::for_rule(rule, tauten_check::Severity::Error, krate, None, message).at(
                Some(format!("{krate}/src/lib.rs")),
                None,
                None,
            ),
        );
    }
    if !run.admitted && out.is_empty() {
        out.push(Diagnostic::for_rule(
            rule,
            tauten_check::Severity::Error,
            "workspace",
            None,
            "the translation results were incomplete or repeated".to_owned(),
        ));
    }
    Ok(out)
}

/// The directory of a member crate, relative to the workspace root.
fn crate_dir(ws: &Loaded, krate: &str) -> Option<PathBuf> {
    let member = ws.model.members.iter().find(|m| m.name == krate)?;
    Some(
        Path::new(&member.manifest)
            .parent()
            .map_or_else(PathBuf::new, Path::to_path_buf),
    )
}

/// Every file under `dir`, skipping `target`, `__pycache__`, hidden entries
/// and `skip`.
fn files_under(
    root: &Path,
    dir: &Path,
    skip: &[&str],
    out: &mut Vec<(String, Vec<u8>)>,
) -> Result<()> {
    let full = root.join(dir);
    if !full.is_dir() {
        return Ok(());
    }
    let mut entries: Vec<_> = fs::read_dir(&full)
        .with_context(|| format!("reading {}", full.display()))?
        .collect::<Result<_, _>>()?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let relative = dir.join(&name);
        let key = relative.to_string_lossy().replace('\\', "/");
        if name.starts_with('.')
            || name == "target"
            || name == "__pycache__"
            || skip.contains(&key.as_str())
        {
            continue;
        }
        if entry.file_type()?.is_dir() {
            files_under(root, &relative, skip, out)?;
        } else {
            let bytes = fs::read(entry.path()).with_context(|| format!("reading {}", key))?;
            out.push((key, bytes));
        }
    }
    Ok(())
}

/// `krate` and every workspace member it depends on, transitively (their
/// sources are translated with it).
fn with_workspace_deps<'a>(ws: &'a Loaded, krate: &'a str) -> Vec<&'a str> {
    let mut found = vec![krate];
    let mut index = 0;
    while let Some(&name) = found.get(index) {
        if let Some(member) = ws.model.members.iter().find(|m| m.name == name) {
            for dep in &member.workspace_deps {
                if !found.contains(&dep.as_str()) {
                    found.push(dep);
                }
            }
        }
        index = index.saturating_add(1);
    }
    found
}

/// `text` without its `version = ..` lines: a release bump does not change
/// what extraction sees, and must not stale every claim. (Kernel crates
/// have no external dependencies whose version could matter.)
fn without_versions(text: &str) -> String {
    text.lines()
        .filter(|line| {
            line.trim_start()
                .strip_prefix("version")
                .is_none_or(|rest| !rest.trim_start().starts_with('='))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Root files that change what extraction sees.
const WORKSPACE_FILES: &[&str] = &["Cargo.toml", "rust-toolchain", "rust-toolchain.toml"];

/// Per claimed crate: the fingerprint of the crate's directory, the
/// directories of the workspace members it depends on, the workspace
/// manifest and toolchain pin, and the proofs
/// directory (theorems, extraction, scripts, claims.toml; not the
/// recorded claims.json), as the gate sees them.
fn fingerprints(ws: &Loaded, file: &ClaimsFile) -> Result<BTreeMap<String, String>> {
    let crates: Vec<&str> = file.claim.iter().map(|c| c.krate.as_str()).collect();
    current_fingerprints(ws, &crates)
}

fn current_fingerprints(ws: &Loaded, crates: &[&str]) -> Result<BTreeMap<String, String>> {
    let root = ws.root();
    let mut shared = Vec::new();
    files_under(root, Path::new("proofs"), &[CLAIMS_JSON], &mut shared)?;
    // Extraction also reads the workspace manifest (edition, lints,
    // profiles) and the toolchain pin.
    for name in WORKSPACE_FILES {
        let path = root.join(name);
        if path.is_file() {
            let text = fs::read_to_string(&path).with_context(|| format!("reading {name}"))?;
            shared.push(((*name).to_owned(), without_versions(&text).into_bytes()));
        }
    }
    let mut out = BTreeMap::new();
    for &krate in crates {
        if out.contains_key(krate) {
            continue;
        }
        let mut files = shared.clone();
        for member in with_workspace_deps(ws, krate) {
            if let Some(dir) = crate_dir(ws, member) {
                files_under(root, &dir, &[], &mut files)?;
            }
        }
        out.insert(krate.to_owned(), proofs::fingerprint(&files));
    }
    Ok(out)
}

/// The committed `proofs/claims.json`, if present: a gate result or a
/// pending merge record (#50).
fn committed_record(root: &Path) -> Result<Option<RecordFile>> {
    let path = root.join(CLAIMS_JSON);
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    RecordFile::parse(&text)
        .map(Some)
        .map_err(|e| anyhow::anyhow!("parsing {}: {e}", path.display()))
}

/// Print one TAU-R002 line per unexcused weakening; `against` names the
/// record compared with (empty for the committed one).
fn report_weakened(found: &proofs::Conservation, against: &str) {
    for (_, f, function, kind, name) in &found.weakened {
        let change = match kind {
            proofs::AmendKind::Assumption => "gain assumption",
            proofs::AmendKind::Theorem => "lose theorem",
            proofs::AmendKind::Mutant => "lose mutant",
        };
        eprintln!(
            "tauten: TAU-R002: {f}: claim `{function}` would {change} `{name}`{against} without an `[[amend]]` entry"
        );
    }
}

/// `git` in `root`: its trimmed standard output, or `None` if it failed.
fn git(root: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// The gate result `proofs/claims.json` held at commit `oid`.
fn parent_record(root: &Path, oid: &str) -> Result<Record> {
    let Some(text) = git(root, &["show", &format!("{oid}:{CLAIMS_JSON}")]) else {
        anyhow::bail!("commit {oid} has no readable {CLAIMS_JSON}");
    };
    match RecordFile::parse(&text) {
        Ok(RecordFile::Result(record)) => Ok(record),
        Ok(RecordFile::Pending(_)) => anyhow::bail!(
            "{CLAIMS_JSON} at {oid} is itself a pending merge record; run its gate first"
        ),
        Err(e) => anyhow::bail!("parsing {CLAIMS_JSON} at {oid}: {e}"),
    }
}

/// TAU-R002 of `file` against each parent's record, decided together by
/// [`proofs::parents_conserved`]; prints every weakening.
fn parents_conserve(root: &Path, file: &ClaimsFile, parents: &[String]) -> Result<bool> {
    let mut found = Vec::new();
    for oid in parents {
        let record = parent_record(root, oid)?;
        let declared: Vec<&proofs::Claim> = record.claims.iter().map(|c| &c.claim).collect();
        let conserved = proofs::conservation_of(file, &declared);
        report_weakened(&conserved, &format!(" (recorded at parent {oid})"));
        found.push(conserved);
    }
    Ok(proofs::parents_conserved(&found))
}

/// Why a gate run may not replace a pending merge record, if it may not:
/// it must check a snapshot of the merge commit the record names (its two
/// parents, in order), and the resolved claims must still conserve both
/// parents' obligations. A hand-written pending file is not promoted just
/// because it parses.
/// Whether every `[[mutant]]` of `file` applies to the bytes under `root`
/// the gate will mutate ([`proofs::mutant_sites`]); prints each problem.
/// All of them are checked, at every level: a reused kill of a mutant whose
/// site is gone would be a result no full run could give.
fn sites_admitted(root: &Path, file: &ClaimsFile) -> bool {
    let read = |rel: &str| fs::read_to_string(root.join(rel)).ok();
    let problems = proofs::mutant_sites(file, &read);
    for p in &problems {
        eprintln!("tauten: {p}");
    }
    problems.is_empty()
}

/// Why a result of `file` could not replace the committed record, as far
/// as that is known without running the gate: an unexcused weakening
/// (TAU-R002, reported line by line), or a pending merge record this run
/// may not replace. Whether every claim passes is known only after it.
fn recording_refused(
    root: &Path,
    file: &ClaimsFile,
    committed: &RecordFile,
    commit: Option<&str>,
) -> Result<Option<String>> {
    let found = proofs::conservation_of(file, &committed.declared());
    if !found.weakened.is_empty() {
        report_weakened(&found, "");
        return Ok(Some("a recorded claim would be weakened".to_owned()));
    }
    if let RecordFile::Pending(pending) = committed
        && let Some(why) = promotion_refused(root, file, pending, commit)?
    {
        return Ok(Some(format!("{why}; the pending merge record is kept")));
    }
    Ok(None)
}

fn promotion_refused(
    root: &Path,
    file: &ClaimsFile,
    pending: &proofs::Pending,
    commit: Option<&str>,
) -> Result<Option<String>> {
    let Some(commit) = commit else {
        return Ok(Some(
            "a pending merge record is replaced only by a gate on a snapshot of the merge commit (run without --live)".to_owned(),
        ));
    };
    let parents: Vec<String> = ["^1", "^2"]
        .iter()
        .filter_map(|n| {
            git(
                root,
                &["rev-parse", "--verify", "-q", &format!("{commit}{n}")],
            )
        })
        .collect();
    if parents != pending.parents {
        return Ok(Some(format!(
            "commit {commit} is not the merge of {} the pending record names",
            pending.parents.join(" and ")
        )));
    }
    if !parents_conserve(root, file, &pending.parents)? {
        return Ok(Some(
            "the resolved claims weaken a parent's obligations".to_owned(),
        ));
    }
    Ok(None)
}

/// TAU-R002: `proofs/claims.toml` against the committed
/// `proofs/claims.json`, when both exist.
/// Applied after the lint baseline, so a baseline can never absorb it.
pub fn conservation(root: &Path, profile: &tauten_check::Profile) -> Result<Vec<Diagnostic>> {
    let toml_path = root.join(CLAIMS_TOML);
    if !toml_path.is_file() {
        return Ok(Vec::new());
    }
    let Some(record) = committed_record(root)? else {
        return Ok(Vec::new());
    };
    let text = fs::read_to_string(&toml_path)
        .with_context(|| format!("reading {}", toml_path.display()))?;
    let file = ClaimsFile::parse(&text)?;
    // A pending merge record's registry is the baseline until its gate runs.
    let found = proofs::conservation_of(&file, &record.declared());
    Ok(proofs::conservation_diagnostics(profile, &file, &found))
}

/// The recorded claims and whether each is fresh, if `proofs/claims.json`
/// exists.
pub struct RecordedProofs {
    record: Record,
    fresh: Vec<bool>,
    functions: BTreeMap<String, Vec<FunctionSpan>>,
}

/// Read `proofs/claims.json`, if present, and compare its fingerprints
/// with the current sources.
pub fn load(ws: &Loaded) -> Result<Option<RecordedProofs>> {
    let path = ws.root().join(CLAIMS_JSON);
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let record = match RecordFile::parse(&text) {
        Ok(RecordFile::Result(record)) => record,
        // A pending merge is not evidence: it subsumes nothing.
        Ok(RecordFile::Pending(_)) => return Ok(None),
        Err(e) => anyhow::bail!("parsing {}: {e}", path.display()),
    };
    let crates: Vec<&str> = record
        .claims
        .iter()
        .map(|c| c.claim.krate.as_str())
        .collect();
    let current = current_fingerprints(ws, &crates)?;
    let fresh = record
        .claims
        .iter()
        .map(|c| proofs::claim_fresh(&c.claim.krate, &c.fingerprint, &current))
        .collect();
    let mut functions = BTreeMap::new();
    for claim in &record.claims {
        let file = &claim.claim.file;
        if functions.contains_key(file) {
            continue;
        }
        let source = ws
            .model
            .members
            .iter()
            .flat_map(|m| &m.sources)
            .find(|s| &s.path == file);
        if let Some(source) = source {
            functions.insert(file.clone(), proofs::functions(&source.text));
        }
    }
    Ok(Some(RecordedProofs {
        record,
        fresh,
        functions,
    }))
}

/// Mark the findings a fresh proof subsumes.
pub fn subsume(diags: Vec<Diagnostic>, loaded: Option<&RecordedProofs>) -> Vec<Diagnostic> {
    let Some(loaded) = loaded else {
        return diags;
    };
    let recorded: Vec<Recorded<'_>> = loaded
        .record
        .claims
        .iter()
        .zip(&loaded.fresh)
        .map(|(record, &fresh)| Recorded { record, fresh })
        .collect();
    proofs::subsume(diags, &recorded, &loaded.functions)
}

/// `cargo tauten proofs reconcile` (#50): in a two-parent merge whose
/// human-authored conflicts are resolved, check the resolved claims against
/// both parents' records, regenerate the declared extraction, and write a
/// pending merge record. It never stages, commits, or edits a hand-written
/// file; only the extraction outputs and `proofs/claims.json` change, and the
/// result is pending, not proved, until a gate runs on the merge commit.
pub fn reconcile(ws: &Loaded, entry_name: Option<&str>) -> Result<ExitCode> {
    let root = ws.root();
    // The merge: HEAD and exactly one MERGE_HEAD.
    let merge_head = git(root, &["rev-parse", "--git-path", "MERGE_HEAD"])
        .map(|p| root.join(p))
        .filter(|p| p.is_file());
    let Some(merge_head) = merge_head else {
        anyhow::bail!(
            "no merge in progress: run reconcile in a two-parent merge, after resolving and staging the human-authored conflicts"
        );
    };
    let heads: Vec<String> = fs::read_to_string(&merge_head)
        .with_context(|| format!("reading {}", merge_head.display()))?
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect();
    let [second] = heads.as_slice() else {
        anyhow::bail!(
            "reconcile supports a two-parent merge; this one has {} merge heads",
            heads.len().saturating_add(1)
        );
    };
    let Some(first) = git(root, &["rev-parse", "--verify", "HEAD"]) else {
        anyhow::bail!("cannot read HEAD");
    };
    if &first == second {
        anyhow::bail!("the merge's parents are the same commit");
    }
    let parents = vec![first.clone(), second.clone()];

    // The declared extraction, unambiguously.
    let entries = crate::generated::entries(root)?;
    let candidates: Vec<&tauten_check::generated::Entry> = entries
        .iter()
        .filter(|e| !e.outputs.is_empty())
        .filter(|e| match entry_name {
            Some(name) => e.name == name,
            None => e
                .outputs
                .iter()
                .any(|o| o == EXTRACTED || o.starts_with(&format!("{EXTRACTED}/"))),
        })
        .collect();
    let [entry] = candidates.as_slice() else {
        anyhow::bail!(
            "reconcile needs exactly one [[workspace.metadata.tauten.generated]] entry with `outputs` producing {EXTRACTED} (or name one with --generated); found {}",
            candidates.len()
        );
    };

    // Only generated outputs and the record may still be unmerged.
    let under = |path: &str| {
        path == CLAIMS_JSON
            || entry
                .outputs
                .iter()
                .any(|o| path == o || path.starts_with(&format!("{o}/")))
    };
    let unmerged = git(root, &["diff", "--name-only", "--diff-filter=U"]).unwrap_or_default();
    let human: Vec<&str> = unmerged
        .lines()
        .filter(|p| !p.is_empty() && !under(p))
        .collect();
    if !human.is_empty() {
        anyhow::bail!(
            "resolve and stage these conflicts first (reconcile only regenerates {} and {CLAIMS_JSON}): {}",
            entry.outputs.join(", "),
            human.join(", ")
        );
    }

    // Both parents' obligations, in the resolved claims.
    let path = root.join(CLAIMS_TOML);
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let file = ClaimsFile::parse(&text)?;
    if !parents_conserve(root, &file, &parents)? {
        eprintln!(
            "tauten: not reconciling: the resolved {CLAIMS_TOML} weakens a parent's obligations (add a reasoned [[amend]] if intended)"
        );
        return Ok(ExitCode::FAILURE);
    }

    // Regenerate in scratch before touching the checkout.
    let (scratch, produced) = match crate::generated::produce(root, entry, &[CLAIMS_JSON])? {
        Ok(found) => found,
        Err(why) => anyhow::bail!("regenerating `{}` failed: {why}", entry.name),
    };
    let mut committed = BTreeMap::new();
    for out in &entry.outputs {
        crate::generated::files_under(root, out, &mut committed)?;
    }
    let changed = match tauten_check::generated::compare(&entry.outputs, &committed, &produced) {
        tauten_check::generated::Outcome::Same => Vec::new(),
        tauten_check::generated::Outcome::Drift(paths) => paths,
        tauten_check::generated::Outcome::Missing(paths) => {
            anyhow::bail!(
                "regenerating `{}` produced no {}",
                entry.name,
                paths.join(", ")
            )
        }
        tauten_check::generated::Outcome::Failed(why) => anyhow::bail!("{why}"),
    };

    // The pending record first, then the regenerated outputs: an
    // interruption leaves the record pending, and a rerun finishes.
    let pending = proofs::Pending {
        version: 2,
        state: proofs::PENDING_MERGE.to_owned(),
        parents: parents.clone(),
        registry: file.claim.clone(),
    };
    let mut json = serde_json::to_string_pretty(&pending)?;
    json.push('\n');
    let target = root.join(CLAIMS_JSON);
    fs::write(&target, json).with_context(|| format!("writing {}", target.display()))?;
    for out in &entry.outputs {
        let dest = root.join(out);
        if dest.is_dir() {
            fs::remove_dir_all(&dest).with_context(|| format!("clearing {}", dest.display()))?;
        } else if dest.is_file() {
            fs::remove_file(&dest).with_context(|| format!("clearing {}", dest.display()))?;
        }
    }
    for (rel, bytes) in &produced {
        let dest = root.join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&dest, bytes).with_context(|| format!("writing {}", dest.display()))?;
    }
    drop(scratch);

    eprintln!(
        "tauten: reconciled the merge of {first} and {second}: {} regenerated path(s) changed",
        changed.len()
    );
    for path in &changed {
        println!("{path}");
    }
    println!("{CLAIMS_JSON}");
    eprintln!(
        "tauten: {CLAIMS_JSON} is pending, not proved: stage the paths above, commit the merge, then run `cargo tauten proofs --level ship` on the merge commit"
    );
    Ok(ExitCode::SUCCESS)
}
