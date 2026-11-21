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

        // Headroom in an entry is stale: TAU-R001, labelled with the crate's
        // real tier (it used to say "untiered").
        let mut doc: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(file).expect("read")).expect("json");
        let entry = doc["entries"]
            .as_array_mut()
            .expect("entries")
            .iter_mut()
            .find(|e| e["crate"] == "rej-unwrap")
            .expect("rej-unwrap entry");
        entry["count"] = serde_json::json!(entry["count"].as_u64().expect("count") + 1);
        std::fs::write(file, doc.to_string()).expect("write");
        let stale = tauten(&["check", "--baseline", file], &manifest);
        assert_eq!(stale.status.code(), Some(1));
        let text = String::from_utf8_lossy(&stale.stdout);
        assert!(text.contains("error[TAU-R001]"), "{text}");
        assert!(text.contains("core crate `rej-unwrap`"), "{text}");
        assert!(!text.contains("untiered"), "{text}");
    }

    /// A baseline from a newer tauten is refused by the kernel's
    /// `version::admit` (A10, #14), never read as the current format.
    #[test]
    fn a_newer_baseline_is_refused() {
        let manifest =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/cases/Cargo.toml");
        let file = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fixtures-baseline-newer.json");
        let file = file.to_str().expect("utf-8 path");
        let _ = std::fs::remove_file(file);
        let recorded = tauten(&["baseline", "--output", file], &manifest);
        assert!(
            recorded.status.success(),
            "{}",
            String::from_utf8_lossy(&recorded.stderr)
        );
        let mut doc: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(file).expect("read")).expect("json");
        doc["version"] = serde_json::json!(99);
        std::fs::write(file, doc.to_string()).expect("write");
        let out = tauten(&["check", "--baseline", file], &manifest);
        assert_eq!(
            out.status.code(),
            Some(2),
            "{}",
            String::from_utf8_lossy(&out.stdout)
        );
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.contains("format version 99, newer than this tauten reads"),
            "{err}"
        );
    }
}
