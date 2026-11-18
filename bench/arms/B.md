# Agent instructions

Run `cargo test` before you report the task as done.

## Coding rules

- `ledger-kernel` is pure: no IO, no floats, no panics; dependencies point
  inward (cli → core → kernel).
- `ledger-core` does no direct IO (no files, environment, clock, printing);
  it receives what it needs as parameters.
- Never use `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!` or
  indexing that can panic (`v[i]`, `&s[a..b]`) outside tests. Propagate
  errors with `?` and typed error enums.
- Use checked arithmetic (`checked_add`, `checked_mul`, …) and `try_from`
  instead of `as`.
- Match every enum variant explicitly; no `_ =>` over enums you own.
- Never ignore a `Result`.
- Never delete, weaken or `#[ignore]` tests to make a task pass.
