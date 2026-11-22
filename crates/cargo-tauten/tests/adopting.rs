//! The adoption routes of `docs/ADOPTING.md`, run on `fixtures/adopt` (a
//! workspace with no tiers, no baseline and no overflow-checks setting).
//! The advisory trial reports its known findings and writes nothing; the
//! opt-in check accepts its reviewed baseline and fails on one new finding.

#[cfg(test)]
mod adopting {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output};

    /// The trial flags exactly as `docs/ADOPTING.md` shows them.
    const ADVISORY: [&str; 9] = [
        "check",
        "--structure-only",
        "--no-baseline",
        "--assume-tier",
        "*=core",
        "--assume-tier",
        "app=edge",
        "--format",
        "json",
    ];

    fn repo() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn tauten(args: &[&str], root: &Path) -> Output {
        Command::new(env!("CARGO_BIN_EXE_cargo-tauten"))
            .arg("tauten")
            .args(args)
            .current_dir(root)
            .output()
            .expect("run cargo-tauten")
    }

    /// A fresh copy of `fixtures/adopt`, so a run can be compared with it.
    fn copy(name: &str) -> PathBuf {
        let dest = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("adopt-{name}"));
        let _ = fs::remove_dir_all(&dest);
        for (rel, bytes) in tree(&repo().join("fixtures/adopt")) {
            let path = dest.join(rel);
            fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            fs::write(path, bytes).expect("write");
        }
        dest
    }

    /// Every file under `root` (skipping `target/`), by relative path.
    fn tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        let mut out = BTreeMap::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(&dir).expect("read dir") {
                let path = entry.expect("entry").path();
                if path.is_dir() {
                    if path.file_name().is_some_and(|n| n != "target") {
                        stack.push(path);
                    }
                } else {
                    let rel = path.strip_prefix(root).expect("prefix").to_path_buf();
                    out.insert(rel, fs::read(&path).expect("read"));
                }
            }
        }
        out
    }

    fn error_ids(output: &Output) -> Vec<String> {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .filter(|d| d["severity"] == "error")
            .filter_map(|d| d["id"].as_str().map(str::to_owned))
            .collect()
    }

    /// The guide names the release being built and its toolchain, and shows
    /// the trial flags this test runs.
    #[test]
    fn the_guide_matches_this_release() {
        let guide = fs::read_to_string(repo().join("docs/ADOPTING.md")).expect("read guide");
        let install = format!(
            "install --locked --version {} cargo-tauten",
            env!("CARGO_PKG_VERSION")
        );
        assert!(guide.contains(&install), "guide lacks `{install}`");
        let toolchain = fs::read_to_string(repo().join("rust-toolchain.toml")).expect("read");
        let channel = toolchain
            .lines()
            .find_map(|l| l.strip_prefix("channel = "))
            .expect("channel")
            .trim_matches('"');
        assert!(guide.contains(&format!("cargo +{channel} tauten check --structure-only")));
        assert!(guide.contains(
            "--no-baseline \\\n    --assume-tier '*=core' --assume-tier app=edge --format json"
        ));
    }

    #[test]
    fn advisory_trial_reports_findings_and_writes_nothing() {
        let root = copy("advisory");
        let before = tree(&root);
        let output = tauten(&ADVISORY, &root);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        // Structural findings only: the `unwrap` in `parse` is a Clippy
        // finding, and Clippy did not run.
        assert_eq!(error_ids(&output), ["TAU-H001", "TAU-X002"]);
        assert_eq!(tree(&root), before, "the trial changed the workspace");

        let preview = tauten(&["init"], &root);
        assert!(preview.status.success());
        assert!(String::from_utf8_lossy(&preview.stdout).contains("Nothing written."));
        assert_eq!(
            tree(&root),
            before,
            "init without --write changed the workspace"
        );
    }

    #[test]
    fn opt_in_check_accepts_its_baseline_and_fails_on_a_new_finding() {
        let root = copy("opt-in");
        let lib = root.join("crates/parse/src/lib.rs");
        let one = "//! Parsing.\n\n/// Parse a port.\n#[must_use]\npub fn port(text: &str) -> u16 {\n    text.trim().parse().unwrap()\n}\n";
        fs::write(&lib, one).expect("write");
        let manifest = fs::read(root.join("Cargo.toml")).expect("read");
        assert!(tauten(&["init", "--write"], &root).status.success());
        // The reviewer keeps overflow behaviour as it was: TAU-H001 goes into
        // the baseline instead.
        fs::write(root.join("Cargo.toml"), manifest).expect("write");
        let recorded = tauten(&["baseline"], &root);
        assert!(
            recorded.status.success(),
            "{}",
            String::from_utf8_lossy(&recorded.stderr)
        );
        let baseline = fs::read(root.join("tauten-baseline.json")).expect("baseline");

        let accepted = tauten(&["check", "--format", "json"], &root);
        assert_eq!(
            accepted.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&accepted.stdout)
        );

        let two = format!(
            "{one}\n/// Parse a count.\n#[must_use]\npub fn count(text: &str) -> u32 {{\n    text.trim().parse().unwrap()\n}}\n"
        );
        fs::write(&lib, two).expect("write");
        let refused = tauten(&["check", "--format", "json"], &root);
        assert_eq!(refused.status.code(), Some(1));
        assert_eq!(error_ids(&refused), ["TAU-P001"]);
        assert_eq!(
            fs::read(root.join("tauten-baseline.json")).expect("baseline"),
            baseline,
            "check rewrote the baseline"
        );
    }
}
