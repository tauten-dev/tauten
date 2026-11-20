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

    #[tauten::refined(|s: &String| !s.is_empty())]
    #[derive(Clone, Debug, PartialEq)]
    struct Label(String);

    #[tauten::refined(|v: &Vec<(u8, &'static str)>| v.len() <= 2)]
    struct Short(Vec<(u8, &'static str)>);

    #[test]
    fn frozen_inner_types_are_accepted() {
        fn inner<T: tauten::refine::Refined>(_: &T) {}
        let name = Label::try_new("a".to_owned()).expect("non-empty");
        inner(&name);
        assert_eq!(name.clone(), name);
        assert!(Label::try_new(String::new()).is_err());
        assert!(Short::try_new(vec![(1, "a"), (2, "b")]).is_ok());
        assert!(Short::try_new(vec![(1, "a"), (2, "b"), (3, "c")]).is_err());
    }

    #[test]
    fn compile_time_guarantees() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/ui/*.rs");
    }
}

/// A5: the three carriers of a check, by how far the fact travels.
#[cfg(test)]
mod carriers {
    use tauten::origin::{Permit, Stale, Stamped, Versioned};
    use tauten::{Bindings, evidence};

    pub struct Queue {
        jobs: Vec<u32>,
        version: u64,
    }

    impl Versioned for Queue {
        fn version(&self) -> u64 {
            self.version
        }
    }

    impl Queue {
        fn push(&mut self, job: u32) {
            self.jobs.push(job);
            self.version = self.version.wrapping_add(1);
        }

        fn check_mut(&mut self, at: usize) -> Option<Permit<'_, Queued>> {
            (at < self.jobs.len()).then(|| Queued::mint_exclusive(self, at))
        }

        fn check_later(&self, at: usize) -> Option<Stamped<Queued>> {
            (at < self.jobs.len()).then(|| Queued::mint_stamped(self, at))
        }
    }

    /// Position `at` holds a job.
    #[evidence(subject = Queue, exclusive, stamped)]
    pub struct Queued {
        at: usize,
    }

    #[test]
    fn a_permit_checks_and_acts_in_one_chain() {
        let mut queue = Queue {
            jobs: vec![4, 5],
            version: 0,
        };
        let taken = queue
            .check_mut(1)
            .map(|permit| permit.act(|q, fact| q.jobs.remove(*fact.at())));
        assert_eq!(taken, Some(5));
        assert!(queue.check_mut(3).is_none());
    }

    #[test]
    fn a_stamped_fact_is_refused_once_the_subject_moved() {
        let mut queue = Queue {
            jobs: vec![4],
            version: 0,
        };
        let stamp = queue.check_later(0).expect("checked");
        assert_eq!(stamp.version(), 0);
        let fresh = queue.check_later(0).expect("checked");
        assert_eq!(fresh.verify(&queue).map(|ev| *ev.fact().at()), Ok(0));
        queue.push(9);
        assert_eq!(stamp.verify(&queue).map(|_| ()), Err(Stale));
    }

    #[derive(Debug, Clone, PartialEq, Bindings)]
    pub struct Lock {
        name: String,
        revision: u32,
        checksum: [u8; 4],
    }

    #[test]
    fn bindings_compare_every_field() {
        let recorded = Lock {
            name: "core".into(),
            revision: 3,
            checksum: [1, 2, 3, 4],
        };
        let same = recorded.clone();
        assert!(recorded.diff(&same).is_empty());
        assert!(recorded.revalidate(&same).is_ok());
        // `checksum` was added after `diff` was first written: it is
        // compared without further edits.
        let moved = Lock {
            checksum: [1, 2, 3, 5],
            ..recorded.clone()
        };
        assert_eq!(recorded.diff(&moved), vec!["checksum"]);
        assert_eq!(recorded.revalidate(&moved).map(|_| ()), Err(Stale));
        let renamed = Lock {
            name: "edge".into(),
            revision: 4,
            ..recorded.clone()
        };
        assert_eq!(recorded.diff(&renamed), vec!["name", "revision"]);
    }
}
