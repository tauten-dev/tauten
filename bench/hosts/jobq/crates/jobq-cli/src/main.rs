//! The `jobq` command: `jobq run <script> [--now <ms>]`.

use std::io::Write;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use jobq_core::{Clock, Kind};

const USAGE: &str = "usage: jobq run <script> [--now <ms>]";

/// The clock given by `--now`.
struct FixedClock(u64);

impl Clock for FixedClock {
    fn now_ms(&self) -> u64 {
        self.0
    }
}

/// The system clock (milliseconds since the Unix epoch).
struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
    }
}

struct Options {
    script: String,
    now: Option<u64>,
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Option<Options> {
    if args.next()? != "run" {
        return None;
    }
    let mut script = None;
    let mut now = None;
    while let Some(arg) = args.next() {
        if arg == "--now" {
            let value = args.next()?;
            let parsed = jobq_core::parse::parse_number(&value).ok()?;
            if now.replace(parsed).is_some() {
                return None;
            }
        } else if arg.starts_with('-') || script.replace(arg).is_some() {
            return None;
        }
    }
    Some(Options { script: script?, now })
}

const fn status(kind: Kind) -> u8 {
    match kind {
        Kind::Syntax => 3,
        Kind::State => 4,
        Kind::Time => 5,
    }
}

fn main() -> ExitCode {
    let Some(options) = parse_args(std::env::args().skip(1)) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let script = match std::fs::read_to_string(&options.script) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("error: cannot read input: {e}");
            return ExitCode::FAILURE;
        }
    };
    let result = match options.now {
        Some(now) => jobq_core::run(&script, &FixedClock(now)),
        None => jobq_core::run(&script, &SystemClock),
    };
    let sched = match result {
        Ok(sched) => sched,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(status(e.problem.kind()));
        }
    };
    let mut out = std::io::stdout().lock();
    if jobq_core::table::render(&sched, &mut out).and_then(|()| out.flush()).is_err() {
        eprintln!("error: cannot write output");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
