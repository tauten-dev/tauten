#![no_std]
//! Balance rules: exact, total, no IO.

/// One ledger entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    /// Add funds.
    Deposit(u64),
    /// Remove funds.
    Withdraw(u64),
    /// Remove funds less the volume discount.
    Purchase(u64),
}

/// Why an entry could not be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    /// The withdrawal exceeds the balance.
    Insufficient,
    /// The balance would exceed the largest representable amount.
    Overflow,
}

/// Apply one entry to a balance.
///
/// # Errors
/// Returns a [`Rejection`] when the entry cannot be applied.
pub fn apply(balance: u64, entry: Entry) -> Result<u64, Rejection> {
    match entry {
        Entry::Deposit(amount) => balance.checked_add(amount).ok_or(Rejection::Overflow),
        Entry::Withdraw(amount) => balance.checked_sub(amount).ok_or(Rejection::Insufficient),
        Entry::Purchase(amount) => {
            let first = amount.saturating_sub(1_000).min(9_000);
            let second = amount.saturating_sub(10_000);
            let discount = first
                .checked_mul(5)
                .zip(second.checked_mul(10))
                .and_then(|(a, b)| a.checked_add(b))
                .and_then(|sum| sum.checked_add(50))
                .and_then(|sum| sum.checked_div(100))
                .ok_or(Rejection::Overflow)?;
            let due = amount.checked_sub(discount).ok_or(Rejection::Overflow)?;
            balance.checked_sub(due).ok_or(Rejection::Insufficient)
        }
    }
}
