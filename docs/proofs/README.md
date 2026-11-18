# Proofs over tauten's own decisions

Lean 4 theorems about the decision code of the checker, stated over the
Charon/Aeneas translation of the shipped Rust. Plan: `docs/PROOF-PLAN.md`.
P0 (toolchain probe): `docs/proofs/probe.md`. This page covers P1 and P2.

## What is proved (P1, P2)

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

This is not a proof of tauten. It covers three small decisions and the
ratchet's counting. Scanning, diagnostic mapping, interning, rendering and
the macros (planned for P3) are not covered.

## The gate

`proofs/gate.sh` runs in the pinned proofs image and has four steps:

1. **Extract.** `proofs/extract.sh` runs Charon then Aeneas on
   `crates/tauten-check-kernel` (`[package.metadata.charon] start_from`
   names the six functions) with `-abort-on-error`.
2. **Drift check.** The fresh translation must equal the committed
   `proofs/Extracted/` byte for byte. It records source line numbers, so run
   `cargo fmt` before refreshing.
3. **`lake build`.** This builds the theorems and `Audit.lean`, which checks:
   - every theorem under `Tauten` uses no axioms beyond `propext`,
     `Classical.choice` and `Quot.sound` (the transitive closure, so
     `sorryAx` or an Aeneas `axiom` for an opaque function fails it);
   - all 27 required claims exist;
   - every function extracted from `tauten_check_kernel` (27, counting
     loops and loop bodies) is on a reviewed list and is stated in a
     required claim or called, through the extracted definitions, by a
     function that is.

   Constructor enumeration does not apply: the kernel has no refined type.
   It returns with the macro instances in P3.
4. **Mutants.** `proofs/mutants.py` applies 17 mutations to the kernel's
   Rust source. For each one it re-extracts, rebuilds, and requires the
   build to break exactly the named claims (a claim breaks when its proof
   fails or depends on a failed lemma; helper lemmas may fail) and no
   error outside the theorem files. Every theorem a mutant names must be a
   required claim. Each of the 27 claims is broken by at least one mutant.

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
