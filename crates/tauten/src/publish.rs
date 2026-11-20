//! Publishing a file atomically, as a [`Machine`]: write a temporary file,
//! sync it, rename it over the target, sync the directory; on any failure,
//! remove the temporary file.
//!
//! The machine decides the order; the caller carries out each [`Command`]
//! and reports its outcome as the next [`Event`]. Proved over its
//! translation (`proofs/Tauten/Publish.lean`): against a world in which any
//! command may fail (leaving partial effects) and the temporary file may
//! vanish at any time, in every reachable terminal phase no temporary file
//! is left behind (`publish_discharged`). What the content is after a crash
//! is the subject of roadmap item A7.

use crate::machine::Machine;

/// Where a publish is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Not started.
    Idle,
    /// The temporary file is being written.
    Writing,
    /// Written; being synced.
    Written,
    /// Synced; being renamed over the target.
    Synced,
    /// Renamed; the directory is being synced.
    Renamed,
    /// Something failed; the temporary file is being removed.
    Cleaning,
    /// Published and durable.
    Done,
    /// Failed, with nothing left behind.
    Failed,
}

/// What happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Begin publishing.
    Start,
    /// The last command succeeded.
    Ok,
    /// The last command failed.
    Err,
}

/// What to do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Nothing.
    Nothing,
    /// Create and write the temporary file.
    WriteTemp,
    /// Sync the temporary file's contents.
    SyncTemp,
    /// Rename the temporary file over the target.
    Rename,
    /// Rename the temporary file to the target only if no target exists
    /// (`renameat2` with `RENAME_NOREPLACE`); fails otherwise.
    RenameNoReplace,
    /// Sync the directory, making the rename durable.
    SyncDir,
    /// Remove the temporary file.
    RemoveTemp,
}

/// One step of a publish.
#[must_use]
pub const fn step(phase: Phase, event: Event) -> (Phase, Command) {
    match (phase, event) {
        (Phase::Idle, Event::Start) => (Phase::Writing, Command::WriteTemp),
        (Phase::Writing, Event::Ok) => (Phase::Written, Command::SyncTemp),
        (Phase::Written, Event::Ok) => (Phase::Synced, Command::Rename),
        (Phase::Synced, Event::Ok) => (Phase::Renamed, Command::SyncDir),
        (Phase::Renamed, Event::Ok) => (Phase::Done, Command::Nothing),
        (Phase::Writing | Phase::Written | Phase::Synced | Phase::Cleaning, Event::Err) => {
            (Phase::Cleaning, Command::RemoveTemp)
        }
        (Phase::Cleaning, Event::Ok) | (Phase::Renamed, Event::Err) => {
            (Phase::Failed, Command::Nothing)
        }
        (Phase::Idle, Event::Ok | Event::Err)
        | (
            Phase::Writing
            | Phase::Written
            | Phase::Synced
            | Phase::Renamed
            | Phase::Cleaning
            | Phase::Done
            | Phase::Failed,
            Event::Start,
        )
        | (Phase::Done | Phase::Failed, Event::Ok | Event::Err) => (phase, Command::Nothing),
    }
}

/// One step of a create-new publish: the same protocol, but the final move
/// never replaces an existing target ([`Command::RenameNoReplace`]). If a
/// target appeared since the check, the move fails and the publish cleans
/// up like any other failure.
#[must_use]
pub const fn create_step(phase: Phase, event: Event) -> (Phase, Command) {
    match step(phase, event) {
        (next, Command::Rename) => (next, Command::RenameNoReplace),
        other => other,
    }
}

/// Whether the machine is responsible for a temporary file in `phase`.
#[must_use]
pub const fn holds_temp(phase: Phase) -> bool {
    match phase {
        Phase::Writing | Phase::Written | Phase::Synced | Phase::Cleaning => true,
        Phase::Idle | Phase::Renamed | Phase::Done | Phase::Failed => false,
    }
}

/// Whether a publish has finished, either way.
#[must_use]
pub const fn terminal(phase: Phase) -> bool {
    match phase {
        Phase::Done | Phase::Failed => true,
        Phase::Idle
        | Phase::Writing
        | Phase::Written
        | Phase::Synced
        | Phase::Renamed
        | Phase::Cleaning => false,
    }
}

/// Whether the new content is durable in `phase`: only once the publish is
/// `Done` (the directory sync reported success). Proved in
/// `proofs/Tauten/Durable.lean` under named filesystem assumptions.
#[must_use]
pub const fn durable(phase: Phase) -> bool {
    match phase {
        Phase::Done => true,
        Phase::Idle
        | Phase::Writing
        | Phase::Written
        | Phase::Synced
        | Phase::Renamed
        | Phase::Cleaning
        | Phase::Failed => false,
    }
}

/// The publish protocol as a [`Machine`].
#[derive(Debug, Clone, Copy)]
pub struct Publish;

impl Machine for Publish {
    type State = Phase;
    type Event = Event;
    type Command = Command;
    fn step(state: Phase, event: Event) -> (Phase, Command) {
        step(state, event)
    }
}

/// The create-new publish protocol as a [`Machine`].
#[derive(Debug, Clone, Copy)]
pub struct CreateNew;

impl Machine for CreateNew {
    type State = Phase;
    type Event = Event;
    type Command = Command;
    fn step(state: Phase, event: Event) -> (Phase, Command) {
        create_step(state, event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::{Divergence, replay, run};
    use alloc::vec;

    #[test]
    fn the_happy_path_and_its_replay() {
        let (end, commands) = run::<Publish>(
            Phase::Idle,
            [Event::Start, Event::Ok, Event::Ok, Event::Ok, Event::Ok],
        );
        assert_eq!(end, Phase::Done);
        assert_eq!(
            commands,
            vec![
                Command::WriteTemp,
                Command::SyncTemp,
                Command::Rename,
                Command::SyncDir,
                Command::Nothing
            ]
        );
        let tape = [Event::Start, Event::Ok, Event::Ok, Event::Ok, Event::Ok]
            .into_iter()
            .zip(commands);
        assert_eq!(replay::<Publish>(Phase::Idle, tape), Ok(Phase::Done));
        let wrong = [
            (Event::Start, Command::WriteTemp),
            (Event::Ok, Command::Rename),
        ];
        assert_eq!(
            replay::<Publish>(Phase::Idle, wrong),
            Err(Divergence {
                at: 1,
                recorded: Command::Rename,
                produced: Command::SyncTemp
            })
        );
    }

    #[test]
    fn a_failure_cleans_up_until_it_succeeds() {
        let (end, commands) = run::<Publish>(
            Phase::Idle,
            [Event::Start, Event::Ok, Event::Err, Event::Err, Event::Ok],
        );
        assert_eq!(end, Phase::Failed);
        assert_eq!(
            commands[2..],
            [Command::RemoveTemp, Command::RemoveTemp, Command::Nothing]
        );
        assert!(!holds_temp(end) && terminal(end));
    }

    #[test]
    fn create_new_never_issues_a_replacing_rename() {
        let (end, commands) = run::<CreateNew>(
            Phase::Idle,
            [Event::Start, Event::Ok, Event::Ok, Event::Ok, Event::Ok],
        );
        assert_eq!(end, Phase::Done);
        assert!(commands.contains(&Command::RenameNoReplace));
        assert!(!commands.contains(&Command::Rename));
        // A target that appeared meanwhile: the move fails, and the
        // publish cleans up.
        let (end, _) = run::<CreateNew>(
            Phase::Idle,
            [Event::Start, Event::Ok, Event::Ok, Event::Err, Event::Ok],
        );
        assert_eq!(end, Phase::Failed);
    }
}
