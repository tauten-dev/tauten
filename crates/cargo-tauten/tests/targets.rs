//! Target identity comes from Cargo (#55): a library declared at a
//! non-default path or file name gets the same structural findings and the
//! same `init` tier proposal as one at `src/lib.rs`.

#[cfg(test)]
mod targets {
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

    /// A one-crate workspace: `target` is the manifest's target table (or
    /// empty), `files` are (path, contents) under the crate.
    fn adopter(name: &str, target: &str, files: &[(&str, &str)]) -> PathBuf {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("targets-{name}"));
        let _ = fs::remove_dir_all(&dir);
        for (path, text) in files {
            let full = dir.join(path);
            fs::create_dir_all(full.parent().expect("parent")).expect("mkdir");
            fs::write(full, text).expect("write");
        }
        fs::write(
            dir.join("Cargo.toml"),
            format!(
                "[workspace]\n\n[package]\nname = \"adopter\"\nversion = \"0.1.0\"\n\
                 edition = \"2024\"\n{target}"
            ),
        )
        .expect("write");
        dir
    }

    const LIB: &str = "pub fn f() {}\n";

    fn k003_count(root: &Path) -> usize {
        let out = tauten(
            &[
                "check",
                "--structure-only",
                "--assume-tier",
                "adopter=kernel",
                "--format",
                "json",
            ],
            root,
        );
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .filter(|d| d["id"] == "TAU-K003")
            .count()
    }

    fn proposal(root: &Path) -> String {
        let out = tauten(&["init"], root);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        stdout
            .lines()
            .find(|l| l.starts_with("adopter"))
            .expect("a proposal for the crate")
            .to_owned()
    }

    #[test]
    fn declared_library_roots_match_the_conventional_one() {
        let cases = [
            adopter("conventional", "", &[("src/lib.rs", LIB)]),
            adopter(
                "custom-dir",
                "\n[lib]\npath = \"code/lib.rs\"\n",
                &[("code/lib.rs", LIB)],
            ),
            adopter(
                "custom-name",
                "\n[lib]\npath = \"code/kernel.rs\"\n",
                &[("code/kernel.rs", LIB)],
            ),
            adopter(
                "package-dir",
                "\n[lib]\npath = \"root.rs\"\n",
                &[("root.rs", LIB)],
            ),
        ];
        for root in &cases {
            assert_eq!(k003_count(root), 1, "{}", root.display());
            let line = proposal(root);
            assert!(
                line.contains("tier = \"core\"") && line.contains("(library;"),
                "{}: {line}",
                root.display()
            );
        }
    }

    #[test]
    fn no_std_at_a_declared_root_is_seen() {
        let root = adopter(
            "custom-no-std",
            "\n[lib]\npath = \"code/kernel.rs\"\n",
            &[("code/kernel.rs", "#![no_std]\npub fn f() {}\n")],
        );
        assert_eq!(k003_count(&root), 0);
        assert!(proposal(&root).contains("tier = \"kernel\""));
    }

    #[test]
    fn binary_only_targets_stay_edge() {
        let root = adopter(
            "custom-bin",
            "\n[[bin]]\nname = \"tool\"\npath = \"cmd/tool.rs\"\n",
            &[("cmd/tool.rs", "fn main() {}\n")],
        );
        assert_eq!(k003_count(&root), 0);
        assert!(proposal(&root).contains("tier = \"edge\""));
    }
}
