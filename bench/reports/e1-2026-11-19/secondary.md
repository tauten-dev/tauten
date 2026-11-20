Secondary (escaped trials):
- r2 Sonnet K02-lists #0: oracle input__leading_zeros_and_minus_zero fails on `get app.conf xs[-0]` (prints "index out of range"). Failing logic: kvconf-core/src/lib.rs:282 (`negative && magnitude == 0` branch). Findings in the repo: P003 x2 at :264, E001 at :279; none on the failing path.
- r3 Sonnet H15-ratelimit #1: 0 deny-level findings in the final repo; failed base/input/origin/overflow/rounding/sequence classes. No finding on the failing path.
