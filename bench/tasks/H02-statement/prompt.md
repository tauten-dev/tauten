Add a `--statement` option to the `ledger` command:

    ledger --statement <file>

Entries may now carry a memo: the text after the first `;` on an entry line,
with surrounding whitespace removed. The memo is free text. Without
`--statement` it is accepted and ignored.

With `--statement`, the output starts with one row per entry, in file order,
followed by the usual balance line. A row has these columns, separated by one
space:

1. the line number, right-aligned in 5 characters;
2. the entry kind, left-aligned in 8 characters;
3. the amount, right-aligned in 20 characters;
4. the balance after the entry, right-aligned in 20 characters;
5. the memo, only when it is not empty (otherwise the row ends after the
   balance, with no trailing space).

A memo longer than 16 characters is shown as its first 15 characters
followed by `…` (U+2026); shorter memos are shown whole.

Any other option, or a wrong number of arguments, prints
`usage: ledger [--statement] <file>` on standard error and exits with
status 2. Add tests.
