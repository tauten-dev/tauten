# Status

Updated 2026-11-18. Every line says how far an item got:
**researched → specified → implemented → exercised**. "Exercised" means it
ran against real input in this repository, and the evidence is named.

## Milestones (see PLAN.md §6)

| # | Item | State | Evidence |
|---|---|---|---|
| M0 | Repo, plan, mapping, agent profile, rule table | **done** | `docs/`, `profile/AGENTS.md`, `crates/tauten-check/rules.toml` (37 rules, ~200 banned API paths) |
| M0 | Fixtures ported from Sovel reject/accept examples | **exercised** | `fixtures/cases`: 26 reject crates + 3 accept crates, exact expected rule IDs, asserted by `crates/cargo-tauten/tests/fixtures.rs` |
| M0b | Writfold dry run | **exercised** | `docs/dry-runs/writfold-2026-11-18.md`. It found 4 defects in tauten, now fixed. |
| M1 | `cargo tauten check` (human + JSON), `explain`, `explain --markdown`, `profile`, `--assume-tier` | **exercised** | fixture suite; self-check (tauten passes its own profile); writfold dry run |
| M1 | Structural checks T001–T003, K003, A002, S002, X002–X004 | **exercised** | unit tests in `tauten-check`; fixtures |
| M1 | `init` | not started | |
| M1 | Ratchet (baseline file, block only new violations) | not started; the dry run shows it is needed for adoption | |
| M2 | `tauten` (capability tokens) and `tauten-macros` (`newtype!`, `#[refined]`, contracts) | not started | |
| M3 | Proof tier (Aeneas/Lean template from writfold) | not started | |
| M4 | Benchmark harness and pilot | not started | |
| M5–M8 | Name, publish, website, writfold adoption | blocked on owner | |

## Gate

`docker compose run --rm gate` runs: `cargo fmt --check`; build; unit and
fixture tests; tauten's profile on tauten; a check that `docs/RULES.md` is
current; `cargo deny check`. `docker compose run --rm mutants` runs
cargo-mutants on `tauten-check`.

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
- **The build directory is keyed by a hash of the profile and tier map**,
  because cargo does not fingerprint what a wrapper adds.

## Known gaps

- The capability rule is discipline, not confinement (see MAPPING.md).
- The kernel's arithmetic and indexing rules duplicate what an Aeneas proof
  already establishes. The proof tier should subsume them; see the dry-run
  report.
- S002 (placeholder comments) is a heuristic.
- Diagnostics from macro expansions point at the macro call site, as clippy
  reports them.
- No measurement of agent behaviour yet. Nothing here supports a claim that
  agents make fewer mistakes.
