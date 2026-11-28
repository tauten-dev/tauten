# Roadmap

Every item is a GitHub issue. An item is accepted only if it is **general** (useful across projects, not a check for one codebase), **decided by the proved kernel or the compiler** (anything weaker is labelled bounded, tested or heuristic, never "enforced"), and makes the flow **lighter**. Ideas that did not pass are in the [parked and rejected tracking issue](https://github.com/tauten-dev/tauten/issues/30), each with the trigger that would reopen it.

Designs behind items, and any new proposal that is costly to reverse or changes decision logic, are [RFCs](rfcs/README.md); the roadmap itself is the [five-primitives RFC](rfcs/2026-11-19-roadmap-five-primitives.md).

Order within a milestone is by return on investment: projects helped × strength of the guarantee ÷ cost. Sizes are estimates in agent-days.

**Released:** [0.2.0](https://github.com/tauten-dev/tauten/issues/70#issuecomment-7074180614). Bounded reads, result/world induction and the rustdoc gate shipped; onboarding (#66/#68) is complete. The custom-inventory comparison (#61) closed as **no fit**, not parity. The real merge exercise (#62) is still open.

## 0.2.1 (next)

[Live pickup order](https://github.com/tauten-dev/tauten/milestone/7) · [release checklist and blockers](https://github.com/tauten-dev/tauten/issues/83).

**Make ship-level proof reuse trustworthy before making it faster.** Writfold now
[builds a split translation](https://github.com/writfold-dev/writfold/commit/3b49ee6f8edbf1c2b2b31da0881fe840e77ce4be)
but keeps an unbuilt single file for Tauten’s dependency scan. It also
[spent 5,779 seconds recalibrating before a known registry weakening was refused](https://github.com/writfold-dev/writfold/blob/010420beb1174a610754a555429fa1da8adc143b/docs/rfcs/2026-11-22-release-speed.md#L96-L105).
Neither problem requires adopting a common gate.

1. **Start with [#79](https://github.com/tauten-dev/tauten/issues/79), then [#80](https://github.com/tauten-dev/tauten/issues/80).** A caller’s inputs must include callees in other extracted files. A mutant name does not identify an unread definition. Where a binding cannot be established, rerun rather than carry an old kill. Compare ship and full on the same changed commit, starting with an established record; keep an unaffected claim as the reuse control.
2. **Independently, [#77](https://github.com/tauten-dev/tauten/issues/77) and [#81](https://github.com/tauten-dev/tauten/issues/81).** Check the existing TAU-R002 conservation rule before launching a recording gate; keep the final write guard. Check stock textual mutant sites before extraction. Passing either preflight is not proof evidence.
3. **[#75](https://github.com/tauten-dev/tauten/issues/75) and [#78](https://github.com/tauten-dev/tauten/issues/78).** Exercise an inert version bump separately for check-time subsumption and ship-time mutant selection. Document the cost of adding `[gate] try`; do not assume two arbitrary runners are interchangeable.
4. **[#83](https://github.com/tauten-dev/tauten/issues/83) closes the patch.** Compare claims, badges, assumptions and exact mutant witnesses, not just totals. The ordinary gate and full proofs remain mandatory on the exact tag commit; owner approval precedes tagging. No new public command or record format is in scope.

**State (2026-11-28):** all six fixes are implemented on [PR #95](https://github.com/tauten-dev/tauten/pull/95); what remains is [#83](https://github.com/tauten-dev/tauten/issues/83): merge, the full proofs gate on the merged commit, a real-tool ship-versus-full comparison on one changed commit, and owner approval to tag. The former stretch item [#62](https://github.com/tauten-dev/tauten/issues/62) moved to [0.2.2](#022) so that this milestone holds exactly what the tag waits for. No adopter timing, pin change or migration gates 0.2.1.

## Sequence after 0.2.1

Groomed 2026-11-28 (Fable plan, for the owner to confirm). Every open issue is in exactly one milestone. Milestones are ordered by dependency and by return; inside each, the list is the pickup order. A milestone is a release only when its blocking items are closed: adopter evidence and stretch items never hold a tag.

| Milestone | Goal | Issues, in order | Before work starts |
|---|---|---|---|
| [0.2.1](https://github.com/tauten-dev/tauten/milestone/7) | Ship-level reuse bound correctly; early refusals | [#79](https://github.com/tauten-dev/tauten/issues/79), [#80](https://github.com/tauten-dev/tauten/issues/80), [#77](https://github.com/tauten-dev/tauten/issues/77), [#81](https://github.com/tauten-dev/tauten/issues/81), [#75](https://github.com/tauten-dev/tauten/issues/75), [#78](https://github.com/tauten-dev/tauten/issues/78), [#83](https://github.com/tauten-dev/tauten/issues/83) | Owner approval to tag |
| [0.2.2](#022) | Adoption plumbing with no new verdict and no format change | [#102](https://github.com/tauten-dev/tauten/issues/102), [#90](https://github.com/tauten-dev/tauten/issues/90), [#97](https://github.com/tauten-dev/tauten/issues/97), [#98](https://github.com/tauten-dev/tauten/issues/98), [#62](https://github.com/tauten-dev/tauten/issues/62) | Nothing |
| [0.3.0](#030) | A check an adopter can require in CI | [#101](https://github.com/tauten-dev/tauten/issues/101), [#99](https://github.com/tauten-dev/tauten/issues/99), [#100](https://github.com/tauten-dev/tauten/issues/100), [#93](https://github.com/tauten-dev/tauten/issues/93), [#87](https://github.com/tauten-dev/tauten/issues/87), [#86](https://github.com/tauten-dev/tauten/issues/86) | Two RFCs accepted |
| [0.4.0](#040) | Evidence and proof credit that say exactly what they rest on | [#96](https://github.com/tauten-dev/tauten/issues/96), [#89](https://github.com/tauten-dev/tauten/issues/89), [#88](https://github.com/tauten-dev/tauten/issues/88), [#85](https://github.com/tauten-dev/tauten/issues/85), [#91](https://github.com/tauten-dev/tauten/issues/91), [#94](https://github.com/tauten-dev/tauten/issues/94), [#92](https://github.com/tauten-dev/tauten/issues/92) | Four RFCs accepted; kernel proofs first for [#89](https://github.com/tauten-dev/tauten/issues/89), [#88](https://github.com/tauten-dev/tauten/issues/88) |
| [0.5.0](#050) | Tested evidence under kernel-decided verdicts | [#103](https://github.com/tauten-dev/tauten/issues/103), [#112](https://github.com/tauten-dev/tauten/issues/112) | The playground RFC accepted, with the owner's answer on kernel-first |
| [later](#later-not-scheduled) | Not scheduled; each has a pickup trigger | the rest | Its trigger |

## 0.2.2

**Adoption plumbing: nothing here adds a verdict, a finding or a file format, so nothing waits on an RFC.** This is what the main agent can pick up the day 0.2.1 is tagged, while the 0.3.0 RFCs are drafted and reviewed.

1. [#102](https://github.com/tauten-dev/tauten/issues/102) **A checksummed prebuilt `cargo-tauten`.** A GitHub release per tag with one Linux binary and `SHA256SUMS`, built by the existing release workflow, and the qualified Rust toolchain named in the release notes. Writfold's adoption stalled a week on having no pinned artifact. The toolchain-mismatch warning (item 3 of the issue) is a new stable ID: it ships only with a short RFC, otherwise it stays documentation.
2. [#90](https://github.com/tauten-dev/tauten/issues/90) **The total-shape audit as its own Lean module**, vendorable like `Tauten.Machine`. The template gate's output must be byte-identical before and after. Two adopters accept conditional claims without knowing it because they have no such audit.
3. [#97](https://github.com/tauten-dev/tauten/issues/97) **MAPPING.md says what ships.** Include the limits of the contract macros recorded in the [contracts draft](rfcs/2026-11-27-component-contract-testing.md#existing-tauten-abstractions): `ensures` checks only `Ok`, a violation does not roll back effects, `old` takes `Copy` values.
4. [#98](https://github.com/tauten-dev/tauten/issues/98) **An interrupted gate run keeps its finished calibrations.** A side file outside the record; reuse decided by the proved `binding::fresh` under the bindings ship reuse already uses; never promoted into `claims.json` without a completed run. If the implementation needs anything beyond that, stop and write an RFC. This pays for itself in every later release qualification.
5. [#62](https://github.com/tauten-dev/tauten/issues/62) **Real two-parent reconcile.** Evidence for a shipped command; stretch, carried from 0.2 and 0.2.1, never holds the tag.

## 0.3.0

**A check an adopter can require.** Writfold is about to make `cargo tauten check` a required step, and Gripsack and Strop could not run the cheapest useful trial. Today a stale record is indistinguishable from 182 new kernel errors, any edit under the kernel crate withdraws every subsumption, and a kernel crate cannot be checked without building the workspace.

1. [#101](https://github.com/tauten-dev/tauten/issues/101) **Check-time freshness per claim.** The decision stays the proved `binding::fresh`; what changes is which inputs it is given, so that an edit the translation cannot see no longer stales a claim. First, because [#99](https://github.com/tauten-dev/tauten/issues/99) and [#100](https://github.com/tauten-dev/tauten/issues/100) are stated in its terms. Needs an RFC with the proof obligations stated, even if they are "the existing theorems, new inputs".
2. [#99](https://github.com/tauten-dev/tauten/issues/99) **Retiring a spent `[[amend]]` stales nothing it does not affect.** Falls out of the previous item if amendments leave the fingerprint; otherwise a small retire step.
3. [#100](https://github.com/tauten-dev/tauten/issues/100) **A stale record is one finding of its own**, with a stable ID and a JSON marker on the sites it explains. Same RFC as [#101](https://github.com/tauten-dev/tauten/issues/101).
4. [#93](https://github.com/tauten-dev/tauten/issues/93) **Show the ship plan without starting the gate.** Read-only. It also makes the planner testable without Lean, which [#112](https://github.com/tauten-dev/tauten/issues/112) uses.
5. [#87](https://github.com/tauten-dev/tauten/issues/87) **`--package` for `check` and `obligations`**, with workspace-wide rules reported as not run, never clean. No-fit exit: document the one-member-workspace recipe.
6. [#86](https://github.com/tauten-dev/tauten/issues/86) **A versioned agent profile that can be refreshed**, and a finding when the committed copy is stale.

**RFCs to draft and have accepted first:** (a) check-time freshness and the stale-record finding ([#99](https://github.com/tauten-dev/tauten/issues/99) to [#101](https://github.com/tauten-dev/tauten/issues/101)); (b) the three read-only additions ([#93](https://github.com/tauten-dev/tauten/issues/93), [#87](https://github.com/tauten-dev/tauten/issues/87), [#86](https://github.com/tauten-dev/tauten/issues/86)), which by [the RFC rules](rfcs/README.md#when-an-rfc-is-required) need one because each adds a flag, a finding or an output line. One short RFC can cover all three, or the owner can waive it per item.

## 0.4.0

**Evidence and proof credit that say exactly what they rest on.** One recalibration for every adopter, taken once: the binding changes are batched.

1. [#96](https://github.com/tauten-dev/tauten/issues/96) **A stamp is bound to its subject as well as its revision.** A shipped carrier accepts a stamp minted for another instance with an equal revision. An API decision, so an RFC; the fix is a type, and it is the stronger route for the stale and wrong-subject laws that [#110](https://github.com/tauten-dev/tauten/issues/110) would otherwise only test.
2. [#89](https://github.com/tauten-dev/tauten/issues/89) **Subsumption under a named allocation-length assumption.** New decision logic in `subsume`: kernel, theorems and mutants before wiring. Two adopters have findings that can never close otherwise.
3. [#88](https://github.com/tauten-dev/tauten/issues/88) **A kernel-scoped overflow fact for TAU-H001.** New decision logic in `manifest`: proofs first. Three adopters declined the workspace-wide setting for the same reason.
4. [#85](https://github.com/tauten-dev/tauten/issues/85) **The record says how it was produced**: tool version, level, which results were carried and from where. A record format change.
5. [#91](https://github.com/tauten-dev/tauten/issues/91) **Ship binds the gate scripts and the extraction tool identity.** Same format change and the same one-time recalibration as the previous item; do them together.
6. [#94](https://github.com/tauten-dev/tauten/issues/94) **Narrower proof-file mention binding**, without losing soundness. Changes keys, so it rides the same recalibration.
7. [#92](https://github.com/tauten-dev/tauten/issues/92) **Finer reuse for exact-set mutants.** Last, and with a no-fit exit: if a gate cannot show the dependency set it reports is complete, the binding stays "every Lean file".

**RFCs to draft and have accepted first:** [#96](https://github.com/tauten-dev/tauten/issues/96); [#89](https://github.com/tauten-dev/tauten/issues/89); [#88](https://github.com/tauten-dev/tauten/issues/88); and one for the record and binding batch ([#85](https://github.com/tauten-dev/tauten/issues/85), [#91](https://github.com/tauten-dev/tauten/issues/91), [#94](https://github.com/tauten-dev/tauten/issues/94), [#92](https://github.com/tauten-dev/tauten/issues/92)).

## 0.5.0

**Tested evidence, with the verdict decided by the kernel.** The owner authorized a scenario-testing experiment on 2026-11-27 ([#103](https://github.com/tauten-dev/tauten/issues/103)). The [review of 2026-11-28](rfcs/2026-11-27-executable-property-playground.md#decision-log) keeps it and changes three things: the runner's pass, shrink and replay verdicts go through `control::admit`, `binding::fresh` and a small proposed `campaign` kernel module instead of new unproved code; the code lands in the repository and runs in the gate before anything is called exercised; and the first real trial is on tauten's own CLI.

1. [#103](https://github.com/tauten-dev/tauten/issues/103) **The `campaign` kernel module, then the runner.** Seven theorems and seven mutants through the proofs gate first; then `playground/scenarios` with the queue example, its planted defect and a no-op control. Results earn the `Tested` badge and nothing more.
2. [#112](https://github.com/tauten-dev/tauten/issues/112) **Trial on tauten's own CLI: ship-level reuse agrees with a full run under generated edits.** The full run is the oracle; the defects fixed by [#79](https://github.com/tauten-dev/tauten/issues/79) and [#80](https://github.com/tauten-dev/tauten/issues/80) are the planted faults.

The external trials ([#105](https://github.com/tauten-dev/tauten/issues/105), [#106](https://github.com/tauten-dev/tauten/issues/106), [#107](https://github.com/tauten-dev/tauten/issues/107)), the per-case Docker backend ([#104](https://github.com/tauten-dev/tauten/issues/104)) and behavioural contracts ([#109](https://github.com/tauten-dev/tauten/issues/109), [#110](https://github.com/tauten-dev/tauten/issues/110)) are in `later` with triggers. They are adopter evidence or wait on evidence, and none of them holds a tauten tag. The playground is isolated in its own package, so RFC review can run while 0.3.0 is built; its place here is its order of return, and the owner may pull it forward.

## Five primitives

The roadmap reduces about sixty proposals (a durability plan, a survey of seven Rust projects, and a survey of safety mechanisms in other languages and the C++ safety-profile proposals) to five general primitives:

| Primitive | What it decides | Absorbs |
|---|---|---|
| **Controls and badge** | whether a set of checks passed (exact set; a timeout, missing tool or empty inventory never passes; a negative control must fail for its named reason), and which badge a claim earns: proved, proved under named assumptions, bounded, tested | claim outcomes, named assumptions, mutants, negative model-checker configs, reject fixtures |
| **Binding** | whether a recorded result is still about the current inputs | fingerprints, drift checks, generated files, persisted evidence, stamps |
| **Machine** | step functions over state, with a Lean induction library and replay | protocols, atomic publish, crash recovery, the explorer |
| **Registry** | that nothing may be quietly removed or weakened | the baseline ratchet, theorems and mutants per claim, assumptions that may not grow |
| **Effect row** | that a signature's capabilities match its declared effects | TAU-F001, later a declared label table |

## 0.0.6 (released 2026-11-21)

Proof records an adopter can recover, and advice that works. Triaged
2026-11-21 (Fable plan; #57 and #59 admitted from adopter reports, #58
deferred to 0.1.0).

| Issue | Item | State |
|---|---|---|
| [#51](https://github.com/tauten-dev/tauten/issues/51) | An empty claim inventory never passes | closed |
| [#52](https://github.com/tauten-dev/tauten/issues/52) | A missing proof tool leaves the record untouched, so a retry works | closed |
| [#50](https://github.com/tauten-dev/tauten/issues/50) | `proofs reconcile`: a two-parent merge is pending until its merge commit is proved | closed |
| [#53](https://github.com/tauten-dev/tauten/issues/53) | Release publishes only after the gate and the full proofs pass on the tagged commit | closed |
| [#49](https://github.com/tauten-dev/tauten/issues/49) | `proofs draft`: reviewable claim entries | closed; `--inventory` on a real custom gate follows in [#61](https://github.com/tauten-dev/tauten/issues/61) |
| [#57](https://github.com/tauten-dev/tauten/issues/57) | TAU-N002 no longer promises a proof discharge | closed |
| [#59](https://github.com/tauten-dev/tauten/issues/59) | TAU-X001: shared test modules (advice, and the accepted `allow(dead_code)` exemption) | closed |

Release criteria: the CI gate and the full proofs gate pass on the tagged
commit, which `release.yml` now enforces before publishing (#53). The owner
dropped the writfold comparison for #49 as a release gate (#61).

## 0.0.5 (released 2026-11-21)

A proof loop an adopter can live with: a mutant verdict says exactly what
broke, a small kernel edit re-proves only what it touched, and a
dependency's proof can be cited. Also ships everything done since 0.0.4.
Triaged 2026-11-20 (Fable plan; #40 and #47 merged into #39, #43 and #45
declined and parked in #30).

| Issue | ID | Item | Days | Depends on |
|---|---|---|---|---|
| [#21](https://github.com/tauten-dev/tauten/issues/21) | B6 | Declared layers inside a tier, decided by the proved order | 1.5 | — (implemented) |
| [#46](https://github.com/tauten-dev/tauten/issues/46) | — | Fast stage builds the extracted Lean, so Lean decides | 0.5 | A4 |
| [#42](https://github.com/tauten-dev/tauten/issues/42) | — | Gate stages run as the invoking user; root is a recorded opt-in | 0.5 | — |
| [#39](https://github.com/tauten-dev/tauten/issues/39) | — | Exact direct mutant failures, reported apart from affected claims | 1.5 | S2 |
| [#36](https://github.com/tauten-dev/tauten/issues/36) | D2 | Cadence levels and incremental proofs: ship, verify --level | 3 | S4 |
| [#48](https://github.com/tauten-dev/tauten/issues/48) | A7b | Inherit a dependency's shipped claim | 1 | A7, D2 |

Release criteria: all six closed with their fixtures in the gate; the CI
gate, the proofs gate and the scanner mutation gate green on the tagged
commit, `proofs/claims.json` recorded on it; D2 measured on writfold (a
one-function kernel edit re-proves in under 15 minutes, or its no-fit exit
is recorded); STATUS.md and this file current.

Released 2026-11-21 with every criterion met except the writfold D2
measurement, which the owner deferred past the release (writfold's record
does not yet carry inputs; it needs one full writfold run first). Measured on
tauten instead: a one-function kernel edit re-proves in 89 s (docs/STATUS.md).

Done before 0.0.5 (closed 2026-11-19/20): S1–S4, A1–A8, A11, A12, D3,
T4–T7, #41, #44.

## 0.1.0

Released 2026-11-21. Rescoped the same day (owner, on a Fable review): the
release ships what tauten itself implements and proves. Done:
[#54](https://github.com/tauten-dev/tauten/issues/54), [#55](https://github.com/tauten-dev/tauten/issues/55),
[#58](https://github.com/tauten-dev/tauten/issues/58) (publication to a previously absent target, proved),
[#14](https://github.com/tauten-dev/tauten/issues/14) (shared proved helpers in `tauten-check-kernel`;
bounded reads moved to [#64](https://github.com/tauten-dev/tauten/issues/64)) and
[#38](https://github.com/tauten-dev/tauten/issues/38) (docs, version bump, tag, site).
[#13](https://github.com/tauten-dev/tauten/issues/13) (A9, driver) and
[#35](https://github.com/tauten-dev/tauten/issues/35) (D1, shared gate) moved to 0.2: both depend on adopter
evidence, which does not gate a release. At that cut [#60](https://github.com/tauten-dev/tauten/issues/60), [#61](https://github.com/tauten-dev/tauten/issues/61) and [#62](https://github.com/tauten-dev/tauten/issues/62) went to `later`; see [0.2](#02) for the subsequent triage.

| Issue | ID | Item | Days | Depends on |
|---|---|---|---|---|
| [#7](https://github.com/tauten-dev/tauten/issues/7) | A3 | Manifest rule: overflow checks in release builds (TAU-H001) | 0.5 | — (in T4) |
| [#15](https://github.com/tauten-dev/tauten/issues/15) | A11 | Generated-file stage: regenerate and byte-compare | 0.5 | S4 (in D1) |
| [#6](https://github.com/tauten-dev/tauten/issues/6) | A2 | Rule metadata and fail-closed facts: no heuristic blocks or passes | 1.0 | — (in T5–T7) |
| [#8](https://github.com/tauten-dev/tauten/issues/8) | A4 | Replace the kernel shape lint with a fast extract-only stage | 1.0 | — |
| [#16](https://github.com/tauten-dev/tauten/issues/16) | A12 | Agent profile: the architecture on one page; cargo tauten new | 0.5 | A5, A6 |
| [#12](https://github.com/tauten-dev/tauten/issues/12) | A8 | Assumptions in the gate: derived, satisfiable, inherited | 1.5 | S3, A1 |
| [#14](https://github.com/tauten-dev/tauten/issues/14) | A10 | tauten-kernel-std: shared proved helpers and bounded reads | 2.0 | S4, A8 |
| [#5](https://github.com/tauten-dev/tauten/issues/5) | A1 | Registry conservation: one ratchet for every may-not-weaken list | 1.5 | S3 |
| [#10](https://github.com/tauten-dev/tauten/issues/10) | A6 | Machine: step-function trait, Lean induction library, replay | 1.5 | — |
| [#9](https://github.com/tauten-dev/tauten/issues/9) | A5 | Evidence that cannot go stale: Permit, stamps, derived bindings | 2.5 | S4 |
| [#11](https://github.com/tauten-dev/tauten/issues/11) | A7 | Atomic publish, proved: old-or-new and durable under named FS laws | 3.5 | A6, S3 |
| [#31](https://github.com/tauten-dev/tauten/issues/31) | T4 | Exceptions into the kernel; scanner sees cfg_attr, groups, tests/ | 3 | — |
| [#32](https://github.com/tauten-dev/tauten/issues/32) | T5 | Compiler mapping, rule applicability and the exit verdict, proved | 2 | — |
| [#33](https://github.com/tauten-dev/tauten/issues/33) | T6 | Structure rules and Unparsed facts decided by the kernel | 1 | — |
| [#34](https://github.com/tauten-dev/tauten/issues/34) | T7 | Scanner mutation gate and a rewritten TRUST.md | 1.5 | T4–T6 |
| [#36](https://github.com/tauten-dev/tauten/issues/36) | D2 | Cadence levels and incremental proofs: ship, verify --level | 3 | S4, D1 |
| [#37](https://github.com/tauten-dev/tauten/issues/37) | D3 | Obligations: arithmetic and indexing findings as proof obligations | 2 | D1 |
| [#38](https://github.com/tauten-dev/tauten/issues/38) | D8 | Site, README and rustdoc for 0.1.0; release | 2–3 | everything above |

About 44 days as first planned. A9 and D1 were cut to 0.2 (2026-11-21); if more must shrink, cut bounded reads in A10 next. Never cut S2, S3, S4, A1, A2 or A8: they are what makes every later badge and every "enforced" label true.

## 0.2

**Goal:** make partial adoption useful. A team should be able to try the checker without changing release policy, reuse one Lean theorem without taking a runtime dependency, or draft claims without trusting a second ledger. The [0.1.0 release run](https://github.com/tauten-dev/tauten/actions/runs/44953176654) passed the gate and full proofs and published from `719d4e9`. The public proofs image already exists.

- **Writfold:** the [main-branch adoption record](https://github.com/writfold-dev/writfold/blob/bb9d8fa637b2f4feec5445ed91d63c8db249b3a4/docs/writfold/tauten-adoption.md#L24-L37) distinguishes the non-gating lint baseline, the custom native proof gate and its recorded claims. The owner [approved overflow checks](https://github.com/writfold-dev/writfold/issues/73#issuecomment-7063879621); the [dev.15 pin change](https://github.com/writfold-dev/writfold/commit/67f11e62541feed88fd123b72e8e1010163c1fdb) uses Tauten `eb1b594`, **not** a released Writfold move to Tauten 0.1. Its [manifest](https://github.com/writfold-dev/writfold/blob/67f11e62541feed88fd123b72e8e1010163c1fdb/proofs/claims.toml#L1-L10) supplies a gate command, not the audited `[gate] try` entry that `proofs draft` needs.
- **Gripsack:** the [0.58 candidate's proof image](https://github.com/gripsack-dev/gripsack/blob/c62afc5c05087e9e261b68fce1744f0f6301eeef/proofs/Dockerfile#L96-L103) takes one checksum-pinned Lean module and its license. The [publication proof](https://github.com/gripsack-dev/gripsack/blob/c62afc5c05087e9e261b68fce1744f0f6301eeef/proofs/Protocol/Publication.lean#L95-L140) builds a total model to use world induction. Separately, its [diagnostics trial](https://github.com/gripsack-dev/gripsack/issues/383#issuecomment-7063883177) ended **advisory**: no baseline, mandatory gate, runtime change or evidence of a storage defect.

| Pickup | Issue | Return and cost | Exit |
|---|---|---|---|
| 1 | [#65](https://github.com/tauten-dev/tauten/issues/65) Rustdoc | Five known warnings; small patch, every API reader benefits. | Fix all five at their declarations; the gate runs all-features workspace docs with warnings denied. |
| 2 | [#64](https://github.com/tauten-dev/tauten/issues/64) Bounded capability reads | The one item that changes what 0.2 ships; the break is what justifies the minor bump. Reuses budget arithmetic proved in 0.1. | [Accepted read contract](rfcs/2026-11-21-bounded-capability-reads.md): `Limit<Bytes>` required, typed `TooLarge`, `compile_fail` for the 0.1 call, Breaking changelog section, `Env::only`. |
| 3 | [#60](https://github.com/tauten-dev/tauten/issues/60) `rsys_inv_of_step` | The missing fourth induction corner; one three-case proof helps any result-returning machine with a world. | [Accepted lemma](rfcs/2026-11-21-result-world-induction.md) plus a non-vacuous miniature, audited and mirrored in the template. Adopter trial is [#69](https://github.com/tauten-dev/tauten/issues/69) (later). |
| 4 | [#68](https://github.com/tauten-dev/tauten/issues/68), [#66](https://github.com/tauten-dev/tauten/issues/66) Website and onboarding | #68: Rules headings, feedback link, README wording against the 0.1.0 pin. #66: advisory trial, opt-in CI ratchet and proof-only reuse. | #68 live now; #66 with source examples in core and `build.py --check` green. Re-pin and captures belong to the release issue. |
| 5 | [#61](https://github.com/tauten-dev/tauten/issues/61) Custom claim inventory | Evidence only; capped at half a day on a disposable Writfold clone. | A matching draft (RFC `complete`) or a recorded **no fit** with specific mismatches filed (RFC stays `partial`). Never gates a tag. |
| 6 | [#62](https://github.com/tauten-dev/tauten/issues/62) Real merge reconciliation | Stretch: tauten-only evidence for a shipped command covered only by a stand-in test. | As in the issue; non-blocking. |

**Release line:** [#70](https://github.com/tauten-dev/tauten/issues/70) cuts 0.2.0 once #65, #64 and #60 are closed: Breaking changelog section, publish order with the new `tauten → tauten-check-kernel` edge, gate and full proofs on the tag commit, then the site re-pin. #61, #62, #66 and the adopter follow-ups may carry forward. Neither adopter has to migrate, ship or weaken its gate; never turn a missing comparison into a parity claim.

Moved to later during grooming: [#35](https://github.com/tauten-dev/tauten/issues/35) (shared-gate fit). Its positive exit is entirely adopter-side and its no-fit exit (public image, native gates kept) already describes today.

## Later (not scheduled)

The [later milestone](https://github.com/tauten-dev/tauten/milestone/4) keeps these issues open. The five-primitives design and original acceptance controls still apply; the change is what warrants picking each one up.

| Issue | Defer until / cheaper alternative |
|---|---|
| [#13](https://github.com/tauten-dev/tauten/issues/13) Storage driver | Map two real routines to the same commands first. Keep a staged short write followed by failure and ordering of later durable replacements; don't retrofit Gripsack's filesystem to a simulator. The owner already chose a `tauten` feature. |
| [#17](https://github.com/tauten-dev/tauten/issues/17) Multi-world explorer | First get a useful single-crash driver from #13. Then show a planted fault and replay in a second world; a finite search is bounded evidence only. |
| [#18](https://github.com/tauten-dev/tauten/issues/18) Recovery induction | Fit two protocols and name interference and recheck assumptions. Preserve [Gripsack's visible-but-not-durable counterexample](https://github.com/gripsack-dev/gripsack/issues/364#issuecomment-7063981545); a local crash model isn't a host guarantee. |
| [#19](https://github.com/tauten-dev/tauten/issues/19) Extensible effect rows | Bring an adopter-owned capability/label the fixed table cannot express. Try one generated table before a new configuration language. |
| [#20](https://github.com/tauten-dev/tauten/issues/20) Stale results / replay | Map two existing guards; if their delivery semantics differ, try the smaller `once` kernel. Explorer-dependent acceptance waits for #17. |
| [#22](https://github.com/tauten-dev/tauten/issues/22) Record invariants | Start with one actual cross-field validator and a private checked constructor. Justify a macro by what that cannot do ergonomically; don't call arbitrary predicates proved. |
| [#23](https://github.com/tauten-dev/tauten/issues/23) Claim ledger | Compare a rendering of an existing reviewed record to [Writfold’s published assurance map](https://github.com/writfold-dev/writfold/blob/010420beb1174a610754a555429fa1da8adc143b/docs/guide/assurance.md). [Gripsack’s ledger](https://github.com/gripsack-dev/gripsack/issues/313) still depends on protocol evidence. Rendering does not require a new inventory or verdict. |
| [#35](https://github.com/tauten-dev/tauten/issues/35) Shared-gate fit | Schedule only when an adopter asks to delete its fork. Writfold now has recorded inputs; that is not a two-adopter comparison. The exact direct-failure fixture shipped in #39. |
| [#69](https://github.com/tauten-dev/tauten/issues/69) `rsys_inv_of_step` adopter trial | #60 shipped. Await a real adopter proof using a pin that contains the lemma, or a documented no-fit; no downstream pin change is required. |
| [#71](https://github.com/tauten-dev/tauten/issues/71)–[#74](https://github.com/tauten-dev/tauten/issues/74) Custom inventory | Four distinct authoring gaps, not four failed proofs. If an adopter asks to retry [the 108/160 comparison](https://github.com/tauten-dev/tauten/issues/61#issuecomment-7071276444), start with #73’s missing checked subject. Keep total selection (#71), shared mutants (#72) and source-site-to-item mapping (#74) distinct. |
| [#76](https://github.com/tauten-dev/tauten/issues/76), [#82](https://github.com/tauten-dev/tauten/issues/82) Mutant cost | After #79, measure selective Lake rebuilds and exact verdicts before designing a cache. CPU allocation for #82 landed with the [bounded-build-resources RFC](rfcs/2026-11-28-bounded-build-resources.md) (container limits, jobs × Lean threads within the allocation); #82 closes after the qualification pair (`TAUTEN_MUTANT_JOBS=1` against the default budget) shows equal records. Changed-claim-first full runs stay parked in #30 pending a measured feedback delay after preflight. |
| [#24](https://github.com/tauten-dev/tauten/issues/24) Spans and edit maps | Show a second caller with matching units and a translatable dependency; start with `bind_span` in the existing kernel, not a new crate. |
| [#25](https://github.com/tauten-dev/tauten/issues/25) Schema locks | Start from 0.1's version-admission function. Require a versioned decoder with a real unmet shape-drift need before adding a macro and lockfile. |
| [#26](https://github.com/tauten-dev/tauten/issues/26) Process capabilities | Wait for the `proc` label (#19) and an actual attenuation use; `classify` alone need not grow into a supervisor. The scenario runner ([#103](https://github.com/tauten-dev/tauten/issues/103)) is a first consumer for `classify` if its process-result mapping outgrows a three-arm match. |
| [#27](https://github.com/tauten-dev/tauten/issues/27) Digest framing | Require a caller and a migration for persisted digests; keep the hash assumption explicit. |
| [#28](https://github.com/tauten-dev/tauten/issues/28) Undo journals; [#29](https://github.com/tauten-dev/tauten/issues/29) external effects | Build on #18, and #20 for replay, only when the actual protocols fit. An abstract simulation or `once` is preferable to an unjustified host guarantee. |
| [#105](https://github.com/tauten-dev/tauten/issues/105), [#106](https://github.com/tauten-dev/tauten/issues/106) Scenario trials: Strop, Gripsack | After [#103](https://github.com/tauten-dev/tauten/issues/103) and [#112](https://github.com/tauten-dev/tauten/issues/112) are on `main`. Adopter evidence for whether the runner's interface survives two unrelated executables; the laws stay project-owned. If Hypothesis inside Gripsack's pytest suite is the cheaper home, record the no-fit and move that work to Gripsack. Never gates a tauten release. |
| [#107](https://github.com/tauten-dev/tauten/issues/107) Scenario trial: Writfold gate freshness | After [#112](https://github.com/tauten-dev/tauten/issues/112), which tests the same binding property on tauten itself. A third external trial adds little to the generality question; pick it up when Writfold wants the test, not to complete a set. |
| [#104](https://github.com/tauten-dev/tauten/issues/104) Per-case Docker backend | When a retained trial records contamination or host exposure that a fresh directory, a cleared environment and the gate container did not prevent. Tested-only edge code; [backend RFC](rfcs/2026-11-27-scenario-environments-and-replay.md). |
| [#109](https://github.com/tauten-dev/tauten/issues/109), [#110](https://github.com/tauten-dev/tauten/issues/110) Behavioural contracts across a boundary | [#110](https://github.com/tauten-dev/tauten/issues/110) (Strop) after [#96](https://github.com/tauten-dev/tauten/issues/96): evidence for or against a subject-bound stamp on a real boundary. [#109](https://github.com/tauten-dev/tauten/issues/109) when [#23](https://github.com/tauten-dev/tauten/issues/23) gives tested laws a place in the record and a second boundary in another project shows reuse; the composition decision is already proved ([contracts RFC](rfcs/2026-11-27-component-contract-testing.md)). Until then the three-check method is guidance, not a tauten feature. |
| [#30](https://github.com/tauten-dev/tauten/issues/30) Parked/rejected register | Index, not release work. Its recorded triggers, not a milestone move, reopen an idea. |

## Dropped from earlier plans

- **TAU-K004 (kernel proof-shape lint) and a shape-checking `#[kernel]` macro.** They would predict what the translation toolchain accepts, with false negatives, and the rule set would follow every toolchain release. Running the translation decides; see A4.
