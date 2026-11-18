//! Functions with checked contracts. Each condition is the one its
//! theorems name.

use tauten::ContractViolation;

/// Why an operation failed. No derives: nothing here needs them, and the
/// translation stays to what the theorems are about.
pub enum OpError {
    /// The result would exceed the type's range.
    Overflow,
    /// The withdrawal exceeds the balance.
    Insufficient,
    /// A contract was violated.
    Contract(ContractViolation),
}

impl From<ContractViolation> for OpError {
    fn from(violation: ContractViolation) -> Self {
        Self::Contract(violation)
    }
}

/// Add a positive amount to a balance.
///
/// # Errors
/// `Overflow`, or a contract violation (`amount` is zero).
#[tauten::requires(amount > 0)]
#[tauten::ensures(|after: &u64| *after > old(balance))]
pub fn deposit(balance: u64, amount: u64) -> Result<u64, OpError> {
    balance.checked_add(amount).ok_or(OpError::Overflow)
}

/// Take a positive amount out of a balance in place and return the new
/// balance. The postcondition compares with the balance before the call.
///
/// # Errors
/// `Insufficient`, or a contract violation (`amount` is zero).
#[tauten::requires(amount > 0)]
#[tauten::ensures(|after: &u64| *after < old(*balance))]
pub fn withdraw(balance: &mut u64, amount: u64) -> Result<u64, OpError> {
    let next = balance.checked_sub(amount).ok_or(OpError::Insufficient)?;
    *balance = next;
    Ok(next)
}

/// Multiply by a bounded factor. The postcondition (the result is not
/// smaller than the input) is false for a zero factor and a non-zero input,
/// so all three outcomes of the wrapper are reachable.
///
/// # Errors
/// `Overflow`, or a contract violation (`factor` above 1000, or a result
/// smaller than `value`).
#[tauten::requires(factor <= 1000)]
#[tauten::ensures(|scaled: &u32| *scaled >= old(value))]
pub fn scale(value: u32, factor: u32) -> Result<u32, OpError> {
    value.checked_mul(factor).ok_or(OpError::Overflow)
}

/// Split a total into equal shares. The body divides, which panics on a
/// zero divisor, so the precondition must be checked before the body runs.
///
/// # Errors
/// A contract violation (`divisor` is zero).
#[tauten::requires(divisor != 0)]
pub fn share(total: u32, divisor: u32) -> Result<u32, OpError> {
    #[expect(
        clippy::arithmetic_side_effects,
        clippy::integer_division,
        reason = "the division that panics on zero is the point: the precondition must run first"
    )]
    let each = total / divisor;
    Ok(each)
}

#[cfg(test)]
mod tests {
    use super::{OpError, deposit, scale, share, withdraw};
    use tauten::ContractViolation;

    fn requires(result: Result<impl Sized, OpError>) -> bool {
        matches!(
            result,
            Err(OpError::Contract(ContractViolation::Requires { .. }))
        )
    }

    fn ensures(result: Result<impl Sized, OpError>) -> bool {
        matches!(
            result,
            Err(OpError::Contract(ContractViolation::Ensures { .. }))
        )
    }

    #[test]
    fn deposit_outcomes() {
        assert!(requires(deposit(5, 0)));
        assert!(matches!(deposit(5, 2), Ok(7)));
        assert!(matches!(deposit(u64::MAX, 1), Err(OpError::Overflow)));
    }

    #[test]
    fn withdraw_outcomes() {
        let mut balance = 10;
        assert!(requires(withdraw(&mut balance, 0)));
        assert_eq!(balance, 10);
        assert!(matches!(withdraw(&mut balance, 3), Ok(7)));
        assert_eq!(balance, 7);
        assert!(matches!(
            withdraw(&mut balance, 8),
            Err(OpError::Insufficient)
        ));
        assert_eq!(balance, 7);
    }

    #[test]
    fn scale_outcomes() {
        assert!(requires(scale(1, 1001)));
        assert!(ensures(scale(5, 0)));
        assert!(matches!(scale(0, 0), Ok(0)));
        assert!(matches!(scale(5, 3), Ok(15)));
        assert!(matches!(scale(u32::MAX, 2), Err(OpError::Overflow)));
    }

    #[test]
    fn share_outcomes() {
        assert!(requires(share(7, 0)));
        assert!(matches!(share(7, 2), Ok(3)));
    }
}
