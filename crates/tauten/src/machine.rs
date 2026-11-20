//! Step machines: a protocol as a pure function of state and event.
//!
//! A [`Machine`] decides; it does no IO. `step` takes the current state and
//! one event (an input, or the result of the last command) and returns the
//! next state and one command for the caller to carry out. Because `step`
//! has no effects, a recorded tape of events and commands can be replayed
//! exactly ([`replay`]), and the protocol can be proved once by induction
//! over steps (`proofs/Tauten/Machine.lean`: `inv_of_step`, and
//! `sys_inv_of_step` against a world that interprets commands and may take
//! adversary steps). The empty effect row of `step` is what makes replay
//! meaningful: a step that read a clock or a file would not replay.
//!
//! The trait fits a by-value kernel machine directly. A `&mut self` reducer
//! fits by moving the state in and out:
//!
//! ```
//! use tauten::machine::Machine;
//!
//! struct Counter;
//! impl Machine for Counter {
//!     type State = u32;
//!     type Event = ();
//!     type Command = u32;
//!     fn step(state: u32, (): ()) -> (u32, u32) {
//!         let next = state.saturating_add(1);
//!         (next, next)
//!     }
//! }
//! assert_eq!(Counter::step(1, ()), (2, 2));
//!
//! // A `&mut self` reducer, adapted by moving its state through `step`.
//! #[derive(Default)]
//! struct Tally(u32);
//! impl Tally {
//!     fn reduce(&mut self, by: u32) -> u32 {
//!         self.0 = self.0.saturating_add(by);
//!         self.0
//!     }
//! }
//! struct Reducer;
//! impl Machine for Reducer {
//!     type State = Tally;
//!     type Event = u32;
//!     type Command = u32;
//!     fn step(mut state: Tally, by: u32) -> (Tally, u32) {
//!         let out = state.reduce(by);
//!         (state, out)
//!     }
//! }
//! assert_eq!(tauten::machine::run::<Reducer>(Tally::default(), [2, 3]).1, [2, 5]);
//! ```

use alloc::vec::Vec;

/// A protocol as a pure step function.
pub trait Machine {
    /// What the machine remembers between events.
    type State;
    /// What happens to it: an input, or the outcome of its last command.
    type Event;
    /// What it asks its caller to do next.
    type Command;
    /// The next state and command. No IO: everything the machine knows
    /// arrives as the state or the event.
    fn step(state: Self::State, event: Self::Event) -> (Self::State, Self::Command);
}

/// Where a replay first disagreed with its tape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Divergence<C> {
    /// The position in the tape (0-based).
    pub at: usize,
    /// The command the tape recorded.
    pub recorded: C,
    /// The command the machine produced.
    pub produced: C,
}

/// Replay a recorded tape of (event, command) pairs from `initial`: the
/// final state when every produced command equals the recorded one, else
/// the first disagreement.
///
/// # Errors
/// [`Divergence`] at the first step whose command differs from the tape.
pub fn replay<M: Machine>(
    initial: M::State,
    tape: impl IntoIterator<Item = (M::Event, M::Command)>,
) -> Result<M::State, Divergence<M::Command>>
where
    M::Command: PartialEq,
{
    let mut state = initial;
    for (at, (event, recorded)) in tape.into_iter().enumerate() {
        let (next, produced) = M::step(state, event);
        if produced != recorded {
            return Err(Divergence {
                at,
                recorded,
                produced,
            });
        }
        state = next;
    }
    Ok(state)
}

/// Run `events` from `initial`: the final state and the commands, in order
/// (to record a tape).
pub fn run<M: Machine>(
    initial: M::State,
    events: impl IntoIterator<Item = M::Event>,
) -> (M::State, Vec<M::Command>) {
    let mut state = initial;
    let mut commands = Vec::new();
    for event in events {
        let (next, command) = M::step(state, event);
        commands.push(command);
        state = next;
    }
    (state, commands)
}
