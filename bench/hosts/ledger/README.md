# ledger

A tiny command-line ledger.

```console
$ ledger book.txt
balance: 70
```

Each line of the input file is one entry:

```text
# comments and blank lines are ignored
deposit 100
withdraw 30
```

Amounts are whole, non-negative units. Withdrawing more than the balance is
an error. Errors print `error: line N: <reason>` to stderr and exit with
status 1.

Crates:

- `ledger-kernel`: the balance rules (pure);
- `ledger-core`: parsing and the report (no direct IO; reads from any `Read`);
- `ledger-cli`: the `ledger` binary.

Run the tests with `cargo test`.
