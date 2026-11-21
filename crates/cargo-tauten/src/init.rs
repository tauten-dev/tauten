//! `cargo tauten init`: propose tiers for untiered crates and write the
//! agent profile. Prints the plan; changes files only with `--write`.

use std::fs;
use tauten_check::scan::NoStd;

use anyhow::{Context, Result};
use tauten_check::Tier;
use tauten_check::profile::agent_profile;

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
        let lib_root = member.sources.iter().find(|f| f.is_lib_root);
        let (tier, reason) = match lib_root {
            Some(root)
                if matches!(
                    tauten_check::scan::no_std(&root.text),
                    NoStd::Unconditional | NoStd::NotTest
                ) =>
            {
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
    // Every edit is computed and checked before anything is written, so a
    // refused manifest leaves the whole workspace untouched.
    let mut edits = Vec::new();
    for p in &proposals {
        println!(
            "{:<28} tier = \"{}\"   ({}; {})",
            p.name, p.tier, p.reason, p.manifest
        );
        let path = ws.root().join(p.manifest);
        let text =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let edited = with_tier(&text, p.tier)
            .with_context(|| format!("cannot add the tier to {}", p.manifest))?;
        edits.push((path, edited));
    }
    if write {
        for (path, text) in &edits {
            fs::write(path, text).with_context(|| format!("writing {}", path.display()))?;
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
        fs::write(&agents, agent_profile())
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

/// `manifest` with `package.metadata.tauten.tier = tier` added, keeping
/// every other key, comment and table representation. An existing
/// `[package.metadata.tauten]` table (explicit, dotted or inline) gets the
/// key; a shape that cannot hold it is refused, and the result is re-read
/// with a TOML parser before it is returned.
fn with_tier(manifest: &str, tier: Tier) -> Result<String> {
    use toml_edit::{DocumentMut, Item, Table, Value, value};
    let mut doc: DocumentMut = manifest.parse().context("the manifest is not valid TOML")?;
    let package = doc
        .get_mut("package")
        .and_then(Item::as_table_like_mut)
        .context("no [package] table")?;
    if package.get("metadata").is_none() {
        let mut metadata = Table::new();
        metadata.set_implicit(true);
        package.insert("metadata", Item::Table(metadata));
    }
    let metadata = package
        .get_mut("metadata")
        .and_then(Item::as_table_like_mut)
        .context("`package.metadata` is not a table; add `tier` by hand")?;
    if metadata.get("tauten").is_none() {
        metadata.insert("tauten", Item::Table(Table::new()));
    }
    let tauten = metadata.get_mut("tauten").context("tauten table")?;
    let tier_value = tier.to_string();
    match tauten {
        Item::Table(table) => {
            refuse_existing(table.contains_key("tier"))?;
            table.insert("tier", value(tier_value.as_str()));
        }
        Item::Value(Value::InlineTable(table)) => {
            refuse_existing(table.contains_key("tier"))?;
            table.insert("tier", tier_value.as_str().into());
            table.fmt();
        }
        _ => anyhow::bail!("`package.metadata.tauten` is not a table; add `tier` by hand"),
    }
    let edited = doc.to_string();
    let check: toml::Table = edited
        .parse()
        .context("the edited manifest would not parse; nothing written")?;
    let written = check
        .get("package")
        .and_then(|p| p.get("metadata"))
        .and_then(|m| m.get("tauten"))
        .and_then(|t| t.get("tier"))
        .and_then(toml::Value::as_str);
    anyhow::ensure!(
        written == Some(tier_value.as_str()),
        "the edited manifest would not declare the tier; nothing written"
    );
    Ok(edited)
}

/// A `tier` key that is present but not a string is the user's to fix:
/// `init` only fills a missing tier.
fn refuse_existing(present: bool) -> Result<()> {
    anyhow::ensure!(
        !present,
        "`package.metadata.tauten.tier` is present but not a string; fix it by hand"
    );
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
        "proofs/Proofs/TautenMachine.lean",
        include_str!("../templates/proofs/Proofs/TautenMachine.lean"),
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
/// unless they are already on. A `[profile.release]` table that already
/// sets `overflow-checks` is left for the user to edit; otherwise the key
/// is added to the existing table (in whatever form) or a new one.
fn hardening(ws: &Loaded, write: bool) -> Result<()> {
    use tauten_check::manifest::Fact;
    use toml_edit::{DocumentMut, Item, Table, value};
    if ws.model.overflow_checks == Some(Fact::Yes) {
        println!("{:<28} overflow-checks already on", "Cargo.toml");
        return Ok(());
    }
    let path = ws.root().join("Cargo.toml");
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let mut doc: DocumentMut = text
        .parse()
        .with_context(|| format!("parsing {}", path.display()))?;
    let by_hand = || {
        println!(
            "{:<28} set `overflow-checks = true` under [profile.release] by hand (TAU-H001)",
            "Cargo.toml"
        );
        Ok(())
    };
    let profile = doc.entry("profile").or_insert_with(|| {
        let mut t = Table::new();
        t.set_implicit(true);
        Item::Table(t)
    });
    let Some(profile) = profile.as_table_like_mut() else {
        return by_hand();
    };
    let fresh = profile.get("release").is_none();
    if fresh {
        let mut release = Table::new();
        release
            .decor_mut()
            .set_prefix("\n# TAU-H001: release builds check integer overflow.\n");
        profile.insert("release", Item::Table(release));
    }
    let Some(release) = profile
        .get_mut("release")
        .and_then(Item::as_table_like_mut)
        .filter(|r| !r.contains_key("overflow-checks"))
    else {
        return by_hand();
    };
    release.insert("overflow-checks", value(true));
    println!(
        "{:<28} [profile.release] overflow-checks = true",
        "Cargo.toml"
    );
    if write {
        fs::write(&path, doc.to_string()).with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(())
}

/// `LedgerKernel` for `ledger-kernel`: the name Aeneas gives the
/// translation file, and the theorem module.
pub fn camel(name: &str) -> String {
    name.split(['-', '_'])
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect()
}

/// `text` with the proofs template's placeholders filled for `krate`.
pub fn fill_proofs(text: &str, krate: &str, crate_dir: &str) -> String {
    let module = camel(krate);
    text.replace("{{CRATE_DIR}}", crate_dir)
        .replace("{{CRATE_SNAKE}}", &krate.replace('-', "_"))
        .replace("{{CRATE}}", krate)
        .replace("{{LEAN_FILE}}", &module)
        .replace("{{MODULE}}", &module)
}

/// The template's files for `krate`, placeholders filled.
pub fn proofs_files(krate: &str, crate_dir: &str) -> Vec<(String, String)> {
    let fill = |text: &str| fill_proofs(text, krate, crate_dir);
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
