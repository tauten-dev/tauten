//! Hidden acceptance oracle for H02-statement. Black-box: runs the `ledger` binary.
#[cfg(test)]
mod oracle {
    include!("support/oracle_support.rs");

    fn statement(input: &str, stdout: &str) {
        expect_ok_with(&["--statement"], input, stdout);
    }

    #[test]
    fn base__ascii_rows() {
        // prompt ¶2 (memo after the first `;`), ¶3 (columns), SPEC Input (comments are not entries).
        statement(
            "deposit 100 ; salary\n# note\nwithdraw 30;rent\n",
            "    1 deposit                   100                  100 salary\n    3 withdraw                   30                   70 rent\nbalance: 70\n",
        );
    }

    #[test]
    fn base__memo_ignored_without_statement() {
        // prompt ¶2: without --statement the memo is accepted and ignored.
        expect_ok("deposit 100 ; salary\nwithdraw 30 ;rent\n", "balance: 70\n");
    }

    #[test]
    fn base__memo_may_contain_semicolons() {
        // prompt ¶2: the memo is the text after the FIRST `;`.
        statement("deposit 5 ; a;b\n", "    1 deposit                     5                    5 a;b\nbalance: 5\n");
    }

    #[test]
    fn unicode__sixteen_wide_characters_are_not_cut() {
        // prompt ¶4 + SPEC G3: 16 characters (48 bytes) is not longer than 16.
        statement("deposit 1 ;\u{4e00}\u{4e8c}\u{4e09}\u{56db}\u{4e94}\u{516d}\u{4e03}\u{516b}\u{4e5d}\u{5341}\u{767e}\u{5343}\u{4e07}\u{4ebf}\u{5146}\u{4eac}\n", "    1 deposit                     1                    1 \u{4e00}\u{4e8c}\u{4e09}\u{56db}\u{4e94}\u{516d}\u{4e03}\u{516b}\u{4e5d}\u{5341}\u{767e}\u{5343}\u{4e07}\u{4ebf}\u{5146}\u{4eac}\nbalance: 1\n");
        statement("deposit 1 ;abcdefghijklmnop\n", "    1 deposit                     1                    1 abcdefghijklmnop\nbalance: 1\n");
    }

    #[test]
    fn unicode__long_memos_are_cut_by_characters() {
        // prompt ¶4 + SPEC G3: first 15 scalar values, then U+2026.
        statement("deposit 1 ;\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}\n", "    1 deposit                     1                    1 \u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{2026}\nbalance: 1\n");
        statement("deposit 1 ;abcdefghijklmnopq\n", "    1 deposit                     1                    1 abcdefghijklmno\u{2026}\nbalance: 1\n");
        // Combining marks are scalar values of their own (SPEC G3).
        statement("deposit 1 ;e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}\n", "    1 deposit                     1                    1 e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}e\u{2026}\nbalance: 1\n");
    }

    #[test]
    fn input__empty_and_blank_memos() {
        // prompt ¶3 item 5: no memo column and no trailing space when the memo is empty.
        statement("deposit 5 ;\nwithdraw 2 ;   \n", "    1 deposit                     5                    5\n    2 withdraw                    2                    3\nbalance: 3\n");
    }

    #[test]
    fn input__widest_amount_fits_its_column() {
        // prompt ¶3 items 3-4: 20-character columns; SPEC G1 largest amount.
        statement("deposit 18446744073709551615\nwithdraw 18446744073709551615\n", "    1 deposit  18446744073709551615 18446744073709551615\n    2 withdraw 18446744073709551615                    0\nbalance: 0\n");
    }

    #[test]
    fn sequence__an_error_prints_no_rows() {
        // SPEC Errors: the first invalid line stops processing and nothing is printed on standard output.
        let out = case(&["--statement", "input.txt"]).file("input.txt", "deposit 5\nwithdraw 9\n").run();
        out.fails(1, "error: line 2: insufficient funds\n");
        assert_eq!(out.stdout, "", "SPEC Errors: no standard output on failure");
    }

    #[test]
    fn input__unknown_option_is_a_usage_error() {
        // prompt ¶5.
        case(&["--bogus", "input.txt"]).file("input.txt", "deposit 1\n").run().fails(2, "usage: ledger [--statement] <file>\n");
        case(&["--statement"]).run().fails(2, "usage: ledger [--statement] <file>\n");
    }

    #[test]
    fn ioerr__statement_to_a_full_device() {
        // SPEC G5: standard output cannot be written.
        case(&["--statement", "input.txt"])
            .file("input.txt", "deposit 5 ; memo\nwithdraw 2\n")
            .stdout_to_dev_full()
            .run()
            .fails(1, "error: cannot write output\n");
    }
}
