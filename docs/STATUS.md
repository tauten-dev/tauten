# Status

Updated 2026-11-18 (evening). Name final: tauten; license MIT; code at github.com/tauten-dev/tauten (private until first release). Every line says how far an item got:
**specified → implemented → exercised**. "Exercised" means it ran against
real input in this repository, and the evidence is named. Nothing is
published. One pilot on the dev split has been run with a real agent; nothing held-out has been measured.

## Milestones (see PLAN.md §6)

| # | Item | State | Evidence |
|---|---|---|---|
| M0 | Plan, mapping, agent profile, rule table (37 rules, ~200 banned API paths) | **done** | `docs/`, `crates/tauten-check/agents.md`, `crates/tauten-check/rules.toml`, generated `docs/RULES.md` (drift-checked) |
| M0 | Fixtures ported from Sovel examples | **exercised** | `fixtures/cases`: 28 reject + 3 accept crates, exact expected IDs, one tier per crate (`tests/fixtures.rs`) |
| M0b | Writfold dry run | **exercised** | `docs/dry-runs/writfold-2026-11-18.md`. It found 4 defects in tauten, now fixed. |
| M1 | `cargo tauten check` (human/JSON), `explain` (+`--markdown`), `profile`, `--assume-tier` | **exercised** | fixtures, self-check, writfold |
| M1 | Ratchet: `cargo tauten baseline`, `--baseline`, stale-baseline rule TAU-R001, increases refused | **exercised** | unit tests; `tests/ratchet.rs`; writfold clone (707 accepted → 1 new `unwrap` = 1 error → stale slot = R001 → increase refused) |
| M1 | `init` (proposes tiers; `--write` adds them and `AGENTS.tauten.md`) | **exercised** (manually) | scratch two-crate workspace, 2026-11-18; no automated test yet |
| M2 | `tauten`: `Authority`, `Clock`/`SystemClock`/`ManualClock`, `Env`, `dir` (feature `fs`, cap-std) | **exercised** | doc tests; `crates/tauten/tests/ui/forged_authority.rs`; fixture `rej-authority-alias` |
| M2 | `tauten-macros`: `#[newtype]`, `#[refined]`, `#[requires]`, `#[ensures]` with `old()` | **exercised** | `crates/tauten/tests/behaviour.rs` (runtime), 6 trybuild compile-fail tests |
| P0 | Proof plan probe: Charon/Aeneas on tauten-shaped code | **exercised** (local, 32 CPUs) | `docs/proofs/probe.md`; probe retired in P1 (kept at `2885ee1`) |
| P1 | `tauten-check-kernel` (tier order, severity, lint exceptions); `tauten-check` calls it; 13 Lean theorems over the extraction; axiom + coverage audit; drift check; 9 mutants; `proofs.yml` workflow | **exercised** (local and GitHub CI) | `docs/proofs/README.md`; `proofs.yml` green on main 2026-11-18 (gate 2 min, job 7 min incl. image pull). Every claim is broken by at least one mutant |
| P2 | Baseline ratchet in `tauten-check-kernel` (`apply`, `stale`, `count_errors`, `increases`); `tauten-check` interns keys and calls it; 14 more Lean claims (R-conserve, R-cover, R-stale, R-exact, R-headroom, R-shrink, 2 non-vacuity examples); 8 more mutants; differential test against the old `BTreeMap` ratchet | **exercised** (local only; branch `proofs-p2`) | `docs/proofs/README.md`; local proof gate passed (27 claims, 17 mutants); interning is trusted and covered by the differential test; not yet run on GitHub |
| M3 | Proof tier for users (Aeneas/Lean template from writfold) | not started | |
| M4 | Benchmark harness: host, 4 dev tasks, runner, analysis, admission gate | **exercised with built-in agents only** | `bench/README.md` (validation table); `scripts/bench-validate` in the gate |
| M4 | Pilot with a real agent (dev split) | **exercised** | `bench/reports/2026-11-18-pilot`: 36 trials, Sonnet 5.5. Ceiling: 0 escaped defects in every arm; enforcement removed unchecked arithmetic (A 6/12, B 5/12, C 0/12 trials). Held-out suite being designed |
| M5 | Repo (public), CI gate, tag-driven release workflow with crates.io trusted publishing (no stored token) | **exercised** | 0.0.1 (stored token) and 0.0.2 (trusted publishing) of all four crates published 2026-11-18 by `release.yml` after the gate passed. A new crate's first publish still needs a token |
| M7 | Website (tauten.dev) | **exercised** (live) | `tauten-dev/tauten-dev.github.io`, generated from a pinned commit with drift checks; benchmark section shows the dev-split pilot |
| M8 | Writfold adoption | not started | |
| — | Plyward review | done | `docs/reviews/plyward-2026-11-18.md`; three defects fixed in `e7fbc8c` |

## Quality

- Gate (`docker compose run --rm gate`):
  - format;
  - build;
  - unit, fixture, ratchet, behaviour and compile-fail tests;
  - self-check (tauten passes its own profile);
  - `docs/RULES.md` drift check;
  - benchmark admission;
  - `cargo deny`.
- Mutation testing (`docker compose run --rm mutants`): first run 14 of 137
  viable mutants missed in `tauten-check`; after the new tests, 0 missed.

## Design decisions made while building (with reasons)

- **The CLI is the rustc wrapper.** One `cargo check` with
  `RUSTC_WORKSPACE_WRAPPER=cargo-tauten`, so each crate gets exactly its own
  tier's flags. `cargo clippy -- flags` applied flags to dependencies too.
- **Lints compile at warn level; the profile assigns severity.** With deny
  levels, rustc fails and dependents are never checked.
- **No rustc `forbid`.** Derive macros (clap, among others) emit
  `#[allow(clippy::...)]`, which rustc rejects under forbid. "No exceptions"
  is a structural check instead (TAU-X004).
- **Separate profile for test targets** (`in_tests`). Calibrated on writfold,
  where most panic and IO findings were legitimate test code.
- **Refined types live in a private generated module**, so even the defining
  module cannot bypass `try_new`.
- **Contracts only on `Result`-returning functions.** A violation is a
  returned error in every build profile; there is no release-mode disabling
  and no panic.
- **Baseline keyed without line numbers, and it may only shrink.**
- **The build directory is keyed by a hash of the profile and tier map**,
  because cargo does not fingerprint what a wrapper adds.

## Known gaps

- The capability rule is discipline, not confinement (see MAPPING.md).
- The kernel's arithmetic and indexing rules duplicate what an Aeneas proof
  already establishes. The proof tier should subsume them; see the dry-run
  report.
- S002 (placeholder comments) is a heuristic.
- The benchmark has 4 dev tasks on one host. The plan needs about 24 across
  3 hosts, with ≥ 16 held out. The dev-split pilot hit a ceiling, so harder held-out tasks are needed.
- Contracts do not yet support `async fn` or generic newtypes; they have no
  Kani (`cfg(kani)`) lowering yet.
- **No measurement of agent behaviour yet. Nothing here supports a claim that
  agents make fewer mistakes.**
