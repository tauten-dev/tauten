#![no_std]
//! A pure decision kernel: exact, total, no IO.

/// Why a withdrawal was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Not enough balance.
    Insufficient,
    /// The account is frozen.
    Frozen,
}

/// Account state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Normal.
    Open,
    /// No withdrawals.
    Frozen,
}

/// Decide a withdrawal; returns the new balance.
///
/// # Errors
/// Returns a [`Refusal`] naming why the withdrawal is not allowed.
pub fn withdraw(state: State, balance: u64, amount: u64) -> Result<u64, Refusal> {
    match state {
        State::Frozen => Err(Refusal::Frozen),
        State::Open => balance.checked_sub(amount).ok_or(Refusal::Insufficient),
    }
}
