Add an `undo` entry to the ledger. `undo` (with no amount) reverts the most
recent entry that has not already been undone, restoring the balance from
before it. Several `undo` lines in a row revert several entries, newest
first. Comments and blank lines are not entries.

`undo` when there is nothing left to undo is an error reported like the
other line errors, with the reason `nothing to undo`:

    error: line 1: nothing to undo

`undo` followed by anything else on the line is the existing
`unexpected text after the amount` error. Add tests.
