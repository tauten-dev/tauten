# Proofs over tauten's own decisions

Lean 4 theorems about the decision code of the checker, stated over the
Charon/Aeneas translation of the shipped Rust. Plan: `docs/PROOF-PLAN.md`.
P0 (toolchain probe): `docs/proofs/probe.md`. This page covers P1, P2,
P3 (the macros' generated code, for listed instances), P4 (rule-table
well-formedness) and P5 (the proof tier for users' kernel crates,
subsumption of arithmetic and indexing lints, badge data).

## What is proved (P1, P2, P4, P5)

The proved code is `crates/tauten-check-kernel`: a dependency-free
`no_std` crate in the `kernel` tier. `tauten-check` calls it for each of
these decisions. Its public API did not change.

| Rust function (kernel) | Called from `tauten-check` | Theorems (`proofs/Tauten/`) |
|---|---|---|
| `tiers::rank`, `tiers::may_depend_on` | `Tier::may_depend_on` (TAU-T002) | `rank_spec`, `may_depend_on_spec` (agrees with rank kernel 0 < core 1 < edge 2), `may_depend_on_refl`, `_trans`, `_total`, `_antisymm` (a total order), `kernel_depends_only_on_kernel` |
| `severity::is_blocking`, `severity::severity` | `Level::is_blocking`, `Severity::decide` (compiler findings and structural findings) | `is_blocking_spec` (only `deny` blocks), `severity_spec` (a level decides: deny is an error, warn is a warning; without a level the fallback stands) |
| `exception::permitted`, `exception::excuse` | `structure::Ctx::exceptions` (TAU-X004 and TAU-X002) | `permitted_spec`, `exception_spec` (the complete decision table), `kernel_never_excuses_levelled_rule`, `no_exceptions_never_excused` |
| `ratchet::apply` (with `take`, `take_at`, `count_at`) | `baseline::apply` | see the ratchet table below |
| `ratchet::stale` | `baseline::apply` (one TAU-R001 per key returned) | `ratchet_stale_iff`, `ratchet_passes_iff`, `ratchet_example` |
| `ratchet::count_errors` (with `zeros`, `add_error`, `add_error_at`) | `Baseline::from_findings` | `count_errors_counts`, `regenerated_baseline_never_grows` |
| `ratchet::increases` (with `grows_at`) | `Baseline::increases` (`cargo tauten baseline` refuses growth) | `increases_flags`, `increases_none_iff`, `regenerated_baseline_never_grows`, `increase_example` |
| `subsume::subsumes` | `proofs::subsume` (`cargo tauten check`, P5) | `subsumes_spec`, `stale_proof_never_subsumes`, `subsumes_only_in_kernel`, `subsumes_only_arithmetic_and_indexing` (P5, below) |
| `table::validate` (with `duplicate`, `first_duplicate`, `first_missing`, `find_before`, `contains`, `absent`, `repeat_at`, `missing_at`, `is_at`, `owner_at`) | `Profile::parse` (every rule table, the embedded one included) | `validate_ok_iff`, `validate_examples` (P4, below) |

The P1 theorems are proved directly against the extracted definitions,
none derived from another, so a mutant breaks each P1 claim it falsifies.

### The baseline ratchet (P2)

`tauten-check` interns every distinct finding key (rule, crate, file,
lint, message) to an index in key order and passes the kernel indices and
counts: findings as `(key, is_error)`, the baseline as accepted counts per
key (of duplicate entries the later counts, as before). Statements below
use `errors k` for the number of error findings with key `k` and
`accepted k` for the baseline's count (0 for a key it lacks).

| Plan name | Theorem | Statement |
|---|---|---|
| R-conserve | `apply_conserves` | `apply` returns exactly one covered flag per finding and one count per key; every finding is covered or reported, none lost or duplicated |
| R-cover | `apply_covered_iff` | finding `i` is covered iff it is an error and fewer than `accepted k` errors of its key come before it (so warnings are never covered) |
| R-cover | `apply_cover_count` | per key, covered findings = min(`accepted k`, `errors k`) |
| no over-suppression | `apply_suppressed_le_accepted` | per key, covered findings ≤ `accepted k` |
| left counts | `apply_left_spec` | left over per key = `accepted k` − `errors k`, truncated at 0 |
| R-stale | `ratchet_stale_iff` | `stale` lists `k` iff `accepted k` > `errors k`, strictly increasing, so one TAU-R001 per such key |
| R-exact | `ratchet_passes_iff` | with every key interned: all errors covered and nothing stale iff `accepted k = errors k` for every key |
| R-headroom | `ratchet_headroom` | from such a passing state, one more error finding is not covered |
| counts | `count_errors_counts` | the new baseline's count per key = min(`errors k`, `u32::MAX`) |
| R-shrink | `increases_flags` | entry `i` of a new baseline is flagged iff it accepts more than the old one does for its key |
| R-shrink | `increases_none_iff` | no entry is flagged iff the new baseline is pointwise ≤ the old one |
| R-shrink | `regenerated_baseline_never_grows` | a baseline regenerated from a passing run never grows |
| non-vacuity | `ratchet_example` | baseline `[1, 2]`, findings error/error/warning of key 0 and an error of key 1: flags `[true, false, false, true]`, left `[0, 1]`, key 1 stale |
| non-vacuity | `increase_example` | old `[1]`, new entry (key 0, 2): flagged |

