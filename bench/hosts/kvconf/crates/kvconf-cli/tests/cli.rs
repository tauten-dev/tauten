//! Black-box tests of the `kvconf` binary.

#[cfg(test)]
mod cli {
    use std::path::PathBuf;
    use std::process::Command;

    /// Write `files` into a fresh directory and run kvconf there with `args`.
    fn run(files: &[(&str, &str)], args: &[&str]) -> (i32, String, String) {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("kvconf-test-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for (name, text) in files {
            let path: PathBuf = dir.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, text).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_kvconf")).args(args).current_dir(&dir).env_clear().output().unwrap();
        (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stdout).into(), String::from_utf8_lossy(&out.stderr).into())
    }

    #[test]
    fn get_prints_the_canonical_value() {
        let files = [("conf/app.conf", "[db]\nhosts = [\"a\", \"b\"]\nport = 5432\n")];
        assert_eq!(run(&files, &["get", "conf/app.conf", "db.port"]), (0, "5432\n".into(), String::new()));
        assert_eq!(run(&files, &["get", "conf/app.conf", "db.hosts"]).1, "[\"a\", \"b\"]\n");
        assert_eq!(run(&files, &["get", "--str", "conf/app.conf", "db.port"]).2, "error: key `db.port` is not of type str\n");
    }

    #[test]
    fn dump_and_check() {
        let files = [("a.conf", "# c\nz = true\na = \"x\\ty\"\n")];
        assert_eq!(run(&files, &["dump", "a.conf"]), (0, "a = \"x\\ty\"\nz = true\n".into(), String::new()));
        assert_eq!(run(&files, &["check", "a.conf"]).1, "ok: 2 keys\n");
    }

    #[test]
    fn line_errors_and_usage() {
        let files = [("a.conf", "a = 1\nb = nope\n")];
        assert_eq!(run(&files, &["check", "a.conf"]), (1, String::new(), "error: a.conf: line 2: bad value `nope`\n".into()));
        assert_eq!(run(&files, &["frobnicate"]).0, 2);
        assert_eq!(run(&files, &["get", "a.conf"]).0, 2);
    }

    #[test]
    fn env_file_overrides_values() {
        let files = [("a.conf", "[s]\nport = 1\n"), ("vars", "KVCONF_S__PORT=2\n")];
        assert_eq!(run(&files, &["--env-file", "vars", "get", "a.conf", "s.port"]).1, "2\n");
    }

    #[test]
    fn set_prints_the_edited_file() {
        let files = [("a.conf", "# keep me\na = 1\n")];
        assert_eq!(run(&files, &["set", "a.conf", "a", "[1, 2]"]).1, "# keep me\na = [1, 2]\n");
        assert_eq!(run(&files, &["set", "a.conf", "B", "1"]).2, "error: bad key `B`\n");
    }

    #[test]
    fn env_file_errors() {
        let files = [("a.conf", "a = 1\n"), ("vars", "# c\nNOPE\n")];
        assert_eq!(run(&files, &["--env-file", "vars", "check", "a.conf"]).2, "error: env file line 2: expected NAME=VALUE\n");
        assert_eq!(run(&files, &["--env-file", "missing", "check", "a.conf"]).0, 1);
    }

    #[test]
    fn crlf_lines_and_comments() {
        let files = [("a.conf", "# c\r\n\r\n[s]\r\nx = \"y\"\r\n")];
        assert_eq!(run(&files, &["dump", "a.conf"]).1, "s.x = \"y\"\n");
    }
}
