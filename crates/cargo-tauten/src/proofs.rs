//! `cargo tauten proofs`: run the project's proof gate and record one
//! outcome per claim of `proofs/claims.toml` in `proofs/claims.json`; and,
//! for `cargo tauten check`, read that record back to mark the findings a
//! fresh proof subsumes (`tauten_check::proofs`).

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use anyhow::{Context, Result};
use tauten_check::Diagnostic;
use tauten_check::proofs::{
    self, ClaimRecord, ClaimsFile, Event, FunctionSpan, Outcome, Record, Recorded, Report,
};

use crate::workspace::Loaded;

/// The claims a project declares.
pub const CLAIMS_TOML: &str = "proofs/claims.toml";
/// The outcomes `cargo tauten proofs` recorded (and the badge data).
pub const CLAIMS_JSON: &str = "proofs/claims.json";

/// Run the gate and record the outcomes. Exit 0 iff every claim is proved.
pub fn run(ws: &Loaded, write: bool) -> Result<ExitCode> {
    let root = ws.root();
    let path = root.join(CLAIMS_TOML);
    let text = fs::read_to_string(&path).with_context(|| {
        format!(
            "reading {}; set up the proof tier with `cargo tauten init --proofs <crate>`",
            path.display()
        )
    })?;
    let file = ClaimsFile::parse(&text)?;
    let before = fingerprints(ws, &file)?;
    let mut report = run_gate(root, &file.gate.command)?;
    report.changed_during_run = fingerprints(ws, &file)? != before;

    let mut records = Vec::new();
    for claim in &file.claim {
        let (outcome, detail) = proofs::outcome(claim, &report);
        let fingerprint = before.get(&claim.krate).cloned().unwrap_or_default();
        records.push(ClaimRecord {
            claim: claim.clone(),
            outcome,
            detail,
            fingerprint,
        });
    }
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for record in &records {
        serde_json::to_writer(&mut out, record)?;
        writeln!(out)?;
    }
    if records.is_empty() {
        eprintln!("tauten: {CLAIMS_TOML} lists no claim");
    }
    if write {
        let record = Record {
            version: 1,
            generated_by: "cargo tauten proofs".to_owned(),
            claims: records.clone(),
        };
        let mut json = serde_json::to_string_pretty(&record)?;
        json.push('\n');
        let target = root.join(CLAIMS_JSON);
        fs::write(&target, json).with_context(|| format!("writing {}", target.display()))?;
    }
    let proved = records
        .iter()
        .filter(|r| r.outcome == Outcome::Proved)
        .count();
    eprintln!(
        "tauten: {proved} of {} claim(s) proved{}",
        records.len(),
        if write {
            format!(" (recorded in {CLAIMS_JSON})")
        } else {
            String::new()
        }
    );
    Ok(if proved == records.len() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Run the gate command from the workspace root. Its output goes to stderr
/// (progress); result lines are collected. A command that cannot start is
/// reported, never treated as a pass.
fn run_gate(root: &Path, command: &[String]) -> Result<Report> {
    let mut report = Report::default();
    let Some((program, args)) = command.split_first() else {
        report.not_started = Some("[gate] command is empty".to_owned());
        return Ok(report);
    };
    let spawned = Command::new(program)
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn();
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

/// Per claimed crate: the fingerprint of the crate's directory, the
/// directories of the workspace members it depends on, and the proofs
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
    let record: Record =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    let crates: Vec<&str> = record
        .claims
        .iter()
        .map(|c| c.claim.krate.as_str())
        .collect();
    let current = current_fingerprints(ws, &crates)?;
    let fresh = record
        .claims
        .iter()
        .map(|c| current.get(&c.claim.krate) == Some(&c.fingerprint))
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
