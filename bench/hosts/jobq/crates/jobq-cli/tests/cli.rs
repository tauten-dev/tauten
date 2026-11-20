//! Black-box tests of the `jobq` binary.

#[cfg(test)]
mod cli {
    use std::process::Command;

    fn run(script: &str, extra: &[&str]) -> (i32, String, String) {
        let dir = std::env::temp_dir().join(format!("jobq-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{:x}.jobs", script.len() * 31 + script.bytes().map(usize::from).sum::<usize>()));
        std::fs::write(&path, script).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_jobq")).arg("run").arg(&path).args(extra).output().unwrap();
        (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stdout).into(), String::from_utf8_lossy(&out.stderr).into())
    }

    #[test]
    fn prints_the_table() {
        let (code, out, _) = run("submit a\nstart a\ntick 7\nfinish a\nsubmit b\n", &["--now", "1000"]);
        assert_eq!(code, 0);
        assert_eq!(out, "a done\nb pending\ntime: 1007\npending: 1\n");
    }

    #[test]
    fn syntax_errors_exit_3() {
        let (code, out, err) = run("submit a\n\ntick soon\n", &["--now", "0"]);
        assert_eq!((code, out.as_str(), err.as_str()), (3, "", "error: line 3: bad number `soon`\n"));
    }

    #[test]
    fn state_errors_exit_4() {
        let (code, _, err) = run("submit a\nfinish a\n", &["--now", "0"]);
        assert_eq!((code, err.as_str()), (4, "error: line 2: job `a` is not running\n"));
    }

    #[test]
    fn time_errors_exit_5() {
        let (code, _, err) = run("tick 1\n", &["--now", "18446744073709551615"]);
        assert_eq!((code, err.as_str()), (5, "error: line 1: time overflow\n"));
    }

    #[test]
    fn bad_command_lines_are_usage_errors() {
        let (code, _, err) = run("", &["--now", "-1"]);
        assert_eq!((code, err.as_str()), (2, "usage: jobq run <script> [--now <ms>]\n"));
        let out = Command::new(env!("CARGO_BIN_EXE_jobq")).arg("walk").output().unwrap();
        assert_eq!(out.status.code(), Some(2));
    }

    #[test]
    fn comments_blank_lines_and_crlf() {
        let (code, out, _) = run("# setup
submit a priority 3

next
", &["--now", "5"]);
        assert_eq!((code, out.as_str()), (0, "a running\ntime: 5\npending: 0\n"));
    }

    #[test]
    fn same_script_same_now_same_output() {
        let script = "submit a retry 10\nstart a\nfail a\n";
        let first = run(script, &["--now", "42"]);
        assert_eq!(first, run(script, &["--now", "42"]));
        assert_eq!(first.1, "a pending (ready at 52)\ntime: 42\npending: 1\n");
    }

    #[test]
    fn unwritable_output_is_an_error() {
        let dir = std::env::temp_dir().join(format!("jobq-full-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("s.jobs");
        std::fs::write(&path, "submit a\n").unwrap();
        let full = std::fs::OpenOptions::new().write(true).open("/dev/full").unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_jobq")).args(["run", "--now", "0"]).arg(&path).stdout(full).output().unwrap();
        assert_eq!(out.status.code(), Some(1));
        assert_eq!(String::from_utf8_lossy(&out.stderr), "error: cannot write output\n");
    }
}
