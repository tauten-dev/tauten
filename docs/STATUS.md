# Status

Updated 2026-11-19. Name final: tauten; license MIT; code at github.com/tauten-dev/tauten (public). Every line says how far an item got:
**specified → implemented → exercised**. "Exercised" means it ran against
real input in this repository, and the evidence is named. Versions 0.0.1
0.0.2 and 0.0.3 are on crates.io; the site is at tauten.dev. Development pilot
completed; held-out benefit not established.

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
| M2 | `tauten-macros`: `#[newtype]`, `#[refined]`, `#[requires]`, `#[ensures]` with `old()` | **exercised** | `crates/tauten/tests/behaviour.rs` (runtime), 6 trybuild compile-fail tests; generated code proved in Lean for listed instances (P3) |
| P0 | Proof plan probe: Charon/Aeneas on tauten-shaped code | **exercised** (local, 32 CPUs) | `docs/proofs/probe.md`; probe retired in P1 (kept at `2885ee1`) |
| P1 | `tauten-check-kernel` (tier order, severity, lint exceptions); `tauten-check` calls it; 13 Lean theorems over the extraction; axiom + coverage audit; drift check; 9 mutants; `proofs.yml` workflow | **exercised** (local and GitHub CI) | `docs/proofs/README.md`; `proofs.yml` green on main 2026-11-18 (gate 2 min, job 7 min incl. image pull). Every claim is broken by at least one mutant |
| P2 | Baseline ratchet in `tauten-check-kernel` (`apply`, `stale`, `count_errors`, `increases`); `tauten-check` interns keys and calls it; 14 more Lean claims (R-conserve, R-cover, R-stale, R-exact, R-headroom, R-shrink, 2 non-vacuity examples); 8 more mutants; differential test against the old `BTreeMap` ratchet | **exercised** (local and GitHub CI) | `docs/proofs/README.md`; local proof gate passed (27 claims, 17 mutants); interning is trusted and covered by the differential test; `proofs.yml` green on main 2026-11-18 (gate 206 s) |
| P3 | Proofs over what the macros generate, for listed instances: `crates/tauten-proof-instances` (unpublished; 4 `#[refined]` types, a `#[newtype]` control, 4 contract functions incl. `&mut` and `old`); a schematic shape per macro; per instance a shape theorem plus Plyward F01 targets 1, 2, 3, 5 and the contract claims; constructor audit; 60 more Lean claims (87 in all); 10 macro-source mutants | **exercised** (local and GitHub CI) | `docs/proofs/README.md` (P3 section); local proof gate passed (87 claims, 27 mutants). Wording: "proved for listed instances"; the macros themselves are not verified. `proofs.yml` green on main 2026-11-19 (gate 408 s) |
| P4 | Rule-table well-formedness in `tauten-check-kernel` (`table::validate`: unique IDs, unique lint owner, API bans name existing rules, every emitted ID present); `Profile::parse` interns names and calls it; `validate_ok_iff` + non-vacuity examples (2 claims, 89 in all); 5 mutants (32 in all); differential test against the old checks; reject fixture `fixtures/profiles/rej-missing-r001.toml` | **exercised** (local and GitHub CI) | `docs/proofs/README.md` (P4 section); local proof gate passed (89 claims, 32 mutants, 339 s warm). Error order is not a theorem; the differential test compares error texts on 3,000 random tables |
| P5 | Proof tier for users (M3): `proofs/claims.toml`; `cargo tauten proofs` (JSON outcome per claim: proved, drift, build-failed, audit-failed, mutant-survived, tool-missing; records `proofs/claims.json` with a source fingerprint); template `crates/cargo-tauten/templates/proofs` + `cargo tauten init --proofs`; N001/P003 findings in kernel functions with a fresh, total, proved claim reported as subsumed (kernel `subsume::subsumes`, 4 claims, 4 mutants); `proofs/TRUST.md`; badge data `proofs/claims.json` | **exercised** (local and GitHub CI) | Worked example `bench/hosts/ledger` (`apply_spec`, 2 mutants, `proved`, gate 53 s cold); fixture `fixtures/proofs-subsume` with a recorded gate run and `tests/proofs.rs` (fresh/stale/non-total/unclaimed, tool-missing without the toolchain); tauten's own gate via `cargo tauten proofs`: 20 of 20 claims `proved` (93 audited claims, 36 mutants, 409 s cold). CI does not run `cargo tauten proofs` yet; the site's proofs page is rendered from `claims.json` |
| M3 | Proof tier for users (Aeneas/Lean template from writfold) | **exercised** (local and GitHub CI) | see P5; `proofs.yml` green on main 2026-11-19 (93 claims, 36 mutants, gate 928 s) |
| M4 | Benchmark harness: host, 4 dev tasks, runner, analysis, admission gate | **exercised with built-in agents only** | `bench/README.md` (validation table); `scripts/bench-validate` in the gate |
| M4 | Pilot with a real agent (dev split) | **exercised** | `bench/reports/2026-11-18-pilot`: 36 trials, Sonnet 5.5. Ceiling: 0 escaped defects in every arm; enforcement removed unchecked arithmetic (A 6/12, B 5/12, C 0/12 trials). Held-out suite being designed |
| M5 | Repo (public), CI gate, tag-driven release workflow with crates.io trusted publishing (no stored token) | **exercised** | 0.0.1 (stored token) and 0.0.2 (trusted publishing) of all four crates published 2026-11-18 by `release.yml` after the gate passed. `tauten-check-kernel` first published 0.0.2 with a one-off token (2026-11-19), then trusted publishing; 0.0.3 of all five crates by `release.yml` |
| M7 | Website (tauten.dev) | **exercised** (live) | `tauten-dev/tauten-dev.github.io`, generated from a pinned commit with drift checks; benchmark section shows the dev-split pilot |
| M8 | Writfold adoption | **exercised** (worktree, not merged) | branch `tauten-adoption` in a separate worktree: tiers, 19 reasoned expects, baseline 712 → 679, then proof claims subsume 130 of 159 kernel N001/P003 findings → baseline 549; then four helpers rewritten to be total and re-proved (151 claims proved, 105-minute gate) → 142 subsumed, baseline 527; `scripts/tauten-check` passes with cargo-tauten 0.0.3 from crates.io. Awaiting the owner's review |
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
  already establishes (dry-run report). P5 subsumes them only for functions
  with a recorded, current, total proof. Writfold's adoption branch
  subsumes 142 of its 159 extracted-kernel findings this way, after
  rewriting four helpers to be total (`get` instead of indexing; 10 more
  findings disappeared with the indexing). The remaining 7 are in `splice`,
  whose only panic is a length overflow past `usize::MAX`.
