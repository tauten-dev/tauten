#![no_std]
//! Job states, transitions, ordering and time arithmetic: pure and exact.

use core::cmp::Ordering;

/// The state of a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Waiting to be started.
    Pending,
    /// Started and not yet finished.
    Running,
    /// Finished successfully.
    Done,
    /// Failed with no retry.
    Failed,
}

impl State {
    /// The word used for this state in the job table.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }
}

/// Whether a job in this state will never change again.
#[must_use]
pub const fn is_terminal(state: State) -> bool {
    matches!(state, State::Done | State::Failed)
}

/// Whether a job in this state may be started (it is neither running nor finished).
#[must_use]
pub const fn is_runnable(state: State) -> bool {
    !is_terminal(state) && !matches!(state, State::Running)
}

/// Something that happens to one job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// The job starts running.
    Start,
    /// The running job finishes successfully.
    Finish,
    /// The running job fails; `retry` says whether it goes back to pending.
    Fail {
        /// The job has a retry policy.
        retry: bool,
    },
}

/// Why a step does not apply to a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The job is not in a state that may start.
    CannotStart,
    /// The step needs a running job.
    NotRunning,
}

/// The state a job moves to when `step` happens to it.
///
/// # Errors
/// Returns a [`Refusal`] when the step does not apply in `state`.
pub const fn transition(state: State, step: Step) -> Result<State, Refusal> {
    match step {
        Step::Start => {
            if is_runnable(state) {
                Ok(State::Running)
            } else {
                Err(Refusal::CannotStart)
            }
        }
        Step::Finish => match state {
            State::Running => Ok(State::Done),
            State::Pending | State::Done | State::Failed => Err(Refusal::NotRunning),
        },
        Step::Fail { retry } => match state {
            State::Running => Ok(if retry { State::Pending } else { State::Failed }),
            State::Pending | State::Done | State::Failed => Err(Refusal::NotRunning),
        },
    }
}

/// What decides the order in which pending jobs are started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rank<'a> {
    /// 0 to 255; higher starts first.
    pub priority: u8,
    /// When the job was submitted (ms).
    pub submitted_at: u64,
    /// The job's name.
    pub name: &'a str,
}

/// Start order: priority descending, then submit time ascending, then name.
#[must_use]
pub fn compare(a: &Rank<'_>, b: &Rank<'_>) -> Ordering {
    b.priority
        .cmp(&a.priority)
        .then(a.submitted_at.cmp(&b.submitted_at))
        .then(a.name.cmp(b.name))
}

/// `now` plus `ms`, or `None` when the result is not a representable time.
#[must_use]
pub const fn later(now: u64, ms: u64) -> Option<u64> {
    now.checked_add(ms)
}

/// When a job that failed at `now` with retry delay `delay` may start again.
#[must_use]
pub const fn retry_at(now: u64, delay: u64) -> Option<u64> {
    later(now, delay)
}

/// A token bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bucket {
    rate: u64,
    per: u64,
    burst: u64,
    /// Whole tokens.
    tokens: u64,
}

impl Bucket {
    /// A full bucket: `burst` tokens, refilled at `rate` tokens per `per` ms.
    #[must_use]
    pub fn new(rate: u64, per: u64, burst: u64) -> Self {
        Self { rate, per, burst, tokens: burst }
    }

    /// Time moved forward by `elapsed` ms.
    pub fn refill(&mut self, elapsed: u64) {
        let added = elapsed * self.rate / self.per;
        self.tokens = (self.tokens + added).min(self.burst);
    }

    /// Take one whole token, if there is one.
    pub fn take(&mut self) -> bool {
        match self.tokens.checked_sub(1) {
            Some(rest) => {
                self.tokens = rest;
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle() {
        assert_eq!(transition(State::Pending, Step::Start), Ok(State::Running));
        assert_eq!(transition(State::Running, Step::Finish), Ok(State::Done));
        assert_eq!(transition(State::Running, Step::Fail { retry: true }), Ok(State::Pending));
        assert_eq!(transition(State::Running, Step::Fail { retry: false }), Ok(State::Failed));
    }

    #[test]
    fn refusals() {
        assert_eq!(transition(State::Done, Step::Start), Err(Refusal::CannotStart));
        assert_eq!(transition(State::Running, Step::Start), Err(Refusal::CannotStart));
        assert_eq!(transition(State::Pending, Step::Finish), Err(Refusal::NotRunning));
        assert_eq!(transition(State::Failed, Step::Fail { retry: true }), Err(Refusal::NotRunning));
    }

    #[test]
    fn order() {
        let a = Rank { priority: 5, submitted_at: 10, name: "b" };
        let b = Rank { priority: 5, submitted_at: 10, name: "a" };
        let c = Rank { priority: 9, submitted_at: 99, name: "z" };
        assert_eq!(compare(&c, &a), Ordering::Less);
        assert_eq!(compare(&b, &a), Ordering::Less);
        assert_eq!(compare(&a, &a), Ordering::Equal);
    }

    #[test]
    fn bucket_refills() {
        let mut b = Bucket::new(1, 3, 1);
        assert!(b.take());
        b.refill(2);
        assert!(!b.take());
        b.refill(3);
        assert!(b.take());
    }

    #[test]
    fn time_is_exact() {
        assert_eq!(later(u64::MAX - 1, 1), Some(u64::MAX));
        assert_eq!(later(u64::MAX, 1), None);
        assert_eq!(retry_at(5, 10), Some(15));
    }
}
