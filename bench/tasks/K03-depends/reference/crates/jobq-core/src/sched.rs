//! The scheduler: jobs, the current time and the effect of each command.

use std::collections::BTreeMap;

use jobq_kernel::{Rank, Refusal, State, Step, compare, later, retry_at, transition};

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
    /// Jobs (by index) that must be done before this one starts.
    pub deps: Vec<usize>,
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
}

impl Scheduler {
    /// An empty scheduler whose clock reads `now`.
    #[must_use]
    pub fn new(now: u64) -> Self {
        Self { jobs: Vec::new(), by_name: BTreeMap::new(), now }
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

    fn index(&self, name: &str) -> Result<usize, Problem> {
        self.by_name.get(name).copied().ok_or_else(|| Problem::UnknownJob(name.to_owned()))
    }

    /// Every dependency of the job at `index` is done.
    fn deps_done(&self, index: usize) -> bool {
        self.jobs.get(index).is_some_and(|job| {
            job.deps.iter().all(|&d| self.jobs.get(d).is_some_and(|dep| dep.state == State::Done))
        })
    }

    /// Whether `from` depends on `to`, directly or through other jobs (or is `to`).
    /// Each job is visited once, so the search is linear in the number of dependencies.
    fn reaches(&self, from: usize, to: usize) -> bool {
        let mut seen = vec![false; self.jobs.len()];
        let mut stack = vec![from];
        while let Some(i) = stack.pop() {
            if i == to {
                return true;
            }
            if let Some(flag) = seen.get_mut(i) {
                if *flag {
                    continue;
                }
                *flag = true;
            }
            if let Some(job) = self.jobs.get(i) {
                stack.extend(job.deps.iter().copied());
            }
        }
        false
    }

    /// Fail every pending job that depends, directly or through others, on a failed job.
    fn spread_failure(&mut self, failed: usize) {
        let mut work = vec![failed];
        while let Some(f) = work.pop() {
            for (i, job) in self.jobs.iter_mut().enumerate() {
                if job.state == State::Pending && job.deps.contains(&f) {
                    job.state = State::Failed;
                    work.push(i);
                }
            }
        }
    }

    fn after(&mut self, job: &str, dependency: &str) -> Result<(), Problem> {
        let job = self.index(job)?;
        let dependency = self.index(dependency)?;
        if self.reaches(dependency, job) {
            return Err(Problem::DependencyCycle);
        }
        let deps = &mut self.jobs.get_mut(job).ok_or(Problem::DependencyCycle)?.deps;
        if !deps.contains(&dependency) {
            deps.push(dependency);
        }
        Ok(())
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
            Command::After(job, dependency) => self.after(&job, &dependency),
            Command::Tick(ms) => {
                self.now = later(self.now, ms).ok_or(Problem::TimeOverflow)?;
                Ok(())
            }
            Command::At(ms) => {
                self.now = ms;
                Ok(())
            }
        }
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
            deps: Vec::new(),
            state: State::Pending,
        });
        Ok(())
    }

    fn start(&mut self, name: &str) -> Result<(), Problem> {
        let now = self.now;
        if !self.deps_done(self.index(name)?) {
            return Err(Problem::CannotStart(name.to_owned()));
        }
        let job = self.job_mut(name)?;
        if job.ready_at > now {
            return Err(Problem::CannotStart(name.to_owned()));
        }
        job.state = transition(job.state, Step::Start).map_err(|r| refusal(r, name))?;
        job.started_at = Some(now);
        Ok(())
    }

    /// Start the first ready pending job in start order, if there is one.
    fn next(&mut self) {
        let now = self.now;
        let first = self
            .jobs
            .iter()
            .enumerate()
            .filter(|(i, j)| jobq_kernel::is_runnable(j.state) && j.ready_at <= now && self.deps_done(*i))
            .min_by(|(_, a), (_, b)| compare(&a.rank(), &b.rank()))
            .map(|(i, _)| i);
        if let Some(job) = first.and_then(|i| self.jobs.get_mut(i)) {
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
        if state == State::Failed {
            let index = self.index(name)?;
            self.spread_failure(index);
        }
        Ok(())
    }
}

fn refusal(refusal: Refusal, name: &str) -> Problem {
    match refusal {
        Refusal::CannotStart => Problem::CannotStart(name.to_owned()),
        Refusal::NotRunning => Problem::NotRunning(name.to_owned()),
    }
}
