//! The machines of `tauten` whose steps are proved over their translation
//! (`proofs/Tauten/Publish.lean`).

use tauten::publish::{self, Command, Event, Phase};

/// One step of [`tauten::publish`].
#[must_use]
pub const fn publish_step(phase: Phase, event: Event) -> (Phase, Command) {
    publish::step(phase, event)
}

/// [`tauten::publish::holds_temp`].
#[must_use]
pub const fn publish_holds_temp(phase: Phase) -> bool {
    publish::holds_temp(phase)
}

/// [`tauten::publish::terminal`].
#[must_use]
pub const fn publish_terminal(phase: Phase) -> bool {
    publish::terminal(phase)
}

/// One step of [`tauten::publish::create_step`].
#[must_use]
pub const fn publish_create_step(phase: Phase, event: Event) -> (Phase, Command) {
    publish::create_step(phase, event)
}

/// [`tauten::publish::durable`].
#[must_use]
pub const fn publish_durable(phase: Phase) -> bool {
    publish::durable(phase)
}
