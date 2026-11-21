//! `cargo tauten new <path>`: a fresh workspace laid out as the agent
//! profile describes. A kernel crate with one total function and its proved
//! claim (through the proofs template of `init --proofs`), a core crate
//! that takes a capability and declares `#[effects]`, one `Evidence`
//! check, and a thin edge binary that calls `run(caps, args)`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use tauten_check::profile::agent_profile;

use crate::init::{AGENTS_FILE, camel, fill_proofs, proofs_files};

/// The workspace template (`crates/cargo-tauten/templates/new`): written
/// path, contents. `{{...}}` placeholders are filled by [`fill`].
const TEMPLATE: &[(&str, &str)] = &[
    ("Cargo.toml", include_str!("../templates/new/Cargo.toml.in")),
    ("AGENTS.md", include_str!("../templates/new/AGENTS.md")),
    (".gitignore", include_str!("../templates/new/gitignore")),
    (
        "rust-toolchain.toml",
        include_str!("../templates/new/rust-toolchain.toml"),
    ),
    (
        "crates/{{KERNEL}}/Cargo.toml",
        include_str!("../templates/new/kernel/Cargo.toml.in"),
    ),
    (
        "crates/{{KERNEL}}/src/lib.rs",
        include_str!("../templates/new/kernel/src/lib.rs"),
    ),
    (
        "crates/{{CORE}}/Cargo.toml",
        include_str!("../templates/new/core/Cargo.toml.in"),
    ),
    (
        "crates/{{CORE}}/src/lib.rs",
        include_str!("../templates/new/core/src/lib.rs"),
    ),
    (
        "crates/{{NAME}}/Cargo.toml",
        include_str!("../templates/new/edge/Cargo.toml.in"),
    ),
    (
        "crates/{{NAME}}/src/main.rs",
        include_str!("../templates/new/edge/src/main.rs"),
    ),
];

/// Replaces the proofs template's empty theorem file.
const THEOREMS: &str = include_str!("../templates/new/proofs/Module.lean");
/// Appended to the proofs template's `claims.toml`.
const CLAIM: &str = include_str!("../templates/new/proofs/claim.toml");

/// A package name Cargo accepts and that cannot shadow a standard crate.
fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        && !["core", "std", "alloc", "test", "proc_macro", "tauten"].contains(&name)
}

/// The files of a workspace named `name`, placeholders filled. `tauten`
/// is the dependency specification for the `tauten` crate.
fn files(name: &str, tauten: &str) -> Vec<(String, String)> {
    let kernel = format!("{name}-kernel");
    let core = format!("{name}-core");
    let crate_dir = format!("crates/{kernel}");
    let fill = |text: &str| {
        let text = text
            .replace("{{KERNEL_SNAKE}}", &kernel.replace('-', "_"))
            .replace("{{CORE_SNAKE}}", &core.replace('-', "_"))
            .replace("{{KERNEL}}", &kernel)
            .replace("{{CORE}}", &core)
            .replace("{{NAME}}", name)
            .replace("{{TAUTEN}}", tauten);
        fill_proofs(&text, &kernel, &crate_dir)
    };
    let mut out: Vec<(String, String)> = TEMPLATE
        .iter()
        .map(|(path, text)| (fill(path), fill(text)))
        .collect();
    let module = format!("proofs/Proofs/{}.lean", camel(&kernel));
    for (path, mut text) in proofs_files(&kernel, &crate_dir) {
        if path == "proofs/claims.toml" {
            text.push_str(&fill_proofs(CLAIM, &kernel, &crate_dir));
        } else if path == module {
            text = fill_proofs(THEOREMS, &kernel, &crate_dir);
        }
        out.push((path, text));
    }
    out.push((AGENTS_FILE.to_owned(), agent_profile()));
    out
}

/// `cargo tauten new <path>`: write the workspace. Refuses a path that
/// exists and is not an empty directory.
pub fn run(path: &Path, tauten_path: Option<&Path>) -> Result<()> {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        bail!("`{}` does not end in a directory name", path.display());
    };
    if !valid_name(name) {
        bail!(
            "`{name}` is not a usable package name: start with a lowercase letter, then \
             lowercase letters, digits, `-` or `_` (and not `core`, `std`, `alloc` or `tauten`)"
        );
    }
    if path.exists() {
        let empty = path.is_dir()
            && fs::read_dir(path)
                .with_context(|| format!("reading {}", path.display()))?
                .next()
                .is_none();
        if !empty {
            bail!("{} already exists; nothing written", path.display());
        }
    }
    let tauten = match tauten_path {
        Some(dir) => {
            let dir: PathBuf =
                fs::canonicalize(dir).with_context(|| format!("resolving {}", dir.display()))?;
            format!("{{ path = {:?} }}", dir.to_string_lossy())
        }
        None => format!("\"{}\"", env!("CARGO_PKG_VERSION")),
    };
    for (rel, text) in files(name, &tauten) {
        let full = path.join(&rel);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        fs::write(&full, text).with_context(|| format!("writing {}", full.display()))?;
        println!("{rel}");
    }
    println!("\nNext, in {}:", path.display());
    println!("  cargo test && cargo tauten check");
    println!(
        "  sh proofs/refresh.sh                  # translate the kernel (Docker, proofs image)"
    );
    println!("  git init && git add -A && git commit -m init");
    println!(
        "  cargo tauten proofs                   # prove the claim, record proofs/claims.json"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{files, valid_name};

    #[test]
    fn names() {
        assert!(valid_name("ledger"));
        assert!(valid_name("my-app2"));
        assert!(!valid_name("Ledger"));
        assert!(!valid_name("2app"));
        assert!(!valid_name("core"));
        assert!(!valid_name(""));
    }

    #[test]
    fn every_placeholder_is_filled() {
        for (path, text) in files("ledger", "\"0.0.0\"") {
            assert!(!path.contains("{{"), "{path}");
            assert!(!text.contains("{{"), "{path}: unfilled placeholder");
        }
    }
}
