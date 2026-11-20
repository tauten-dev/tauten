//! `cargo tauten obligations`: in kernel crates, arithmetic and indexing
//! findings (TAU-N001, TAU-N002, TAU-P003) read as proof obligations.
//!
//! Each one is `proved` (subsumed by a recorded proof that the function
//! never fails, `proofs/claims.json`), `open, baselined` (accepted by the
//! ratchet) or `open` (new: it fails `check`). The counts are the ones
//! `check` already decides; nothing new is judged here.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;

use anyhow::Result;
use tauten_check::baseline;
use tauten_check::{Diagnostic, Profile, Tier};

use crate::baseline_file;

/// The rules whose kernel findings are obligations.
pub const RULES: [&str; 3] = ["TAU-N001", "TAU-N002", "TAU-P003"];

#[derive(Default)]
struct Counts {
    proved: u64,
    baselined: u64,
    open: u64,
}

impl Counts {
    /// Counts saturate: a workspace cannot hold `u64::MAX` findings.
    const fn total(&self) -> u64 {
        self.proved
            .saturating_add(self.baselined)
            .saturating_add(self.open)
    }

    const fn add(&mut self, other: &Self) {
        self.proved = self.proved.saturating_add(other.proved);
        self.baselined = self.baselined.saturating_add(other.baselined);
        self.open = self.open.saturating_add(other.open);
    }
}

/// Print the obligations of every kernel crate. Fails when an obligation
/// is open and not baselined, exactly as `check` does for it.
pub fn run(profile: &Profile, root: &Path, diags: Vec<Diagnostic>, list: bool) -> Result<ExitCode> {
    let obligations: Vec<Diagnostic> = diags
        .into_iter()
        .filter(|d| d.tier == Some(Tier::Kernel) && RULES.contains(&d.id.as_str()))
        .collect();
    let mut counts: BTreeMap<String, Counts> = BTreeMap::new();
    let (proved, unproved): (Vec<Diagnostic>, Vec<Diagnostic>) = obligations
        .into_iter()
        .partition(|d| d.subsumed_by.is_some());
    for d in &proved {
        let c = counts.entry(d.krate.clone()).or_default();
        c.proved = c.proved.saturating_add(1);
    }
    let path = root.join(baseline_file::DEFAULT_NAME);
    let open: Vec<Diagnostic> = if path.is_file() {
        let accepted = baseline_file::read(&path)?;
        let applied = baseline::apply(profile, &accepted, unproved.clone());
        // Entries for other findings are not this command's business.
        let open: Vec<Diagnostic> = applied
            .diagnostics
            .into_iter()
            .filter(|d| RULES.contains(&d.id.as_str()))
            .collect();
        let mut remaining = open.clone();
        for d in &unproved {
            if let Some(i) = remaining.iter().position(|r| r == d) {
                remaining.swap_remove(i);
            } else {
                let c = counts.entry(d.krate.clone()).or_default();
                c.baselined = c.baselined.saturating_add(1);
                if list {
                    print_one(d, "open, baselined");
                }
            }
        }
        open
    } else {
        unproved
    };
    for d in &open {
        let c = counts.entry(d.krate.clone()).or_default();
        c.open = c.open.saturating_add(1);
    }
    if list {
        for d in &proved {
            let by = d.subsumed_by.as_deref().unwrap_or("");
            print_one(d, &format!("proved by {by}"));
        }
        for d in &open {
            print_one(d, "open");
        }
    }
    let mut total = Counts::default();
    for (krate, c) in &counts {
        println!(
            "{krate}: {} obligation(s): {} proved, {} open and baselined, {} open",
            c.total(),
            c.proved,
            c.baselined,
            c.open
        );
        total.add(c);
    }
    println!(
        "tauten: {} obligation(s) in kernel crates: {} proved, {} open and baselined, {} open",
        total.total(),
        total.proved,
        total.baselined,
        total.open
    );
    Ok(if total.open == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn print_one(d: &Diagnostic, state: &str) {
    println!(
        "{}:{}:{}: {} ({}): {state}",
        d.file.as_deref().unwrap_or(&d.krate),
        d.line.unwrap_or(0),
        d.column.unwrap_or(0),
        d.id,
        d.message
    );
}
