//! Hidden acceptance oracle for K01-discount. Black-box: runs the `ledger` binary.
#[cfg(test)]
mod oracle {
    include!("support/oracle_support.rs");

    #[test]
    fn base__no_discount_up_to_1000() {
        // prompt ¶2: the discount applies only above 1000.
        expect_ok("deposit 5000\npurchase 1000\n", "balance: 4000\n");
        expect_ok("deposit 5000\npurchase 7\n", "balance: 4993\n");
    }

    #[test]
    fn base__first_tier() {
        // prompt ¶2: 5% of 2000 is 100, so 2900 is withdrawn.
        expect_ok("deposit 5000\npurchase 3000\n", "balance: 2100\n");
    }

    #[test]
    fn base__both_tiers() {
        // prompt ¶2: 5% of 9000 (450) plus 10% of 10000 (1000).
        expect_ok("deposit 100000\npurchase 20000\n", "balance: 81450\n");
    }

    #[test]
    fn base__insufficient_funds_like_withdraw() {
        // prompt ¶3: same errors as withdraw.
        expect_err("deposit 10\npurchase 20\n", "error: line 2: insufficient funds\n");
        expect_err("deposit 2000\npurchase 2100\n", "error: line 2: insufficient funds\n");
    }

    #[test]
    fn rounding__half_rounds_up() {
        // prompt ¶2: 0.5 rounds up to 1; 0.45 rounds to 0; 1.5 rounds to 2.
        expect_ok("deposit 2000\npurchase 1010\n", "balance: 991\n");
        expect_ok("deposit 2000\npurchase 1009\n", "balance: 991\n");
        expect_ok("deposit 2000\npurchase 1030\n", "balance: 972\n");
    }

    #[test]
    fn overflow__largest_purchase() {
        // SPEC G1: no entry fails because an intermediate is large; prompt ¶2 discount.
        expect_ok("deposit 18446744073709551615\npurchase 18446744073709551615\n", "balance: 1844674407370954612\n");
        expect_ok("deposit 18446744073709551615\npurchase 18446744073709551614\n", "balance: 1844674407370954612\n");
    }

    #[test]
    fn input__amount_syntax() {
        // SPEC G2: digits only; Errors table: bad amount.
        expect_err("deposit 10\npurchase +5\n", "error: line 2: bad amount `+5`\n");
        expect_err("deposit 10\npurchase 1.5\n", "error: line 2: bad amount `1.5`\n");
        expect_err("deposit 10\npurchase\n", "error: line 2: bad amount ``\n");
        // SPEC Errors table: unexpected text after the amount.
        expect_err("deposit 10\npurchase 5 now\n", "error: line 2: unexpected text after the amount\n");
    }

    #[test]
    fn base__other_entries_unchanged() {
        // prompt ¶3: other entries unchanged.
        expect_ok("deposit 100\nwithdraw 30\n", "balance: 70\n");
    }

    #[test]
    fn base__tier_boundary() {
        // prompt ¶2: up to 10000 only the 5% tier (450); 10% applies to the part above 10000 (10 -> 1).
        expect_ok("deposit 100000\npurchase 10000\n", "balance: 90450\n");
        expect_ok("deposit 100000\npurchase 10010\n", "balance: 90441\n");
    }
}
