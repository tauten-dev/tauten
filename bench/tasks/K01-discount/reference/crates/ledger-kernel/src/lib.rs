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
            let due = amount.checked_sub(discount(amount)).ok_or(Rejection::Overflow)?;
            balance.checked_sub(due).ok_or(Rejection::Insufficient)
        }
    }
}

/// 5% of the part above 1000 up to 10000 plus 10% of the part above 10000,
/// rounded half up. Computed in `u128`: the discount of any `u64` fits.
fn discount(amount: u64) -> u64 {
    let wide = u128::from(amount);
    let first = wide.saturating_sub(1_000).min(9_000);
    let second = wide.saturating_sub(10_000);
    let hundredths = first
        .checked_mul(5)
        .zip(second.checked_mul(10))
        .and_then(|(a, b)| a.checked_add(b))
        .and_then(|sum| sum.checked_add(50))
        .and_then(|sum| sum.checked_div(100))
        .unwrap_or(0);
    u64::try_from(hundredths).unwrap_or(0)
}