How these are proved (`proofs/Tauten/Ratchet.lean`): each extracted
function with a loop has a refinement lemma (`apply_spec`, `stale_spec`,
`count_errors_spec`, `increases_spec`, by loop invariants) that relates it
to a list function in `Tauten/RatchetModel.lean`; the model file mentions
only the extracted types, never an extracted function. The claims are
stated over the extracted functions and proved from those lemmas and the
model lemmas. `apply_conserves` has its own loop invariant and a
branch-generic proof of `take_at`'s length, so it does not depend on what
`take` decides.

Consequence for the mutants: a mutant of one ratchet function breaks every
claim proved through that function's refinement lemma, including claims it
does not falsify (for example `warnings-covered` also breaks
`apply_suppressed_le_accepted`, which stays true but is no longer proved).
`proofs/mutants.py` counts a claim as broken when its own proof fails or
when its axiom closure contains `sorryAx` (Lean replaces a failed proof by
`sorry`); a reporter appended to each theorem file of the mutant's scratch
copy lists those. Each ratchet mutant still falsifies at least one of the
claims it breaks (column "falsifies" below).

Every theorem is proved directly against the extracted definitions. None is
derived from another theorem. So a mutant of the Rust source breaks each
claim it falsifies, and `proofs/mutants.py` records exactly which claims
each mutant breaks.

### Rule-table well-formedness (P4)

`Profile::parse` reads the TOML rule table, then interns every rule ID it
sees (each rule's own ID, the rule each API ban names, and the IDs the
checker emits itself, `EMITTED_BY_CODE`) to one index per distinct string,
and every lint name likewise. It passes the kernel the rules' IDs, the
flattened lint list with each lint's owning rule, the API bans' rules and
the required IDs; `table::validate` returns the first problem, and the
shell maps its positions back to names for the unchanged `ProfileError`.

| Claim | Statement |
|---|---|
| `validate_ok_iff` | `validate` never panics, and returns `Ok` iff the rule IDs are pairwise distinct, the lints are pairwise distinct (no lint owned by two rules, or twice by one), every API ban names a rule ID of the table, and every required ID is a rule ID |
| `validate_examples` | non-vacuity: a small table is accepted; the same table is rejected with a repeated ID, with a lint owned twice, with an API ban naming an unknown rule, and with a required ID missing (the case of a table without TAU-R001) |

Each extracted function has a lemma (`find_before_spec`,
`first_duplicate_spec`, `first_missing_spec`, `duplicate_spec`, ...; the
loops by invariants) and the claims are proved from them, so a mutant of
any table function breaks both claims. Which problem is reported first
(the table's reading order: rule by rule, its ID before its lints; then API
bans; then required IDs, in `EMITTED_BY_CODE` order) is not a theorem. The
old `BTreeSet`/`BTreeMap` checks are kept as `profile/reference.rs`, and a
differential test compares the full error text on 3,000 randomly edited
copies of the embedded table (rules dropped, duplicated and renamed, lints
inserted and repeated, API bans renamed; each of the six outcomes, `Ok`,
duplicate ID, lint repeated within one rule, lint owned by two rules,
unknown rule and missing rule, must occur at least 50 times).

Reject fixture: `fixtures/profiles/rej-missing-r001.toml` defines every ID
the checker emits except TAU-R001. `profile::tests::rejects_the_fixture_without_r001`
requires the error "rule table lacks TAU-R001, which the checker emits",
and that the fixture plus a TAU-R001 rule is accepted.

Trusted for P4: the interning (equal strings, equal index), which the
differential test covers; the mapping of positions back to names; and that
the table read by `toml`/`serde` is the one the user wrote.

| Mutant | Change | Claims that must break | Failing lemma |
|---|---|---|---|
| `duplicate-ids-accepted` | the ID duplicate check never fires | `validate_ok_iff`, `validate_examples` | `duplicate_spec` |
| `duplicate-lints-accepted` | a lint owned twice passes when no ID repeats | `validate_ok_iff`, `validate_examples` | `duplicate_spec` |
| `unknown-api-rule-accepted` | API bans not checked | `validate_ok_iff`, `validate_examples` | — |
| `required-id-missing-accepted` | missing required IDs ignored (a table without TAU-R001 passes) | `validate_ok_iff`, `validate_examples` | — |
| `search-stops-short` | `find_before` stops one position early | `validate_ok_iff`, `validate_examples` | `find_before_spec` |

What is trusted, not proved:
- Ratchet interning (`baseline::Interned`): one index per distinct key, in
  key order, and of duplicate baseline entries the later count. The shell's
  use of the kernel's output: report the uncovered findings in order (the
  kernel returns one flag per finding, `apply_conserves`), render one
  TAU-R001 per key `stale` returns, and keep the entries `increases` flags.
  The old `BTreeMap` implementation is kept as `baseline/reference.rs`;
  differential tests (`baseline/tests.rs`) compare the two on 3,000 random
  cases (small key space, so keys collide; duplicated, zero, raised,
  lowered and foreign baseline entries; the generator must produce more
  than 1,000 covered findings, 300 TAU-R001 and 300 increases) and on the
  unit-test cases.
- The shell's conversions between `tauten_check::Tier`/`Level`/`Severity`
  and the kernel enums. They are one-to-one `match`es, tested.
- Treating "the lint is in `Profile::blocking_lints(tier)`" as "the lint's
  owning rule has `deny` in the tier". This holds because
  `Profile::parse` rejects a lint owned by two rules. A differential test
  compares the old and new decisions on every lint of the embedded profile
  (`kernel_excuse_agrees_with_the_reference_on_every_profile_lint`).
- The toolchain: Charon, Aeneas and its Lean library, Lean, and rustc.

This is not a proof of tauten. It covers three small decisions, the
ratchet's counting, the rule table's well-formedness (P4), and (P3, below) the code the macros generated for a
list of instances. Scanning, diagnostic mapping, interning and rendering
are not covered.

