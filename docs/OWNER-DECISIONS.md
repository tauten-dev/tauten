# Decisions and assets needed from the owner

Updated 2026-11-19 (morning). Settled items are kept at the bottom for the record.

## Open

| # | Decision | What it unblocks | Recommendation |
|---|---|---|---|
| 1 | **Writfold adoption branch** (`tauten-adoption` in `~/workspace/writfold-tauten`, not pushed). | Merging into writfold | Review `docs/writfold/tauten-adoption.md` there: tiers, the xtask `#[expect]` fix (needs a quality gate run on main), the `#![expect]` style, and the proof claims (the choices are settled below). |
| 2 | **Publish the held-out suite only after it runs.** The branch stays local so the hidden oracles are not public before the run. | Honest held-out results | Merge and publish it together with the results. |
| 3 | **Revoke the one-off crates.io token** used for the kernel crate's first publish (the local copy is deleted). | Hygiene | Revoke it at crates.io → Account settings → API tokens. |

## Settled

| Item | Decision |
|---|---|
| Name | tauten (2026-11-18) |
| License | MIT (2026-11-18) |
| Domain, org | tauten.dev at Porkbun; GitHub org tauten-dev (2026-11-18) |
| Crate publishing | 0.0.1 with a token, 0.0.2 with trusted publishing; "require trusted publishing" on; the original token revoked (2026-11-18) |
| Repo visibility | tauten-dev/tauten public (2026-11-18) |
| Pilot budget | 36 trials on one model; run 2026-11-18 for $6.29 |
| Proof toolchain image | ghcr.io/tauten-dev/proofs-toolchain, private, readable by the tauten repo's Actions (2026-11-18) |
| Proof tier | Ship, then prove one slice at a time; proofs never block unrelated work (2026-11-18) |
| Held-out benchmark | Approved 2026-11-19: calibration, then Sonnet 5.5 primary and Haiku 5.5 replication (about $400) |
| Container credentials | Long-lived `claude setup-token` token, stored outside the repo (2026-11-19) |
| `tauten-check-kernel` first publish | 0.0.2 with a one-off token, then trusted publishing and "require trusted publishing" via the crates.io API (2026-11-19); the token is to be revoked |
| Writfold proof claims | Accept the added mutants; rewrite `newer`, `better`, `char_step`, `path_step` to be total; accept a full claims re-run after kernel edits for now (2026-11-19) |
| Sovel repo | Leave as is for now; may be made private later (2026-11-18) |
| Writfold adoption | In a separate worktree; the owner merges (2026-11-18) |
