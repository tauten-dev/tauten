# Decisions and assets needed from the owner

Updated 2026-11-19 (morning). Settled items are kept at the bottom for the record.

## Open

| # | Decision | What it unblocks | Recommendation |
|---|---|---|---|
| 1 | **Writfold adoption branch** (`tauten-adoption` in `~/workspace/writfold-tauten`, not pushed). | Merging into writfold | Review `docs/writfold/tauten-adoption.md` there: tiers, the xtask `#[expect]` fix (needs a quality gate run on main), the `#![expect]` style, the proof claims, and the total rewrites of `newer`, `better`, `char_step`, `path_step` (commit `b61532c`; out-of-precondition results are documented there). |
| 2 | **What to publish from `bench-heldout`.** The held-out run was stopped by its go/no-go rule (calibration: 2 escaped in 200 trials) and E1 found 0/120 findings to be defects. The branch also holds 16 unrun held-out tasks with hidden oracles. | Publishing the calibration rounds and E1, as preregistered | Publish the harness, the 7 calibration tasks, the results and the E1 report; keep the 16 unrun tasks private so they stay usable. |
| 3 | **Maintenance chains (E2).** Tests whether the bounds E1 relied on survive later edits: 5-8 fresh sessions each adding a feature to one repo, union oracle at the end, arms A vs C. | The only remaining defect-reduction question | Decide whether it is worth a budget; not started. |
| 4 | **Revoke the one-off crates.io token** used for the kernel crate's first publish (the local copy is deleted). | Hygiene | Revoke it at crates.io → Account settings → API tokens. |

## Settled

| Item | Decision |
|---|---|
| Repositioning | Code discipline + proofs; defect reduction only as a side effect (2026-11-19) |
| Discipline plan | `docs/DISCIPLINE-PROPOSAL.md` approved: D0, then X1-X3, then D1-D8 (2026-11-19) |
| Verus / TLAPS | No adapters for now; claims format reserves the tool names (2026-11-19) |
| Proofs image | Public on ghcr once the shared gate runs writfold and gripsack unchanged (2026-11-19) |
| Product shape | Fast checks on stable; deep checks in the pinned image at ship/nightly/release; "effects visible in signatures" (2026-11-19) |
| Name | tauten (2026-11-18) |
| License | MIT (2026-11-18) |
| Domain, org | tauten.dev at Porkbun; GitHub org tauten-dev (2026-11-18) |
| Crate publishing | 0.0.1 with a token, 0.0.2 with trusted publishing; "require trusted publishing" on; the original token revoked (2026-11-18) |
| Repo visibility | tauten-dev/tauten public (2026-11-18) |
| Pilot budget | 36 trials on one model; run 2026-11-18 for $6.29 |
| Proof toolchain image | ghcr.io/tauten-dev/proofs-toolchain, private, readable by the tauten repo's Actions (2026-11-18) |
| Proof tier | Ship, then prove one slice at a time; proofs never block unrelated work (2026-11-18) |
| Held-out benchmark | Approved 2026-11-19: calibration, then Sonnet 5.5 primary and Haiku 5.5 replication (about $400). Calibration round 3 was no-go by the preregistered rule; the held-out run was not started and its budget not spent |
| Container credentials | Long-lived `claude setup-token` token, stored outside the repo (2026-11-19) |
| `tauten-check-kernel` first publish | 0.0.2 with a one-off token, then trusted publishing and "require trusted publishing" via the crates.io API (2026-11-19); the token is to be revoked |
| Writfold proof claims | Accept the added mutants; rewrite `newer`, `better`, `char_step`, `path_step` to be total; accept a full claims re-run after kernel edits for now (2026-11-19) |
| Sovel repo | Leave as is for now; may be made private later (2026-11-18) |
| Writfold adoption | In a separate worktree; the owner merges (2026-11-18) |