## What the macros generate, for listed instances (P3)

The site wording for this section is **"proved for listed instances"**.
The macros are not verified: `tauten-macros` (a proc-macro over
`syn`/`quote`) is not translated, and nothing here quantifies over every
input it accepts.

`crates/tauten-proof-instances` (`publish = false`, so `cargo publish
--workspace` skips it; tier `edge` only because the `tauten` runtime is an
edge crate in this workspace) applies the real macros as a user would.
Charon translates the *expansion*, so the Lean definitions are the
macro's actual output. `include = ["tauten::contract", "tauten::error"]`
makes `holds`, `snapshot`, `RefineError` and `ContractViolation`
transparent; no `axiom` is emitted.

| Instance | Macro input |
|---|---|
| `Percent(u8)` | `#[refined(\|v\| *v <= 100)]` |
| `Port(u16)` | `#[refined(\|p\| *p >= 1024 && *p <= 49151)]` |
| `Offset(i32)` | `#[refined(\|x\| *x >= -1000 && *x <= 1000)]` |
| `Count(u64)` | `#[refined(\|n\| *n != 0)]` |
| `Tag(u32)` | `#[newtype]` with `derive(Default)`: the control of the constructor audit |
| `deposit(balance: u64, amount: u64)` | `#[requires(amount > 0)]`, `#[ensures(\|after\| *after > old(balance))]` |
| `withdraw(balance: &mut u64, amount: u64)` | `#[requires(amount > 0)]`, `#[ensures(\|after\| *after < old(*balance))]` |
| `scale(value: u32, factor: u32)` | `#[requires(factor <= 1000)]`, `#[ensures(\|scaled\| *scaled >= old(value))]` (false for a zero factor: every outcome is reachable) |
| `share(total: u32, divisor: u32)` | `#[requires(divisor != 0)]`; the body divides and would panic on 0 |

### How the instance proofs constrain the generator

`proofs/Tauten/MacroShapes.lean` defines what an expansion reduces to and
proves the facts once, for an arbitrary predicate, body and postcondition
(schematic claims, no extracted function):

- `refinedTryNew p err x`: `Ok x` if `p x`, else `Err err`.
  `refined_ok_iff`, `refined_ok_value` (the `Ok` value is `x`),
  `refined_err_iff`, `refined_total` (no third outcome, no panic).
- `contractCall pre reqErr post ensErr body` (by-value parameters) and
  `contractCallMut ... s0 body` (a `&mut` parameter as state in and out).
  `contract_requires`/`contract_mut_requires`: a false precondition gives
  `Err reqErr` (and the entry state) *for every body*, including one that
  panics: the body is not evaluated. `contract_ok`/`contract_mut_ok`: the
  body's `Ok t` is returned iff `post t`, else `Err ensErr`.
  `contract_err`/`contract_mut_err`: the body's `Err` is returned unchanged.

Each instance then has a `*_shape` theorem: the extracted function
*equals* the shape at that instance's predicate or conditions, stated over
the mathematical values (`.val`), with the exact error values the macro
emits (type name, predicate text, function name, condition text). Every
other instance claim is proved through its shape theorem. A change to what
the macro emits breaks the shape theorems and, through them, the claims
(mutants below).

### Claims (`proofs/Tauten/Refined.lean`, `proofs/Tauten/Contracts.lean`)

Per refined type `T` in `percent`, `port`, `offset`, `count` (Plyward
`spec/REFINEMENTS.md` "Formal targets for Percent", targets 1, 2, 3, 5):

| Claim | Target | Statement |
|---|---|---|
| `T_try_new_shape` | binding | `try_new` is `refinedTryNew` at the predicate and the macro's `RefineError` |
| `T_try_new_ok_iff` | F01-1 reflection | `try_new x` is `Ok` iff the predicate holds of `x` |
| `T_try_new_sound` | F01-2 introduction, raw value | `try_new x = Ok v` implies `get v`, `into_inner v` and `as_ref v` return `x`, and the predicate holds of `x` |
| `T_try_new_err_iff` | F01-3 rejection | `try_new x` is `Err` iff the predicate is false (with `refined_total`: no clamping, no panic) |
| `T_try_from_spec` | F01-5 | `TryFrom::try_from` is `try_new` |
| `T_derived_spec` | F01-5 | derived `clone` is the identity, `eq` compares the values, `assert_fields_are_eq` is a no-op |
| `T_boundaries` | witnesses | Percent: 0 and 100 accepted unchanged, 101 and 255 rejected; Port: 0, 1023, 49152, 65535 rejected, 1024, 49151 accepted; Offset: `i32::MIN`, -1001, 1001, `i32::MAX` rejected, -1000, 0, 1000 accepted; Count: 0 rejected, 1 and `u64::MAX` accepted |

`tag_control_spec` states the control's `new` and derived `default`.

F01-5, constructor enumeration, is also an audit (`Audit.lean`): for
each refined type, every extracted function whose result is `Result T`
with `T` mentioning the type is listed (from the unreduced signature, with
no `isInternal` filter, which hides the `__tauten_*` module). The list must
be exactly `try_new`, `try_from` and the derived `clone`. For the control
it must contain `new` and the derived `default`, so the audit sees derived
impls. Result today:

```
CONSTRUCTORS …Percent: [….Insts.CoreCloneClone.clone, ….Insts.CoreConvertTryFromU8RefineError.try_from, ….try_new]
CONSTRUCTORS …Tag (control): [….Insts.CoreCloneClone.clone, ….Insts.CoreDefaultDefault.default, ….new]
CONSTRUCTOR_AUDIT ok: 4 refined types, control found 2 constructors
```

