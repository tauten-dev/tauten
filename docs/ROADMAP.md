# Roadmap

Every item is a GitHub issue. An item is accepted only if it is **general** (useful across projects, not a check for one codebase), **decided by the proved kernel or the compiler** (anything weaker is labelled bounded, tested or heuristic, never "enforced"), and makes the flow **lighter**. Ideas that did not pass are in the [parked and rejected tracking issue](https://github.com/tauten-dev/tauten/issues/30), each with the trigger that would reopen it.

Designs behind items, and any new proposal that is costly to reverse or changes decision logic, are [RFCs](rfcs/README.md); the roadmap itself is the [five-primitives RFC](rfcs/2026-11-19-roadmap-five-primitives.md).

Order within a milestone is by return on investment: projects helped × strength of the guarantee ÷ cost. Sizes are estimates in agent-days.

**Next release:** [0.2 — open issues](https://github.com/tauten-dev/tauten/milestone/3).

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
evidence, which does not gate a release. [#60](https://github.com/tauten-dev/tauten/issues/60), [#61](https://github.com/tauten-dev/tauten/issues/61) and [#62](https://github.com/tauten-dev/tauten/issues/62) are in later.

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

| Issue | ID | Item | Days | Depends on |
|---|---|---|---|---|
| [#13](https://github.com/tauten-dev/tauten/issues/13) | A9 | tauten::durable: Storage, publish driver, DirLock, crash-sim tests | 3.0 | A7, A8 |
| [#35](https://github.com/tauten-dev/tauten/issues/35) | D1 | Shared proof gate: public image, audit library, tauten-gate | 5–6 | S2, S3, A8, A11 |
| [#19](https://github.com/tauten-dev/tauten/issues/19) | B3 | Effect rows over a declared label table (proc, rand, project caps) | 2.5 | — |
| [#22](https://github.com/tauten-dev/tauten/issues/22) | B7 | Record invariants: #[refined] for structs | 2.0 | — |
| [#20](https://github.com/tauten-dev/tauten/issues/20) | B4 | Admission kernels: Gate for stale results, once for replays | 3.0 | A6, B1 |
| [#23](https://github.com/tauten-dev/tauten/issues/23) | B10 | Claim ledger: resolved symbols, generated table, bounded kinds | 3.0 | S3 |
| [#17](https://github.com/tauten-dev/tauten/issues/17) | B1 | Explorer: machines against crash, delivery and cancel worlds | 3.5 | A6, A9 |
| [#18](https://github.com/tauten-dev/tauten/issues/18) | B2 | Crash-recovery induction and a guarded single-file replace | 6.0 | A7, B1 |

## Later (accepted, not scheduled)

| Issue | ID | Item | Days | Depends on |
|---|---|---|---|---|
| [#27](https://github.com/tauten-dev/tauten/issues/27) | L4 | Framed digest preimages and fingerprint soundness | 2.0 | S4, B2 |
| [#25](https://github.com/tauten-dev/tauten/issues/25) | L2 | #[schema] and schemas.lock on the version admission kernel | 2.5 | A1, A10 |
| [#24](https://github.com/tauten-dev/tauten/issues/24) | L1 | Spans and edit maps in tauten-kernel-std | 2.5 | A10 |
| [#26](https://github.com/tauten-dev/tauten/issues/26) | L3 | Process capability: classify, Spawner::only, mandatory limits | 2.5 | B3 |
| [#29](https://github.com/tauten-dev/tauten/issues/29) | L6 | At-most-once external effect machine | 5.0 | B2, B4 |
| [#28](https://github.com/tauten-dev/tauten/issues/28) | L5 | Undo-journal simulation theorem and reference machine | 7.0 | B2 |

## Dropped from earlier plans

- **TAU-K004 (kernel proof-shape lint) and a shape-checking `#[kernel]` macro.** They would predict what the translation toolchain accepts, with false negatives, and the rule set would follow every toolchain release. Running the translation decides; see A4.
