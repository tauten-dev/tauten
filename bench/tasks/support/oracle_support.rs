// Shared by every oracle: run the host's built binary on a prepared case.
// Included (not a module) so each oracle is a single self-contained test file.
// run.py prepends `const ORACLE_BIN: &str = env!("CARGO_BIN_EXE_<binary>");`
// for the host's binary (bench.toml `binary`).
//
// Every case runs in a fresh temporary directory (the working directory unless
// `.cwd()` says otherwise), with an empty environment unless `.env()` adds to
// it, and a time limit (10 s by default). A crash of the binary (a panic, an
// abort such as a stack overflow, or death by a signal) fails the test with
// `ORACLE-PANIC`; running past the limit fails it with `ORACLE-HANG`. An
// expected error (`fails`, `expect_err_with`) also requires empty standard
// output.

#[allow(dead_code)]
struct Case {
    args: Vec<String>,
    files: Vec<(String, Vec<u8>)>,
    dirs: Vec<String>,
    env: Vec<(String, String)>,
    cwd: Option<String>,
    stdin: Vec<u8>,
    stdout_full: bool,
    limit: std::time::Duration,
}

#[allow(dead_code)]
struct Outcome {
    code: i32,
    stdout: String,
    stderr: String,
    dir: std::path::PathBuf,
    what: String,
}

#[allow(dead_code)]
fn case(args: &[&str]) -> Case {
    Case {
        args: args.iter().map(|a| (*a).to_owned()).collect(),
        files: Vec::new(),
        dirs: Vec::new(),
        env: Vec::new(),
        cwd: None,
        stdin: Vec::new(),
        stdout_full: false,
        limit: std::time::Duration::from_secs(10),
    }
}

#[allow(dead_code)]
impl Case {
    /// Create `rel` (and its parent directories) in the case directory.
    fn file(mut self, rel: &str, contents: impl AsRef<[u8]>) -> Self {
        self.files.push((rel.to_owned(), contents.as_ref().to_vec()));
        self
    }
    /// Create an empty directory `rel` in the case directory.
    fn dir(mut self, rel: &str) -> Self {
        self.dirs.push(rel.to_owned());
        self
    }
    /// Set a variable in the binary's (otherwise empty) environment.
    fn env(mut self, key: &str, value: &str) -> Self {
        self.env.push((key.to_owned(), value.to_owned()));
        self
    }
    /// Run the binary from this subdirectory of the case directory.
    fn cwd(mut self, rel: &str) -> Self {
        self.cwd = Some(rel.to_owned());
        self
    }
    fn stdin(mut self, input: impl AsRef<[u8]>) -> Self {
        self.stdin = input.as_ref().to_vec();
        self
    }
    /// Standard output is `/dev/full`: every write to it fails.
    fn stdout_to_dev_full(mut self) -> Self {
        self.stdout_full = true;
        self
    }
    fn limit_secs(mut self, secs: u64) -> Self {
        self.limit = std::time::Duration::from_secs(secs);
        self
    }

