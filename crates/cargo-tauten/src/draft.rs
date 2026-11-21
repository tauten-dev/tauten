//! `cargo tauten proofs draft` (#49): print a reviewable patch to
//! `proofs/claims.toml` for new or changed functions. The decisions are in
//! [`tauten_check::proofs::draft`]; this reads the files, runs the gate up
//! to the audit (`[gate] try` with no mutant selected) and prints.
//!
//! Writes nothing: not `proofs/claims.toml`, not `proofs/claims.json`, not
//! the index. Exit 0 means the draft succeeded (stdout is the patch, empty
//! when there is nothing to draft), never that a gate passed.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use tauten_check::proofs::ClaimsFile;
use tauten_check::proofs::draft::{self, Inputs, Inventory, Package};

use crate::proofs::{CLAIMS_TOML, run_gate};
use crate::workspace::Loaded;

/// The packages of `[gate] crates`, or only `only`.
fn packages(ws: &Loaded, file: &ClaimsFile, only: Option<&str>) -> Result<Vec<Package>> {
    let configured: Vec<&str> = file
        .gate
        .crates
        .iter()
        .map(|c| c.trim_end_matches('/'))
        .collect();
    let mut out = Vec::new();
    for member in &ws.model.members {
        let dir = Path::new(&member.manifest)
            .parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let lib = ws
            .metadata
            .packages
            .iter()
            .find(|p| p.name.as_str() == member.name)
            .and_then(|p| p.targets.iter().find(|t| t.is_lib()))
            .map_or_else(
                || member.name.replace('-', "_"),
                |t| t.name.replace('-', "_"),
            );
        let package = Package {
            name: member.name.clone(),
            dir: dir.clone(),
            lib,
        };
        match only {
            Some(name) if name == member.name => {
                if !configured.contains(&dir.as_str()) {
                    bail!(
                        "package `{name}` (in `{dir}`) is not one of [gate] crates in {CLAIMS_TOML}, so the gate does not translate it; no patch"
                    );
                }
                out.push(package);
            }
            Some(_) => {}
            None if configured.contains(&dir.as_str()) => out.push(package),
            None => {}
        }
    }
    if let Some(name) = only
        && out.is_empty()
    {
        bail!("unknown package `{name}`: not a member of this workspace; no patch");
    }
    if out.is_empty() {
        bail!("no workspace package is one of [gate] crates in {CLAIMS_TOML}; no patch");
    }
    Ok(out)
}

/// `cargo tauten proofs draft [--crate C] [--inventory PATH]`.
pub fn run(ws: &Loaded, only: Option<&str>, inventory: Option<&Path>) -> Result<ExitCode> {
    let root = ws.root();
    let path = root.join(CLAIMS_TOML);
    let text = fs::read_to_string(&path).with_context(|| {
        format!(
            "reading {}; set up the proof tier with `cargo tauten init --proofs <crate>`",
            path.display()
        )
    })?;
    let claims = ClaimsFile::parse(&text)?;
    let packages = packages(ws, &claims, only)?;
    let inventory = inventory
        .map(|p| -> Result<Inventory> {
            let text = fs::read_to_string(p).with_context(|| format!("reading {}", p.display()))?;
            Inventory::parse(&text).map_err(|e| anyhow::anyhow!("{}: {e}", p.display()))
        })
        .transpose()?;
    let Some(mut command) = claims.gate.try_.clone() else {
        bail!(
            "{CLAIMS_TOML} has no `[gate] try` command, which a draft runs with no mutant selected to build and audit the theorems; the template's is \
             [\"sh\", \"proofs/run.sh\", \"python3\", \"proofs/gate.py\", \"--only-mutants\"]"
        );
    };
    let mut extracted = BTreeMap::new();
    let dir = root.join("proofs/Extracted");
    if dir.is_dir() {
        let mut entries: Vec<_> = fs::read_dir(&dir)
            .with_context(|| format!("reading {}", dir.display()))?
            .collect::<Result<_, _>>()?;
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".lean") {
                let text = fs::read_to_string(entry.path())
                    .with_context(|| format!("reading {}", entry.path().display()))?;
                extracted.insert(name, text);
            }
        }
    }
    if extracted.is_empty() {
        bail!(
            "proofs/Extracted/ holds no translation; run `sh proofs/refresh.sh` and commit it; no patch"
        );
    }
    let names: Vec<&str> = packages.iter().map(|p| p.name.as_str()).collect();
    let sources: BTreeMap<String, String> = ws
        .model
        .members
        .iter()
        .filter(|m| names.contains(&m.name.as_str()))
        .flat_map(|m| &m.sources)
        .map(|s| (s.path.clone(), s.text.clone()))
        .collect();
    // Every stage up to the audit, and no mutant: an empty selection.
    command.push(String::new());
    eprintln!("tauten: draft: building and auditing the theorems (no mutants)");
    let report = run_gate(root, &command)?;
    let inputs = Inputs {
        claims: &claims,
        text: &text,
        packages: &packages,
        extracted: &extracted,
        sources: &sources,
        report: &report,
        inventory: inventory.as_ref(),
    };
    match draft::draft(&inputs) {
        Ok(drafted) => {
            for note in &drafted.notes {
                eprintln!("tauten: draft: {note}");
            }
            print!("{}", drafted.patch);
            if drafted.patch.is_empty() {
                eprintln!("tauten: draft: nothing to add to {CLAIMS_TOML}");
            } else {
                eprintln!(
                    "tauten: draft: {} new claim(s), {} extended; review the patch, `git apply` it, commit, then run `cargo tauten proofs`. A draft is not a gate result and grants no proof credit",
                    drafted.added, drafted.extended
                );
            }
            Ok(ExitCode::SUCCESS)
        }
        Err(reasons) => {
            for reason in &reasons {
                eprintln!("tauten: draft refused: {reason}");
            }
            eprintln!("tauten: draft: no patch printed; {CLAIMS_TOML} is unchanged");
            Ok(ExitCode::FAILURE)
        }
    }
}
