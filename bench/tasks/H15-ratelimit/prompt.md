Add a start rate limit: a new event `limit <n> per <ms> burst <b>`. From
that event on, starting a job uses one token. The limit starts with `b`
tokens. Tokens refill continuously and exactly at `n` tokens per `<ms>`
milliseconds, up to at most `b` tokens. A later `limit` event replaces the
earlier one and starts again with `b` tokens. Before the first `limit`
event, starts are not limited.

A start needs one whole token. Without one, `start` fails with the reason
`rate limited` (a state error) and `next` does nothing.

Tokens refill in proportion to the time the clock moves forward through
`tick` and `at`; moving it backwards adds nothing. Splitting a wait into
smaller ticks never changes the outcome.

`per` and `burst` must be at least 1 (`n` may be 0). A `limit` line of any
other shape, or with a bad number, is the syntax error `bad limit`.

Add tests.
