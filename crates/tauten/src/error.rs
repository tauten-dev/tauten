//! Error types produced by refinements and contracts.

use core::fmt;

/// A value did not satisfy a refinement predicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefineError {
    type_name: &'static str,
    predicate: &'static str,
}

impl RefineError {
    /// Used by `#[refined]`; not normally called directly.
    #[must_use]
    pub const fn new(type_name: &'static str, predicate: &'static str) -> Self {
        Self {
            type_name,
            predicate,
        }
    }

    /// The refinement type's name.
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// The predicate, as written.
    #[must_use]
    pub const fn predicate(&self) -> &'static str {
        self.predicate
    }
}

impl fmt::Display for RefineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "value does not satisfy {}: {}",
            self.type_name, self.predicate
        )
    }
}

impl core::error::Error for RefineError {}

/// A contract was violated. Convert it into your function's error type with
/// `impl From<ContractViolation> for MyError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractViolation {
    /// A precondition was false; the body did not run.
    Requires {
        /// Function name.
        function: &'static str,
        /// The condition, as written.
        condition: &'static str,
    },
    /// A postcondition was false for the value the body returned.
    Ensures {
        /// Function name.
        function: &'static str,
        /// The condition, as written.
        condition: &'static str,
    },
}

impl ContractViolation {
    /// Used by `#[requires]`.
    #[must_use]
    pub const fn requires(function: &'static str, condition: &'static str) -> Self {
        Self::Requires {
            function,
            condition,
        }
    }

    /// Used by `#[ensures]`.
    #[must_use]
    pub const fn ensures(function: &'static str, condition: &'static str) -> Self {
        Self::Ensures {
            function,
            condition,
        }
    }
}

impl fmt::Display for ContractViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Requires {
                function,
                condition,
            } => {
                write!(f, "precondition of `{function}` violated: {condition}")
            }
            Self::Ensures {
                function,
                condition,
            } => {
                write!(f, "postcondition of `{function}` violated: {condition}")
            }
        }
    }
}

impl core::error::Error for ContractViolation {}
