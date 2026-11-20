//! `cargo tauten new` writes a workspace that passes `cargo tauten check`
//! with no findings, is rustfmt-clean, and refuses to overwrite.

#[cfg(test)]
mod new {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    fn repo() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn tauten(args: &[&str], dir: &Path) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_cargo-tauten"))
            .arg("tauten")
            .args(args)
            .current_dir(dir)
            .output()
            .expect("run cargo-tauten")
    }

    #[test]
    fn the_new_workspace_passes_check() {
        let parent = Path::new(env!("CARGO_TARGET_TMPDIR")).join("new");
        let dir = parent.join("ledger");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("clean");
        }
        std::fs::create_dir_all(&parent).expect("mkdir");
        let tauten_crate = repo().join("crates/tauten");
        let out = tauten(
            &[
                "new",
                "ledger",
                "--tauten-path",
                tauten_crate.to_str().expect("utf8"),
            ],
            &parent,
        );
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        for file in [
            "AGENTS.md",
            "AGENTS.tauten.md",
            "crates/ledger-kernel/src/lib.rs",
            "crates/ledger-core/src/lib.rs",
            "crates/ledger/src/main.rs",
            "proofs/claims.toml",
            "proofs/Proofs/LedgerKernel.lean",
        ] {
            assert!(dir.join(file).is_file(), "{file} missing");
        }
        let claims = std::fs::read_to_string(dir.join("proofs/claims.toml")).expect("claims");
        assert!(claims.contains("lean = \"ledger_kernel.affordable\""));
        let theorems =
            std::fs::read_to_string(dir.join("proofs/Proofs/LedgerKernel.lean")).expect("lean");
        assert!(theorems.contains("theorem affordable_spec"));
        assert!(!theorems.contains("sorry"));

        // Resolve dependencies as tauten itself does, so the check needs no
        // registry index beyond what tauten's own build fetched.
        std::fs::copy(repo().join("Cargo.lock"), dir.join("Cargo.lock")).expect("lock");

        let fmt = Command::new("cargo")
            .args(["fmt", "--check"])
            .current_dir(&dir)
            .output()
            .expect("cargo fmt");
        assert!(
            fmt.status.success(),
            "generated code is not rustfmt-clean:\n{}",
            String::from_utf8_lossy(&fmt.stdout)
        );

        let check = tauten(&["check", "--format", "json"], &dir);
        let stdout = String::from_utf8_lossy(&check.stdout);
        assert_eq!(
            check.status.code(),
            Some(0),
            "stdout: {stdout}\nstderr: {}",
            String::from_utf8_lossy(&check.stderr)
        );
        assert!(stdout.trim().is_empty(), "findings: {stdout}");

        let again = tauten(&["new", "ledger"], &parent);
        assert!(!again.status.success(), "overwrote an existing workspace");
    }

    /// The generated workspace pins tauten's own toolchain (the `tauten`
    /// crate's `rust-version` needs it).
    #[test]
    fn the_toolchain_is_tautens() {
        let ours = std::fs::read_to_string(repo().join("rust-toolchain.toml")).expect("ours");
        let template = std::fs::read_to_string(
            repo().join("crates/cargo-tauten/templates/new/rust-toolchain.toml"),
        )
        .expect("template");
        assert_eq!(ours, template);
    }
}
