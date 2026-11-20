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
//!
//! # Three carriers, by how far the fact travels
//!
//! | Carrier | Declared with | Holds | Guarantee |
//! |---|---|---|---|
//! | [`Evidence`] | `#[evidence(subject = T)]` | `&T` | compiler-enforced until `redeem` |
//! | [`Permit`] | `#[evidence(subject = T, exclusive)]` | `&mut T` | compiler-enforced: check and act are one chained call ([`Permit::act`]) |
//! | [`Stamped`] | `#[evidence(subject = T, stamped)]`, `T: Versioned` | the version | runtime-checked: [`Stamped::verify`] returns [`Stale`] when the version moved; the lemma shape `stale_refused` is proved for an example kernel action (`proofs/Tauten/Stamped.lean`) |
//! | `#[derive(Bindings)]` | on a struct of recorded values | the values | generated comparison of every field (`diff`, `revalidate` into `Evidence<'_, Fresh<T>>`); tested, not proved |
//!
//! Hand-written tokens in the surveyed projects (docs/PATTERNS.md) take one
//! of three shapes, each a carrier here:
//! * an index or handle bound to the structure it was checked against
//!   (a bound output index): `Evidence` over the structure;
//! * a permission to mutate that must not outlive the check (a mutation
//!   permit on a published run): `Permit`;
//! * a check recorded now and acted on later or elsewhere (a capture, a
//!   ledger entry): `Stamped`, or `Bindings` when the recorded values
//!   themselves are compared.

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

/// An exclusive check: the fact together with the only mutable borrow of
/// its subject, so nothing can change the subject between the check and
/// the action, and the action is one chained call:
///
/// ```
/// use tauten::evidence;
/// use tauten::origin::Permit;
///
/// pub struct Table { slots: Vec<u64> }
///
/// /// Slot `slot` exists.
/// #[evidence(subject = Table, exclusive)]
/// pub struct Present { slot: usize }
///
/// impl Table {
///     pub fn check_mut(&mut self, slot: usize) -> Option<Permit<'_, Present>> {
///         (slot < self.slots.len()).then(|| Present::mint_exclusive(self, slot))
///     }
/// }
///
/// fn main() {
///     let mut table = Table { slots: vec![7] };
///     let cleared = table
///         .check_mut(0)
///         .map(|permit| permit.act(|t, fact| t.slots.get_mut(*fact.slot()).map(|s| *s = 0)));
///     assert_eq!(cleared, Some(Some(())));
///     assert_eq!(table.slots[0], 0);
/// }
/// ```
///
/// Compiler-enforced: a permit holds `&mut` to its subject, cannot be
/// cloned, and only the declaring module can mint one.
#[must_use = "a permit is the result of a check; act on it"]
pub struct Permit<'s, P: Proposition> {
    subject: &'s mut P::Subject,
    fact: P,
}

impl<'s, P: Proposition> Permit<'s, P> {
    /// Build a permit. Used by the code `#[evidence(.., exclusive)]`
    /// generates; the fact itself can only be built in its declaring
    /// module.
    #[doc(hidden)]
    pub const fn __mint(fact: P, subject: &'s mut P::Subject) -> Self {
        Self { subject, fact }
    }

    /// The fact.
    #[must_use]
    pub const fn fact(&self) -> &P {
        &self.fact
    }

    /// Read the subject.
    #[must_use]
    pub fn subject(&self) -> &P::Subject {
        self.subject
    }

    /// Act on the checked subject: the check and the action are one chain,
    /// with no window between them.
    pub fn act<R>(self, action: impl FnOnce(&mut P::Subject, P) -> R) -> R {
        action(self.subject, self.fact)
    }
}

impl<P: Proposition + fmt::Debug> fmt::Debug for Permit<'_, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Permit").field(&self.fact).finish()
    }
}

/// A subject whose every change bumps a version, so that a fact recorded
/// against one version can be re-checked later.
pub trait Versioned {
    /// The current version. It must change whenever the state a fact may
    /// depend on changes.
    fn version(&self) -> u64;
}

/// The subject changed since the fact was recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stale;

impl fmt::Display for Stale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the subject changed since the fact was checked")
    }
}

impl core::error::Error for Stale {}

/// A fact recorded with its subject's version, so it can travel without
/// borrowing (across an `await`, into a queue): before acting,
/// [`Stamped::verify`] re-checks the version and returns [`Stale`] when it
/// moved.
///
/// Runtime-checked: the version comparison happens when the fact is used;
/// it is only as good as the subject's [`Versioned`] implementation.
#[must_use = "a stamped fact must be verified before it is used"]
pub struct Stamped<P: Proposition> {
    fact: P,
    version: u64,
}

impl<P: Proposition> Stamped<P> {
    /// Build a stamped fact. Used by the code `#[evidence(.., stamped)]`
    /// generates.
    #[doc(hidden)]
    pub const fn __mint(fact: P, version: u64) -> Self {
        Self { fact, version }
    }

    /// The version the fact was checked at.
    #[must_use]
    pub const fn version(&self) -> u64 {
        self.version
    }
}

impl<P> Stamped<P>
where
    P: Proposition,
    P::Subject: Versioned,
{
    /// The fact as [`Evidence`] about `subject` now, if `subject` is still
    /// at the version the fact was checked at.
    ///
    /// # Errors
    /// [`Stale`] when the version moved.
    pub fn verify(self, subject: &P::Subject) -> Result<Evidence<'_, P>, Stale> {
        if subject.version() == self.version {
            Ok(Evidence::__mint(self.fact, subject))
        } else {
            Err(Stale)
        }
    }
}

impl<P: Proposition + fmt::Debug> fmt::Debug for Stamped<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stamped")
            .field("fact", &self.fact)
            .field("version", &self.version)
            .finish()
    }
}

/// The fact that a value equals the current one in every field: what
/// `#[derive(Bindings)]`'s `revalidate` returns.
pub struct Fresh<T> {
    _value: core::marker::PhantomData<fn() -> T>,
}

impl<T> Fresh<T> {
    /// Used by the code `#[derive(Bindings)]` generates.
    #[doc(hidden)]
    #[must_use]
    pub const fn __new() -> Self {
        Self {
            _value: core::marker::PhantomData,
        }
    }
}

impl<T> Proposition for Fresh<T> {
    type Subject = T;
}

impl<T> fmt::Debug for Fresh<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Fresh")
    }
}
