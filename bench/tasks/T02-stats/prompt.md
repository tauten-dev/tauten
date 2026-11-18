Add a `--stats` option to the `ledger` command:

    ledger --stats book.txt

It prints the usual balance line, followed by a line with the largest single
withdrawal in the file:

    balance: 70
    largest withdrawal: 30

If the file has no withdrawals, the second line is
`largest withdrawal: none`. Without `--stats` the output is unchanged.
Errors in the file are reported exactly as before. An unknown option such as
`--bogus` prints `usage: ledger [--stats] <file>` to stderr and exits with
status 2. Add tests.
