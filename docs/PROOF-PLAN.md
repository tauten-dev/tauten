# tauten: Lean proofs for its own enforcement code (plan)

2026-11-18. Planner output only: nothing was compiled, extracted or run. Every
statement about what Aeneas will accept from tauten code is a prediction from
reading the sources and writfold's recorded experience; P0 exists to test them.

**Status (2026-11-18):**
- **P0 done.** Results are in `docs/proofs/probe.md`. The probe was retired
  in P1.
- **P1 done locally.**
  - `crates/tauten-check-kernel` exists, and `tauten-check` calls it.
  - 13 required claims are proved and audited.
  - The gate also runs a drift check and 9 mutants.
  - `.github/workflows/proofs.yml` is green on main (see `docs/STATUS.md`).
  - Details: `docs/proofs/README.md`.
- **P2 done locally** (branch `proofs-p2`, not yet merged or run on
  GitHub). The ratchet decision is in `tauten_check_kernel::ratchet`;
  `tauten-check` interns keys and calls it; the old `BTreeMap` code is the
  reference of a differential test. Theorems R-conserve, R-cover, R-stale,
  R-exact, R-headroom and R-shrink are audited claims (14 new, 27 in all),
  with non-vacuity examples; 8 new mutants (17 in all). Deviation from P1's
  discipline: the ratchet claims share refinement lemmas, so the mutant
  calibration counts a claim as broken when it depends on a failed lemma
  (`docs/proofs/README.md`). Interning is trusted and tested, as planned.
- **P3 done locally** (branch `proofs-p3`, not yet merged or run on
  GitHub). `crates/tauten-proof-instances` (unpublished) applies the real
  macros: 4 refined types, a `#[newtype]` control with `derive(Default)`,
  4 contract functions (one with a `&mut` parameter, one whose body would
  panic without the precondition). Charon/Aeneas translate the expansion
  as is: no macro change was needed (the kill criterion did not apply).
  A schematic shape per macro (`Tauten/MacroShapes.lean`), a `*_shape`
  theorem per instance, F01 targets 1, 2, 3 and 5 per refined type
  (target 5 also as the constructor audit, with the control showing it
  sees derived impls), the contract claims (`Requires` iff the
  precondition is false and the body is not evaluated; `Ensures` iff the
  body's `Ok` fails the postcondition; otherwise the body's result;
  `old(..)` is the pre-state). 60 new audited claims (87 in all), 10
  macro-source mutants, all killed. Wording: "proved for listed
  instances", never "macros verified" (`docs/proofs/README.md`).
- **P4 done locally** (branch `proofs-p45`, not yet merged or run on
  GitHub). `tauten_check_kernel::table::validate` decides rule-table
  well-formedness over interned names; `Profile::parse` calls it (the
  plain-Rust fix for missing emitted IDs was already on main). Claims
  `validate_ok_iff` (accepted iff IDs unique, lints owned once, API bans
  name existing rules, every emitted ID present; never panics) and
  `validate_examples` (89 claims in all); 5 mutants, all killed (32 in
  all). The order in which errors are reported is checked by a
  differential test against the old code, not proved. Reject fixture:
  `fixtures/profiles/rej-missing-r001.toml`.
