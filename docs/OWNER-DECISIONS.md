# Decisions and assets needed from the owner

Updated 2026-11-19. Settled items are kept at the bottom for the record.

## Open

| # | Decision | What it unblocks | Recommendation |
|---|---|---|---|
| 1 | **Budget for the held-out benchmark.** Calibration: 3 tasks, arm A, ≤ 90 trials, about $16–32. Held-out: 20 tasks × 3 arms × 10 trials = 600 trials per model, about $120–240 on Sonnet 5.5, about $400 with a Haiku 5.5 replication. Commands are in `bench/README.md` on the local branch `bench-heldout`. | The first result that can separate the arms (the website's headline number) | Approve calibration now; decide the held-out size after it. Also confirm the Haiku 5.5 model id. |
| 2 | **Credentials for container trials.** Trials run in isolated containers and need `CLAUDE_CODE_OAUTH_TOKEN` (from `claude setup-token`) or `ANTHROPIC_API_KEY`, passed by name. | Item 1 | A long-lived token from `claude setup-token`, exported in the shell that runs the bench. |
| 3 | **First publish of `tauten-check-kernel`.** Trusted publishing cannot create a new crate, and the next release (0.0.3) adds it. | Releasing the proved kernel | A one-off crates.io token scoped to publish-new for `tauten-check-kernel`, used once (`cargo publish -p tauten-check-kernel`), then revoked; then add trusted publishing for that crate. |
| 4 | **Writfold adoption branch** (`tauten-adoption` in `~/workspace/writfold-tauten`, not pushed). | Merging into writfold | Review `docs/writfold/tauten-adoption.md` there: the tiers (writfold-core kernel, writfold-kb edge), the xtask `#[expect]` fix (needs a quality gate run on main), the `#![expect]` style, and the proof-claims adoption (commit `0d5372a`). Three choices there: (a) accept the 10 added mutants and required claims in writfold's `Audit.lean`; (b) for `newer`, `better`, `char_step`, `path_step`: make them private and wait for tauten caller credit, or rewrite and re-prove them; (c) whether a ~1.5 h claims re-run after each kernel edit is acceptable before the claims gate the build. |
| 5 | **Publish the held-out suite only after it runs.** The branch stays local so the hidden oracles are not public before the run. | Honest held-out results | Merge and publish it together with the results. |

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
| Sovel repo | Leave as is for now; may be made private later (2026-11-18) |
| Writfold adoption | In a separate worktree; the owner merges (2026-11-18) |
