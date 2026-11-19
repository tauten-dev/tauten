<!-- Planner plan (2026-11-19): prove the toolkit's own decisions. Owner requirement: proofs first. -->

# Toolkit proof plan for tauten (`~/workspace/tauten`, branch `discipline-d0`)

Read-only research: nothing was compiled or run, and every defect below is read from source, not reproduced. Paths are relative to `/home/tarek/workspace/tauten/`.

**State that matters first:** `crates/tauten-check-kernel/src/effects.rs` (the TAU-F001 decision) already exists, uncommitted and listed in the crate's `start_from`. It has no Lean file, no entry in `proofs/claims.toml` and no mutants, and no scanner calls it yet. `#[kernel]` and TAU-K004 do not exist yet. Proofs-first is still possible for both.

## 1. Inventory

P = proved. M = unproved, movable into the kernel. N = not provable by Aeneas, with the strongest substitute.

| Decision | Where | Status |
|---|---|---|
| Tier order, severity, exception, ratchet, table validity, subsumption | `crates/tauten-check-kernel/src/{tiers.rs:16,28; severity.rs:23,37; exception.rs:23,40; ratchet.rs:69,84,127,150; table.rs:156; subsume.rs:31}` | P |
| F001 row derive/agree | `tauten-check-kernel/src/effects.rs:95,144` | M, written but unproved |
| T001, T003, X003, K003, X001 predicates; A002 tier test | `crates/tauten-check/src/structure.rs:122-138, 158-175, 177-193, 230, 252, 201` | M |
| Compiler level to severity; untiered filter | `tauten-check/src/diag.rs:161-198`; `crates/cargo-tauten/src/clippy.rs:96` | M |
| Which lints apply per tier and test target | `tauten-check/src/profile.rs:286-365`; `cargo-tauten/src/wrapper.rs:29` | M |
| Proof outcome, which awards the "Proved" badge | `tauten-check/src/proofs.rs:330-405` | M, highest value |
| Innermost enclosing function | `proofs.rs:559-564` | M |
| Exit code; dedup before the ratchet | `cargo-tauten/src/main.rs:280, 272-276` | M |
| Interning | `tauten-check/src/baseline.rs:84-111` | N: property tests (injective, order-preserving) beside the existing differential test |
| Lint-exception scan | `tauten-check/src/scan.rs:59-130`; `structure.rs:263` | N: golden fixtures plus differential (section 3). **Likely false negatives:** `#[cfg_attr(all(), allow(..))]` is not recorded (`scan.rs:79`), and group names (`clippy::restriction`, `warnings`) have no owning rule, so `structure.rs:263` skips them |
| `no_std` scan | `scan.rs:33-41` | N: fixtures. **Defect:** substring match at `scan.rs:38`, so `cfg_attr(feature = "no_std_x", ..)` passes K003 |
| Authority scan | `scan.rs:135-158` | N. Name-based, and duplicates the resolved clippy ban (`tauten-check/rules.toml:405-410`). Cut (section 5) |
| Placeholder comments | `scan.rs:173-206` | N: heuristic |
| Parse failure treated as "no facts" | `structure.rs:202, 226, 248`; `proofs.rs:550` | N for syn, but the decision is M: emit an `Unparsed` fact and fail closed |
| Function path naming | `proofs.rs:494-544, 591-594` | N. **Likely false negative:** `impl A` and `impl Trait for A` both yield `A::f`, so a proved inherent method can subsume findings in the trait method |
| Workspace model | `cargo-tauten/src/workspace.rs:74-116, 171-213` | N: golden workspaces. **Gap:** only `src/` is scanned (`workspace.rs:106`), yet X001/X002/X004 claim "Tests: yes" (`docs/RULES.md`) |
| Clippy text mapping | `diag.rs:201-220` | N: pinned-clippy golden JSON |
| Wrapper tier selection | `wrapper.rs:45-60` | N: fixture workspace |
| Fingerprint file set | `cargo-tauten/src/proofs.rs:141-246` | N: property tests (edit any translated file, fingerprint changes) |
| Macros | `crates/tauten-macros/src/lib.rs:359, 408, 531` | N: type system, pinned by trybuild (`crates/tauten/tests/ui/`), plus listed-instance proofs. Effect labels exist in three hand-synced copies (macros, `crates/tauten/src/lib.rs:71`, kernel `Row`) |
| Gate scripts, total-shape audit | `proofs/mutants.py`, `proofs/Audit.lean` | N: already listed in `proofs/TRUST.md` |

