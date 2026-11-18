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
