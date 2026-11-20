//! `cargo tauten generated`: regenerate every declared generated file and
//! compare it byte for byte with the committed one
//! ([`tauten_check::generated`]).
//!
//! Entries live in the root manifest:
//!
//! ```toml
//! [[workspace.metadata.tauten.generated]]
//! name = "rules"
//! command = ["cargo", "run", "--quiet", "-p", "cargo-tauten", "--", "tauten", "explain", "--markdown"]
//! stdout = "docs/RULES.md"
//!
//! [[workspace.metadata.tauten.generated]]
//! name = "translation"
//! command = ["sh", "proofs/refresh.sh"]
//! outputs = ["proofs/Extracted"]
//! ```
//!
//! A `stdout` entry runs in the workspace and compares its standard output.
//! An `outputs` entry runs in a scratch copy of the workspace (the files Git
//! knows about, or every file outside `target/` and `.git/` without Git), so
//! the checkout is never written, and compares the files under each output.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use anyhow::{Context, Result, bail};
use tauten_check::generated::{self, Entry, Outcome};

/// Read the entries from the root manifest.
fn entries(root: &Path) -> Result<Vec<Entry>> {
    let path = root.join("Cargo.toml");
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let table: toml::Table = text
        .parse()
        .with_context(|| format!("parsing {}", path.display()))?;
    let Some(list) = table
        .get("workspace")
        .and_then(|w| w.get("metadata"))
        .and_then(|m| m.get("tauten"))
        .and_then(|t| t.get("generated"))
    else {
        return Ok(Vec::new());
    };
    let entries: Vec<Entry> = list
        .clone()
        .try_into()
        .context("reading [[workspace.metadata.tauten.generated]]")?;
    for e in &entries {
        if e.command.is_empty() {
            bail!("generated entry `{}` has an empty command", e.name);
        }
        if e.stdout.is_some() == !e.outputs.is_empty() {
            bail!(
                "generated entry `{}` needs exactly one of `stdout` or `outputs`",
                e.name
            );
        }
    }
    Ok(entries)
}

/// Every regular file at or under `rel` in `base`, keyed by path relative
/// to `base`.
fn files_under(base: &Path, rel: &str, out: &mut BTreeMap<String, Vec<u8>>) -> Result<()> {
    let path = base.join(rel);
    if path.is_file() {
        out.insert(rel.to_owned(), fs::read(&path)?);
    } else if path.is_dir() {
        let mut names: Vec<_> = fs::read_dir(&path)?
            .map(|e| e.map(|e| e.file_name()))
            .collect::<Result<_, _>>()?;
        names.sort();
        for name in names {
            let child = format!("{rel}/{}", name.to_string_lossy());
            files_under(base, &child, out)?;
        }
    }
    Ok(())
}

/// A scratch copy of the workspace, removed on drop.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _removed = fs::remove_dir_all(&self.0).is_ok();
    }
}

fn copy_workspace(root: &Path) -> Result<Scratch> {
    let dir = std::env::temp_dir().join(format!("tauten-generated-{}", std::process::id()));
    if dir.exists() {
        fs::remove_dir_all(&dir).with_context(|| format!("clearing {}", dir.display()))?;
    }
    fs::create_dir_all(&dir)?;
    let scratch = Scratch(dir.clone());
    let listed = Command::new("git")
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .current_dir(root)
        .output();
    let files: Vec<String> = match listed {
        // A workspace Git does not track (for example a scratch copy inside
        // an ignored directory) is walked instead.
        Ok(out)
            if out.status.success()
                && String::from_utf8_lossy(&out.stdout)
                    .split('\0')
                    .any(|f| f == "Cargo.toml") =>
        {
            String::from_utf8_lossy(&out.stdout)
                .split('\0')
                .filter(|f| !f.is_empty())
                .map(str::to_owned)
                .collect()
        }
        Ok(_) | Err(_) => {
            let mut all = BTreeMap::new();
            walk(root, root, &mut all)?;
            all.into_keys().collect()
        }
    };
    for file in files {
        let from = root.join(&file);
        if !from.is_file() {
            continue;
        }
        let to = dir.join(&file);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&from, &to).with_context(|| format!("copying {file}"))?;
    }
    Ok(scratch)
}

fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, ()>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
        if matches!(name.as_deref(), Some("target" | ".git")) {
            continue;
        }
        if path.is_dir() {
            walk(root, &path, out)?;
        } else if let Ok(rel) = path.strip_prefix(root) {
            out.insert(rel.to_string_lossy().into_owned(), ());
        }
    }
    Ok(())
}

fn run_one(root: &Path, entry: &Entry) -> Result<Outcome> {
    let (program, args) = entry.command.split_first().context("empty command")?;
    if let Some(stdout) = &entry.stdout {
        let output = Command::new(program)
            .args(args)
            .current_dir(root)
            .stderr(Stdio::inherit())
            .output();
        let output = match output {
            Ok(o) if o.status.success() => o,
            Ok(o) => {
                return Ok(Outcome::Failed(format!(
                    "`{}` exited with {}",
                    entry.command.join(" "),
                    o.status
                )));
            }
            Err(e) => {
                return Ok(Outcome::Failed(format!(
                    "`{program}` could not be run: {e}"
                )));
            }
        };
        let mut committed = BTreeMap::new();
        files_under(root, stdout, &mut committed)?;
        let produced = BTreeMap::from([(stdout.clone(), output.stdout)]);
        return Ok(generated::compare(
            std::slice::from_ref(stdout),
            &committed,
            &produced,
        ));
    }
    let scratch = copy_workspace(root)?;
    for out in &entry.outputs {
        let target = scratch.0.join(out);
        if target.is_dir() {
            fs::remove_dir_all(&target)?;
        } else if target.is_file() {
            fs::remove_file(&target)?;
        }
    }
    let status = Command::new(program)
        .args(args)
        .current_dir(&scratch.0)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            return Ok(Outcome::Failed(format!(
                "`{}` exited with {s}",
                entry.command.join(" ")
            )));
        }
        Err(e) => {
            return Ok(Outcome::Failed(format!(
                "`{program}` could not be run: {e}"
            )));
        }
    }
    let mut committed = BTreeMap::new();
    let mut produced = BTreeMap::new();
    for out in &entry.outputs {
        files_under(root, out, &mut committed)?;
        files_under(&scratch.0, out, &mut produced)?;
    }
    Ok(generated::compare(&entry.outputs, &committed, &produced))
}

/// Run the stage; `only` restricts it to the named entries.
pub fn run(root: &Path, only: &[String]) -> Result<ExitCode> {
    let all = entries(root)?;
    for name in only {
        if !all.iter().any(|e| &e.name == name) {
            bail!("no generated entry named `{name}`");
        }
    }
    let selected: Vec<&Entry> = all
        .iter()
        .filter(|e| only.is_empty() || only.contains(&e.name))
        .collect();
    if selected.is_empty() {
        bail!("no [[workspace.metadata.tauten.generated]] entries in Cargo.toml: nothing to check");
    }
    let mut outcomes = Vec::new();
    for entry in selected {
        let outcome = run_one(root, entry)?;
        match &outcome {
            Outcome::Same => println!("generated {}: ok", entry.name),
            Outcome::Drift(paths) => {
                println!("generated {}: drift in {}", entry.name, paths.join(", "));
            }
            Outcome::Missing(paths) => println!(
                "generated {}: failed: the command did not produce {}",
                entry.name,
                paths.join(", ")
            ),
            Outcome::Failed(why) => println!("generated {}: failed: {why}", entry.name),
        }
        outcomes.push(outcome);
    }
    Ok(if generated::passes(&outcomes) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}