**Kani:** not recommended for syn code. It is too heavy and adds a second trust base for little gain. Use proptest and `cargo-mutants` on the scanner instead.

## 2. Architecture rule

No `Diagnostic` is constructed, kept, dropped or re-levelled outside a kernel verdict. The scanner emits facts and renders messages. Facts are Aeneas-shaped: enums, bools, `usize` indices and slices, with no strings.

Every kernel function gets:
- a `_total` theorem (no hypotheses, so it self-subsumes);
- an `_iff` spec, which is soundness and completeness relative to the facts;
- at least one non-vacuity example.

| Family | Facts | Kernel proves |
|---|---|---|
| Effects (F001) | `sources: &[Source]`, `fallible`, `asynchronous`, `declared: Option<Row>`, `required`, `resolved` | A label is derived iff some source grants it. Derive is order-independent and monotone under added sources. `agrees ↔ declared = derived`. `missing` and `extra` are exact and disjoint. A finding is an Error only if `required ∧ resolved` |
| Kernel shape (K004) | `tier`, `kinds: &[ShapeKind]` per item (`AsCast, Closure, ForLoop, IterAdaptor, DynOrImpl, Derive, Float, MacroCall`) | A finding iff the tier is kernel and a forbidden kind occurs, one per kind. Non-kernel tiers never get one |
| Exceptions (X001-X004) | `kind: Allow\|Expect`, `crate_level`, `conditional`, `group`, `rule_allows`, `level`; manifest `weakens` | The existing `excuse` theorems. Add: a group or conditional exception in a kernel crate is never `Allowed`; X003 iff `weakens ∧ blocking` |
| Structure (T, K003) | `TierDecl {Missing, Unknown, Valid(t)}`, `dep_tiers`, `external`, `allow`, `is_lib`, `no_std: {Yes, Conditional, No, Unparsed}` | Each rule's iff. `Unparsed` never yields "clean" |
| Compiler | `level`, `summary`, `rule_level`, `tiered`; `applies(level, in_tests, test)` | Kept iff it is a finding. Severity spec. A lint is flagged iff `applies`, and the wrapper and the flag generator call the same function |
| Proof status | `stages: [Status; 5]`, `theorems_ok: &[bool]`, `total_ok`, `mutants_killed: &[bool]`, `changed`; `spans: &[(u32,u32)]`, `line` | `Proved` iff every fact is good. Monotone: turning any fact bad never yields `Proved`. `enclosing` returns a containing, minimal span, and `None` iff there is none |
| Verdict | `&[Severity]` | Fails iff an Error exists. Composed with `ratchet_passes_iff`, this gives one end-to-end "check passes iff" theorem |

## 3. Trusted base afterwards

| Trusted | Tested by |
|---|---|
| syn fact extraction | Golden fact snapshots per fixture. Proptest over generated attribute and signature shapes. `cargo-mutants` on `scan.rs`, with every mutant killed by a fixture. Differential tests: K004 against Aeneas itself (every reject fixture must fail extraction, every accept fixture must extract); F001 `Source` classification against rustdoc-JSON resolved types in the proofs image |
| One-to-one enum conversions and interning | Exhaustive tests and property tests |
| rustc privacy and typing behind the macros | trybuild compile-fail per guarantee. Rule: no macro claim without a `.stderr` file |
| clippy and cargo JSON text | Golden message files per pinned toolchain. A gate check that every profile lint exists in the pinned clippy |
| Charon, Aeneas, Lean, gate scripts | As in `proofs/TRUST.md`: drift check, axiom audit, mutants |
| The specifications themselves | Review and negative fixtures. No theorem covers the threat model (`docs/PROOF-PLAN.md` §0) |

