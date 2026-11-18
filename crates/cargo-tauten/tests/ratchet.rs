//! The ratchet end to end: a baseline recorded on the fixture workspace makes
//! the check pass; the same check without it fails.

#[cfg(test)]
mod ratchet {
    use std::path::Path;
    use std::process::Command;

    fn tauten(args: &[&str], manifest: &Path) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_cargo-tauten"))
            .arg("tauten")
            .args(args)
            .arg("--manifest-path")
            .arg(manifest)
            .output()
            .expect("run cargo-tauten")
    }

    #[test]
    fn baseline_accepts_exactly_the_recorded_findings() {
        let manifest =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/cases/Cargo.toml");
        let file = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fixtures-baseline.json");
        let file = file.to_str().expect("utf-8 path");
        let _ = std::fs::remove_file(file);

        let recorded = tauten(&["baseline", "--output", file], &manifest);
        assert!(
            recorded.status.success(),
            "{}",
            String::from_utf8_lossy(&recorded.stderr)
        );

        let with = tauten(&["check", "--baseline", file], &manifest);
        assert_eq!(
            with.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&with.stdout)
        );
        assert!(String::from_utf8_lossy(&with.stdout).contains("accepted by the baseline"));

        let without = tauten(&["check", "--no-baseline"], &manifest);
        assert_eq!(without.status.code(), Some(1));

        // Re-recording with nothing changed is allowed and idempotent.
        let again = tauten(&["baseline", "--output", file], &manifest);
        assert!(again.status.success());
    }
}
