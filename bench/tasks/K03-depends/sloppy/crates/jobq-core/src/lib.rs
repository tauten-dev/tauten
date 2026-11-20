//! Reading job scripts and simulating the queue. No direct IO: the clock is
//! passed in as a [`Clock`] and output goes to any `Write`.

pub mod parse;
pub mod sched;
pub mod table;

use core::fmt;

use parse::{SyntaxError, parse_line};
use sched::Scheduler;

/// Where the time a run starts at comes from.
pub trait Clock {
    /// Milliseconds since the Unix epoch.
    fn now_ms(&self) -> u64;
}

/// What stops a script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// The line is not a valid event.
    Syntax(SyntaxError),
    /// No job has this name.
    UnknownJob(String),
    /// A job with this name already exists.
    DuplicateJob(String),
    /// The job is not pending, or not ready yet.
    CannotStart(String),
    /// The event needs a running job.
    NotRunning(String),
    /// A time would exceed the largest representable time.
    TimeOverflow,
    /// The dependency would make a job depend on itself.
    DependencyCycle,
}

/// The class of a problem, which decides the exit status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The script text is malformed.
    Syntax,
    /// The event does not fit the jobs' states.
    State,
    /// Time arithmetic left the representable range.
    Time,
}

impl Problem {
    /// The class of this problem.
    #[must_use]
    pub const fn kind(&self) -> Kind {
        match self {
            Self::Syntax(_) => Kind::Syntax,
            Self::UnknownJob(_)
            | Self::DuplicateJob(_)
            | Self::CannotStart(_)
            | Self::NotRunning(_)
            | Self::DependencyCycle => Kind::State,
            Self::TimeOverflow => Kind::Time,
        }
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax(e) => e.fmt(f),
            Self::UnknownJob(n) => write!(f, "unknown job `{n}`"),
            Self::DuplicateJob(n) => write!(f, "duplicate job `{n}`"),
            Self::CannotStart(n) => write!(f, "job `{n}` cannot start"),
            Self::NotRunning(n) => write!(f, "job `{n}` is not running"),
            Self::TimeOverflow => f.write_str("time overflow"),
            Self::DependencyCycle => f.write_str("dependency cycle"),
        }
    }
}

/// A problem on a numbered script line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineError {
    /// 1-based line number.
    pub line: usize,
    /// What went wrong.
    pub problem: Problem,
}

impl fmt::Display for LineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.problem)
    }
}

/// Run a script, starting at the clock's current time.
///
/// # Errors
/// Returns the first line that cannot be applied.
pub fn run(script: &str, clock: &dyn Clock) -> Result<Scheduler, LineError> {
    let mut sched = Scheduler::new(clock.now_ms());
    for (index, text) in script.lines().enumerate() {
        let line = index.saturating_add(1);
        let fail = |problem| LineError { line, problem };
        if let Some(command) = parse_line(text).map_err(|e| fail(Problem::Syntax(e)))? {
            sched.apply(command).map_err(fail)?;
        }
    }
    Ok(sched)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixed(u64);
    impl Clock for Fixed {
        fn now_ms(&self) -> u64 {
            self.0
        }
    }

    fn table(script: &str, now: u64) -> String {
        let sched = run(script, &Fixed(now)).unwrap();
        let mut out = Vec::new();
        table::render(&sched, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn lifecycle_table() {
        let out = table("submit a\nsubmit b\nstart a\ntick 5\nfinish a\n", 100);
        assert_eq!(out, "a done\nb pending\ntime: 105\npending: 1\n");
    }

    #[test]
    fn next_follows_priority_then_submit_time() {
        let out = table("submit low\ntick 1\nsubmit high priority 9\nsubmit high2 priority 9\nnext\nnext\n", 0);
        assert_eq!(out, "low pending\nhigh running\nhigh2 running\ntime: 1\npending: 1\n");
    }

    #[test]
    fn retry_waits_for_its_delay() {
        let out = table("submit a retry 30\nstart a\nfail a\n", 10);
        assert_eq!(out, "a pending (ready at 40)\ntime: 10\npending: 1\n");
        let err = run("submit a retry 30\nstart a\nfail a\ntick 29\nstart a\n", &Fixed(0)).unwrap_err();
        assert_eq!(err.to_string(), "line 5: job `a` cannot start");
    }

    #[test]
    fn errors_have_kinds() {
        let err = run("submit a\nsubmit a\n", &Fixed(0)).unwrap_err();
        assert_eq!((err.line, err.problem.kind()), (2, Kind::State));
        let err = run("at 18446744073709551615\ntick 1\n", &Fixed(0)).unwrap_err();
        assert_eq!(err.to_string(), "line 2: time overflow");
        assert_eq!(err.problem.kind(), Kind::Time);
        let err = run("finish ghost\n", &Fixed(0)).unwrap_err();
        assert_eq!(err.to_string(), "line 1: unknown job `ghost`");
    }

    #[test]
    fn dependencies() {
        let out = table("submit a\nsubmit b\nafter b a\nnext\nnext\n", 0);
        assert_eq!(out, "a running\nb pending\ntime: 0\npending: 1\n");
        let err = run("submit a\nsubmit b\nafter b a\nafter a b\n", &Fixed(0)).unwrap_err();
        assert_eq!(err.to_string(), "line 4: dependency cycle");
        let out = table("submit a\nsubmit b\nafter b a\nstart a\nfail a\n", 0);
        assert_eq!(out, "a failed\nb failed\ntime: 0\npending: 0\n");
    }

    #[test]
    fn failing_without_retry_is_terminal() {
        let out = table("submit a\nstart a\nfail a\n", 0);
        assert_eq!(out, "a failed\ntime: 0\npending: 0\n");
        let err = run("submit a\nstart a\nfail a\nstart a\n", &Fixed(0)).unwrap_err();
        assert_eq!(err.to_string(), "line 4: job `a` cannot start");
    }
}
