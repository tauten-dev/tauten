//! P0 probe for the tauten proof plan (docs/PROOF-PLAN.md section 5, P0;
//! results in docs/proofs/probe.md). Each module is one probe item and is
//! translated on its own (`proofs/probe.sh`), so one failure does not hide
//! another. Nothing here is product code.
#![no_std]

extern crate alloc;

/// Item 1: the tier relation, with the derived comparison replaced by a
/// `rank` match, next to the original derived-`PartialOrd` form.
pub mod tier {
    /// Crate tiers, innermost first (as `tauten_check::tier::Tier`).
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum Tier {
        /// Pure decisions.
        Kernel,
        /// No ambient authority.
        Core,
        /// Startup and IO.
        Edge,
    }

    /// The position of a tier, innermost first.
    #[must_use]
    pub const fn rank(t: Tier) -> u8 {
        match t {
            Tier::Kernel => 0,
            Tier::Core => 1,
            Tier::Edge => 2,
        }
    }

    /// Whether a crate of tier `t` may depend on one of tier `dep`.
    #[must_use]
    pub const fn may_depend_on(t: Tier, dep: Tier) -> bool {
        rank(dep) <= rank(t)
    }

    /// The shipped form: a derived `PartialOrd`.
    #[must_use]
    pub fn may_depend_on_derived(t: Tier, dep: Tier) -> bool {
        dep <= t
    }
}

/// Item 2: a ratchet-style loop over `Vec<u32>`: each error finding whose
/// key still has accepted headroom is covered and consumes one unit.
pub mod ratchet {
    use alloc::vec::Vec;

    /// One finding, its key already interned to an index.
    pub struct Finding {
        /// Interned key.
        pub key: usize,
        /// Error level (only errors can be covered).
        pub error: bool,
    }

    /// Whether finding `f` is covered by `left`.
    #[must_use]
    pub fn covers(left: &[u32], f: &Finding) -> bool {
        f.error && f.key < left.len() && left[f.key] > 0
    }

    /// Returns the covered mask and the accepted counts left over.
    #[must_use]
    pub fn apply(mut left: Vec<u32>, findings: &[Finding]) -> (Vec<bool>, Vec<u32>) {
        let mut covered = Vec::new();
        let mut i = 0;
        while i < findings.len() {
            let c = covers(&left, &findings[i]);
            if c {
                let k = findings[i].key;
                left[k] -= 1;
            }
            covered.push(c);
            i += 1;
        }
        (covered, left)
    }
}

/// Item 3: `Percent(u8)` through the real `#[tauten::refined]` macro, with
/// the derives a user would write.
pub mod percent {
    /// A percentage.
    #[tauten::refined(|v| *v <= 100)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Percent(u8);
}

/// Item 3b: the same type written by hand as a closure-free expansion: the
/// predicate is inlined over `let v = &value;`, everything else as the macro
/// emits it, except that the derives are omitted (they are probed by item 3).
pub mod percent_hand {
    mod inner {
        /// A percentage.
        pub struct PercentHand(u8);

        impl PercentHand {
            /// Wrap a value if it is at most 100.
            ///
            /// # Errors
            /// Returns [`tauten::RefineError`] if the predicate is false.
            pub fn try_new(value: u8) -> Result<Self, tauten::RefineError> {
                let v = &value;
                if *v <= 100 {
                    Ok(Self(value))
                } else {
                    Err(tauten::RefineError::new("PercentHand", "| v | * v <= 100"))
                }
            }

            /// Borrow the wrapped value.
            #[must_use]
            pub const fn get(&self) -> &u8 {
                &self.0
            }
        }
    }
    pub use inner::PercentHand;
}

/// The error type the contract items and the ledger share.
pub mod error {
    /// Why a ledger operation failed.
    pub enum LedgerError {
        /// The balance would exceed `u64::MAX`.
        Overflow,
        /// The withdrawal exceeds the balance.
        Insufficient,
        /// A contract was violated.
        Contract(tauten::ContractViolation),
    }

    impl From<tauten::ContractViolation> for LedgerError {
        fn from(v: tauten::ContractViolation) -> Self {
            Self::Contract(v)
        }
    }
}

/// Item 4: one function through the real `#[requires]`/`#[ensures]` with
/// `old()`.
pub mod contract {
    use crate::error::LedgerError;

    /// Deposit a positive amount.
    ///
    /// # Errors
    /// `Overflow`, or a contract violation.
    #[tauten::requires(amount > 0)]
    #[tauten::ensures(|after: &u64| *after > old(balance))]
    pub fn deposit(balance: u64, amount: u64) -> Result<u64, LedgerError> {
        balance.checked_add(amount).ok_or(LedgerError::Overflow)
    }
}

/// Item 4b: the same contract written by hand without closures: the body
/// moves into a nested function (so a `return` in it stays local, as the
/// closure guarantees today), the postcondition is inlined over a binding.
pub mod contract_hand {
    use crate::error::LedgerError;

    /// Deposit a positive amount.
    ///
    /// # Errors
    /// `Overflow`, or a contract violation.
    pub fn deposit_hand(balance: u64, amount: u64) -> Result<u64, LedgerError> {
        fn body(balance: u64, amount: u64) -> Result<u64, LedgerError> {
            balance.checked_add(amount).ok_or(LedgerError::Overflow)
        }
        if !(amount > 0) {
            return Err(From::from(tauten::ContractViolation::requires(
                "deposit_hand",
                "amount > 0",
            )));
        }
        let old_0 = balance;
        let result = body(balance, amount);
        if let Ok(after) = &result {
            if !(*after > old_0) {
                return Err(From::from(tauten::ContractViolation::ensures(
                    "deposit_hand",
                    "| after : & u64 | * after > old(balance)",
                )));
            }
        }
        result
    }
}

/// Item 5: the ledger `apply` (bench/hosts/ledger's kernel).
pub mod ledger {
    use crate::error::LedgerError;

    /// One ledger entry.
    pub enum Entry {
        /// Add funds.
        Deposit(u64),
        /// Remove funds.
        Withdraw(u64),
    }

    /// Apply one entry to a balance.
    ///
    /// # Errors
    /// `Overflow` or `Insufficient`.
    pub fn apply(balance: u64, entry: Entry) -> Result<u64, LedgerError> {
        match entry {
            Entry::Deposit(amount) => balance.checked_add(amount).ok_or(LedgerError::Overflow),
            Entry::Withdraw(amount) => balance.checked_sub(amount).ok_or(LedgerError::Insufficient),
        }
    }

    /// `saturating_sub`, which the plan also asks about (section 6, Q3).
    #[must_use]
    pub fn shrink(left: u32, by: u32) -> u32 {
        left.saturating_sub(by)
    }
}

/// Item 6: the `derive(Default)` control, a plain (not refined) struct. If
/// Charon extracts derived impls, `Default::default` shows up here as a
/// constructor of `Plain` (plan section 2, binding 3).
pub mod control {
    /// A plain struct.
    #[derive(Default)]
    pub struct Plain {
        /// A counter.
        pub count: u32,
        /// A flag.
        pub set: bool,
    }

    /// A caller of the derived impl.
    #[must_use]
    pub fn fresh() -> Plain {
        Plain::default()
    }
}
