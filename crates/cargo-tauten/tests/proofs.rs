//! The proof tier end to end (proof plan P5), on `fixtures/proofs-subsume`:
//! a kernel crate whose `proofs/claims.json` was recorded by a real gate
//! run. Fresh proofs subsume the arithmetic/indexing findings of their
//! functions; a stale record, a non-total theorem or an unclaimed function
//! subsumes nothing; `cargo tauten proofs` without the toolchain reports
//! `tool-missing` and never `proved`; `init --proofs` writes the template.

#[cfg(test)]
mod proofs {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output};

    fn fixture() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/proofs-subsume")
    }

    fn tauten(args: &[&str], root: &Path) -> Output {
        Command::new(env!("CARGO_BIN_EXE_cargo-tauten"))
            .arg("tauten")
            .args(args)
            .arg("--manifest-path")
            .arg(root.join("Cargo.toml"))
            .output()
            .expect("run cargo-tauten")
    }

    fn copy(from: &Path, to: &Path) {
        fs::create_dir_all(to).expect("mkdir");
        for entry in fs::read_dir(from).expect("read dir") {
            let entry = entry.expect("entry");
            let name = entry.file_name();
            if name == "target" || name == ".lake" {
                continue;
            }
            if entry.file_type().expect("type").is_dir() {
                copy(&entry.path(), &to.join(&name));
            } else {
                fs::copy(entry.path(), to.join(&name)).expect("copy");
            }
        }
    }

    /// A fresh copy of the fixture in the test's scratch directory.
    fn scratch(name: &str) -> PathBuf {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("proofs-{name}"));
        let _ = fs::remove_dir_all(&dir);
        copy(&fixture(), &dir);
        dir
    }

    /// (line, rule, severity, subsumed_by) of every TAU finding.
    fn findings(output: &Output) -> Vec<(u64, String, String, Option<String>)> {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("json"))
            .filter(|d| d["id"].as_str().is_some_and(|id| id.starts_with("TAU-")))
            .filter(|d| d["id"] != "TAU-D001")
            .map(|d| {
                (
                    d["line"].as_u64().unwrap_or(0),
                    d["id"].as_str().unwrap_or("").to_owned(),
                    d["severity"].as_str().unwrap_or("").to_owned(),
                    d["subsumed_by"].as_str().map(str::to_owned),
                )
            })
            .collect()
    }

    fn row(
        line: u64,
        id: &str,
        severity: &str,
        by: Option<&str>,
    ) -> (u64, String, String, Option<String>) {
        (
            line,
            id.to_owned(),
            severity.to_owned(),
            by.map(str::to_owned),
        )
    }

    const ADD: &str = "Proofs.SubKernel.add_capped_spec";
    const FIRST: &str = "Proofs.SubKernel.first_or_zero_spec";

    #[test]
    fn fresh_proofs_subsume_the_findings_of_their_functions_only() {
        let output = tauten(&["check", "--format", "json"], &fixture());
        assert_eq!(
            output.status.code(),
            Some(1),
            "double and triple still fail"
        );
        assert_eq!(
            findings(&output),
            [
                row(8, "TAU-N001", "warning", Some(ADD)),
                row(8, "TAU-N001", "warning", Some(ADD)),
                row(14, "TAU-P003", "warning", Some(FIRST)),
                // `double`: its theorem has a hypothesis (audit-failed)
                row(21, "TAU-N001", "error", None),
                // `triple`: no claim
                row(27, "TAU-N001", "error", None),
            ]
        );
        let human = tauten(&["check", "--warnings"], &fixture());
        let text = String::from_utf8_lossy(&human.stdout);
        assert!(
            text.contains(&format!("= subsumed by proof {ADD}")),
            "{text}"
        );
        assert!(text.contains("3 subsumed by proofs"), "{text}");
    }

    fn assert_nothing_subsumed(root: &Path) {
        let output = tauten(&["check", "--format", "json"], root);
        let all = findings(&output);
        assert_eq!(all.len(), 5, "{all:?}");
        assert!(
            all.iter()
                .all(|(_, _, severity, by)| severity == "error" && by.is_none()),
            "{all:?}"
        );
    }

    #[test]
    fn a_stale_proof_never_silences_a_lint() {
        // The Rust source changed after the gate ran.
        let edited = scratch("stale-source");
        let lib = edited.join("sub-kernel/src/lib.rs");
        let text = fs::read_to_string(&lib).expect("read");
        fs::write(&lib, text.replace("u32::MAX } else", "u32::MAX - 0 } else")).expect("write");
        assert_nothing_subsumed(&edited);

        // A theorem changed after the gate ran.
        let edited = scratch("stale-proof");
        let theorems = edited.join("proofs/Proofs/SubKernel.lean");
        let text = fs::read_to_string(&theorems).expect("read");
        fs::write(&theorems, format!("{text}\n-- edited\n")).expect("write");
        assert_nothing_subsumed(&edited);

        // The workspace manifest, which extraction reads, changed.
        let edited = scratch("stale-manifest");
        let manifest = edited.join("Cargo.toml");
        let text = fs::read_to_string(&manifest).expect("read");
        fs::write(&manifest, format!("{text}\n# edited\n")).expect("write");
        assert_nothing_subsumed(&edited);
    }

    #[test]
    fn a_version_bump_keeps_proofs_fresh() {
        let bumped = scratch("version-bump");
        let manifest = bumped.join("Cargo.toml");
        let text = fs::read_to_string(&manifest).expect("read");
        fs::write(
            &manifest,
            text.replace("version = \"0.0.0\"", "version = \"0.0.1\""),
        )
        .expect("write");
        let output = tauten(&["check"], &bumped);
        let text = String::from_utf8_lossy(&output.stdout);
        assert!(text.contains("3 subsumed by proofs"), "{text}");
    }

    fn outcomes(output: &Output) -> Vec<String> {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("json"))
            .map(|r| r["outcome"].as_str().unwrap_or("").to_owned())
            .collect()
    }

    #[test]
    fn without_the_toolchain_every_claim_is_tool_missing() {
        for (name, command) in [
            ("absent", r#"["tauten-no-such-gate-command"]"#),
            ("silent", r#"["sh", "-c", "exit 0"]"#),
            (
                "missing",
                r#"["sh", "-c", "echo 'TAUTEN-PROOFS {\"stage\":\"tools\",\"status\":\"missing\",\"detail\":\"no image\"}'; exit 3"]"#,
            ),
        ] {
            let root = scratch(&format!("tools-{name}"));
            let claims = root.join("proofs/claims.toml");
            let text = fs::read_to_string(&claims).expect("read");
            let text = text.replace(
                r#"command = ["sh", "proofs/run.sh"]"#,
                &format!("command = {command}"),
            );
            fs::write(&claims, text).expect("write");
            let output = tauten(&["proofs"], &root);
            assert_eq!(output.status.code(), Some(1), "{name}");
            assert_eq!(outcomes(&output), ["tool-missing"; 4], "{name}");
            let record = fs::read_to_string(root.join("proofs/claims.json")).expect("recorded");
            assert!(!record.contains("\"proved\""), "{name}: {record}");
            // and the check then subsumes nothing
            assert_nothing_subsumed(&root);
        }
    }

    #[test]
    fn init_writes_the_template_once() {
        let root = scratch("init");
        fs::remove_dir_all(root.join("proofs")).expect("remove proofs");
        let plan = tauten(&["init", "--proofs", "sub-kernel"], &root);
        assert!(plan.status.success());
        assert!(!root.join("proofs").exists(), "a plan writes nothing");
        let written = tauten(&["init", "--proofs", "sub-kernel", "--write"], &root);
        assert!(
            written.status.success(),
            "{}",
            String::from_utf8_lossy(&written.stderr)
        );
        for file in [
            "claims.toml",
            "gate.py",
            "run.sh",
            "Audit.lean",
            "Proofs/SubKernel.lean",
            "TRUST.md",
        ] {
            let text = fs::read_to_string(root.join("proofs").join(file)).expect(file);
            assert!(!text.contains("{{"), "{file} has an unfilled placeholder");
        }
        let claims = fs::read_to_string(root.join("proofs/claims.toml")).expect("claims");
        assert!(claims.contains(r#"crates = ["sub-kernel"]"#), "{claims}");
        let again = tauten(&["init", "--proofs", "sub-kernel", "--write"], &root);
        assert!(!again.status.success(), "never over an existing proofs/");
        let unknown = tauten(&["init", "--proofs", "no-such-crate"], &root);
        assert!(!unknown.status.success());
    }
    #[test]
    fn deleting_a_mutant_fails_check_unless_amended() {
        let edited = scratch("mutant-deleted");
        let claims = edited.join("proofs/claims.toml");
        let text = fs::read_to_string(&claims).expect("read");
        let swapped = text.replacen(
            r#"mutants = ["guard-dropped"]"#,
            r#"mutants = ["guard-weakened"]"#,
            1,
        );
        assert_ne!(swapped, text);
        fs::write(&claims, &swapped).expect("write");
        let out = tauten(&["check", "--format", "json"], &edited);
        let r002: Vec<_> = findings(&out)
            .into_iter()
            .filter(|f| f.1 == "TAU-R002")
            .collect();
        assert_eq!(r002.len(), 1, "{}", String::from_utf8_lossy(&out.stdout));
        assert_eq!(r002[0].2, "error");
        assert!(!out.status.success());

        let amend = "\n[[amend]]\nfile = \"sub-kernel/src/lib.rs\"\nfunction = \"add_capped\"\nkind = \"mutant\"\nname = \"guard-dropped\"\nreason = \"replaced by guard-weakened\"\n";
        fs::write(&claims, format!("{swapped}{amend}")).expect("write");
        let out = tauten(&["check", "--format", "json"], &edited);
        assert!(findings(&out).iter().all(|f| f.1 != "TAU-R002"));
    }
    #[test]
    fn init_turns_on_release_overflow_checks() {
        let root = scratch("init-hardening");
        let manifest = root.join("Cargo.toml");
        let text = fs::read_to_string(&manifest).expect("read");
        let start = text.find("\n# TAU-H001").expect("the fixture sets it");
        fs::write(&manifest, &text[..start]).expect("write");
        let before = tauten(&["check", "--format", "json"], &root);
        assert!(findings(&before).iter().any(|f| f.1 == "TAU-H001"));
        let plan = tauten(&["init"], &root);
        assert!(String::from_utf8_lossy(&plan.stdout).contains("overflow-checks = true"));
        assert!(
            !fs::read_to_string(&manifest)
                .expect("read")
                .contains("overflow-checks")
        );
        tauten(&["init", "--write"], &root);
        assert!(
            fs::read_to_string(&manifest)
                .expect("read")
                .contains("overflow-checks = true")
        );
        let after = tauten(&["check", "--format", "json"], &root);
        assert!(findings(&after).iter().all(|f| f.1 != "TAU-H001"));
    }
}
