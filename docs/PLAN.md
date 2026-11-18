# Tauten plan (v0, 2026-11-18)

> Provenance: drafted by a planning subagent (Fable) after reviewing the Sovel
> handover and writfold; adopted by the owner as "Plan A". `tauten` is a
> **provisional** name — rename is cheap until the first crates.io publish.
> Status markers below are updated as work lands; see `docs/STATUS.md`.

## 1. Product

**Pitch.** Tauten is a strictness profile for Rust aimed at code written by AI
agents: a small set of crates (capability tokens; newtype / refinement /
contract macros), one command (`cargo tauten check`) that runs a pinned lint +
structure profile and emits repair-hint JSON, and an optional proof tier that
extracts a pure kernel crate to Lean via Charon/Aeneas and gates on drift,
build, axiom audit and mutants. Every rule says whether it is compiler-,
lint-, proof- or convention-enforced, and the effect on agent defect rates is
measured by a public, reproducible benchmark.

**v0.1 scope:** tiers (`kernel` / `core` / `edge`); lint + structure profile on
stable clippy; `cargo tauten init | check | explain` with JSON diagnostics;
`tauten` lib (capability tokens, `newtype!`, `#[refined]`, runtime-checked
`#[requires]`/`#[ensures]`); an agent profile (`AGENTS.md` fragment); a proofs
gate template generalised from writfold; a benchmark harness with first
results.

**Later, only if measured to help:** dylint custom lints, Kani harness
generation from contracts, SARIF/LSP output, mutants diff mode in CI, async
structured-concurrency helpers, Flux/Creusot/Verus bridges.

**Non-goals:** a new language/compiler/type-system plugin; effect rows in
Rust; whole-program verification; "formally verified" claims beyond listed
kernel theorems; a package manager; replacing clippy; claiming capability
discipline is sound in the presence of `unsafe` or arbitrary dependencies.

Name candidates (crates.io 404 at check time, domains/orgs unchecked):
`tauten`, `scantling`, `stanchion`, `trunnion`; spares `kerfline`, `hewline`,
`trueline`, `gudgeon`, `stricta`, `rigora`.

## 2. Architecture

Tiers replace effect rows, declared as
`[package.metadata.tauten] tier = "kernel" | "core" | "edge"`:

| Tier | Sovel analogue | Rules |
|---|---|---|
| `kernel` | `! {}` | `no_std`, no/allowlisted deps, no floats, no `dyn`, no panics/indexing/unchecked arithmetic; Aeneas-eligible |
| `core` | effects only via `using` | no ambient authority (fs/net/env/clock/rand/process only via capability params); typed errors |
| `edge` | trusted startup | may mint authority; kept thin |

Crates: `tauten` (capability tokens, checked helpers), `tauten-macros`
(`newtype!`, `#[refined]`, `#[requires]`/`#[ensures]`, tier attributes),
`tauten-check` (structural checks, pure), `cargo-tauten` (CLI). Plus
`profile/` (rules, AGENTS fragment), `template/proofs/`, `bench/`.

Cargo cannot inherit lint tables from a dependency, so **the CLI owns the
profile**: it passes lint levels on the command line and points clippy at a
generated config, so deleting a project config file cannot weaken the check.

Diagnostic schema: `{id, tier, severity, file, span, rule, why, enforced_by:
"rustc|clippy|tauten|proof|convention", fix, docs}` with stable IDs.

## 3. Sovel idea → Rust mechanism

See `docs/MAPPING.md` (kept as the single, unedited enforcement table that the
website will reproduce).

## 4. Proof tier

Generalise writfold's gate: Charon/Aeneas extraction of the `kernel` crate →
drift check against committed `proofs/Extracted/` → `lake build` →
`Audit.lean` (axiom allowlist + required claims) → mutants (each mutant must
break exactly its named theorem). Three rungs: **P0** property tests +
cargo-mutants; **P1** Kani from contracts ("bounded model-checked"); **P2**
Aeneas/Lean ("proved over the shipped Rust"). No Verus/Creusot.