- **P5 done locally** (branch `proofs-p45`, not yet merged or run on
  GitHub). `proofs/claims.toml` (function → theorems → mutants) and
  `cargo tauten proofs`, which runs a project's gate and prints one JSON
  outcome per claim (`proved`, `drift`, `build-failed`, `audit-failed`,
  `mutant-survived`, `tool-missing`; without the toolchain everything is
  `tool-missing`) and records them, with a source fingerprint, in
  `proofs/claims.json` (also the website's badge data; tauten's own file
  lists 20 claims). Template `crates/cargo-tauten/templates/proofs/` and
  `cargo tauten init --proofs <crate>`, instantiated on
  `bench/hosts/ledger` (`apply_spec`, 2 mutants, `proved`). In kernel
  crates, TAU-N001/P003 findings inside a function whose claim is `proved`,
  has a total no-failure theorem (shape checked by the template's audit),
  lists the rule and matches the current fingerprint are reported as
  subsumed by the proof; the decision is the new kernel function
  `subsume::subsumes` (4 claims, 4 mutants; 93 claims, 36 mutants in all).
  `proofs/TRUST.md` is the trust list. Not done: CI does not yet run
  `cargo tauten proofs` or check that `claims.json` is current; the site
  does not yet read `claims.json`.

Sources read: tauten `crates/*/src`, `docs/STATUS.md`, `docs/PLAN.md` section 4,
`docs/reviews/plyward-2026-11-18.md`; writfold `proofs/` (Dockerfile, extract.sh,
gate.sh, Audit.lean, mutants.py, lakefile.toml), `compose.proofs.yaml`,
`docs/writfold/aeneas.md`, `crates/writfold-core/Cargo.toml`; Plyward
`spec/REFINEMENTS.md`, `proofs/PROOF-PLAN.md`.

## 0. Position

- The earlier Plyward review said "Lean gates for rules, macros or the checker
  repeat Sovel". That stays true for *admission gates*. What the owner asks for
  now is different and sound: post-hoc theorems over the shipped decision code,
  one slice at a time, in a separate workflow that never blocks unrelated work.
- Honest limit: of today's two bugs, proofs over extracted code would have
  caught at most one. The `derive(Default)` bypass is catchable (section 2,
  constructor enumeration audit). The A002 alias/fn-pointer bypass is not: it
  was a wrong *model* of "minting authority" (syntax instead of resolution), and
  a proof of the syntactic scanner against its own spec would have passed.
  Proofs remove arithmetic/branch/ordering bugs in decisions; they do not
  remove threat-model gaps. Review and negative fixtures stay.
- Reuse writfold's pins unchanged: Lean v4.35.0, Aeneas
  `nightly-2026.11.07-c5eb4a2`, Charon `8cae8fd` built from source on
  `nightly-2026-09-17`, elan v4.2.4, linux/amd64 only. That nightly is newer
  than tauten's 1.98 (2026-08-18), so edition 2024 sources should load.

## 1. What to prove, ranked (bug-risk x feasibility)

Aeneas limits as writfold actually hit them (`aeneas.md`, "Writing kernels for
translation"): owned data, no nested borrows; no derives in translated modules
(derived `==` replaced by `matches!` or an explicit `same`); no integer casts;
explicit `while` loops (no iterator chains or closures: the iterator version of
a comparator was kept only as a test reference); one loop per function when it
returns early; flat matches; loops updating several variables under nested
`if`s fail ("could not match the contexts"); `&str`/`String` functions are kept
outside `start_from`; a dependency-free `no_std + alloc` crate; functions named
one by one in `start_from` when a module must keep derives.

Consequence: **nothing in tauten-check is extractable as written.** It uses
`BTreeMap<Key, u32>` with `String` keys, iterator chains with closures, `serde`
derives, `format!`, let-chains, and `syn`/`toml` dependencies that Charon would
have to compile. `Tier::may_depend_on` is `dep <= self` on a derived
`PartialOrd`, which is exactly the derived-comparison case writfold rewrote.

Refactor: a new dependency-free `no_std + alloc` crate `tauten-check-kernel`
(tier `kernel`, must be published because `tauten-check` depends on it).
`tauten-check` keeps parsing, strings, rendering and interning, and calls the
kernel for every decision. The old implementations stay as `#[cfg(test)]`
references for a differential test (writfold's pattern).

| # | Target | Bug risk | Extractable today | Kernel shape | Verdict |
|---|---|---|---|---|---|
| 1 | Baseline ratchet (`baseline::apply`, `increases`, `from_findings`) | High: counting, saturation, stale detection, the adoption path every user takes | No (BTreeMap, String keys, closures) | Shell interns each distinct `Key` to an index. Kernel: `apply(accepted: Vec<u32>, findings: &[Finding{key: usize, error: bool}]) -> (covered mask, left: Vec<u32>)`; `increases(old, next)`; `verdict` | **P2. Highest value.** |
| 2 | Severity and exception decisions (`from_compiler` level mapping, `Ctx::emit`, `exceptions` -> X004/X002) | Medium: small but it is the line between "blocks" and "passes"; X004's kernel clause is easy to invert | No as written, trivial to move | `severity(level: Option<Level>, compiler: Severity) -> Severity`; `exception(rule_allows, tier, has_level, crate_level, blocking) -> Excuse{Forbidden, CrateWide, Allowed}` | **P1** with #3 |
| 3 | Tier relation (T002) | Low: three-value enum | After replacing derived `<=` with a `rank` match | `may_depend_on(a, b)` | **P1.** Cheap; its job is to prove the pipeline, not to find bugs |
| 4 | `#[refined]` constructor (Plyward F01 targets 1, 2, 3, 5) | High: today's bug class | Unknown: expansion calls generic `holds(&value, |v| ..)` with an `FnOnce` closure | Instances in an unpublished crate; closure-free expansion if P0 says so | **P3**, gated on P0 |
| 5 | Contract expansion (`requires`/`ensures`, `old`) | Medium-high: ordering of checks, `old` evaluated before the body, Err-passthrough | Unknown: body wrapped in an immediately-invoked closure | Same instances crate | **P3**, gated on P0 |
| 6 | Profile well-formedness (`Profile::parse` checks: duplicate id, duplicate lint owner, unknown rule) plus *required structural IDs exist* | Medium: see latent defect below | No (strings, BTreeSet) | Interned: `validate(rule_lints: &[Vec<u32>], api_rule: &[Option<usize>], required: &[bool])` | **P4**, after a plain-Rust fix |
| 7 | Diagnostic mapping (`rule_for`: disallowed lint note -> `TAU-` id; `is_summary`) | Medium, but it is string splitting on clippy's message format | No, and not worth it | none | **Do not prove.** Property tests plus a pinned-clippy fixture. The risk is clippy changing its text, which no theorem covers |
| 8 | `scan.rs` (syn visitors), wrapper, workspace loading | High, but it is IO and third-party ASTs | No | none | **Do not prove.** Fixtures and mutation testing |

Latent defect found while reading (fix in plain Rust first, then make it
theorem 6): `baseline::apply` does `if let Some(rule) = profile.rule("TAU-R001")`
and `Ctx::emit` does `let Some(rule) = ... else { return }`. A rule table that
lacks a structural ID makes stale baselines and structural violations pass
silently. `Profile::parse` should reject a table missing any ID the code emits.
Not verified by running; read from `baseline.rs` and `structure.rs`.

Theorems for #1 (statement level, each with an `_iff` or a witness):
- R-conserve: every finding is either covered or reported; reported is the
  input with covered positions removed, order kept; nothing is duplicated.
- R-cover: per key, covered = min(accepted, errors now); a warning is never
  covered.
- R-stale: `left[k] > 0` iff accepted[k] > errors now[k]; one R001 per such key.
- R-exact: the check passes iff no uncovered error exists and no key is stale.
- R-headroom: from a passing state, adding one error finding fails.
- R-shrink: `increases(old, next)` is exactly the keys with next > old; a
  baseline regenerated from a passing run has no increases.
- Trusted and stated: interning is injective (differential test against the
  BTreeMap reference, not a theorem).

## 2. How proofs bind to macro output

- `tauten-macros` itself (proc-macro, `syn`, `quote`) cannot be extracted. A
  theorem about the generator ("for every predicate, the emitted tokens...")
  needs a Lean model of token output and of Rust elaboration. That is the
  Sovel/Plyward trap; do not attempt it.
- Charon works after expansion, so no hand-expanded copy is needed: an
  unpublished kernel-tier crate `tauten-proof-instances` applies the real
  macros (`Percent(u8)` with `v <= 100`, a `u16` range, a length-bounded
  `Vec<u8>`, the ledger `apply` with `#[requires]`/`#[ensures]`/`old`), and the
  extraction is the macro's actual output.
- Three bindings make instance proofs say something about the generator:
  1. A schematic Lean lemma proved once over an arbitrary total predicate `p`
     (`try_new_shape p x = ok iff p x`, value preserved). Each instance theorem
     is "extracted `try_new` equals the shape at this `p`", closed by `simp`.
     If the macro changes shape, every instance breaks.
  2. Mutants applied to `tauten-macros/src/lib.rs` (swap the `Ok`/`Err`
     branches, drop the `!` in the precondition check, take `old` after the
     body, skip the postcondition). Each must break a named instance theorem.
     This is the evidence that instance proofs constrain the generator.
  3. Constructor enumeration (F01 target 5): a Lean meta check in `Audit.lean`
     lists every extracted definition whose result type mentions the refined
     type and fails unless each is in a reviewed allowlist (`try_new`,
     `try_from`, `clone`). If Charon extracts derived impls, a
     `derive(Default)` would appear as an unlisted constructor. Whether it
     does is a P0 question.
- Honest website wording: "The constructor and contract wrapper generated for
  these N instance types are proved in Lean over the extracted Rust. The
  macro is covered by those instances, by mutants of the macro source, and by
  compile-fail tests. It is not proved for every input." Never "the macros
  are verified". User types get the same proof only if the user runs the proof
  tier on their own kernel crate.
- Not provable through this route, to be listed in `proofs/TRUST.md`: privacy
  of the generated module (rustc), the derive/attribute allowlist being
  complete, const-versus-runtime agreement (F01 target 6), user predicates
  that panic or diverge.

## 3. Charon as a checker frontend

Recommendation: **no. Keep Charon inside the proof tier.**

- Cost: Charon is built from source at one commit against one nightly with
  `rustc-dev`, `rust-src` and `miri`; writfold's Dockerfile notes no release
  exists at the pinned commit and the LLBC version moved between adjacent
  nightlies. x86_64 only. It must compile the whole target crate and its
  dependencies with that nightly, so a user crate with a newer MSRV, a build
  script quirk or a proc-macro ABI mismatch fails to load. tauten's promise is
  "one `cargo check` on the user's stable toolchain"; Charon breaks that.
- Coverage: Charon/Aeneas reject large parts of ordinary Rust (trait objects,
  much closure and iterator code, async). A frontend that cannot load `core`
  and `edge` crates is useless for the rules that live there (authority, IO,
  global state).
- Benefit already available: the rustc/clippy wrapper resolves paths.
  `clippy::disallowed_methods` matches on resolved definitions, so aliases and
  `use ... as` are handled there. A002 should be enforced through that route
  (ban `tauten::Authority::acquire` outside `edge`), with the syn scan kept
  only as a fast hint. Whether clippy also flags a bare path used as a
  function value (`let f = Authority::acquire;`) must be confirmed by the
  existing `rej-authority-alias` fixture plus a new fn-pointer fixture; I did
  not run it.
- Where Charon does belong: (a) extraction for proofs; (b) later, optional:
  because the proof gate already has the LLBC of kernel crates, a small check
  over it could replace the duplicated N001/P003 lints for functions that
  have a claim (the dry-run finding). Only after P5, only for kernel crates.
- If resolution-level custom checks are ever needed beyond clippy lists, the
  candidate is dylint on stable-pinned clippy (already "later" in PLAN), not
  Charon.

## 4. Refinement surface, existentials, async

- **Predicate fragment (do it, small).** Keep `#[refined(|v| ...)]`. The macro
  classifies the closure body: comparisons, `&&`, `||`, `!`, integer literals,
  named constants, `*v`, `v.len()`; no calls, no arithmetic, no floats in the
  first step. In-fragment predicates expand without a closure
  (`let v = &value; if <body>`), which also removes the main extraction risk.
  An opt-in `#[refined(provable, |v| ...)]` makes leaving the fragment a
  compile error. Out-of-fragment predicates stay runtime-checked and are
  labelled "tested", never "proved". Arithmetic enters only with checked
  operations and a real example.
- **Existentials: no new syntax.** A refined newtype already is the package
  "some raw value such that P". `impl Trait` exists in Rust and adds nothing
  checkable here; trait-heavy code is also where Aeneas is weakest. Refinements
  over several values (index into a specific collection) need a stability rule
  for the context; keep out, as Plyward's own text advises.
- **Generic newtypes:** defer until an example needs them.
- **Async contracts (small, tested only).** Support `async fn` by wrapping the
  body in an `async move` block instead of the closure. Stated semantics:
  `requires` and `old` snapshots are evaluated at first poll, `ensures` only on
  completed `Ok`; a cancelled future checks no postcondition; nothing is
  claimed about state between awaits. No Lean claim: Aeneas does not translate
  async. Add compile-fail and behaviour tests, including a cancellation test.
  No cross-await invariants.

## 5. Milestones

Sizes are agent-days. Each milestone merges on its own; none blocks release
work. Total 9.5 to 12.5 days if nothing is killed.

| # | Content | Size | Exit criterion | Kill criterion |
|---|---|---|---|---|
| P0 (**done**) | Toolchain reuse and probe on a branch. Copy writfold's `proofs/` skeleton and pins. Probe crate with: tier `rank`; a ratchet loop over `Vec<u32>`; `Percent(u8)` through the real macro; one `requires`/`ensures`/`old` function; ledger `apply` (`checked_add`, `ok_or`); a `derive(Default)` control | 0.5 to 1 | A table in `docs/proofs/probe.md`: per item, extracts or not, with the Charon/Aeneas error text. Decision recorded: closure-free expansion needed or not; derived impls visible or not | Image cannot be built or the probe crate cannot be loaded within 1 day: stop, report, keep proofs to user kernels (M3) only |
| P1 (**done locally; CI run pending**) | `tauten-check-kernel` with tier relation, severity and exception decisions; `tauten-check` calls it. `proofs/` with extract, drift check, `lake build`, `Audit.lean`, at least 4 mutants; proof workflow in CI | 2 | Proof workflow green on main; theorems: `may_depend_on` is a total order agreeing with rank, kernel depends only on kernel; `severity_spec`; `exception_spec` with "kernel never excuses a levelled rule"; each mutant breaks exactly its theorem | More than 3 days, or the gate takes over 30 minutes warm |
| P2 (**done locally**) | Ratchet in the kernel, interning in the shell, differential test against the old BTreeMap code, theorems R-conserve to R-shrink, at least 6 mutants | 3 | Theorems in the audit claims list; existing `tests/ratchet.rs` and writfold dry-run numbers unchanged | More than 5 days: ship the kernel refactor with the differential test and R-conserve only; record the rest as open |
| P3 (**done locally**) | `tauten-proof-instances`; closure-free expansion if P0 requires it; schematic lemma; F01 targets 1, 2, 3; constructor enumeration (target 5) if P0 showed derived impls are visible; contract theorems (Err(requires) iff precondition false, body not evaluated then; Err(ensures) iff body Ok and postcondition false; otherwise the body's result; `old` is the pre-state value); macro-source mutants | 2 to 3 | At least 3 refined instances and 2 contract instances proved; at least 4 macro mutants killed | Expansion cannot be made extractable without changing user-visible behaviour: stop, label macros "tested" on the site |
| P4 (**done locally**) | Plain-Rust fix: `Profile::parse` rejects tables lacking emitted IDs. Then interned validator in the kernel with `validate_ok_iff` (unique ids, unique lint owner, bans reference existing rules, required IDs present) | 1.5 | Theorem plus 3 mutants; a reject fixture for a table without TAU-R001 | More than 2 days: keep the fix and tests, drop the theorem |
| P5 (**done locally**) | Fold into M3: the same `proofs/` files become the user template; `proofs/claims.toml` (function -> theorem -> mutants); `cargo tauten proofs` JSON outcomes `proved, drift, build-failed, audit-failed, mutant-survived, tool-missing`; `proofs/TRUST.md`; website badge data generated from the audit | 1 to 2 | Template instantiated on `bench/hosts/ledger` kernel with one theorem; site badge list generated, not hand-written | Existing PLAN rule: template not done in 3 days, ship without |

### CI integration

- New `compose.proofs.yaml` and `.github/workflows/proofs.yml`, separate from
  the `gate` workflow. Triggers: changes under `crates/tauten-check-kernel/**`,
  `crates/tauten-proof-instances/**`, `crates/tauten-macros/**`, `proofs/**`;
  plus a nightly run and `workflow_dispatch`. Not a required check for pull
  requests that touch none of those paths.
- Release workflow: require the proof workflow green on the tagged commit,
  because the badge refers to that commit.
- Image: writfold's Dockerfile builds Charon from source and fetches the
  mathlib cache, so a cold build is long. Use the buildx GitHub Actions cache
  first. Pushing a prebuilt image to a registry is a publishing step and needs
  the owner's go-ahead. Note writfold has no GitHub workflow for its proofs
  gate (it runs through Compose locally), so the CI wiring is new work and its
  timing is unknown until P1.
- Gate order, as in writfold: extract, refuse drift against committed
  `proofs/Extracted/`, `lake build`, axiom audit (transitive axioms within
  `propext`, `Classical.choice`, `Quot.sound`; required claims must exist),
  mutant calibration (each mutant fails exactly its named theorems; every
  named theorem is a required claim).
- `cargo fmt` before refreshing the extraction: it records line numbers.

### The "Proved" badge rule

A rule, function or feature shows "Proved" only when all hold at the released
commit:
1. the theorem is in `Audit.lean`'s required claims and its axioms are within
   the standard three;
2. it is stated over the committed extraction of the shipped crate and the
   drift check passed;
3. at least one mutant of the Rust source is killed by it;
4. production calls the extracted function (no model copy); where a shell
   step is trusted (interning, parsing), the badge text says so;
5. the badge names the function and links the statement and `TRUST.md`.

Other labels, used as-is: "Proved for listed instances" (macros), "Tested",
"Lint-enforced", "Convention". Never "formally verified tauten", and no badge
on A002, diagnostics mapping, scanning or async contracts.

## 6. Open questions P0 must answer

1. Does Aeneas translate `holds(&value, |v| ...)` and the immediately-invoked
   body closure? (Predicted: unreliable; plan assumes closure-free expansion.)
2. Are derived trait impls on a refined type visible in the extraction when
   the generated module is in `start_from`, and do they translate or abort?
3. Are `checked_add`, `checked_sub`, `saturating_sub` and `Option::ok_or`
   modelled by the pinned Aeneas library, or do they become opaque
   declarations (which the audit would then reject as axioms)?
4. Does the shadow-workspace trick in `extract.sh` work with tauten's
   `resolver = "3"` and `rust-version` inheritance?
5. Cold and warm gate time on a 2-CPU GitHub runner.
