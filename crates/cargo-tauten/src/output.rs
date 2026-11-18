//! Human and JSON rendering.

use std::io::{self, Write};

use anyhow::Result;
use tauten_check::{Diagnostic, Profile, Severity, Tier};

pub fn print(diags: &[Diagnostic], json: bool) -> Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if json {
        for d in diags {
            serde_json::to_writer(&mut out, d)?;
            writeln!(out)?;
        }
        return Ok(());
    }
    for d in diags {
        let label = match d.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        let tier = d
            .tier
            .map_or_else(|| "untiered".to_owned(), |t| t.to_string());
        writeln!(out, "{label}[{}]: {}", d.id, d.message)?;
        let location = match (&d.file, d.line, d.column) {
            (Some(f), Some(l), Some(c)) => format!("{f}:{l}:{c}"),
            (Some(f), _, _) => f.clone(),
            _ => String::from("<workspace>"),
        };
        writeln!(out, "  --> {location} ({} crate `{}`)", tier, d.krate)?;
        if let Some(rule) = &d.rule {
            writeln!(out, "  = rule: {rule} (enforced by {:?})", d.enforced_by)?;
        }
        if let Some(fix) = &d.fix {
            writeln!(out, "  = fix: {fix}")?;
        }
        writeln!(out)?;
    }
    let errors = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let warnings = diags.len().saturating_sub(errors);
    writeln!(out, "tauten: {errors} error(s), {warnings} warning(s)")?;
    Ok(())
}

pub fn explain(profile: &Profile, id: Option<&str>) -> Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let Some(id) = id else {
        for rule in profile.rules() {
            writeln!(out, "{}  {}", rule.id, rule.title)?;
        }
        return Ok(());
    };
    let Some(rule) = profile.rule(id) else {
        anyhow::bail!("unknown rule `{id}`; run `cargo tauten explain` to list rules");
    };
    writeln!(out, "{}: {}\n", rule.id, rule.title)?;
    writeln!(out, "Why: {}\n", rule.why)?;
    writeln!(out, "Fix: {}\n", rule.fix)?;
    writeln!(out, "Enforced by: {:?}", rule.enforced_by)?;
    for tier in Tier::ALL {
        let level = rule.level(tier).map_or("-", |l| match l {
            tauten_check::Level::Warn => "warn",
            tauten_check::Level::Deny => "deny",
        });
        writeln!(out, "  {tier:<6} {level}")?;
    }
    if !rule.lints.is_empty() {
        writeln!(out, "Lints: {}", rule.lints.join(", "))?;
    }
    for api in profile.api_bans().iter().filter(|a| a.rule == rule.id) {
        writeln!(out, "Banned ({}): {}", api.note, api.paths.join(", "))?;
    }
    Ok(())
}

pub fn profile(profile: &Profile, tier: Tier) -> Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    writeln!(
        out,
        "# lint flags for {tier}\n{}\n",
        profile.lint_flags(tier).join(" ")
    )?;
    writeln!(
        out,
        "# clippy.toml for {tier}\n{}",
        profile.clippy_toml(tier)?
    )?;
    Ok(())
}