Sovel's Lean tree is archived, not salvaged. Carried over as text: the attack
catalog and accept/reject examples (benchmark defect taxonomy + fixtures), the
axiom-audit idea, the design rationale.

## 5. Measurement

Question: does the profile reduce defects that escape an agent's "done", at
what cost in success, tokens and time?

- 24–30 tasks across 2 purpose-built fixture repos + 1 pinned real repo;
  visible tests, hidden acceptance tests, hidden defect oracle. 8 dev tasks
  (tuning), ≥16 held-out.
- Arms: **A** plain Rust; **B** guidelines only; **C** full profile in loop.
- Same model/harness/caps, fresh container per trial, ≥5 trials/task/arm, all
  trials reported.
- Primary metric (preregistered): **escaped defects per submitted task**,
  judged by behaviour, never by lint counts.
- Secondary: task success, reachable panics under fuzz, test-tamper events,
  stub rate, mutation score of agent tests, tokens/turns/time,
  could-not-finish rate, diagnostics by rule.
- Paired-by-task bootstrap 95% CI; "no measured difference" when the CI
  includes 0.
- Anti-gaming: preregistration committed before held-out run, oracles frozen
  by hash, raw transcripts published, second model.
- Website claims tagged **Measured / Observed / Proved / Designed**; "flags" not
  "prevents" for lint/convention rules; null results published.

## 6. Milestones

| # | Deliverable | Effort | Needs owner? |
|---|---|---|---|
| M0 | Repo, GUIDELINES, MAPPING, profile v0, tier fixtures, reject fixtures ported from Sovel | 1d | no |
| M0b | Dry-run profile on a scratch copy of writfold; violation counts per rule | 0.5d | no |
| M1 | `cargo-tauten` `init`/`check --format json`/`explain`; `tauten-check` structural checks | 2–3d | no |
| M2 | `tauten` + `tauten-macros`, trybuild compile-fail tests | 2–3d | no |
| M3 | Proofs template + `cargo tauten proofs`, starter kernel + theorem + mutant | 1.5–2d | no (image cache) |
| M4 | Bench harness, 2 fixture repos, 8 dev tasks, preregistration draft, pilot 8×3×3 | 3d | model budget |
| M5 | Final name, publish 0.1.0, public repo, CI | 0.5d | **yes** |
| M6 | Held-out benchmark + results page | 2d + run cost | **yes** |
| M7 | Website, every claim tagged | 1d | **yes** |
| M8 | Writfold adoption stages 1–2 | 1–2d | **yes** |

## 7. Adoption (writfold first, then gripsack, splicery, weft)

1. **Observe:** warn-only check beside `scripts/gate quality`; baseline only.
2. **Ratchet:** commit baseline; new code adds no violations; declare tiers.
3. **Replace:** swap generic `writfold-xtask` checks and `proofs/*.sh` for
   tauten's with identical gate behaviour.
4. **Tighten:** capability tokens at fs/git/clock boundaries, crate by crate.

Revert a stage if gate time rises >25% or agents loop on a rule. Each stage
needs the owner's go-ahead.

## 8. Sovel repo

Docs-only banner commit ("archived research, superseded by <name>"), tag
`archive-2026-11`, GitHub archive; delete nothing. Needs owner go-ahead.

## 9. Kill criteria

- After M4 + one tuning pass, arm C shows no escaped-defect reduction vs A
  (CI includes 0 and point estimate < 15% relative): stop product + website.
- C drops task success > 10 points or raises median tokens > 40% without
  ≥ 30% defect reduction: cut rules or stop.
- M0–M4 exceeds 10 agent-days by > 50% without pilot numbers: reassess.
- Proof tier can't be templated in 3 agent-days: ship without it.

## 10. Owner decisions

Name; license (decided: MIT); Sovel archive; benchmark models
and budget; writfold adoption timing; P2 in 0.1; early name reservation.
