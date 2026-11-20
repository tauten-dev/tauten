# ledger specification

`ledger [options] <file>` reads a ledger file and prints the final balance.

## Input

The file is UTF-8 text, read as lines (a line ends at `\n`; a `\r` before it
is ignored). Lines are numbered from 1. Leading and trailing whitespace on a
line is ignored. A line that is empty, or whose first non-space character is
`#`, is a comment: it is not an entry, but it still counts for line numbers.

Every other line is one **entry**: a kind word, then the entry's arguments,
separated by whitespace.

| Entry | Meaning |
|---|---|
| `deposit <amount>` | add the amount to the balance |
| `withdraw <amount>` | subtract the amount from the balance |

Entries apply in file order, starting from a balance of 0.

## Output

On success: one line `balance: <amount>` on standard output, status 0.

## Errors

The first invalid line stops processing. Nothing is printed on standard
output; standard error gets exactly one line `error: line N: <reason>` and
the status is 1.

| Reason | When |
|---|---|
| ``unknown entry kind `<word>` `` | the kind word is not an entry kind |
| ``bad amount `<text>` `` | the amount is missing or not a valid amount (G2) |
| `unexpected text after the amount` | more words follow the last argument |
| `insufficient funds` | a withdrawal is larger than the balance |
| `balance overflow` | a result would exceed the largest amount (G1) |

An unreadable file gives `error: cannot read input: <detail>` and status 1.
A wrong command line gives a usage line on standard error and status 2.

## Ground rules

These apply to every feature, present and future.

- **G1. Amounts.** Amounts and balances are whole numbers from 0 to
  18446744073709551615 (the largest `u64`). Every operation gives the exact
  result, or the `balance overflow` error when the result is above that
  limit. No entry fails because an intermediate value of a calculation is
  large: only the final result of an entry counts.
- **G2. Number syntax.** An amount is one or more ASCII digits `0`–`9` and
  nothing else: no sign, no spaces, no decimal point, no exponent. Leading
  zeros are allowed. The same applies to every other number in an entry
  unless its rule says otherwise.
- **G3. Text.** Input is UTF-8. Free text (for example a memo) may contain
  any Unicode. Lengths of text are counted in Unicode scalar values
  (Rust `char`s), not bytes, unless a rule says bytes.
- **G4. No crashes.** The program never crashes, panics or hangs, whatever
  the input. Every failure is reported as described under Errors.
- **G5. Output failures.** If standard output cannot be written, the program
  prints `error: cannot write output` on standard error and exits with
  status 1.
