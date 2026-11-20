//! The generated-file stage (A11) on `fixtures/generated`: unchanged files
//! are `ok`; a one-byte difference is `drift`; an output the command does
//! not produce, or a command that fails, is `failed`, never `ok`.

#[cfg(test)]
mod generated {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output};

    fn scratch(name: &str) -> PathBuf {
        let from = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/generated");
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("generated-{name}"));
        let _ = fs::remove_dir_all(&dir);
        for rel in [
            "Cargo.toml",
            "report.txt",
            "gen.sh",
            "src/lib.rs",
            "out/a.txt",
            "out/b.txt",
        ] {
            let to = dir.join(rel);
            fs::create_dir_all(to.parent().expect("parent")).expect("mkdir");
            fs::copy(from.join(rel), to).expect("copy");
        }
        dir
    }

    fn generated(root: &Path) -> (bool, String) {
        let out: Output = Command::new(env!("CARGO_BIN_EXE_cargo-tauten"))
            .args(["tauten", "generated", "--manifest-path"])
            .arg(root.join("Cargo.toml"))
            .output()
            .expect("run cargo-tauten");
        (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    }

    #[test]
    fn unchanged_files_are_ok() {
        let (ok, out) = generated(&scratch("ok"));
        assert!(ok, "{out}");
        assert_eq!(out, "generated report: ok\ngenerated tree: ok\n");
    }

    #[test]
    fn one_byte_is_drift() {
        let root = scratch("drift");
        fs::write(root.join("out/a.txt"), "onE\n").expect("edit");
        fs::write(root.join("report.txt"), "hellO\n").expect("edit");
        let (ok, out) = generated(&root);
        assert!(!ok);
        assert!(
            out.contains("generated report: drift in report.txt"),
            "{out}"
        );
        assert!(out.contains("generated tree: drift in out/a.txt"), "{out}");
    }

    #[test]
    fn a_missing_output_or_a_failing_command_is_never_ok() {
        let root = scratch("missing");
        fs::write(root.join("gen.sh"), "#!/bin/sh\ntrue\n").expect("edit");
        let (ok, out) = generated(&root);
        assert!(!ok);
        assert!(
            out.contains("generated tree: failed: the command did not produce out"),
            "{out}"
        );
        let root = scratch("failing");
        fs::write(root.join("gen.sh"), "#!/bin/sh\nexit 3\n").expect("edit");
        let (ok, out) = generated(&root);
        assert!(!ok);
        assert!(
            out.contains("generated tree: failed: `sh gen.sh` exited with"),
            "{out}"
        );
    }
}
