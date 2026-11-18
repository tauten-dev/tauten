//! Hidden acceptance oracle for T02-stats.
#[cfg(test)]
mod oracle {
    include!("support/oracle_support.rs");

    #[test]
    fn stats_line() {
        expect_ok_with(&["--stats"], "deposit 100\nwithdraw 30\nwithdraw 5\n", "balance: 65\nlargest withdrawal: 30\n");
        expect_ok_with(&["--stats"], "deposit 9\nwithdraw 4\nwithdraw 4\n", "balance: 1\nlargest withdrawal: 4\n");
    }

    #[test]
    fn no_withdrawals() {
        expect_ok_with(&["--stats"], "deposit 5\n", "balance: 5\nlargest withdrawal: none\n");
        expect_ok_with(&["--stats"], "", "balance: 0\nlargest withdrawal: none\n");
        expect_ok_with(&["--stats"], "# only a comment\n\n", "balance: 0\nlargest withdrawal: none\n");
    }

    #[test]
    fn zero_withdrawal_counts() {
        expect_ok_with(&["--stats"], "deposit 5\nwithdraw 0\n", "balance: 5\nlargest withdrawal: 0\n");
    }

    #[test]
    fn without_stats_unchanged() {
        expect_ok("deposit 100\nwithdraw 30\n", "balance: 70\n");
        expect_err_with(&["--stats"], "deposit 1\nwithdraw 2\n", "error: line 2: insufficient funds\n");
    }

    #[test]
    fn unknown_option_is_usage_error() {
        let (code, _, err) = run_ledger(&["--bogus"], "deposit 1\n");
        assert_eq!((code, err.as_str()), (2, "usage: ledger [--stats] <file>\n"));
    }
}
