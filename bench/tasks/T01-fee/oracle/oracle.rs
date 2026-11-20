//! Hidden acceptance oracle for T01-fee. Black-box: runs the `ledger` binary.
#[cfg(test)]
mod oracle {
    include!("support/oracle_support.rs");

    #[test]
    fn basic_fee() {
        expect_ok("deposit 250\nfee 10\n", "balance: 225\n");
        expect_ok("deposit 9\nfee 50\n", "balance: 5\n");
    }

    #[test]
    fn edge_percentages() {
        expect_ok("deposit 77\nfee 0\n", "balance: 77\n");
        expect_ok("deposit 77\nfee 100\n", "balance: 0\n");
        expect_ok("fee 30\n", "balance: 0\n");
    }

    #[test]
    fn huge_balance_does_not_overflow() {
        // 18446744073709551615 * 50 overflows u64; the fee itself does not.
        expect_ok("deposit 18446744073709551615\nfee 50\n", "balance: 9223372036854775808\n");
        expect_ok("deposit 18446744073709551615\nfee 99\n", "balance: 184467440737095517\n");
    }

    #[test]
    fn invalid_percent_is_a_line_error() {
        expect_err("deposit 10\n\nfee 101\n", "error: line 3: bad percent `101`\n");
        expect_err("fee -1\n", "error: line 1: bad percent `-1`\n");
        expect_err("fee 1.5\n", "error: line 1: bad percent `1.5`\n");
    }

    #[test]
    fn other_entries_unchanged() {
        expect_ok("deposit 100\nwithdraw 30\n", "balance: 70\n");
        expect_err("deposit 1\nwithdraw 2\n", "error: line 2: insufficient funds\n");
    }

    #[test]
    fn huge_balance_odd_percent() {
        // M × 33 / 100 = 6087425544324152032.95, rounded down.
        expect_ok("deposit 18446744073709551615\nfee 33\n", "balance: 12359318529385399583\n");
    }

    #[test]
    fn percent_syntax() {
        // SPEC G2: leading zeros are allowed.
        expect_ok("deposit 100\nfee 007\n", "balance: 93\n");
        // prompt: more words after the percent.
        expect_err("deposit 100\nfee 5 x\n", "error: line 2: unexpected text after the amount\n");
    }
}