- No credit through callers: a helper with a precondition gets no
  subsumption even when every caller is proved total.
- tauten trusts the project's gate output, including its total-shape check;
  it does not re-check the Lean theorems itself.
- Any edit under a claimed crate or `proofs/` stales every claim of that
  crate (writfold: a 1.5 h re-run). The workspace manifest (without its
  `version` lines) and the toolchain pin count too, since 2026-11-19.
- Mutant counting differs: writfold's gate counts only direct proof
  failures, tauten's also counts theorems that depend on a broken one.
- S002 (placeholder comments) is a heuristic.
- The benchmark's dev split (4 tasks, one host) hit a ceiling in the pilot.
  A held-out suite (20 tasks, 3 hosts, per-trial containers) is built on the
  local branch `bench-heldout` and kept unpublished until it has run; running
  it needs the owner's budget approval.
- `#[refined]` wraps only `tauten::refine::Frozen` types (primitives,
  `String`, std containers of those): interior mutability or a user-written
  `Clone` could break a checked predicate (external review, 2026-11-19).
  Predicates are arbitrary closures, not a checked fragment.
- Contracts do not yet support `async fn` or generic newtypes; they have no
  Kani (`cfg(kani)`) lowering yet.
- **Development pilot completed; held-out benefit not established.** The
  dev-split pilot could not separate the arms on escaped defects; nothing
  here supports a claim that the profile reduces escaped defects.
