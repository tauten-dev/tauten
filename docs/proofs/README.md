# Proofs over tauten's own decisions

Lean 4 theorems about the decision code of the checker, stated over the
Charon/Aeneas translation of the shipped Rust. Plan: `docs/PROOF-PLAN.md`.
P0 (toolchain probe): `docs/proofs/probe.md`. This page covers P1.

## What is proved (P1)

The proved code is `crates/tauten-check-kernel`: a dependency-free
`no_std` crate in the `kernel` tier. `tauten-check` calls it for each of
these decisions. Its public API did not change.

| Rust function (kernel) | Called from `tauten-check` | Theorems (`proofs/Tauten/`) |
|---|---|---|
| `tiers::rank`, `tiers::may_depend_on` | `Tier::may_depend_on` (TAU-T002) | `rank_spec`, `may_depend_on_spec` (agrees with rank kernel 0 < core 1 < edge 2), `may_depend_on_refl`, `_trans`, `_total`, `_antisymm` (a total order), `kernel_depends_only_on_kernel` |
| `severity::is_blocking`, `severity::severity` | `Level::is_blocking`, `Severity::decide` (compiler findings and structural findings) | `is_blocking_spec` (only `deny` blocks), `severity_spec` (a level decides: deny is an error, warn is a warning; without a level the fallback stands) |
| `exception::permitted`, `exception::excuse` | `structure::Ctx::exceptions` (TAU-X004 and TAU-X002) | `permitted_spec`, `exception_spec` (the complete decision table), `kernel_never_excuses_levelled_rule`, `no_exceptions_never_excused` |

Every theorem is proved directly against the extracted definitions. None is
derived from another theorem. So a mutant of the Rust source breaks each
claim it falsifies, and `proofs/mutants.py` records exactly which claims
each mutant breaks.

What is trusted, not proved:
- The shell's conversions between `tauten_check::Tier`/`Level`/`Severity`
  and the kernel enums. They are one-to-one `match`es, tested.
- Treating "the lint is in `Profile::blocking_lints(tier)`" as "the lint's
  owning rule has `deny` in the tier". This holds because
  `Profile::parse` rejects a lint owned by two rules. A differential test
  compares the old and new decisions on every lint of the embedded profile
  (`kernel_excuse_agrees_with_the_reference_on_every_profile_lint`).
- The toolchain: Charon, Aeneas and its Lean library, Lean, and rustc.

This is not a proof of tauten. It covers three small decisions. Scanning,
diagnostic mapping, the ratchet (planned for P2) and the macros (planned for
P3) are not covered.

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
   - all 13 required claims exist;
   - every function extracted from `tauten_check_kernel` is on a reviewed
     list and appears in the statement of at least one required claim.

   Constructor enumeration does not apply: the kernel has no refined type.
   It returns with the macro instances in P3.
4. **Mutants.** `proofs/mutants.py` applies 9 mutations to the kernel's Rust
   source. For each one it re-extracts, rebuilds, and requires the build to
   fail in exactly the named theorems and nowhere else. Every theorem a
   mutant names must be a required claim. Each of the 13 claims is broken
   by at least one mutant.

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

## Timings (2026-11-18, local)

Measured on the 32-CPU WSL2 host while a benchmark was running, so the host
was loaded (load average about 15 to 19).

| Run | Extract | Drift | `lake build` + audit | 9 mutants | Total |
|---|---|---|---|---|---|
| Cold container (`docker compose … run --rm proofs`) | 27 s (includes Charon's sysroot setup) | <1 s | 10 s | 74 s | 111 s (112 s wall time) |
| Warm (same container, second run) | 1 s | <1 s | 2 s | 72 s | 75 s |

Each mutant re-extracts the kernel and rebuilds the theorems, which takes
about 8 s. The time on a GitHub runner, including pulling the image, has
not been measured: the workflow has not run yet. The plan's kill criterion
is 30 minutes warm.