Contracts (`IsRequires r`: `r` is `Err(Contract(Requires ..))`; likewise
`IsEnsures`):

| Function | Claims |
|---|---|
| `deposit` | `deposit_shape`; `deposit_requires_iff` (`Requires` iff `amount = 0`); `deposit_never_ensures`; `deposit_ok` (no overflow: `Ok` of the sum); `deposit_overflow` (the body's `Err(Overflow)`, unchanged) |
| `withdraw` | `withdraw_shape` (the postcondition compares with the balance **at entry**: `old(*balance)` is the pre-state); `withdraw_requires_iff`; `withdraw_requires_keeps_balance` (a refused call returns the balance unchanged); `withdraw_never_ensures`; `withdraw_ok` (returns and stores `balance - amount`); `withdraw_insufficient` (the body's `Err`, balance untouched) |
| `scale` | `scale_shape`; `scale_requires_iff` (iff `factor > 1000`); `scale_ensures_iff` (`Ensures` iff `factor = 0` and `value ≠ 0`); `scale_ok`; `scale_overflow` |
| `share` | `share_shape`; `share_requires_skips_body` (`share t 0` is `Err(Requires)` while the body on the same arguments is `fail divisionByZero`: the body is not evaluated); `share_requires_iff`; `share_ok` (the quotient) |
| all | `contract_examples`: `deposit(5, 0)` is `Requires`, `scale(5, 0)` is `Ensures`, `scale(1, 1001)` is `Requires`, `withdraw(10, 8)` leaves and returns 2, `withdraw(7, 8)` is `Insufficient` with 7 kept |

Stated, not hidden: a postcondition failure on a `&mut` function keeps
the body's write (`contractCallMut`'s `Err ensErr, s1`). The wrapper
reports the violation; it does not roll back. That is the shipped
behaviour, now stated as a theorem; none of the instances can reach it,
because their postconditions hold.

### Macro mutants

`proofs/mutants.py` also mutates `crates/tauten-macros/src/lib.rs` (10
mutants) and
re-translates the instance crate through the mutated macros. A claim
counts as broken when its proof fails or depends on one that does (the
P2 rule); "falsifies" names the claims whose statement is false for the
mutated code. REFINED = the 20 claims through the four `*_try_new_shape`;
DEPOSIT, WITHDRAW, SCALE, SHARE = the claims of that function;
EX = `contract_examples`.

| Mutant | Change to the generator | Must break | Falsifies (among them) |
|---|---|---|---|
| `refined-check-inverted` | `if holds(..)` becomes `if !holds(..)` | REFINED | every `*_ok_iff`, `*_err_iff`, `*_boundaries` |
| `try-from-unchecked` | `try_from` returns `Ok(Self(value))` | the four `*_try_from_spec` | all four |
| `refined-default` | the generated module gains `impl Default` (the `derive(Default)` bug class) | no claim; the constructor audit fails (`constructor audit: …default returns a …Percent`) | — |
| `attributes-dropped` | the struct's attributes (its derives among them) not forwarded to the generated struct | the four `*_derived_spec`, `tag_control_spec` | all five (the derived impls no longer exist) |
| `requires-inverted` | `if !(cond)` becomes `if (cond)` | DEPOSIT, WITHDRAW, SCALE, SHARE, EX | every `*_requires_iff`, `*_ok` |
| `requires-dropped` | no precondition check | DEPOSIT, WITHDRAW, SCALE, SHARE, EX | every `*_requires_iff`, `share_requires_skips_body` |
| `requires-after-body` | the precondition checked after the body ran | DEPOSIT, WITHDRAW, SCALE, SHARE, EX | `share_requires_skips_body` (the body panics first); for the other functions the body is total and the observable result is unchanged, so their claims break only because the shape proofs no longer go through |
| `snapshot-after-body` | `old(..)` taken after the body | WITHDRAW, EX | `withdraw_shape`, `withdraw_ok`, `contract_examples` (`old(*balance)` sees the write). The by-value instances cannot tell: their parameters do not change, which is why the `&mut` instance exists |
| `ensures-skipped` | no postcondition check | DEPOSIT, WITHDRAW, SCALE, EX (`share` has no postcondition) | `scale_ensures_iff`, `contract_examples` |
| `ensures-reported-as-requires` | a postcondition failure reported as `Requires` | DEPOSIT, WITHDRAW, SCALE, EX | `scale_ensures_iff`, `scale_requires_iff`, `contract_examples` |

What is trusted, not proved, for P3:
- That the instances represent the macro's behaviour on other inputs. The
  shapes are proved for these nine items only; other predicates (calls,
  `String`/`Vec` values, arithmetic that panics, predicates that diverge)
  are not covered. Async contracts are not translated at all.
- Privacy of the generated module (rustc), so that nothing outside it can
  build `T(value)` directly; and the derive/attribute allowlist being
  complete (compile-fail tests).
- That the source texts in errors are what the user wrote (the theorems
  pin the strings the macro emits today).
- The toolchain, as for P1 and P2.

## The proof tier for users (P5, milestone M3)

The setup above, folded into a template that a project instantiates for its
own kernel crate, and the same reporting for tauten's own proofs.

### `proofs/claims.toml`

```toml
version = 1

[gate]
command = ["sh", "proofs/run.sh"]          # run from the workspace root
crates = ["crates/ledger-kernel"]          # what the gate translates

[[claim]]
crate = "ledger-kernel"                    # package
file = "crates/ledger-kernel/src/lib.rs"   # workspace-relative
function = "apply"                         # `name`, `Type::name` or `module::name`
lean = "ledger_kernel.apply"               # the extracted definition
theorems = ["Proofs.LedgerKernel.apply_spec"]
total = "Proofs.LedgerKernel.apply_spec"   # optional: never fails, for every input
subsumes = ["TAU-N001", "TAU-P003"]        # optional, needs `total`
mutants = ["deposit-wraps", "withdraw-reports-overflow"]

[[mutant]]                                 # read by the template's gate.py
name = "deposit-wraps"
file = "crates/ledger-kernel/src/lib.rs"
before = "balance.checked_add(amount).ok_or(Rejection::Overflow)"  # exactly once
after = "Some(balance.wrapping_add(amount)).ok_or(Rejection::Overflow)"
```

`tauten_check::proofs::ClaimsFile::parse` rejects a claim without a theorem
or without a mutant (the badge rule needs a killed mutant), a `total` that
is not among its theorems, a `subsumes` rule other than TAU-N001/TAU-P003 or
without `total`, and a function claimed twice.

### `cargo tauten proofs`

Runs `[gate] command` from the workspace root, passes its output through to
stderr, and reads the lines that start with `TAUTEN-PROOFS ` followed by
JSON: stages (`{"stage": "drift", "status": "ok"}`; stages `tools`,
`extract`, `drift`, `build`, `audit`, `mutants`; status `ok`, `fail` or,
for tools, `missing`), audited theorems (`{"theorem": …, "status": "ok"}`),
total theorems (`{"total": …, "function": …}`) and mutants
(`{"mutant": …, "status": "killed", "broken": [theorems]}`). It prints one
JSON line per claim (the claim's fields plus `outcome`, `detail` and
`fingerprint`), writes them to `proofs/claims.json` (unless `--no-write`)
and exits 0 only if every claim is proved. The outcome
(`tauten_check::proofs::outcome`, unit-tested for each value):

| Outcome | When |
|---|---|
| `tool-missing` | the command cannot start, the gate reports `tools: missing` (no Docker, no daemon, no image; `run.sh` checks), or the gate printed no stage at all |
| `build-failed` | `extract` or `build` failed (or did not run) |
| `drift` | the committed `proofs/Extracted/` differs from a fresh translation, or the sources changed while the gate ran (fingerprints before and after) |
| `audit-failed` | the audit failed, a claim theorem was not audited with the standard axioms, or the `total` theorem was not confirmed to have the total shape |
| `mutant-survived` | a listed mutant has no `killed` result that broke one of the claim's theorems |
| `proved` | none of the above |

A missing line is never read as a pass: every stage up to `audit` must be
reported `ok`, every theorem audited, every mutant reported killed.

### Lint subsumption, bound to a fresh proof

`cargo tauten check` (and `baseline`) reads `proofs/claims.json` if it
exists. For a TAU-N001 or TAU-P003 finding it finds the innermost function
around the finding's line (`syn`), and the claim for that crate, file and
function. The proved kernel function `subsume::subsumes(tier, finding,
listed, proved, fresh)` decides; the finding is then reported with
severity `warning` and `"subsumed_by": "<total theorem>"` (human output:
`subsumed[TAU-N001] … = subsumed by proof <theorem>`, and a count in the
summary), so it fails neither the check nor the baseline.

- `proved`: the recorded outcome is `proved` and the claim has a `total`
  theorem. The template's `Audit.lean` reports a theorem as total only if
  its statement is `∀ (x₁ : α₁) … (xₙ : αₙ), f x₁ … xₙ ⦃ _ ⦄` with no `αᵢ` a
  proposition and `f` applied to exactly those variables. Aeneas's `⦃ ⦄`
  (`WP.spec`) is false for `fail` (panic, overflow, out of bounds) and
  `div`, so such a theorem says `f` never fails, for every input. A theorem
  with a hypothesis does not qualify (the fixture's `double` shows it).
- `fresh`: the fingerprint recorded by `cargo tauten proofs` (FNV-1a 64 over
  every file of the claim's crate directory, of the directories of the
  workspace members it depends on (transitively), and of `proofs/`, except
  `claims.json`, hidden entries, `target` and `__pycache__`) equals the
  fingerprint of the current files. Any edit to the crate or to the proofs
  brings the findings back until the gate runs again.
- `listed`: the claim names the finding's rule under `subsumes`.
- Only kernel crates, only TAU-N001 and TAU-P003.

| Claim (`Tauten/Subsume.lean`) | Statement |
|---|---|
| `subsumes_spec` | `subsumes` returns true iff the tier is kernel, the finding is arithmetic or indexing, and `listed`, `proved` and `fresh` all hold; never fails |
| `stale_proof_never_subsumes` | with `fresh = false` the result is false, whatever else holds |
| `subsumes_only_in_kernel` | core and edge crates: always false |
| `subsumes_only_arithmetic_and_indexing` | any other rule: false; arithmetic and indexing with everything else true: true (non-vacuity) |

| Mutant | Change | Claims that must break |
|---|---|---|
| `stale-proof-subsumes` | `fresh` ignored | `subsumes_spec`, `stale_proof_never_subsumes` |
| `core-subsumed` | core crates subsumed | `subsumes_spec`, `subsumes_only_in_kernel` |
| `any-finding-subsumed` | every rule subsumable | `subsumes_spec`, `subsumes_only_arithmetic_and_indexing` |
| `unlisted-rule-subsumed` | `listed` ignored | `subsumes_spec` |

Trusted for subsumption (`proofs/TRUST.md`): the function lookup by line
span, the fingerprint as change detection (not tamper protection:
`claims.json` is committed and reviewed like a baseline), and the outcome
mapping (unit tests).

Fixture: `fixtures/proofs-subsume` (a kernel crate, its `proofs/` from the
template, and a `claims.json` recorded by a real gate run: `add_capped` and
`first_or_zero` proved total, `double` audit-failed because its theorem has
a hypothesis, `triple` unclaimed). `crates/cargo-tauten/tests/proofs.rs`
checks that exactly the three findings of the two proved functions are
subsumed, that an edit to the Rust source or to a theorem makes all five
findings errors again, that a gate command that is absent, silent or
reports missing tools gives `tool-missing` for every claim (and then
nothing is subsumed), and that `init --proofs` writes the template once.

### The template and `cargo tauten init --proofs`

`crates/cargo-tauten/templates/proofs/` (inside the published crate, so
`cargo install cargo-tauten` carries it). `cargo tauten init --proofs
<crate>` prints the files; with `--write` it writes them to `proofs/`,
filling in the crate's name and directory, and never over an existing
`proofs/`. The files: `claims.toml`, `README.md` (workflow and
writing-for-Aeneas rules), `TRUST.md`, `run.sh` (host: checks Docker and
the image, runs Compose), `compose.yaml`, `container.sh`, `extract.sh`
(tauten's, also rendering non-string inherited `[workspace.package]` keys
such as `publish = false`), `refresh.sh`, `gate.py` (the gate in Python:
tools, extract, drift, `lake build Extracted Proofs`, `lake build Audit`,
mutants from `claims.toml`, each printing result lines), `lakefile.toml`,
`lean-toolchain`, `lake-manifest.json` (the same pins and package name as
tauten's, so the image's prebuilt dependencies apply), `Audit.lean` (axiom
audit of every theorem in the `Proofs` library and the total-shape check),
`Proofs.lean` and `Proofs/<Crate>.lean`.

The template's mutant rule is looser than tauten's own calibration: a
mutant counts as killed when it breaks at least one of the claim's
theorems (its own proof fails, or it depends on one that does) and causes
no error outside `Proofs/`; it does not require the exact set.

### Worked example: `bench/hosts/ledger`

`cargo tauten init --proofs ledger-kernel --write`, then `sh
proofs/refresh.sh` (no change to the crate was needed: Charon/Aeneas
translate `apply`, its enums and their derives as written), one theorem,
`Proofs.LedgerKernel.apply_spec`: for every balance and entry, `apply`
returns the exact sum or `Overflow`, the exact difference or
`Insufficient`, and never fails (total, so it would subsume N001/P003 in
`apply`). Mutants `deposit-wraps` and `withdraw-reports-overflow` both
break it. `cargo tauten proofs --manifest-path
bench/hosts/ledger/Cargo.toml`: `proved`, recorded in
`bench/hosts/ledger/proofs/claims.json`. The benchmark's trial copies of
the host leave out `proofs/` (`bench/run.py`), so trials see the host
exactly as before.

### Badge data

`proofs/claims.toml` lists tauten's own 20 claims (12 kernel functions, 4
refined constructors, 4 contract functions; the theorems as above, the
mutants from `proofs/mutants.py`). `gate.sh`, `report.py` and
`mutants.py` print the result lines, so `cargo tauten proofs` in this
repository runs the full gate and writes `proofs/claims.json`: per
function, theorems, mutants and outcome. The website shows "Proved" for a
`proved` claim of `tauten-check-kernel` and "Proved for listed instances"
for one of `tauten-proof-instances`, linking the statement and `TRUST.md`;
the list is generated from that file, not written by hand. `mutants.py`
reports a mutant whose calibration fails (it breaks more or fewer claims
than named) as `survived`, so a miscalibrated mutant withholds the badge.

## The gate

`proofs/gate.sh` runs in the pinned proofs image and has four steps. It
prints a `TAUTEN-PROOFS` result line per stage, `proofs/report.py` one per
audited theorem, and `proofs/mutants.py` one per mutant (P5):

1. **Extract.** `proofs/extract-all.sh` runs `proofs/extract.sh` (Charon
   then Aeneas, `-abort-on-error`) on `crates/tauten-check-kernel`
   (`[package.metadata.charon] start_from` names its functions) and on
   `crates/tauten-proof-instances` (`start_from` names the `refined` and
   `contracts` modules; this step fetches `syn`, `quote` and
   `proc-macro2` from crates.io).
2. **Drift check.** The fresh translation must equal the committed
   `proofs/Extracted/` byte for byte. It records source line numbers, so run
   `cargo fmt` before refreshing.
3. **`lake build`.** This builds the theorems and `Audit.lean`, which checks:
   - every theorem under `Tauten` uses no axioms beyond `propext`,
     `Classical.choice` and `Quot.sound` (the transitive closure, so
     `sorryAx` or an Aeneas `axiom` for an opaque function fails it);
   - all 93 required claims exist (33 kernel, 50 instance, 10 schematic);
   - constructor enumeration (P3, above);
   - every extracted function (45 from `tauten_check_kernel`, counting
     loops and loop bodies; 59 from `tauten_proof_instances`, counting
     closures, derives and the transparent runtime helpers) is on a
     reviewed list and is stated in a required claim or called, through
     the extracted definitions and trait-instance records, by a function
     that is.
4. **Mutants.** `proofs/mutants.py` applies 26 mutations to the kernel's
   Rust source and 10 to the macros' source (P3, above). For each one it
   re-extracts, rebuilds, and requires the
   build to break exactly the named claims (a claim breaks when its proof
   fails or depends on a failed lemma; helper lemmas may fail) and no
   error outside the theorem files. Every theorem a mutant names must be a
   required claim. Each of the 83 claims stated over the extraction is
   broken by at least one mutant; the 10 schematic claims
   (`Tauten.Macros`) mention no extracted function, so no mutant of the
   Rust source can break them.

| Mutant | Change | Theorems that must fail |
|---|---|---|
| `same-tier-refused` | `<=` becomes `<` in `may_depend_on` | `may_depend_on_spec`, `_refl`, `_total`, `kernel_depends_only_on_kernel` |
| `one-step-outward` | kernel -> core and core -> edge also allowed, kernel -> edge not | `may_depend_on_spec`, `_trans`, `_antisymm`, `kernel_depends_only_on_kernel` |
| `core-edge-swapped` | core and edge ranks swapped (still a total order) | `rank_spec`, `may_depend_on_spec` |
| `kernel-ranks-as-core` | kernel rank 1 | `rank_spec`, `may_depend_on_spec`, `_antisymm`, `kernel_depends_only_on_kernel` |
| `warn-blocks` | `Warn` blocking | `is_blocking_spec`, `severity_spec`, `exception_spec` |
| `no-level-is-error` | no level gives `Error` instead of the fallback | `severity_spec` |
| `kernel-excuses` | the kernel clause dropped | `permitted_spec`, `exception_spec`, `kernel_never_excuses_levelled_rule` |
| `rule-flag-ignored` | `exceptions = false` ignored | `permitted_spec`, `exception_spec`, `no_exceptions_never_excused` |
| `crate-wide-any` | `crate_wide && blocking` becomes `||` | `exception_spec` |

Ratchet mutants. APPLY = the eight claims proved through `apply_spec`
(`apply_covered_iff`, `apply_cover_count`, `apply_suppressed_le_accepted`,
`apply_left_spec`, `ratchet_stale_iff`, `ratchet_passes_iff`,
`ratchet_headroom`, `ratchet_example`); STALE = `ratchet_stale_iff`,
`ratchet_passes_iff`, `ratchet_example`; INCREASES = `increases_flags`,
`increases_none_iff`, `regenerated_baseline_never_grows`,
`increase_example`.

| Mutant | Change | Claims that must break | Falsifies (among them) | Failing lemma |
|---|---|---|---|---|
| `warnings-covered` | `take` covers warnings | APPLY | `apply_covered_iff`, `apply_cover_count` | `take_spec` |
| `headroom-ignored` | `*slot > 0` becomes `>= 0` | APPLY | `apply_covered_iff`, `apply_suppressed_le_accepted`, `ratchet_headroom` | `take_spec` |
| `cover-not-consumed` | covering does not decrement | APPLY | `apply_covered_iff`, `apply_left_spec`, `ratchet_stale_iff` | `take_spec` |
| `covered-flag-dropped` | covered findings get no flag | `apply_conserves` + APPLY | `apply_conserves` | `apply_loop_spec` (and `apply_conserves` itself) |
| `stale-needs-two` | stale only with 2 left | STALE | `ratchet_stale_iff`, `ratchet_example` | `stale_loop_spec` |
| `warnings-counted` | new baseline counts warnings | `count_errors_counts`, `regenerated_baseline_never_grows` | `count_errors_counts` | `add_error_spec` |
| `equal-is-growth` | `<` becomes `<=` in `grows_at` | INCREASES | `increases_flags`, `increases_none_iff`, `regenerated_baseline_never_grows` | `grows_at_spec` |
| `missing-key-accepts-one` | `count_at` returns 1 for a missing key | STALE + INCREASES | `increases_flags`, `increases_none_iff` | `count_at_spec` |

## Running it locally

You need Docker on linux/amd64 and the toolchain image (about 20 GB
unpacked):
`ghcr.io/tauten-dev/proofs-toolchain:lean-4.35.0-aeneas-c5eb4a2-charon-8cae8fd`.
It is the `deps` stage of writfold's `proofs/Dockerfile`, built with
writfold's pins: Lean v4.35.0, Aeneas `nightly-2026.11.07-c5eb4a2`, Charon
`8cae8fd` on `nightly-2026-09-17`, and mathlib as in
`proofs/lake-manifest.json`. A local build of that stage is the same image.

```sh
cargo tauten proofs                                        # the whole gate, one outcome per claim, records proofs/claims.json
docker compose -f compose.proofs.yaml run --rm proofs     # the whole gate
sh proofs/refresh.sh                                       # regenerate proofs/Extracted/ after a kernel change
docker compose -f compose.proofs.yaml run --rm proofs \
  sh /repo/proofs/container.sh python3 proofs/mutants.py warn-blocks   # one mutant
```

The checkout is mounted read-only at `/repo`. `proofs/container.sh`
copies only what the gate reads into `/work`, where the image keeps the
prebuilt Lean dependencies (`/work/proofs/.lake`). The gate needs network
access even though the kernel has no dependencies: in a fresh container,
Charon builds its full-MIR sysroot (`miri setup`), and that fetches crates
from crates.io. With `network_mode: none` the gate hangs at that step.

After you change the kernel:
1. `cargo fmt`.
2. `sh proofs/refresh.sh`, then commit `proofs/Extracted/`.
3. Fix the theorems and add a mutant for any new decision.
4. For a new function: add it to `start_from`, to `reviewedFunctions` in
   `Audit.lean`, and to a claim.

After you change `crates/tauten-macros` or `crates/tauten` (P3): the
instance crate's translation changes whenever the expansion does, so run
`sh proofs/refresh.sh` and expect the drift check to fail until you do.
If the shape of the expansion changed on purpose, update the shapes in
`Tauten/MacroShapes.lean` and the `*_shape` proofs; if the instance's
behaviour changed, a claim should say so. A new refined instance needs
its claims, its row in `refinedTypes` and its functions in
`reviewedFunctions` (`Audit.lean`).

Writing kernel code that translates (writfold's rules, confirmed here):
- The kernel profile forbids indexing and unchecked arithmetic, so loops
  are `while i < xs.len()` over `xs.get(i)` with `saturating_add(1)`, and
  updates go through `get_mut`. Aeneas models all of these (`Slice.get`,
  `Slice.get_mut`, `saturating_*`); the saturating lemmas are proved in
  `Tauten/Ratchet.lean`.
- A `match` on `xs.get(i)` directly inside a loop body crashed Aeneas
  (`Internal error … InterpReduceCollapse.ml, line 1151`). Moving the
  match into a helper (`take_at`, `grows_at`, `add_error_at`) fixed it.
- No derives are reached from translated functions; compare enums with
  `match`.
- No casts, iterator chains or closures.
- A parameter must not have the same name as its module. This is why the
  module is `tiers`, not `tier`.
- Avoid `Option::is_none` and the like; write the `match`.

## CI

`.github/workflows/proofs.yml` is separate from the `gate` workflow. It
runs on pushes and pull requests that touch
`crates/tauten-check-kernel/**`, `crates/tauten-macros/**`, `proofs/**`,
`compose.proofs.yaml` or the workflow itself. It also runs nightly and on
`workflow_dispatch`. The job:
1. frees runner disk space;
2. logs in to ghcr.io with the job's `GITHUB_TOKEN` (`packages: read`);
3. pulls the pinned image;
4. runs the same Compose service.

## Timings (local, 32-CPU WSL2 host, loaded)

P1 (2026-11-18, load average about 15 to 19):

| Run | Extract | Drift | `lake build` + audit | 9 mutants | Total |
|---|---|---|---|---|---|
| Cold container (`docker compose … run --rm proofs`) | 27 s (includes Charon's sysroot setup) | <1 s | 10 s | 74 s | 111 s (112 s wall time) |
| Warm (same container, second run) | 1 s | <1 s | 2 s | 72 s | 75 s |

P2 (2026-11-18, load average about 14 to 21):

| Run | Extract | Drift | `lake build` + audit | 17 mutants | Total |
|---|---|---|---|---|---|
| Cold container (`docker compose … run --rm proofs`) | 20 s | <1 s | 15 s | 197 s | 232 s (233 s wall time) |
| Warm (long-lived container, `proofs/container.sh`) | 2 s | <1 s | 1 s | 211 s | 214 s |

P3 (2026-11-18, load average about 3 to 7):

| Run | Extract (both crates) | Drift | `lake build` + audit | Mutants | Total |
|---|---|---|---|---|---|
| Cold container (`docker compose … run --rm proofs`), 26 mutants (before `attributes-dropped` was added) | 17 s (includes crates.io fetch of `syn` and Charon's sysroot setup) | <1 s | 12 s | 256 s | 285 s (286 s wall time) |
| Warm (long-lived container, `proofs/container.sh`), all 27 mutants | 5 s | <1 s | 1 s | 265 s | 271 s |

P4 (2026-11-18, load average about 10 to 20):

| Run | Extract (both crates) | Drift | `lake build` + audit | 32 mutants | Total |
|---|---|---|---|---|---|
| Warm (long-lived container, `proofs/container.sh`) | 5 s | <1 s | 11 s | 323 s | 339 s |

P5 (2026-11-18/19, load average about 10 to 25; every run through
`cargo tauten proofs`, a fresh `docker compose run` container each time):

| Run | Claims | Mutants | Wall time | Outcome |
|---|---|---|---|---|
| tauten (`proofs/claims.toml`), first run | 20 | 36 | 430 s (extract 19 s, `lake build` + audit 13 s, mutants 396 s; two other gates ran alongside) | gate OK, every claim `drift`: `proofs/TRUST.md` was edited while it ran (the before/after fingerprint check working as intended) |
| tauten, final run | 20 | 36 | 409 s (extract 17 s, `lake build` + audit 13 s, mutants 377 s) | all 20 `proved`; audit: 182 theorems, 93 required claims; coverage: 104 functions; 36 mutants calibrated |
| `bench/hosts/ledger` (template) | 1 | 2 | 53 s and 62 s | `proved` |
| `fixtures/proofs-subsume` (template) | 3 | 3 | 60 s and 68 s | 2 `proved`, 1 `audit-failed` (by design) |
| any, without Docker or the image | | | < 1 s | every claim `tool-missing` |

A macro mutant re-translates the instance crate through the mutated
macros (it compiles `syn` again in a fresh shadow workspace) and rebuilds
every theorem file: about 10 s each.

Each mutant re-extracts the kernel and rebuilds every theorem file, about
10 to 13 s now that the ratchet proofs exist. On GitHub, P1's gate took
2 minutes (job 7 minutes with the image pull); P2 has not run there. The
plan's kill criterion is 30 minutes warm.

## Behaviour check on writfold (P2)

The kernel-backed ratchet was compared with the previous binary (built
from `7118634`, the branch point) on a scratch clone of writfold at
`ff7ce76`, with the dry run's `--assume-tier` flags: `cargo tauten
baseline` wrote byte-identical files (205 entries, 707 findings, as in
`docs/STATUS.md`), and `check` with the baseline, `check --no-baseline`,
a baseline with two raised entries (2 TAU-R001, exit 1) and a lowered one
(exit 1; regeneration refused: 2 entries would grow, exit 2) gave
byte-identical output.
