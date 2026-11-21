# Decisions and assets needed from the owner

Updated 2026-11-21. Settled items are kept at the bottom for the record.

## Open

| # | Decision | What it unblocks | Recommendation |
|---|---|---|---|
| 1 | **Triage of 2026-11-20** (Fable plan, applied): milestone 0.0.5 = #21, #46, #42, #39, #36, #48; #40 and #47 merged into #39; #43 (size budgets) and #45 (build-cache budget) declined and parked in #30; #46 reworded from a keyword table to "the fast stage builds the extracted Lean". | Confirms the milestone | Reopen anything you disagree with. |
| 2 | **Maintenance chains (E2).** Tests whether the bounds E1 relied on survive later edits: 5-8 fresh sessions each adding a feature to one repo, union oracle at the end, arms A vs C. | The only remaining defect-reduction question | Decide whether it is worth a budget; not started. |
| 3 | [Result/world induction](rfcs/2026-11-21-result-world-induction.md), [#60](https://github.com/tauten-dev/tauten/issues/60): document the existing recursor or also export a named theorem? | A proof-only adoption aid; not a Gripsack release dependency | Compare the same partial-step example both ways. Add the lemma only if it removes actual induction boilerplate. |
| 4 | [Bounded capability reads](rfcs/2026-11-21-bounded-capability-reads.md), [#64](https://github.com/tauten-dev/tauten/issues/64): require `Limit<Bytes>` on both `ReadDir` reads at the minor break? | The 0.2 capability API and migration docs | Require the argument, a distinct too-large error and explicit snapshot attenuation; do not offer an unbounded overload. |

## Settled

| Item | Decision |
|---|---|
| Repositioning | Code discipline + proofs; defect reduction only as a side effect (2026-11-19) |
| Discipline plan | `docs/DISCIPLINE-PROPOSAL.md` approved: D0, then X1-X3, then D1-D8 (2026-11-19) |
| Verus / TLAPS | No adapters for now; claims format reserves the tool names (2026-11-19) |
| Proofs image | Public on ghcr for 0.1.0, no longer tied to adopter parity (2026-11-21; supersedes 2026-11-19) |
| 0.1.0 scope | #13 (A9 driver) and #35 (D1 shared gate) moved to 0.2; helpers (A10) fold into `tauten-check-kernel` as a module and A9 later ships as a `tauten` feature, so no new crate needs a first-publish token. Reversible (2026-11-21) |
| Roadmap | Accept only general, proved (or compiler-enforced) items that fit the vision; filed as issues #1-#38, ordered in `docs/ROADMAP.md`; K004 dropped for an extract-only stage (2026-11-19) |
| Product shape | Fast checks on stable; deep checks in the pinned image at ship/nightly/release; "effects visible in signatures" (2026-11-19) |
| Name | tauten (2026-11-18) |
| License | MIT (2026-11-18) |
| Domain, org | tauten.dev at Porkbun; GitHub org tauten-dev (2026-11-18) |
| Crate publishing | 0.0.1 with a token, 0.0.2 with trusted publishing; "require trusted publishing" on; the original token revoked (2026-11-18) |
| Repo visibility | tauten-dev/tauten public (2026-11-18) |
| Pilot budget | 36 trials on one model; run 2026-11-18 for $6.29 |
| Proof toolchain image | ghcr.io/tauten-dev/proofs-toolchain; private at first (2026-11-18), public since 2026-11-21 (owner changed the org package policy and the package visibility; anonymous pull verified) |
| Proof tier | Ship, then prove one slice at a time; proofs never block unrelated work (2026-11-18) |
| Held-out benchmark | Approved 2026-11-19: calibration, then Sonnet 5.5 primary and Haiku 5.5 replication (about $400). Calibration round 3 was no-go by the preregistered rule; the held-out run was not started and its budget not spent |
| Container credentials | Long-lived `claude setup-token` token, stored outside the repo (2026-11-19) |
| `tauten-check-kernel` first publish | 0.0.2 with a one-off token, then trusted publishing and "require trusted publishing" via the crates.io API (2026-11-19); the token is to be revoked |
| Writfold proof claims | Accept the added mutants; rewrite `newer`, `better`, `char_step`, `path_step` to be total; accept a full claims re-run after kernel edits for now (2026-11-19) |
| Sovel repo | Leave as is for now; may be made private later (2026-11-18) |
| Writfold adoption | Developed in a separate worktree (2026-11-18); merged to writfold main on the owner's word (2026-11-20) |
| Benchmark publication | Publish the harness, hosts, dev and calibration tasks, results and E1; withhold the 16 unrun held-out tasks (2026-11-20; `bench/WITHHELD.md`) |
| One-off crates.io token | Kept for now for first publishes of new crates; the owner revokes it later (2026-11-20). The local copy was deleted on 2026-11-19, so a new first publish needs the owner to supply a token |
