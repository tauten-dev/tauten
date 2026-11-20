//! A spending ledger. Decisions come from the kernel (`{{KERNEL_SNAKE}}`);
//! the world (here, the clock) arrives only as a capability parameter, and
//! each `pub fn` states its effect row with `#[effects(..)]`.

use core::fmt;
use core::num::ParseIntError;

use tauten::cap::Clock;
use tauten::evidence;
use tauten::origin::Evidence;
use tauten::{Cap, effects};

/// The capabilities this crate needs, created by the edge binary.
#[derive(Cap)]
pub struct Caps<'a, C: Clock> {
    /// Time source for receipts.
    pub clock: &'a C,
}

/// A budget and how much of it is spent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ledger {
    budget: u64,
    spent: u64,
}

/// The ledger had room for `cost` when it was checked.
///
/// A check travels to the action as evidence: `Evidence<'_, Affordable>`
/// borrows the ledger, so the ledger cannot change between the check and
/// the charge, and only this module can mint the fact. The permit primitive
/// planned in tauten's roadmap (A5, issue #9) does not exist yet; when it
/// lands it replaces this `Evidence`.
#[evidence(subject = Ledger)]
pub struct Affordable {
    cost: u64,
}

/// A completed charge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Receipt {
    /// What was charged.
    pub cost: u64,
    /// What is left of the budget afterwards.
    pub left: u64,
    /// When, in milliseconds since the Unix epoch.
    pub at_ms: u64,
}

impl Ledger {
    /// A ledger with nothing spent.
    #[effects()]
    #[must_use]
    pub const fn new(budget: u64) -> Self {
        Self { budget, spent: 0 }
    }

    /// Evidence that `cost` fits, or `None`. The decision is the kernel's
    /// proved `affordable`.
    #[effects()]
    #[must_use]
    pub fn check(&self, cost: u64) -> Option<Evidence<'_, Affordable>> {
        {{KERNEL_SNAKE}}::affordable(self.budget, self.spent, cost)
            .then(|| Affordable::mint(self, cost))
    }

    /// Charge what `fact` says was checked, stamped with the clock's time.
    #[effects(clock)]
    pub fn charge(&mut self, fact: Affordable, clock: &impl Clock) -> Receipt {
        // The evidence showed spent + cost <= budget, so neither saturates.
        self.spent = self.spent.saturating_add(*fact.cost());
        Receipt {
            cost: *fact.cost(),
            left: self.budget.saturating_sub(self.spent),
            at_ms: clock.now_ms(),
        }
    }
}

/// Why `run` failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunError {
    /// No budget was given.
    Usage,
    /// An argument is not a whole number.
    NotANumber {
        /// The argument.
        arg: String,
        /// Why it did not parse.
        reason: ParseIntError,
    },
    /// A charge does not fit what is left.
    OverBudget {
        /// The refused charge.
        cost: u64,
    },
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => write!(f, "usage: <budget> <cost>..."),
            Self::NotANumber { arg, reason } => write!(f, "not a whole number: {arg}: {reason}"),
            Self::OverBudget { cost } => write!(f, "over budget: {cost}"),
        }
    }
}

fn number(arg: &str) -> Result<u64, RunError> {
    arg.parse().map_err(|reason| RunError::NotANumber {
        arg: arg.to_owned(),
        reason,
    })
}

/// The program: `<budget> <cost>...`. Charges each cost in turn and returns
/// one receipt per charge.
///
/// # Errors
/// [`RunError`] for missing or malformed arguments, or a charge over budget.
#[effects(clock, fail)]
pub fn run<C: Clock>(caps: &Caps<'_, C>, args: &[String]) -> Result<Vec<Receipt>, RunError> {
    let (first, costs) = args.split_first().ok_or(RunError::Usage)?;
    let mut ledger = Ledger::new(number(first)?);
    let mut receipts = Vec::new();
    for arg in costs {
        let cost = number(arg)?;
        let fact = ledger.check(cost).ok_or(RunError::OverBudget { cost })?;
        // Redeem in argument position, straight into the action it guards.
        receipts.push(ledger.charge(fact.redeem(), caps.clock));
    }
    Ok(receipts)
}

#[cfg(test)]
mod tests {
    use super::{Caps, Receipt, RunError, run};
    use tauten::cap::ManualClock;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn charges_until_the_budget_is_used() {
        let clock = ManualClock::at(7);
        let caps = Caps { clock: &clock };
        assert_eq!(
            run(&caps, &args(&["10", "4", "6"])),
            Ok(vec![
                Receipt {
                    cost: 4,
                    left: 6,
                    at_ms: 7
                },
                Receipt {
                    cost: 6,
                    left: 0,
                    at_ms: 7
                },
            ])
        );
        assert_eq!(
            run(&caps, &args(&["10", "4", "7"])),
            Err(RunError::OverBudget { cost: 7 })
        );
        assert_eq!(run(&caps, &args(&[])), Err(RunError::Usage));
    }
}
