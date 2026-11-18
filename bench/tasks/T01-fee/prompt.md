Add a new ledger entry kind: `fee <percent>`.

A fee removes `percent`% of the current balance, rounded down to a whole
unit. For example, with a balance of 250, `fee 10` removes 25 (balance 225),
and with a balance of 9, `fee 50` removes 4 (balance 5).

`percent` is a whole number from 0 to 100. Anything else on a `fee` line is
an error reported like the other line errors, with the reason
`bad percent `<text>``, for example:

    error: line 3: bad percent `101`

Fees must work for every balance the ledger can hold. Keep the existing
behaviour of all other entries unchanged, and add tests for the new entry.
