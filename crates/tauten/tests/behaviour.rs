//! Runtime behaviour of refinements, newtypes and contracts.

#[cfg(test)]
mod behaviour {
    use tauten::{ContractViolation, ensures, newtype, refined, requires};

    #[refined(|v| *v <= 100)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Percent(u8);

    #[refined(|s: &String| !s.trim().is_empty())]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Name(String);

    #[newtype]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct UserId(u64);

    #[derive(Debug, PartialEq, Eq)]
    enum BankError {
        Contract(ContractViolation),
        Insufficient,
    }

    impl From<ContractViolation> for BankError {
        fn from(v: ContractViolation) -> Self {
            Self::Contract(v)
        }
    }

    #[requires(amount > 0)]
    #[ensures(|after: &u64| *after == old(balance) - amount)]
    fn withdraw(balance: u64, amount: u64) -> Result<u64, BankError> {
        balance.checked_sub(amount).ok_or(BankError::Insufficient)
    }

    #[ensures(|after: &u64| *after > old(*counter))]
    fn buggy_increment(counter: &mut u64) -> Result<u64, BankError> {
        // Bug the postcondition must catch: no increment.
        Ok(*counter)
    }

    #[requires(!items.is_empty())]
    fn early_return(items: &[u8]) -> Result<u8, BankError> {
        for &item in items {
            if item > 10 {
                return Ok(item);
            }
        }
        Err(BankError::Insufficient)
    }

    #[test]
    fn refinement_checks_at_construction() {
        assert_eq!(Percent::try_new(100).map(|p| *p.get()), Ok(100));
        let err = Percent::try_new(101).unwrap_err();
        assert_eq!(err.type_name(), "Percent");
        assert!(err.predicate().contains("100"));
        assert!(Percent::try_from(7).is_ok());
        assert!(Name::try_new("  ".into()).is_err());
        assert_eq!(
            Name::try_new("ada".into()).map(Name::into_inner),
            Ok("ada".to_owned())
        );
    }

    #[test]
    fn newtype_round_trips() {
        let id = UserId::new(7);
        assert_eq!(*id.get(), 7);
        assert_eq!(id.into_inner(), 7);
    }

    #[test]
    fn preconditions_reject_before_the_body() {
        assert_eq!(withdraw(10, 3), Ok(7));
        assert_eq!(
            withdraw(10, 0),
            Err(BankError::Contract(ContractViolation::Requires {
                function: "withdraw",
                condition: "amount > 0"
            }))
        );
        // The body's own error passes through untouched.
        assert_eq!(withdraw(1, 3), Err(BankError::Insufficient));
    }

    #[test]
    fn postconditions_catch_wrong_results() {
        let mut counter = 4;
        let result = buggy_increment(&mut counter);
        assert!(matches!(
            result,
            Err(BankError::Contract(ContractViolation::Ensures {
                function: "buggy_increment",
                ..
            }))
        ));
        // The message shows the condition as written, including `old(..)`.
        let Err(BankError::Contract(ContractViolation::Ensures { condition, .. })) = result else {
            return;
        };
        assert!(condition.contains("old"), "{condition}");
        assert!(!condition.contains("__tauten"), "{condition}");
    }

    #[test]
    fn return_inside_the_body_still_runs_postconditions() {
        assert_eq!(early_return(&[1, 20]), Ok(20));
        assert_eq!(early_return(&[1]), Err(BankError::Insufficient));
        assert!(matches!(early_return(&[]), Err(BankError::Contract(_))));
    }

    #[test]
    fn compile_time_guarantees() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/ui/*.rs");
    }
}
