The ledger must support amounts with cents.

An amount is now a whole number optionally followed by `.` and one or two
digits: `12`, `12.5` and `12.34` are valid; `12.`, `.5`, `12.345`, `1e3`
and `-1` are not, and are reported with the existing `bad amount` error.

The balance is always printed with two decimals:

    balance: 12.50

Calculations must be exact (for example, depositing `0.10` and then `0.20`
gives exactly `0.30`). Existing rules are unchanged: overdrafts and overflow
are errors. Update the existing tests to the new output format and add tests
for the new behaviour.
