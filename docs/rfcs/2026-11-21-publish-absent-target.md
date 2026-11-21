---
rfc: "2026-11-21-publish-absent-target"
title: "Atomic publish to an absent target: companion theorems"
status: accepted
implementation: in-progress
authors:
  - astra
created: 2026-11-21
updated: 2026-11-21
issue: https://github.com/tauten-dev/tauten/issues/58
milestone: "0.1.0"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Atomic publish to an absent target: companion theorems

## Summary

The A7 publish theorems (`Tauten.Durable`) cover replacing an existing file:
`init0` requires the target and its durable copy to hold `old`, and `Inv`
asserts the target is present. Publishing a name that does not exist yet has
no theorem. This RFC adds **companion theorems** for the absent start and
leaves every published theorem, claim and name in 0.0.5 byte-stable. It does
not generalise `init0`/`Inv` over the start state.

## Motivation

#58: an adopter planning a first-write path found that the advertised
guarantee says nothing about it. This is a coverage gap, not an unsoundness;
the A9 publish driver (#13) needs both starts before it can advertise one
publish function.

## Behaviour

New claims in `proofs/claims.toml` (tauten's own) and the exported
`crates/tauten/claims.json`; `docs/STATUS.md` states exactly which starts are
covered. No existing claim, theorem name or assumption changes.

## Proof obligations

- **Kernel:** the existing extracted publish step functions; no new kernel
  function.
- **Theorems** (over the extracted step and the existing filesystem model, no
  new axioms):
  - `init0_absent` / `InvAbsent`: the start and invariant with the target and
    its durable copy absent.
  - `publish_absent_or_new`: in every reachable configuration and after any
    modelled crash, the target is absent or holds `new`, under
    `FS.rename-atomic` and `FS.sync-file`.
  - `publish_absent_durable_on_done`: on `Done`, `new` is durable, adding
    `FS.sync-dir`.
  - `posix_absent_done_reachable`: non-vacuity on `posix`.
  - Necessity: `absent_durable_needs_sync_dir` (without it a crash can lose
    the new name), and the rename-atomic and sync-file necessity results
    re-established for the absent start.
  - Failure and partial-temp coverage as for replacement.
  - Optional, same change: the positive create-new result for
    `create_step` from an absent target.
- **Mutants:** the existing step mutants must break the new theorems as they
  break the old; add one that skips the directory sync on first creation.
- **Tested and unproved:** unchanged (the real filesystem binding).

## Design

Companion statements reuse the step, `exec`, `Crash` and profile definitions;
shared lemmas may be factored out, but no existing statement changes shape.

## Compatibility and reversibility

Additive. Generalising `init0`/`Inv` (the rejected alternative) would be the
more uniform statement but changes published theorem statements and the
claims a 0.0.5 adopter inherits.

## Alternatives

Generalise over the start state, making replacement an instance: cleaner,
rejected for now because it breaks published claims; can be revisited at a
major boundary.

## Evidence and tests

Full proofs gate (drift, build, axiom audit, mutants), claims export check.

## Rollout

0.1.0, before #13 (A9 publish driver).

## Unresolved questions

None.

## Decision log

- 2026-11-21: triage put the choice to the owner; companion theorems
  recommended to keep 0.0.5 claims byte-stable.
- 2026-11-21: accepted by the owner ("you have my blessing … file the rfcs
  and mark them as accepted").

## Implementation notes (2026-11-21)

- `publish_absent_or_new` is stated under `FS.sync-file` alone, not under
  `FS.rename-atomic` as well. In the model, a non-atomic rename can only
  unlink the target, and for a fresh name that is the "absent" a reader
  may already see. The rename-atomic necessity therefore cannot be
  re-established for absent-or-new. It holds for durability instead:
  `absent_durable_needs_rename_atomic` reaches `Done` with the target
  missing.
- A function can carry only one claim, so the companion theorems, their
  necessity theorems and the witness join the existing `step` and `durable`
  claims. The claims grow; no theorem, name or assumption is removed or
  changed, and their `assumes` stay the same.
- The directory-sync mutant (`skip-dir-sync`) changes the shared step
  function, so it breaks the replacement and absent-start theorems alike.
