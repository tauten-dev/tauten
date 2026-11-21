# Status

Updated 2026-11-21. Name final: tauten; license MIT; code at github.com/tauten-dev/tauten (public). Every line says how far an item got:
**specified → implemented → exercised**. "Exercised" means it ran against
real input in this repository, and the evidence is named. Versions 0.0.1
0.0.2, 0.0.3, 0.0.4, 0.0.5 and 0.0.6 are on crates.io; the site is at tauten.dev. Benchmark: no
defect-reduction benefit established (ceiling; held-out run stopped by its
go/no-go rule; audited findings were not defects).

## 0.0.6 (2026-11-21)

Milestone 0.0.6 closed: #49, #50, #51, #52, #53, #57, #59 (see
`docs/ROADMAP.md` and `CHANGELOG.md`). Exercised: the release workflow
skipped publishing when proofs failed on a drifted commit, and rehearsed a
publish after the gate (24 min) and full proofs (65 min) passed on main;
`proofs draft` re-drafted a removed claim on a real proof gate, which then
proved it; `proofs reconcile` on an adopter-shaped two-parent fixture with
a stand-in generator. Not exercised: `proofs draft --inventory` on a real
custom gate (#61), and `reconcile` with a real Charon/Aeneas translation.

## 0.0.5 (2026-11-21)

Discipline and proofs roadmap items closed on `main`: T4, T5, T6, T7, A5, A7,
A11, A12, D3 and B6; milestone 0.0.5 closed (D2, #39, #42, #46, A7b; see
`docs/ROADMAP.md`). tauten's own proof gate: 58 claims (3 of them
`proved-under` named filesystem assumptions), 103 calibrated mutants; the
fact-extracting scanners are calibrated by `cargo-mutants` in the CI gate
(0 surviving); the gate runs as the checkout's owner, not root. Writfold has
merged its adoption (156 of 156 claims proved).

D2 measured on tauten (2026-11-20; full gate 3577 s on 6d5012d): an edit to
Rust code outside the translation ran `--level ship` in 64 s with 0 mutants
(58 claims reused); renaming a local in the kernel's `compiler::passes`
re-calibrated exactly that claim (1 mutant, killed) in 89 s. The writfold
measurement needs a first full writfold run that records inputs; the owner
released 0.0.5 without it (2026-11-21), to be measured after release.

Published 2026-11-21 by `release.yml` from tag `v0.0.5` on 0fb7651. The first
tag (c167c36) failed `cargo publish` verification before uploading anything:
`templates/new` was left out of the `cargo-tauten` package because its
subdirectories held `Cargo.toml` files. 0fb7651 renames them to
`Cargo.toml.in`, and the CI gate now packages every published crate from its
tarball in a fresh target directory.

## Milestones (see PLAN.md §6)

| # | Item | State | Evidence |
|---|---|---|---|
| M0 | Plan, mapping, agent profile, rule table (42 rules, ~200 banned API paths) | **done** | `docs/`, `crates/tauten-check/agents.md`, `crates/tauten-check/rules.toml`, generated `docs/RULES.md` (drift-checked) |
| M0 | Fixtures ported from Sovel examples | **exercised** | `fixtures/cases`: 43 reject + 6 accept crates (plus workspace fixtures `layers`, `rej-overflow`, `generated`, `inherit`, `extract`, `proofs-subsume`), exact expected IDs, one tier per crate (`tests/fixtures.rs`) |
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
| M4 | Pilot with a real agent (dev split) | **exercised** | `bench/reports/2026-11-18-pilot`: 36 trials, Sonnet 5.5. Ceiling: 0 escaped defects in every arm; enforcement removed unchecked arithmetic (A 6/12, B 5/12, C 0/12 trials). Calibration rounds 1-3 (200 trials, 2 escaped) and the E1 audit: see Known gaps |
| M5 | Repo (public), CI gate, tag-driven release workflow with crates.io trusted publishing (no stored token) | **exercised** | 0.0.1 (stored token) and 0.0.2 (trusted publishing) of all four crates published 2026-11-18 by `release.yml` after the gate passed. `tauten-check-kernel` first published 0.0.2 with a one-off token (2026-11-19), then trusted publishing; 0.0.3, 0.0.4 and 0.0.5 of all five crates by `release.yml` |
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
- The benchmark hits a ceiling. Three calibration rounds without the
  profile (200 trials, Sonnet 5.5 and Haiku 5.5, 7 tasks on 3 hosts, hidden
  oracles in dev and release builds) had 2 escaped defects. The
  preregistered go/no-go rule (10-26 of 40 on the probe tasks) came out
  no-go (Sonnet 1/40, Haiku 0/40), so the held-out run was not started.
- Finding validity (audit E1, registered before classifying): of 120
  sampled deny-level findings in that agent-written code, two independent
  classifiers confirmed **0** as reachable defects (exact 95% interval
  0-3.0%); 118 refuted with a written bound argument, 2 undetermined. Neither
  escaped defect was on a flagged line. On these hosts the profile's
  findings are predominantly not defects. Untested: whether those bounds
  survive later edits (maintenance chains, not run).
- `#[refined]` wraps only `tauten::refine::Frozen` types (primitives,
  `String`, std containers of those): interior mutability or a user-written
  `Clone` could break a checked predicate (external review, 2026-11-19).
  Predicates are arbitrary closures, not a checked fragment.
- Contracts do not yet support `async fn` or generic newtypes; they have no
  Kani (`cfg(kani)`) lowering yet.
- **No defect-reduction benefit established.** The pilot could not
  separate the arms, the held-out run was stopped by its own go/no-go rule,
  and audited findings were not defects; nothing here supports a claim that
  the profile reduces escaped defects.
