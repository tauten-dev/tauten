# Decisions and assets needed from the owner

Collected 2026-11-18. Each item says what it unblocks.

## Assets

| # | Asset | Unblocks |
|---|---|---|
| 1 | **Final name.** Candidates whose crates.io lookup returned 404 on 2026-11-18: `tauten`, `scantling`, `stanchion`, `trunnion`. Spares: `kerfline`, `hewline`, `trueline`, `gudgeon`, `stricta`, `rigora`. Check domain, GitHub org and trademark yourself. | rename (cheap until first publish), M5 |
| 2 | Domain (porkbun) | M7 website |
| 3 | GitHub org, plus a private repo or permission to create one | push, CI (M5) |
| 4 | crates.io token, scoped to publish-new + publish-update for the chosen crate names | M5 publish / name reservation |
| 5 | Model budget and an API key, or a subscription usable headless in a container, for benchmark trials | M4 pilot, M6 |

## Decisions

| # | Decision | Recommendation |
|---|---|---|
| 1 | License | **Decided 2026-11-18: MIT** |
| 2 | Reserve names now with 0.0.x crates, or wait for 0.1 | Reserve now, once the name is final: the 4 crate names plus the CLI |
| 3 | Benchmark pilot: model(s), trials, budget cap | 4 dev tasks × 3 arms × 3 trials = 36 runs on one model, before writing the held-out tasks |
| 4 | Writfold adoption timing | Stage 1 (observe, warn-only) now. Stage 2 (baseline + tiers) after the pilot. Note: writfold's AGENTS.md pauses benchmark pilots *in writfold*; this benchmark lives in the tauten repo. |
| 5 | Proof tier in 0.1 (Aeneas/Lean template from writfold), or 0.2 | Template in 0.1, as the differentiator, but after the pilot |
| 6 | Sovel repo: docs-only "archived, superseded by <name>" banner commit, tag `archive-2026-11`, GitHub archive | Yes. Nothing in the Sovel repo has been changed yet. |
| 7 | writfold-kb tier: the dry run found it does file and git IO directly | Decide whether it is edge, or whether its decision logic should move to a core crate |