## 4. Milestones

About 16.5 working days in total.

| # | Moved or added | Theorems | Mutants | Acceptance | Days |
|---|---|---|---|---|---|
| T0 | Prove `effects.rs` as it stands. Add the `resolved`/`required` verdict so a bundle from another crate becomes `Unresolved`, not `Plain` | `derive_spec`, `derive_mono`, `derive_perm`, `agrees_iff`, `missing_extra_exact`, `f001_error_iff` | Dir drops write; ReadDir grants write; fail ignored; `agrees` ignores extra; unresolved blocks | Claims proved **before** any scanner code. Labels generated from the kernel into the macro and the library, with an equality test | 1 |
| T1 | F001 scanner emitting facts only | none new | `cargo-mutants` on the scanner | Accept and reject fixtures; fact snapshots; rustdoc-JSON differential | 2 |
| T2 | K004: `shape::verdict`. One shared syn visitor used by both `#[kernel]` and the checker | `shape_iff`, `non_kernel_silent`, total | Kind dropped; tier ignored | Aeneas differential on fixtures; trybuild for `#[kernel]` | 3 |
| T3 | `proofs::outcome`, `enclosing`. Fix the function-path collision with qualified trait paths; ambiguous means not subsumed | `proved_iff`, `outcome_monotone`, `stage_precedence`, `enclosing_spec` | Stage skipped; mutant check dropped; `changed` ignored; outermost span | Differential against the current code as a `#[cfg(test)]` reference; collision fixture | 3 |
| T4 | Exceptions: X001/X003 into the kernel. Scanner handles `cfg_attr`, groups, and `tests/`, `benches/`, `build.rs` | `conditional_never_allowed_in_kernel`, `x003_iff` | Group allowed; conditional allowed | Three new reject fixtures | 3 |
| T5 | Compiler mapping, `applies`, exit verdict | `kept_iff`, `applies_spec`, `check_passes_iff` | Test flag inverted; warning fails | Golden clippy JSON | 2 |
| T6 | T001, T003, K003, `Unparsed` | per-rule iff; `unparsed_not_clean` | Allowlist inverted; conditional counts as `no_std` | Fixtures | 1 |
| T7 | Scanner mutation gate added to `scripts/gate`; `proofs/TRUST.md` rewritten | none | none | Zero surviving scanner mutants | 1.5 |

Ordering rationale: T3 comes before the X-rules because it is the only path that downgrades errors and awards badges.

## 5. Cut or downgrade

- **S002** (`scan.rs:160-206`): add `heuristic = true` to the rule table. The kernel proves a heuristic rule is never an Error and never enters the baseline.
- **A002 syn scan** (`scan.rs:135`): delete it. The clippy `disallowed_methods` ban resolves paths. The scan can false-positive on any user type named `Authority`, and it adds a second diagnostic that `main.rs:276` will not dedup.
- **F001**: `docs/D0-SPEC.md` §5 calls it a heuristic, yet allows `effects = "required"` to make it an error. Keep errors only for `resolved` facts; unresolved stays a labelled heuristic warning.
- **K003**: accept only unconditional `#![no_std]` or `cfg_attr(not(test), no_std)`. Anything else is `Conditional` and a finding.
- **Baseline key includes the message text** (`baseline.rs:38-49`): a clippy wording change produces a new error and a stale R001 at once. Drop `message` from the key, or accept one false positive per toolchain bump and document it.

### Critical Files for Implementation
- /home/tarek/workspace/tauten/crates/tauten-check-kernel/src/effects.rs
- /home/tarek/workspace/tauten/crates/tauten-check/src/proofs.rs
- /home/tarek/workspace/tauten/crates/tauten-check/src/structure.rs
- /home/tarek/workspace/tauten/crates/tauten-check/src/scan.rs
- /home/tarek/workspace/tauten/proofs/claims.toml
