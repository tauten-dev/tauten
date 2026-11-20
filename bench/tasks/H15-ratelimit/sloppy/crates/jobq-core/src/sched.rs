//! The scheduler: jobs, the current time and the effect of each command.

use std::collections::BTreeMap;

use jobq_kernel::{Bucket, Rank, Refusal, State, Step, compare, later, retry_at, transition};

use crate::Problem;
use crate::parse::Command;

/// One job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    /// Unique name.
    pub name: String,
    /// 0 to 255; higher starts first.
    pub priority: u8,
    /// Time of the `submit` event (ms).
    pub submitted_at: u64,
    /// Fixed delay before a failed job may start again.
    pub retry: Option<u64>,
    /// Earliest time the job may start (ms).
    pub ready_at: u64,
    /// Time the job last started (ms).
    pub started_at: Option<u64>,
    /// Number of failures so far.
    pub failures: u64,
    /// Current state.
    pub state: State,
}

impl Job {
    fn rank(&self) -> Rank<'_> {
        Rank { priority: self.priority, submitted_at: self.submitted_at, name: &self.name }
    }
}

/// Jobs in submission order and the current time.
#[derive(Debug, Clone, Default)]
pub struct Scheduler {
    jobs: Vec<Job>,
    by_name: BTreeMap<String, usize>,
    now: u64,
    limit: Option<Bucket>,
}

impl Scheduler {
    /// An empty scheduler whose clock reads `now`.
    #[must_use]
    pub fn new(now: u64) -> Self {
        Self { jobs: Vec::new(), by_name: BTreeMap::new(), now, limit: None }
    }

    /// The current time (ms).
    #[must_use]
    pub const fn now(&self) -> u64 {
        self.now
    }

    /// Jobs in submission order.
    #[must_use]
    pub fn jobs(&self) -> &[Job] {
        &self.jobs
    }

    /// Number of jobs that may still be started.
    #[must_use]
    pub fn pending(&self) -> usize {
        self.jobs.iter().filter(|j| jobq_kernel::is_runnable(j.state)).count()
    }

    fn job_mut(&mut self, name: &str) -> Result<&mut Job, Problem> {
        let index = *self.by_name.get(name).ok_or_else(|| Problem::UnknownJob(name.to_owned()))?;
        self.jobs.get_mut(index).ok_or_else(|| Problem::UnknownJob(name.to_owned()))
    }

    /// Apply one command.
    ///
    /// # Errors
    /// Returns the [`Problem`] that stops the script.
    pub fn apply(&mut self, command: Command) -> Result<(), Problem> {
        match command {
            Command::Submit { name, priority, retry } => self.submit(name, priority, retry),
            Command::Start(name) => self.start(&name),
            Command::Next => {
                self.next();
                Ok(())
            }
            Command::Finish(name) => self.step(&name, Step::Finish),
            Command::Fail(name) => self.fail(&name),
            Command::Tick(ms) => {
                let now = later(self.now, ms).ok_or(Problem::TimeOverflow)?;
                self.set_time(now);
                Ok(())
            }
            Command::At(ms) => {
                self.set_time(ms);
                Ok(())
            }
            Command::Limit { rate, per, burst } => {
                self.limit = Some(Bucket::new(rate, per, burst));
                Ok(())
            }
        }
    }

    /// Move the clock; only forward movement refills the start limit.
    fn set_time(&mut self, now: u64) {
        if let (Some(bucket), Some(elapsed)) = (self.limit.as_mut(), now.checked_sub(self.now)) {
            bucket.refill(elapsed);
        }
        self.now = now;
    }

    /// Take a start token; `false` when the limit allows no start now.
    fn token(&mut self) -> bool {
        self.limit.as_mut().is_none_or(Bucket::take)
    }

    fn submit(&mut self, name: String, priority: u8, retry: Option<u64>) -> Result<(), Problem> {
        if self.by_name.contains_key(&name) {
            return Err(Problem::DuplicateJob(name));
        }
        self.by_name.insert(name.clone(), self.jobs.len());
        self.jobs.push(Job {
            name,
            priority,
            submitted_at: self.now,
            retry,
            ready_at: self.now,
            started_at: None,
            failures: 0,
            state: State::Pending,
        });
        Ok(())
    }

    fn start(&mut self, name: &str) -> Result<(), Problem> {
        let now = self.now;
        let job = self.job_mut(name)?;
        if job.ready_at > now {
            return Err(Problem::CannotStart(name.to_owned()));
        }
        let state = transition(job.state, Step::Start).map_err(|r| refusal(r, name))?;
        if !self.token() {
            return Err(Problem::RateLimited);
        }
        let job = self.job_mut(name)?;
        job.state = state;
        job.started_at = Some(now);
        Ok(())
    }

    /// Start the first ready pending job in start order, if there is one.
    fn next(&mut self) {
        let now = self.now;
        let any = self.jobs.iter().any(|j| jobq_kernel::is_runnable(j.state) && j.ready_at <= now);
        if !any || !self.token() {
            return;
        }
        let first = self
            .jobs
            .iter_mut()
            .filter(|j| jobq_kernel::is_runnable(j.state) && j.ready_at <= now)
            .min_by(|a, b| compare(&a.rank(), &b.rank()));
        if let Some(job) = first {
            job.state = State::Running;
            job.started_at = Some(now);
        }
    }

    fn step(&mut self, name: &str, step: Step) -> Result<(), Problem> {
        let job = self.job_mut(name)?;
        job.state = transition(job.state, step).map_err(|r| refusal(r, name))?;
        Ok(())
    }

    fn fail(&mut self, name: &str) -> Result<(), Problem> {
        let now = self.now;
        let job = self.job_mut(name)?;
        let state = transition(job.state, Step::Fail { retry: job.retry.is_some() }).map_err(|r| refusal(r, name))?;
        if let Some(delay) = job.retry {
            job.ready_at = retry_at(now, delay).ok_or(Problem::TimeOverflow)?;
        }
        job.failures = job.failures.saturating_add(1);
        job.state = state;
        Ok(())
    }
}

fn refusal(refusal: Refusal, name: &str) -> Problem {
    match refusal {
        Refusal::CannotStart => Problem::CannotStart(name.to_owned()),
        Refusal::NotRunning => Problem::NotRunning(name.to_owned()),
    }
}
