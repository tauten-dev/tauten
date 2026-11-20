//! `cargo tauten init`: propose tiers for untiered crates and write the
//! agent profile. Prints the plan; changes files only with `--write`.

use std::fs;

use anyhow::{Context, Result};
use tauten_check::Tier;
use tauten_check::profile::AGENT_PROFILE;

use crate::workspace::Loaded;

pub const AGENTS_FILE: &str = "AGENTS.tauten.md";

/// A proposed tier and why.
struct Proposal<'a> {
    name: &'a str,
    manifest: &'a str,
    tier: Tier,
    reason: &'static str,
}

fn propose(ws: &Loaded) -> Vec<Proposal<'_>> {
    let mut out = Vec::new();
    for member in ws.model.members.iter().filter(|m| m.tier_raw.is_none()) {
        let lib_root = member
            .sources
            .iter()
            .find(|f| f.is_crate_root && f.path.ends_with("lib.rs"));
        let (tier, reason) = match lib_root {
            Some(root) if tauten_check::scan::crate_attrs(&root.text).is_some_and(|a| a.no_std) => {
                (Tier::Kernel, "no_std library")
            }
            Some(_) => (Tier::Core, "library"),
            None => (Tier::Edge, "binary only"),
        };
        out.push(Proposal {
            name: &member.name,
            manifest: &member.manifest,
            tier,
            reason,
        });
    }
    out
}

pub fn run(ws: &Loaded, write: bool) -> Result<()> {
    let proposals = propose(ws);
    if proposals.is_empty() {
        println!("every crate already declares a tier");
    }
    for p in &proposals {
        println!(
            "{:<28} tier = \"{}\"   ({}; {})",
            p.name, p.tier, p.reason, p.manifest
        );
        if write {
            let path = ws.root().join(p.manifest);
            let mut text =
                fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            if !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(&format!(
                "\n[package.metadata.tauten]\ntier = \"{}\"\n",
                p.tier
            ));
            fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
        }
    }
    hardening(ws, write)?;
    let agents = ws.root().join(AGENTS_FILE);
    let note = if agents.is_file() {
        "exists; left unchanged"
    } else {
        "agent instructions"
    };
    println!("{AGENTS_FILE:<28} {note}");
    if write && !agents.is_file() {
        fs::write(&agents, AGENT_PROFILE)
            .with_context(|| format!("writing {}", agents.display()))?;
    }
    if write {
        println!("\nNext: review the tiers, point your AGENTS.md / CLAUDE.md at {AGENTS_FILE},");
        println!("then run `cargo tauten check`, or `cargo tauten baseline` to adopt gradually.");
    } else {
        println!(
            "\nNothing written. Re-run with --write to apply; change any tier afterwards by hand."
        );
    }
    Ok(())
}

/// The proofs template (`crates/cargo-tauten/templates/proofs`): published
/// path, contents. `{{...}}` placeholders are filled per crate.
const PROOFS_TEMPLATE: &[(&str, &str)] = &[
    (
        "proofs/claims.toml",
        include_str!("../templates/proofs/claims.toml"),
    ),
    (
        "proofs/README.md",
        include_str!("../templates/proofs/README.md"),
    ),
    (
        "proofs/TRUST.md",
        include_str!("../templates/proofs/TRUST.md"),
    ),
    ("proofs/run.sh", include_str!("../templates/proofs/run.sh")),
    (
        "proofs/compose.yaml",
        include_str!("../templates/proofs/compose.yaml"),
    ),
    (
        "proofs/container.sh",
        include_str!("../templates/proofs/container.sh"),
    ),
    (
        "proofs/extract.sh",
        include_str!("../templates/proofs/extract.sh"),
    ),
    (
        "proofs/refresh.sh",
        include_str!("../templates/proofs/refresh.sh"),
    ),
    (
        "proofs/gate.py",
        include_str!("../templates/proofs/gate.py"),
    ),
    (
        "proofs/lakefile.toml",
        include_str!("../templates/proofs/lakefile.toml"),
    ),
    (
        "proofs/lean-toolchain",
        include_str!("../templates/proofs/lean-toolchain"),
    ),
    (
        "proofs/lake-manifest.json",
        include_str!("../templates/proofs/lake-manifest.json"),
    ),
    (
        "proofs/Audit.lean",
        include_str!("../templates/proofs/Audit.lean"),
    ),
    (
        "proofs/Proofs.lean",
        include_str!("../templates/proofs/Proofs.lean"),
    ),
    (
        "proofs/Proofs/TautenAssumption.lean",
        include_str!("../templates/proofs/Proofs/TautenAssumption.lean"),
    ),
    (
        "proofs/Proofs/{{MODULE}}.lean",
        include_str!("../templates/proofs/Proofs/Module.lean"),
    ),
    (
        "proofs/.gitignore",
        include_str!("../templates/proofs/gitignore"),
    ),
];