    fn run(self) -> Outcome {
        use std::io::{Read, Write};
        use std::process::{Command, Stdio};
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let n = NEXT.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("oracle-{}-{n}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("clean case dir");
        }
        std::fs::create_dir_all(&dir).expect("case dir");
        for rel in &self.dirs {
            std::fs::create_dir_all(dir.join(rel)).expect("case subdir");
        }
        for (rel, contents) in &self.files {
            let path = dir.join(rel);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("case file parent");
            }
            std::fs::write(&path, contents).expect("case file");
        }
        let shown: Vec<(&str, String)> =
            self.files.iter().map(|(p, c)| (p.as_str(), String::from_utf8_lossy(c).into_owned())).collect();
        let what = format!("args {:?}, files {:?}, env {:?}, cwd {:?}", self.args, shown, self.env, self.cwd);
        let mut cmd = Command::new(ORACLE_BIN);
        cmd.args(&self.args)
            .current_dir(match &self.cwd {
                Some(rel) => dir.join(rel),
                None => dir.clone(),
            })
            .env_clear()
            .envs(self.env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .stdin(Stdio::piped())
            .stderr(Stdio::piped());
        if self.stdout_full {
            cmd.stdout(std::fs::OpenOptions::new().write(true).open("/dev/full").expect("/dev/full"));
        } else {
            cmd.stdout(Stdio::piped());
        }
        let mut child = cmd.spawn().expect("spawn the binary");
        let mut stdin = child.stdin.take().expect("stdin");
        let input = self.stdin.clone();
        let writer = std::thread::spawn(move || {
            // The binary may exit without reading its input; that is not a failure.
            let _ = stdin.write_all(&input);
        });
        let mut out_pipe = child.stdout.take();
        let mut err_pipe = child.stderr.take().expect("stderr");
        let reader = std::thread::spawn(move || {
            let mut out = Vec::new();
            if let Some(pipe) = out_pipe.as_mut() {
                let _ = pipe.read_to_end(&mut out);
            }
            out
        });
        let err_reader = std::thread::spawn(move || {
            let mut err = Vec::new();
            let _ = err_pipe.read_to_end(&mut err);
            err
        });
        let started = std::time::Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().expect("wait") {
                break status;
            }
            if started.elapsed() > self.limit {
                let _ = child.kill();
                let _ = child.wait();
                panic!("ORACLE-HANG: the binary ran longer than {:?} on {what}", self.limit);
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        };
        let _ = writer.join();
        let stdout = String::from_utf8_lossy(&reader.join().expect("stdout reader")).into_owned();
        let stderr = String::from_utf8_lossy(&err_reader.join().expect("stderr reader")).into_owned();
        // A panic, an abort (for example a stack overflow) or any other death by
        // signal is a crash (SPEC: the program never crashes).
        let crashed = stderr.contains("panicked")
            || stderr.contains("has overflowed its stack")
            || stderr.contains("fatal runtime error")
            || status.code().is_none();
        assert!(!crashed, "ORACLE-PANIC: the binary crashed ({status}) on {what}: {stderr}");
        Outcome { code: status.code().unwrap_or(-1), stdout, stderr, dir, what }
    }
}

#[allow(dead_code)]
impl Outcome {
    /// Exit status 0 and exactly this standard output.
    fn ok(&self, stdout: &str) -> &Self {
        assert_eq!((self.code, self.stdout.as_str()), (0, stdout), "{}; stderr {:?}", self.what, self.stderr);
        self
    }
    /// This exit status, exactly this standard error, and nothing on standard output.
    fn fails(&self, code: i32, stderr: &str) -> &Self {
        assert_eq!(
            (self.code, self.stderr.as_str(), self.stdout.as_str()),
            (code, stderr, ""),
            "{}; (status, stderr, stdout)",
            self.what
        );
        self
    }
    /// This exit status, whatever the output.
    fn status(&self, code: i32) -> &Self {
        assert_eq!(self.code, code, "{}; stdout {:?}, stderr {:?}", self.what, self.stdout, self.stderr);
        self
    }
    /// Contents of a file in the case directory after the run.
    fn read(&self, rel: &str) -> Vec<u8> {
        std::fs::read(self.dir.join(rel)).unwrap_or_else(|e| panic!("reading {rel} after {}: {e}", self.what))
    }
}

// ---- Single-input-file helpers (the ledger shape: `<bin> [args] <file>`). ----

#[allow(dead_code)]
fn run_on_input(args: &[&str], input: &str) -> (i32, String, String) {
    let mut all: Vec<&str> = args.to_vec();
    all.push("input.txt");
    let out = case(&all).file("input.txt", input).run();
    (out.code, out.stdout, out.stderr)
}

#[allow(dead_code)]
fn run_ledger(args: &[&str], input: &str) -> (i32, String, String) {
    run_on_input(args, input)
}

#[allow(dead_code)]
fn expect_ok_with(args: &[&str], input: &str, stdout: &str) {
    let (code, out, err) = run_on_input(args, input);
    assert_eq!((code, out.as_str()), (0, stdout), "input {input:?}, stderr {err:?}");
}

#[allow(dead_code)]
fn expect_ok(input: &str, stdout: &str) {
    expect_ok_with(&[], input, stdout);
}

#[allow(dead_code)]
fn expect_err_with(args: &[&str], input: &str, stderr: &str) {
    let (code, out, err) = run_on_input(args, input);
    assert_eq!((code, err.as_str(), out.as_str()), (1, stderr, ""), "input {input:?}; (status, stderr, stdout)");
}

#[allow(dead_code)]
fn expect_err(input: &str, stderr: &str) {
    expect_err_with(&[], input, stderr);
}
