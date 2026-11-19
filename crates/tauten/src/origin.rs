//! Evidence: the result of a check, tied to the state it checked.
//!
//! "Check, then act" goes wrong when the state changes between the check and
//! the action. Here a check returns [`Evidence`], which borrows the state it
//! examined. While the evidence is alive the state cannot be mutated, so the
//! action that consumes it acts on exactly what was checked.
//!
//! Declare the fact as an `#[evidence(subject = State)]` struct. Only the
//! module that declares it can create one (with the generated `mint`), so
//! code elsewhere cannot fake it:
//!
//! ```
//! use tauten::evidence;
//! use tauten::origin::Evidence;
//!
//! pub struct Table { slots: Vec<u64> }
//!
//! /// Slot `slot` exists and holds a value at or after `now`.
//! #[evidence(subject = Table)]
//! pub struct Unexpired { slot: usize, now: u64 }
//!
//! impl Table {
//!     pub fn check_unexpired(&self, slot: usize, now: u64) -> Option<Evidence<'_, Unexpired>> {
//!         let expires = *self.slots.get(slot)?;
//!         (expires >= now).then(|| Unexpired::mint(self, slot, now))
//!     }
//!
//!     pub fn renew(&mut self, fact: Unexpired, until: u64) {
//!         if let Some(s) = self.slots.get_mut(*fact.slot()) { *s = until; }
//!     }
//! }
//!
//! fn main() {
//!     let mut table = Table { slots: vec![10] };
//!     let ev = table.check_unexpired(0, 5).expect("unexpired");
//!     table.renew(ev.redeem(), 20); // pass the evidence straight in
//!     assert_eq!(table.slots[0], 20);
//! }
//! ```
//!
//! Changing the state while evidence about it is alive does not compile,
//! and neither does building the fact outside its module (both are
//! compile-fail tests in `tests/ui`).
//!
//! Declare evidence at module level: the generated module refers to the
//! subject type through `super::*`, so a fact declared inside a function
//! body cannot name types local to that body.
//!
//! The guarantee lasts until [`Evidence::redeem`]: the redeemed fact no
//! longer borrows the state. Redeem in argument position, directly into the
//! call that acts on it.

use core::fmt;

/// A fact about some state, declared with `#[evidence(subject = State)]`.
pub trait Proposition {
    /// The type of state the fact is about.
    type Subject: ?Sized;
}

/// A checked fact `P`, borrowing the state it was checked against. No
/// `Clone`, no `Copy`: one check, one use.
#[must_use = "evidence is the result of a check; pass it to the action it guards"]
pub struct Evidence<'s, P: Proposition> {
    subject: &'s P::Subject,
    fact: P,
}

impl<'s, P: Proposition> Evidence<'s, P> {
    /// Used by the `mint` function that `#[evidence]` generates; call that
    /// instead.
    #[doc(hidden)]
    pub const fn __mint(fact: P, subject: &'s P::Subject) -> Self {
        Self { subject, fact }
    }

    /// The state the fact was checked against.
    #[must_use]
    pub const fn subject(&self) -> &'s P::Subject {
        self.subject
    }

    /// The fact, by reference.
    #[must_use]
    pub const fn fact(&self) -> &P {
        &self.fact
    }

    /// Give up the borrow and take the fact, to pass it to the action it
    /// guards. Do this in argument position.
    #[must_use]
    pub fn redeem(self) -> P {
        self.fact
    }
}

impl<P: Proposition + fmt::Debug> fmt::Debug for Evidence<'_, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Evidence").field(&self.fact).finish()
    }
}
