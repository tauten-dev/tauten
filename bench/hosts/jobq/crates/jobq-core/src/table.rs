//! The job table printed at the end of a run.

use std::io::{self, Write};

use crate::sched::Scheduler;

/// Write the job table: one line per job in submission order, then the
/// final time and the number of jobs that may still start.
///
/// # Errors
/// Returns the error of the first failed write.
pub fn render(sched: &Scheduler, out: &mut impl Write) -> io::Result<()> {
    for job in sched.jobs() {
        write!(out, "{} {}", job.name, job.state.word())?;
        if jobq_kernel::is_runnable(job.state) && job.ready_at > sched.now() {
            write!(out, " (ready at {})", job.ready_at)?;
        }
        writeln!(out)?;
    }
    writeln!(out, "time: {}", sched.now())?;
    writeln!(out, "pending: {}", sched.pending())
}
