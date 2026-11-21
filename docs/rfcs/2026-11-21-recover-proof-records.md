---
rfc: "2026-11-21-recover-proof-records"
title: "Recover proof records after an unavailable tool or a merge"
status: accepted
implementation: complete
authors:
  - astra
created: 2026-11-21
updated: 2026-11-21
issue: https://github.com/tauten-dev/tauten/issues/50
milestone: "0.0.6"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Recover proof records after an unavailable tool or a merge

## Decision to review

`proofs/claims.json` is both a reviewed registry and evidence about particular inputs. An attempt with no available tool should not overwrite it. A two-parent merge needs the obligations from **both** histories, but neither parent's verdict about the new tree. These two cases share a trust boundary, not a prerequisite: the single-branch retry fix can land independently.

In [#52](https://github.com/tauten-dev/tauten/issues/52), an unavailable Docker daemon turned a clean tag's record into 58 `tool-missing` entries; a normal retry then refused the dirty file the previous invocation wrote. In [#50](https://github.com/tauten-dev/tauten/issues/50), an adopter had to hand-merge generated Lean and independently check the result. Neither problem requires a new verifier.

## Proof obligations before wiring

- Reuse `registry::conserves(old, new, amended)` for each parent's may-not-shrink theorem/mutant entries and its existing claims' may-not-grow assumptions (same orientation and exemptions as TAU-R002). Combining parents first is wrong: the union of two different assumption sets can silently authorize a new assumption. Require two distinct, successful parent observations via `control::admit(required, observed)`. A missing parent observation cannot pass.
- A pending registry is **not** a gate outcome. It contributes no established result, total theorem for subsumption, or reusable mutant observation. Route the absence of established evidence through the existing `badge::badge` and `subsume::subsumes`; do not teach the shell to award badges. Once re-proved, use the usual outcome, named-assumption badge and freshness decisions, including for inherited claims.
- Existing audited claims include `Tauten.Kernel.conserves_iff`, `amendment_is_explicit`, `admit_ok_iff`, `badge_proved_iff` and `stale_proof_never_subsumes`. Retain their calibrated mutants (including `removal-accepted`, `amendment-not-required`, `missing-observation-accepted` and `stale-proof-subsumes`). These are *existing* theorems about inputs, not proof of this proposed workflow. No new kernel verdict is proposed. If the two-parent check or a pending-to-established transition needs different decision logic, state a kernel signature, theorem and breaking mutant before implementation.
- Parent discovery, reading Git objects, regeneration, host-side state decoding, safe replacement and reporting remain tested, not proved. Test against real Git histories; do not describe Git or the filesystem as formally verified.

## User-visible behaviour

**Unavailable tool (#52).** A gate that cannot start still prints its usual per-claim `tool-missing` diagnostics and exits nonzero. Before any local proof stage has run, an unavailable executable/daemon/image is an attempt, not a replacement record. Leave an existing record byte-identical; create none if absent. Thus a clean checkout can retry the same command without `--live` or a commit. The result of the *attempt* never passes; a surviving earlier record keeps its original input binding, not a new timestamp or badge. Do not identify this case just by counting outcome strings: empty inventory, inherited-only work and a started gate reporting a real failure are distinct. Keep the existing handling for substantive drift/build/audit/mutant failures and partial runs. Do not relax dirty-tree checks.

**Two-parent merge (#50).** After resolving and staging human-authored source, proofs and manifest conflicts, run the proposed `cargo tauten proofs reconcile` in the merge worktree. It requires a normal two-parent merge, and may modify only the configured generated extraction and `proofs/claims.json`. It never stages, commits, edits a hand-written proof, installs a Git driver or changes global Git settings. On success it identifies the changed generated paths and reports **pending, not proved**. Stage those paths and finish the merge. On the clean merged commit, `cargo tauten proofs --level ship` performs the ordinary gate, recalibrating all local claims for which there is no evidence. Only that successful gate replaces pending with a normal evidence record. A later successful full run must agree on outcomes and assumptions.

## Design and compatibility

1. Read the actual first and second parent OIDs from Git and each parent's committed record from its tree, never from a conflicted working file. Refuse a missing/unreadable parent record, unavailable object, unresolved human input, unsupported merge mode or unknown format rather than guessing. This first version addresses two proof-enabled parents; rebase, octopus merge and first adoption are not auto-resolved. Run the unchanged conservation check twice against the resolved manifest. Do not invent `[[amend]]` reasons.
2. Regenerate from the resolved source with the pinned translation in scratch. Compare the whole output tree using the existing [generated stage](../../crates/cargo-tauten/src/generated.rs), including deletions; compiling an extraction is not a drift comparison. Do not run a generator on conflict-marker text. Use the project's declared extraction output; if it is not known unambiguously, refuse and name the missing configuration. Do not add a second translation engine or diff implementation.
3. Proposed provisional record, deliberately distinct from a v1 gate result: `version: 2`, `state: "pending-merge"`, `parents: [first_oid, second_oid]`, and `registry` containing the resolved manifest's claim declarations after the two parent checks. There is **no** `claims` result array, source fingerprint, gate commit, carried kill or theorem verdict. This avoids misusing `drift` (an observed gate result) or writing `proved` next to a cosmetic pending label. A v1 reader requires the missing `claims` array and must visibly refuse this variant, not treat it as a successful empty record. Read and write normal v1 gate results unchanged. Test old-reader behaviour rather than relying on the version number.
4. Complete input validation and regeneration before touching the checkout. Install the pending record before replacing generated files. Do not promise an atomic multi-file checkout update: after interruption, the state must remain pending or unmerged, and rerunning on the same inputs must finish it. Never stage a partially regenerated tree. Until the gate runs, `check` retains TAU-R002 against the provisional registry and grants no proof subsumption; `proofs export` must refuse to ship it as evidence. Readers showing badges must distinguish the pending state from a gate pass. Missing parent Git objects needed to validate the merge are an actionable refusal, not an excuse to skip conservation.
5. On the first authoritative run, verify the pending parent identities against the source merge and recheck their obligations and the provisional registry; do not promote a hand-written pending file merely because it parses. The merged commit, not either parent or the conflicted worktree, is the snapshot of record. The ship planner has no local parent kills to reuse. Existing dependency-version and inherited-badge checks still apply. Never delete a pending registry to get past a conservation failure.

## Alternatives

Choosing `ours`/`theirs` loses obligations or imports stale evidence. Deleting the record makes TAU-R002 forget its baseline. A line-oriented Git merge driver runs before it knows all human-authored inputs are resolved. Running the full gate inside the merge is expensive and would describe an uncommitted tree. A recipe using `refresh.sh` and `generated --only translation` solves the **extraction** half today, but not the evidence or conservation half. Retrying infrastructure errors automatically is unnecessary; a normal retry should work.

## Evidence, rollout and owner decision

- #52 first: unavailable tool with/without a previous record; stdout/stderr and nonzero exit visible; restore the tool and retry. A genuine audit/mutant failure is not reclassified as an unavailable tool. Independent dirty source still prevents a snapshot.
- #50: two branches edit different functions in one generated Lean file and introduce different obligations. Resolve only human inputs. The regenerated tree must byte-match an independent fresh extraction; pending cannot subsume, export, or reuse a parent kill. Remove a parent's required mutant or add an assumption for a shared claim: refuse until the existing reasoned amendment rule is satisfied. Check missing object/image and interrupted checkout handling. On a successful ship compare with a full run, including conditional badges.
- 0.0.6: run the existing quality and proof gates and one adopter-shaped two-parent fixture. Do not publish, retag, or claim a general Git merge service. No cross-project timing claim without a measured adopter run.
- **Owner decision:** accept the explicit provisional format and first-merge recalibration cost in preference to an implicit stale marker? If not, the retry fix remains independent; do not represent a pending merge as a success to meet a milestone.

## Decision log

- 2026-11-21: proposed for review; no implementation or owner acceptance.
- 2026-11-21: accepted by the owner as proposed, including the explicit provisional `pending-merge` record format and the first-merge recalibration cost. The single-branch retry (#52) is implemented first; the two-parent reconcile (#50) follows.
- 2026-11-21: implemented. #52: an attempt that never reached a proof stage is reported but not recorded (`Report::started`); tested with and without a previous record and from a clean Git clone. #50: `cargo tauten proofs reconcile` and the `pending-merge` record (`RecordFile`, `Pending`); both parents decided by `proofs::parents_conserved` over the proved `control::admit`, no new kernel verdict. Tested on an adopter-shaped two-parent fixture (`crates/cargo-tauten/tests/reconcile.rs`): conflicting translation regenerated to match a fresh one, pending never subsumes or exports, a dropped parent obligation and unresolved human conflicts are refused, a pending record naming other parents is not promoted, and the merge commit's gate replaces it. Git reading, regeneration and file replacement are tested, not proved. Not exercised: a real Charon/Aeneas translation in the merge fixture (a stand-in generator) and comparing the ship run with a full run on a real merge.
