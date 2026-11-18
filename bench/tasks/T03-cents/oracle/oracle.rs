//! Hidden acceptance oracle for T03-cents.
#[cfg(test)]
mod oracle {
    include!("support/oracle_support.rs");

    #[test]
    fn formats_with_two_decimals() {
        expect_ok("deposit 12\n", "balance: 12.00\n");
        expect_ok("deposit 12.5\n", "balance: 12.50\n");
        expect_ok("deposit 12.34\nwithdraw 0.34\n", "balance: 12.00\n");
        expect_ok("", "balance: 0.00\n");
    }

    #[test]
    fn exact_arithmetic() {
        expect_ok("deposit 0.10\ndeposit 0.20\n", "balance: 0.30\n");
        expect_ok("deposit 0.1\nwithdraw 0.07\n", "balance: 0.03\n");
        let many = "deposit 0.01\n".repeat(1000);
        expect_ok(&many, "balance: 10.00\n");
    }

    #[test]
    fn rejects_malformed_amounts() {
        for bad in ["12.", ".5", "12.345", "1e3", "-1", "1.2.3", "1,5", " ", "0x10"] {
            let input = format!("deposit {bad}\n");
            let (code, _, err) = run_ledger(&[], &input);
            assert_eq!(code, 1, "{bad:?} should be rejected");
            assert!(err.starts_with("error: line 1: bad amount"), "{bad:?}: {err}");
        }
    }

    #[test]
    fn overdraft_and_overflow_are_errors() {
        expect_err("deposit 1.00\nwithdraw 1.01\n", "error: line 2: insufficient funds\n");
        let (code, _, err) = run_ledger(&[], "deposit 184467440737095516.15\ndeposit 0.01\n");
        assert_eq!(code, 1, "{err}");
        let (code, _, _) = run_ledger(&[], "deposit 99999999999999999999.99\n");
        assert_eq!(code, 1, "an amount too large to represent must be an error, not a panic or wrap");
    }

    #[test]
    fn largest_representable_balance_prints_exactly() {
        expect_ok("deposit 184467440737095516.15\n", "balance: 184467440737095516.15\n");
    }
}
