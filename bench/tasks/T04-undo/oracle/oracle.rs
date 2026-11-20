//! Hidden acceptance oracle for T04-undo.
#[cfg(test)]
mod oracle {
    include!("support/oracle_support.rs");

    #[test]
    fn undo_reverts_the_last_entry() {
        expect_ok("deposit 100\nwithdraw 30\nundo\n", "balance: 100\n");
        expect_ok("deposit 100\nundo\ndeposit 5\n", "balance: 5\n");
    }

    #[test]
    fn repeated_undo_walks_back() {
        expect_ok("deposit 1\ndeposit 2\ndeposit 4\nundo\nundo\n", "balance: 1\n");
        expect_ok("deposit 1\ndeposit 2\nundo\nundo\n", "balance: 0\n");
    }

    #[test]
    fn comments_are_not_entries() {
        expect_ok("deposit 7\n# note\n\nundo\n", "balance: 0\n");
    }

    #[test]
    fn nothing_to_undo_is_an_error() {
        expect_err("undo\n", "error: line 1: nothing to undo\n");
        expect_err("# start\ndeposit 1\nundo\nundo\n", "error: line 4: nothing to undo\n");
    }

    #[test]
    fn undo_with_arguments_is_an_error() {
        expect_err("deposit 1\nundo 1\n", "error: line 2: unexpected text after the amount\n");
    }

    #[test]
    fn rejected_entries_are_still_errors() {
        expect_err("deposit 1\nundo\nwithdraw 1\n", "error: line 3: insufficient funds\n");
    }

    #[test]
    fn undone_entries_are_skipped() {
        // prompt: undo reverts the most recent entry not already undone.
        expect_ok("deposit 1\ndeposit 2\nundo\ndeposit 4\nundo\nundo\n", "balance: 0\n");
    }

    #[test]
    fn undo_then_overflow() {
        // SPEC G1: after the undo the balance is the largest amount again.
        expect_err("deposit 18446744073709551615\nwithdraw 18446744073709551615\nundo\ndeposit 1\n", "error: line 4: balance overflow\n");
    }
}
