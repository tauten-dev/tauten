---
rfc: "2026-11-21-draft-claim-entries"
title: "Draft claim entries from checked declarations"
status: accepted
implementation: partial
authors:
  - astra
created: 2026-11-21
updated: 2026-11-21
issue: https://github.com/tauten-dev/tauten/issues/49
milestone: "0.0.6"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Draft claim entries from checked declarations

## Decision to review

`cargo tauten proofs draft` would print a reviewable patch to `proofs/claims.toml` for new or changed functions. It would not write a receipt, run mutants, grant proof credit, or generate Lean statements. This is the claim-entry half of the [discipline proposal](../DISCIPLINE-PROPOSAL.md), not its proposed refinement-language work.

[Writfold's generator](https://github.com/writfold-dev/writfold/blob/29a33162a54753a9ed774e62fd90937ea6e68a10/proofs/tauten_claims.py) already joins Lean declarations to mutants. Its Rust brace scan and naming rules are project-specific. [#49](https://github.com/tauten-dev/tauten/issues/49) reports three rounds of manually drafting the manifest in one adoption cycle. A new command needs a contract for what it may suggest, especially when the names are ambiguous.

## Proof obligations and trust boundary

None of the suggestions changes a decision about proof, freshness, totality, or subsumption. Candidate discovery, source locations, patch construction and diagnostics are tested shell code, not theorems. The normal gate must still check the actual theorem about the actual extracted function, its assumptions, the audit and its mutants. The existing `subsume::subsumes` decides credit; `registry::conserves` still governs changes to a recorded claim. A draft has no path into either as evidence. If supporting a case requires a new verdict, that case is out of scope until specified and proved.

## Behaviour

```console
cargo tauten proofs draft --crate ledger > /tmp/ledger-claims.patch
git apply --check /tmp/ledger-claims.patch
# Review the patch before applying it.
git apply /tmp/ledger-claims.patch
```

`--crate` names a Cargo package; without it consider all configured kernel packages. Standard output is a deterministic unified diff against the existing manifest, suitable for `git apply` without hand-editing TOML. An unchanged manifest yields empty stdout and exit 0. An unknown package, missing tools/evidence, stale translation, or ambiguous association needed for an entry yields nonzero, an actionable explanation on stderr and **no partial patch**. A function with no eligible theorem/mutant pair is listed as skipped, not given a claim. Exit 0 means the draft operation succeeded, never that a gate passed. `proofs/claims.json`, the checkout and the index are unchanged. To establish the claim, apply and commit the reviewed patch and run `cargo tauten proofs` on the new commit.

## Design

1. Use the current pinned translation and a successfully built Lean environment. Join a Cargo package, workspace-relative file and qualified Rust item to the fully qualified extracted declaration using extraction/source metadata. Do not join on the last name, predict which tactic unfolds a body, or scan braces. A loop target can contribute a theorem to its parent only when the translation identifies that relationship.
2. Have the Lean-side adapter enumerate checked theorem heads and their subjects. Only a premise-free total triple **about this function itself**, satisfying the existing audit's shape, is a candidate for `total`. A same-named function in another module, a loop's total theorem, or a theorem with an extra premise is not. Add an eligible `subsumes` list only for an unconditional local claim with such a total theorem; never silently drop assumptions, witnesses or amendments.
3. Join the declared mutants by identity. A named expected direct failure is a *requirement*, not an observed kill. Use existing direct-failure observations only when they are bound to the matching inputs; a dependent headline alone is not a direct witness. Never manufacture a `direct` set (#39) or infer it from tactics (parked in #30). A fresh function need not have run its first gate to get a draft; the cited mutant still has to calibrate before any badge exists.
4. The default adapter reads the stock template's checked declarations, extraction and mutant inventory. For custom gates, propose `--inventory PATH`: an ephemeral JSON object with `version: 1`, `functions` (package, file, qualified Rust item, extracted declaration), `theorems` (qualified name, extracted subject, role: spec/loop/total), and `mutants` (declared name, target item, applicable theorem names). References must resolve uniquely in the selected current manifest/translation/environment; check `total` independently of the supplied role. Unsupported versions or contradictory mappings refuse the draft. This input is an adapter interface for suggestions, **not** a second committed ledger or proof record. The stock path needs no project-written exporter. Writfold can export its mutant associations without reimplementing Rust source discovery or TOML editing.
5. Emit additions only. For an existing `(crate, file, function)`, extend unambiguous fields in place; preserve hand-written fields, comments, ordering in unrelated entries and the amendment log. Conflicting `total`, changed assumption requirements or removals need explicit review, not an automatic rewrite. After applying the patch a second draft is empty.

## Alternatives and compatibility

A whole-file generator would overwrite reviewed obligations. Predicting failures from proof text reopens a rejected heuristic. Generating Lean skeletons is a different feature: it cannot establish that an existing theorem states the intended claim. The only durable output proposed here is a patch using fields the current manifest parser already supports; old projects and the ordinary gate need no migration. Removing the new command would not invalidate applied manifests.

## Evidence, rollout and owner decision

- Prove the workflow useful on an ordinary template: add a function with a total theorem and mutant, apply the draft as printed, then get the same gate verdict as a manually authored claim. Re-draft and expect no diff.
- Use two modules with the same short name, a helper/loop theorem, a conditional theorem, a claim with assumptions and an uncalibrated mutant as controls. No cross-assignment, unearned `subsumes`, or receipt writes. Preserve an unrelated comment and amendment byte for byte. A failed audit/ambiguity must produce no applicable patch.
- Before shipping 0.0.6, exercise an actual custom-gate inventory against the pinned downstream example and compare the result to its hand-written entries. Run the existing quality gate and proof gate for any changed audited component. Do not claim the drafter itself is proved.
- **Owner decision:** accept a patch-only command plus an opt-in inventory adapter rather than a writer or a project-specific integration? If the consumer needs its own name scanner even with the adapter, revise the design; do not ship under that premise.

## Decision log

- 2026-11-21: proposed for review; no implementation or owner acceptance.
- 2026-11-21: accepted by the owner as proposed: a patch-only `cargo tauten proofs draft` plus an opt-in inventory adapter, not a writer or a project-specific integration.
- 2026-11-21: implemented (partial). `cargo tauten proofs draft [--crate] [--inventory]` in `crates/cargo-tauten/src/draft.rs` (IO) and `crates/tauten-check/src/proofs/draft.rs` (join, plan, TOML edits, diff; tested, not proved); no kernel change, no new verdict. Evidence comes from `[gate] try` run with no mutant selected, so the existing manifest format and gate configuration are unchanged. The Lean-side adapter is a `SUBJECT <theorem> <function>` line added to the template's `Audit.lean` (beside `TOTAL`), emitted by the template's `gate.py` as `{"theorem", "subject"}`. Stock mutants are only those no claim lists yet, located by their unique `before` site; an inventory mutant must name one of the drafted theorems. Recorded direct-failure observations are not used at all. Controls are covered by unit tests and by `fixtures/proofs-draft` with `crates/cargo-tauten/tests/draft.rs` (patch applied with `git apply`, hand-written comment and amendment kept byte for byte, second draft empty, no receipt). Outstanding, hence `partial`: (1) the template `Audit.lean` change has not been built by Lean and no draft has run against a real gate (the fixture's gate is a stand-in printing recorded audit lines), so "apply the draft, then get the same gate verdict as a manually authored claim" is not yet shown; (2) the custom-gate inventory has not been exercised against the pinned writfold example and compared to its hand-written entries. Both are required before 0.0.6 ships.
- 2026-11-21: real-gate evidence. On a `cargo tauten new` scaffold with the pinned proofs image, the template `Audit.lean` builds and emits `SUBJECT`; with its claim removed, `cargo tauten proofs draft` re-drafted it (same function, lean, theorems, `total` and mutants, plus `subsumes = ["TAU-N001", "TAU-P003"]`), the patch applied, and `cargo tauten proofs` proved it (1 of 1, badge `proved`), with `cargo tauten check` clean. Regression fixture from that real translation: `fixtures/proofs-draft-real`. Still `partial`: the writfold inventory comparison has not been run.
