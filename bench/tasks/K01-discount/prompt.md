Add a `purchase <amount>` entry to the ledger.

A purchase withdraws its amount less a volume discount. The discount is 5% of
the part of the amount above 1000 and up to 10000, plus 10% of the part above
10000. It is rounded to the nearest whole unit; a half rounds up.

The discounted amount is then withdrawn exactly like a `withdraw` entry,
with the same errors. Keep the behaviour of all other entries unchanged and
add tests for the new entry.
