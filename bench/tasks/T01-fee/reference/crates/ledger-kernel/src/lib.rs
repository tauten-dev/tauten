#![no_std]
//! Balance rules: exact, total, no IO.

/// One ledger entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    /// Add funds.
    Deposit(u64),
    /// Remove funds.
    Withdraw(u64),
    /// Remove a percentage (0..=100) of the balance, rounded down.
    Fee(u8),
}

/// Why an entry could not be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    /// The withdrawal exceeds the balance.
    Insufficient,
    /// The balance would exceed the largest representable amount.
    Overflow,
    /// The fee percentage is above 100.
    BadPercent,
}

/// Apply one entry to a balance.
///
/// # Errors
/// Returns a [`Rejection`] when the entry cannot be applied.
pub fn apply(balance: u64, entry: Entry) -> Result<u64, Rejection> {
    match entry {
        Entry::Deposit(amount) => balance.checked_add(amount).ok_or(Rejection::Overflow),
        Entry::Withdraw(amount) => balance.checked_sub(amount).ok_or(Rejection::Insufficient),
        Entry::Fee(percent) => {
            if percent > 100 {
                return Err(Rejection::BadPercent);
            }
            let fee = u128::from(balance)
                .checked_mul(u128::from(percent))
                .and_then(|scaled| scaled.checked_div(100))
                .and_then(|fee| u64::try_from(fee).ok())
                .ok_or(Rejection::Overflow)?;
            balance.checked_sub(fee).ok_or(Rejection::Insufficient)
        }
    }
}
