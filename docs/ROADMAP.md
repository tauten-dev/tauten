# Roadmap

Every item is a GitHub issue. An item is accepted only if it is **general** (useful across projects, not a check for one codebase), **decided by the proved kernel or the compiler** (anything weaker is labelled bounded, tested or heuristic, never "enforced"), and makes the flow **lighter**. Ideas that did not pass are in the [parked and rejected tracking issue](https://github.com/tauten-dev/tauten/issues/30), each with the trigger that would reopen it.

Designs behind items, and any new proposal that is costly to reverse or changes decision logic, are [RFCs](rfcs/README.md); the roadmap itself is the [five-primitives RFC](rfcs/2026-11-19-roadmap-five-primitives.md).

Order within a milestone is by return on investment: projects helped × strength of the guarantee ÷ cost. Sizes are estimates in agent-days.

**Released:** 0.2.0 (2026-11-22): #65, #64, #60; #61 closed as no fit (#71–#74); #66, #62 carry forward. [Live pickup order and issues](https://github.com/tauten-dev/tauten/milestone/3).

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
| [#23](https://github.com/tauten-dev/tauten/issues/23) Claim ledger | Try rendering an existing audited record first: [Writfold's assurance page](https://github.com/writfold-dev/writfold/issues/51) needs accurate coverage; [Gripsack's ledger](https://github.com/gripsack-dev/gripsack/issues/313) waits for protocol proofs. Neither request by itself warrants new verdicts or a second inventory. |
| [#35](https://github.com/tauten-dev/tauten/issues/35) Shared-gate fit | Reopen when an adopter asks to delete its fork. The exact direct-failure fixture already shipped in #39. |
| [#69](https://github.com/tauten-dev/tauten/issues/69) `rsys_inv_of_step` adopter trial | After #60 ships; adopter-side evidence. |
| [#24](https://github.com/tauten-dev/tauten/issues/24) Spans and edit maps | Show a second caller with matching units and a translatable dependency; start with `bind_span` in the existing kernel, not a new crate. |
| [#25](https://github.com/tauten-dev/tauten/issues/25) Schema locks | Start from 0.1's version-admission function. Require a versioned decoder with a real unmet shape-drift need before adding a macro and lockfile. |
| [#26](https://github.com/tauten-dev/tauten/issues/26) Process capabilities | Wait for the `proc` label (#19) and an actual attenuation use; `classify` alone need not grow into a supervisor. |
| [#27](https://github.com/tauten-dev/tauten/issues/27) Digest framing | Require a caller and a migration for persisted digests; keep the hash assumption explicit. |
| [#28](https://github.com/tauten-dev/tauten/issues/28) Undo journals; [#29](https://github.com/tauten-dev/tauten/issues/29) external effects | Build on #18, and #20 for replay, only when the actual protocols fit. An abstract simulation or `once` is preferable to an unjustified host guarantee. |
| [#30](https://github.com/tauten-dev/tauten/issues/30) Parked/rejected register | Index, not release work. Its recorded triggers, not a milestone move, reopen an idea. |

## Dropped from earlier plans

- **TAU-K004 (kernel proof-shape lint) and a shape-checking `#[kernel]` macro.** They would predict what the translation toolchain accepts, with false negatives, and the rule set would follow every toolchain release. Running the translation decides; see A4.
