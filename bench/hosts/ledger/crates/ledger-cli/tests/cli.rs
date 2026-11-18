//! Black-box tests of the `ledger` binary.

#[cfg(test)]
mod cli {
    use std::io::Write;
    use std::process::Command;

    pub fn run(input: &str) -> (i32, String, String) {
        let dir = std::env::temp_dir().join(format!("ledger-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{:x}.txt", input.len() * 31 + input.bytes().map(usize::from).sum::<usize>()));
        std::fs::File::create(&path).unwrap().write_all(input.as_bytes()).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_ledger")).arg(&path).output().unwrap();
        (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stdout).into(), String::from_utf8_lossy(&out.stderr).into())
    }

    #[test]
    fn prints_balance() {
        let (code, out, _) = run("deposit 100\nwithdraw 30\n");
        assert_eq!(code, 0);
        assert_eq!(out, "balance: 70\n");
    }

    #[test]
    fn reports_errors_with_line_numbers() {
        let (code, _, err) = run("deposit 1\nwithdraw 2\n");
        assert_eq!(code, 1);
        assert_eq!(err, "error: line 2: insufficient funds\n");
    }
}