/// TAU-H001: turn on release overflow checks in the workspace manifest,
/// unless they are already on. An existing `[profile.release]` table is
/// left for the user to edit (appending a second table would be invalid).
fn hardening(ws: &Loaded, write: bool) -> Result<()> {
    use tauten_check::manifest::Fact;
    if ws.model.overflow_checks == Some(Fact::Yes) {
        println!("{:<28} overflow-checks already on", "Cargo.toml");
        return Ok(());
    }
    let path = ws.root().join("Cargo.toml");
    let mut text =
        fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    if text.contains("[profile.release") {
        println!(
            "{:<28} set `overflow-checks = true` under [profile.release] by hand (TAU-H001)",
            "Cargo.toml"
        );
        return Ok(());
    }
    println!(
        "{:<28} [profile.release] overflow-checks = true",
        "Cargo.toml"
    );
    if write {
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(
            "\n# TAU-H001: release builds check integer overflow.\n[profile.release]\noverflow-checks = true\n",
        );
        fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(())
}

/// `LedgerKernel` for `ledger-kernel`: the name Aeneas gives the
/// translation file, and the theorem module.
fn camel(name: &str) -> String {
    name.split(['-', '_'])
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect()
}

/// The template's files for `krate`, placeholders filled.
fn proofs_files(krate: &str, crate_dir: &str) -> Vec<(String, String)> {
    let module = camel(krate);
    let fill = |text: &str| {
        text.replace("{{CRATE_DIR}}", crate_dir)
            .replace("{{CRATE_SNAKE}}", &krate.replace('-', "_"))
            .replace("{{CRATE}}", krate)
            .replace("{{LEAN_FILE}}", &module)
            .replace("{{MODULE}}", &module)
    };
    PROOFS_TEMPLATE
        .iter()
        .map(|(path, text)| (fill(path), fill(text)))
        .collect()
}

/// `cargo tauten init --proofs <crate>`: write the proofs template for a
/// kernel crate. Prints the plan; writes only with `--write`, and never
/// over an existing `proofs/`.
pub fn proofs(ws: &Loaded, krate: &str, write: bool) -> Result<()> {
    let Some(member) = ws.model.members.iter().find(|m| m.name == krate) else {
        anyhow::bail!("no workspace member named `{krate}`");
    };
    if member.tier() != Some(Tier::Kernel) {
        println!(
            "note: `{krate}` is not declared `kernel`; Charon/Aeneas need dependency-free, \
             plain code, and only kernel crates get proof subsumption of N001/P003"
        );
    }
    let crate_dir = std::path::Path::new(&member.manifest)
        .parent()
        .map_or_else(String::new, |p| p.to_string_lossy().replace('\\', "/"));
    let target = ws.root().join("proofs");
    if target.exists() {
        anyhow::bail!(
            "{} already exists; nothing written (copy the files of \
             crates/cargo-tauten/templates/proofs by hand to merge)",
            target.display()
        );
    }
    let files = proofs_files(krate, &crate_dir);
    for (path, _) in &files {
        println!("{path}");
    }
    if !write {
        println!("\nNothing written. Re-run with --write to create these files.");
        return Ok(());
    }
    for (path, text) in &files {
        let full = ws.root().join(path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        fs::write(&full, text).with_context(|| format!("writing {}", full.display()))?;
    }
    println!("\nNext (see proofs/README.md):");
    println!("  1. sh proofs/refresh.sh      # translate {crate_dir} into proofs/Extracted/");
    println!("  2. write theorems in proofs/Proofs/{}.lean", camel(krate));
    println!("  3. list them in proofs/claims.toml, with a mutant each");
    println!("  4. cargo tauten proofs       # run the gate, record proofs/claims.json");
    Ok(())
}
