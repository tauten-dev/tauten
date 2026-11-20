# Agent instructions

This workspace follows the tauten profile. Read `AGENTS.tauten.md` first:
its "architecture on one page" says where each kind of code goes.

## Where things are

| Path | Tier | Holds |
|---|---|---|
| `crates/{{KERNEL}}` | kernel | Decisions: total functions, each with a proof claim |
| `crates/{{CORE}}` | core | Logic; the world arrives as capabilities (`Caps`); `run(caps, args)` |
| `crates/{{NAME}}` | edge | `main`: acquires authority, builds `Caps`, calls `run` |
| `proofs/` | | Lean theorems over the kernel (`Proofs/`), `claims.toml`, the gate |

## Before you report a task as done

1. `cargo test` passes.
2. `cargo tauten check` reports 0 errors.
3. If you changed `crates/{{KERNEL}}` or `proofs/`: `cargo fmt`, then
   `sh proofs/refresh.sh` (regenerates `proofs/Extracted/`), commit, and
   `cargo tauten proofs`: every claim must be `proved`. A new kernel
   function gets a theorem in `proofs/Proofs/`, a `[[claim]]` and at least
   one `[[mutant]]` in `proofs/claims.toml`. Never delete a mutant or
   weaken a theorem to pass; never edit `proofs/claims.json` by hand.
