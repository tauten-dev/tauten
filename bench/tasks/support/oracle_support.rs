// Shared by every oracle: run the built `ledger` binary on an input file.
// Included (not a module) so each oracle is a single self-contained test file.

fn run_ledger(args: &[&str], input: &str) -> (i32, String, String) {
    use std::io::Write;
    let dir = std::env::temp_dir().join(format!("ledger-oracle-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let mut hash: u64 = 1469598103934665603;
    for b in input.bytes() {
        hash = (hash ^ u64::from(b)).wrapping_mul(1099511628211);
    }
    let path = dir.join(format!("{hash:016x}.txt"));
    std::fs::File::create(&path).expect("create").write_all(input.as_bytes()).expect("write");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_ledger"))
        .args(args)
        .arg(&path)
        .output()
        .expect("run ledger");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(!stderr.contains("panicked"), "ORACLE-PANIC: the binary panicked on {input:?}: {stderr}");
    (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stdout).into_owned(), stderr)
}

#[allow(dead_code)]
fn expect_ok_with(args: &[&str], input: &str, stdout: &str) {
    let (code, out, err) = run_ledger(args, input);
    assert_eq!((code, out.as_str()), (0, stdout), "input {input:?}, stderr {err:?}");
}

#[allow(dead_code)]
fn expect_ok(input: &str, stdout: &str) {
    expect_ok_with(&[], input, stdout);
}

#[allow(dead_code)]
fn expect_err_with(args: &[&str], input: &str, stderr: &str) {
    let (code, out, err) = run_ledger(args, input);
    assert_eq!((code, err.as_str()), (1, stderr), "input {input:?}, stdout {out:?}");
}

#[allow(dead_code)]
fn expect_err(input: &str, stderr: &str) {
    expect_err_with(&[], input, stderr);
}
