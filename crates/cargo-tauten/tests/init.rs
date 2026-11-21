//! `cargo tauten init --write` on manifests that already carry Tauten
//! metadata (#54): the tier is added to the existing table in whatever
//! form it has, Cargo still reads the manifest, other keys and comments
//! survive, and a second run changes nothing.

#[cfg(test)]
mod init {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output};

    fn tauten(args: &[&str], root: &Path) -> Output {
        Command::new(env!("CARGO_BIN_EXE_cargo-tauten"))
            .arg("tauten")
            .args(args)
            .arg("--manifest-path")
            .arg(root.join("Cargo.toml"))
            .output()
            .expect("run cargo-tauten")
    }

    /// `package.metadata.tauten` as Cargo's own manifest parser sees it.
    fn cargo_metadata(root: &Path) -> serde_json::Value {
        let out = Command::new(env!("CARGO"))
            .args([
                "metadata",
                "--no-deps",
                "--format-version",
                "1",
                "--manifest-path",
            ])
            .arg(root.join("Cargo.toml"))
            .output()
            .expect("run cargo metadata");
        assert!(
            out.status.success(),
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let meta: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
        meta["packages"][0]["metadata"]["tauten"].clone()
    }

    /// A one-crate workspace whose manifest is `[package]` plus `rest`.
    fn crate_with(name: &str, rest: &str) -> PathBuf {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("init-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).expect("mkdir");
        fs::write(dir.join("src/lib.rs"), "pub fn f() {}\n").expect("write");
        fs::write(
            dir.join("Cargo.toml"),
            format!(
                "[workspace]\n\n[package]\nname = \"adopter\"\nversion = \"0.1.0\"\nedition = \"2024\"\n{rest}\n\
                 [profile.release]\nopt-level = 3 # keep\n"
            ),
        )
        .expect("write");
        dir
    }

    fn write_twice(root: &Path) -> String {
        let first = tauten(&["init", "--write"], root);
        assert!(
            first.status.success(),
            "{}",
            String::from_utf8_lossy(&first.stderr)
        );
        let once = fs::read_to_string(root.join("Cargo.toml")).expect("read");
        let second = tauten(&["init", "--write"], root);
        assert!(second.status.success());
        assert_eq!(
            once,
            fs::read_to_string(root.join("Cargo.toml")).expect("read"),
            "a second --write must change nothing"
        );
        once
    }

    #[test]
    fn explicit_table_gains_the_tier_once() {
        let root = crate_with(
            "explicit",
            "\n# layer chosen by the team\n[package.metadata.tauten]\nlayer = \"store\"\n",
        );
        let text = write_twice(&root);
        let meta = cargo_metadata(&root);
        assert_eq!(meta["tier"], "core");
        assert_eq!(meta["layer"], "store");
        assert_eq!(text.matches("[package.metadata.tauten]").count(), 1);
        assert!(text.contains("# layer chosen by the team"));
        assert!(text.contains("opt-level = 3 # keep"));
        assert!(text.contains("overflow-checks = true"));
        assert_eq!(text.matches("[profile.release]").count(), 1);
    }

    #[test]
    fn dotted_keys_gain_the_tier() {
        let root = crate_with("dotted", "metadata.tauten.layer = \"store\"\n");
        write_twice(&root);
        let meta = cargo_metadata(&root);
        assert_eq!(meta["tier"], "core");
        assert_eq!(meta["layer"], "store");
    }

    #[test]
    fn inline_table_gains_the_tier() {
        let root = crate_with(
            "inline",
            "\n[package.metadata]\nother = 1\ntauten = { layer = \"store\" }\n",
        );
        write_twice(&root);
        let meta = cargo_metadata(&root);
        assert_eq!(meta["tier"], "core");
        assert_eq!(meta["layer"], "store");
    }

    #[test]
    fn missing_metadata_gets_a_table() {
        let root = crate_with("missing", "");
        let text = write_twice(&root);
        assert_eq!(cargo_metadata(&root)["tier"], "core");
        assert!(text.contains("[package.metadata.tauten]"));
    }

    #[test]
    fn non_string_tier_is_refused_before_writing() {
        let root = crate_with("non-string", "\n[package.metadata.tauten]\ntier = 3\n");
        let before = fs::read_to_string(root.join("Cargo.toml")).expect("read");
        for args in [&["init"][..], &["init", "--write"][..]] {
            let out = tauten(args, &root);
            assert!(!out.status.success());
            assert!(String::from_utf8_lossy(&out.stderr).contains("fix it by hand"));
        }
        assert_eq!(
            before,
            fs::read_to_string(root.join("Cargo.toml")).expect("read")
        );
        assert!(!root.join("AGENTS.tauten.md").exists());
    }
}
