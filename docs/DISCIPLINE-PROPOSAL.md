<!-- Planner proposal (Fable), 2026-11-19. Not yet approved; decisions pending in OWNER-DECISIONS.md.
     Part 2 amends part 1: its section 5 replaces part 1's milestone table. -->

# tauten as a discipline: code discipline + proofs (proposal, 2026-11-19)

Planner output. Nothing was built, run or modified; every number is quoted from the cited files. Statements about prior-art tools (Flux, Creusot, Prusti, Kani, dylint, generativity) are from memory, not checked today. All Rust APIs shown as "after" are proposals, not existing code.

## 0. Recommendation in one paragraph

Both real projects have already converged on one flow, and the proposal should follow them rather than the historical tool list: **Aeneas → Lean over the shipped kernel is the proof of record, TLC is a counterexample finder, Verus and TLAPS are retired or archived**. Gripsack's plan says so outright (`~/workspace/gripsack-f2-verus/plan/0056-verified-foundation.md` §2.3, §7; Verus removed at `1486f630`), and writfold archived Verus and the Lean calculus on 16 November (`~/workspace/writfold/docs/writfold/aeneas.md`, layer table). So tauten should become:

1. one shared proof gate and image, replacing three forked copies of the same scripts;
2. a small library for the four Sovel ideas (capabilities, effects-as-signatures, origin/evidence, refinements);
3. one obligations model in which lints are things to discharge (fix, prove, or justify), not bugs.

Do not build Verus or TLAPS adapters unless the owner asks; that is the first open question in section 7.

## 1. Inventory

### writfold (`~/workspace/writfold`, main)

| Item | What exists | Where |
|---|---|---|
| Aeneas/Lean gate | extract → drift check against committed `proofs/Extracted/` → `lake build` → axiom audit → mutants | `proofs/gate.sh`, `proofs/extract.sh` (shadow workspace), `proofs/Audit.lean` (444 lines), `proofs/mutants.py` (1,312 lines) |
| Pins | Lean v4.35.0, Aeneas `nightly-2026.11.07-c5eb4a2`, Charon `8cae8fd` built from source on `nightly-2026-09-17`, elan v4.2.4, amd64 only | `proofs/Dockerfile`, `proofs/lean-toolchain`, `proofs/lake-manifest.json` |
| Invocation | `docker compose -f compose.proofs.yaml run --build --rm proofs`; image built locally from source, sources copied in at build time | `compose.proofs.yaml`, `aeneas.md` "Running the gate without a Compose network" |
| Size and time | 674 theorems audited, 382 required claims, 236 mutants, **105 minutes** per gate run | `~/workspace/writfold-tauten/docs/writfold/tauten-adoption.md` (stage 2c) |
| TLA+ | `spec/KbWrite.tla`, `spec/RunGraph.tla` with 16 `*.mutant-*.cfg` and 8 `*.probe-*.cfg` | `spec/`. I did not find the script that runs TLC; `scripts/writfold_tools.py` only records tool versions |
| Tools image | Verus 0.2026.09.06, Z3 4.16.0, Lean **4.21.0**, TLC 1.7.4, TLAPS 1.6.0-pre, all sha256-pinned | `docker/Dockerfile.tools`, `toolchains.lock.json` |
| Receipts | qualification attempts written outside the checkout | `scripts/gate`, `scripts/writfold_gate.py`, `qualification/` |
| tauten adoption | tiers, baseline 712 → 527, 151 claims, 142 findings subsumed | worktree `writfold-tauten`: `proofs/claims.toml`, `proofs/tauten-gate.sh`, `proofs/tauten_gate.py`, `proofs/Total.lean`, `proofs/tauten_claims.py` |

Pain points:
- Any edit under the kernel crate or `proofs/` stales all 151 claims, which costs a 105-minute rerun (adoption doc, finding 8).
- Adoption needed four adapter files because the tauten template assumes a `Proofs` library layout (finding 7).
- Mutant semantics differ between writfold's and tauten's gates (finding 9).
- The tools image still carries Verus, TLAPS and a second Lean version for archived layers.
- The Aeneas-friendly code shapes are written down only in prose (`aeneas.md`, "Writing kernels for translation").

### gripsack

The main checkout (`~/workspace/gripsack`, branch `handover/h0-bundle-import`) is the old multi-tool state. The worktree `gripsack-f2-verus` is the new one.

| Item | Old state (main checkout) | Where |
|---|---|---|
| Verus | `cargo verus verify` on `gripsack-policy`; 333-obligation floor, named family floors, seeded mutants with attribution; Verus 0.2026.09.06.8dea4a2, Z3 4.16.0, Rust 1.98.0 | `Dockerfile` stage `verify`, `scripts/check_verus.py`, `scripts/verus_evidence.py` |
| TLC | positive configs plus calibrated negatives; each negative must violate exactly its named invariant (exit 12) or temporal property (exit 13) | `Dockerfile` stage `model`, `scripts/check_models.sh`, `specs/cfg/` |
| TLAPS | 40 modules, 4,623 obligations, per-module floors in a catalog | `Dockerfile` stage `tlaps`, `scripts/check_tlaps.py`, `scripts/tlaps_catalog.py`, `specs/recovery-proofs.json` |
| Fuzz | 6 libFuzzer targets under bubblewrap; replay by default, scheduled mode opt-in | `scripts/check_fuzz.sh`, `fuzz/run.py`, `fuzz/fuzz_targets/` |
| Evidence | guarantee ledger, 178-requirement delivery manifest, committed logs | `verification/guarantees.md`, `verification/delivery.json`, `verification/reports/` |
| Gate style | gates run at image-build time ("the image existing IS the gate") | `docker-compose.yml` |

New state (`gripsack-f2-verus`): `crates/gripsack-kernel` plus a `proofs/` directory that is a fork of writfold's. The Dockerfiles differ in four lines. Gate time grew from 191 s with 14 mutants at F1 to 3,669 s at F2 step 10a (plan 0056, lines 711-718 and 2431). TLC stays as at most five weekly bug-finders; fuzz goes weekly.

Pain points, from plan 0056 §1:
- TLAPS failed in CI on prover timeouts.
- "No proof is about the composition": Verus kernels, TLA+ protocols and tested bridges were separate.
- About 10k lines of evidence Python and 51 MB of committed logs.
- Hosted CI queues ran up to 14 hours.
- Inferred, not run: the `tlaps` stage pins checksum `a2860384…`, which `writfold/toolchains.lock.json` records as the *rejected* (replaced upstream) asset. A cold rebuild of that stage would probably fail.

Neither project uses Kani, proptest-as-gate or Miri. Loom appears only in `gripsack-exec/Cargo.toml`, and its scheduler was later deleted.

### Generic versus project-specific

| Generic (three forks today: writfold, gripsack F1/F2, tauten) | Project-specific |
|---|---|
| The toolchain Dockerfile (`tools` and `deps` stages) | The kernel crate and its `start_from` list |
| `extract.sh`, drift check, `refresh.sh`, `container.sh` | The theorems (`proofs/Writfold/`, `proofs/Kernel/`) |
| `Audit.lean` machinery: axiom closure, required claims, total-shape check, constructor audit | The required-claims list |
| The mutant runner: apply, re-extract, rebuild, attribute | The mutant definitions |
| The TLC runner with calibrated negatives | Specs and configs |
| Claims file, outcomes, fingerprint | Guarantee wording |

## 2. The discipline: four ideas in Rust

Strength labels follow `docs/MAPPING.md`: **C** rustc, **L** lint or tauten check, **P** proved, **V** convention.

### 2.1 Explicit capability passing and captured capabilities

**Meaning.** Authority is a value, created once at the edge and passed as a parameter. This is Sovel's `using` without implicit forwarding (`spec/AUTHORITY-AND-STARTUP.md` §2).

**What exists.** `tauten::cap::{Authority, Clock, Env}` and `dir` via cap-std (`crates/tauten/src/cap.rs`); rules A001, A002 and G001.

**Add.**
- A marker trait `Cap`, implemented for tauten's capabilities and by `#[derive(Cap)]` on user structs whose fields are all `Cap`.
- Attenuation helpers: a read-only directory, a sub-directory, a network endpoint via cap-std's pool.
- A capability bundle struct with `HasClock`-style accessor traits, to cut parameter noise.
- Two checks. A capability type stored in a struct field must be in a `#[derive(Cap)]` struct. A core function that returns `impl Fn` or `Box<dyn Fn>` and mentions a capability parameter gets a finding: "captured authority: return a named `Cap` struct".

**Enforceable.**
- C: unforgeable constructors.
- L: the ambient API bans; minting only in edge crates.
- L, heuristic: the two capture checks, written with `syn`.
- A precise capture check would be a dylint lint reading closure upvar types. Treat it as optional and image-only, because dylint needs a pinned nightly.

**Convention only.** Dependencies, `unsafe`, FFI and build scripts keep ambient access. Keep saying "discipline, never sandbox".

**Before / after (proposed).**
```rust
// before: core crate
pub fn is_stale(path: &Path) -> io::Result<bool> {
    let age = SystemTime::now().duration_since(fs::metadata(path)?.modified()?)...;
}
// after
pub fn is_stale(dir: &Dir, clock: &impl Clock, name: &str) -> Result<bool, StaleError> { ... }
```

**Prior art.** cap-std and its `ambient-authority` crate (reuse; gripsack-fs already depends on `cap-std = "4"`), Austral's capability types, Scala 3 capture checking (learn from; not expressible in Rust).

### 2.2 Effect tracking

**Meaning.** Rust cannot express effect rows. With A001 in force, a function's capability parameters *are* an upper bound on its row, alongside `Result` for `fail` and `async` for `suspend`.

**Add.**
- `cargo tauten effects`, which prints the derived row per `pub fn`, for example `{clock, fs(read), fail(StaleError)}`.
- A committed `effects.lock`. The check fails when a public function's row widens without re-recording, the same ratchet idea as the baseline.
- An optional `#[effects(clock, fs)]` attribute that asserts the row.
- For kernel crates the row is `{}`, and that is proved rather than linted: an Aeneas translation is a pure Lean function in the `Result` monad.

**Enforceable.**
- L: row derivation from signatures, with compile-time `T: Cap` assertions emitted by the macro so rustc resolves the types.
- L: widening detection.
- P: kernel purity.

**Convention only.** Effect polymorphism, per-resource labels such as `io(console)` versus `io(spare)`, and effects hidden in trait objects or closures. A row is sound only modulo the capability gaps in 2.1.

**Before / after.** The signature in 2.1 is the "after". The row is what the tool prints, and a later PR that adds `net: &Net` shows up as a diff in `effects.lock`.

**Prior art.** Koka and Effekt (the model); `cargo-semver-checks` (the ratchet-on-public-API pattern).

### 2.3 Origin tracking

Sovel's `from` covers three things (`spec/LANGUAGE.md` §3; MAPPING rows "Retained dependencies", "Instance identity", "Provenance / freshness"). Each gets a different answer.

1. **Retained storage: lifetimes (C).** Nothing to add.
2. **Freshness: an evidence token that borrows what it checked (C).** A checking function returns `Evidence<'s, P>` holding `&'s State`. Any mutation of `State` ends the borrow, so stale evidence does not compile. Gripsack already does this by hand: JOURNAL-AUTHORITY-001 says "Mutation permission borrows a durably published run" (`verification/guarantees.md`). tauten supplies the generic type and a `#[evidence]` macro for a private constructor.
3. **Instance identity: two options.**
   - A private-constructor bound handle (C plus P). Gripsack's `BoundOutputIndex` (`crates/gripsack-policy/src/graph.rs:29`, WORKSPACE-INDEX-001) is the pattern, with the "index in range for this catalog" property proved.
   - A generative lifetime brand (C), which cannot leave its closure scope. Offer it as `origin::scope`, and do not push it.

**Convention only.** Freshness against external state (files, Git, a database) cannot be typed. The discipline is a version stamp plus a recheck under the lock, which is writfold's KbWrite protocol, checked with TLC (`spec/KbWrite.mutant-NoRecheck.cfg`). Evidence that crosses an `await` or is stored in a struct outliving the state falls back to runtime version checks.

**Before / after (proposed).**
```rust
// before: check-then-use compiles even if `plan` changed in between
if plan.is_current(step) { plan.advance(step) }
// after
let ok: Evidence<'_, Current> = plan.check_current(step)?;   // borrows &plan
plan.advance(ok)                                             // consumes the evidence
```

**Prior art.** GhostCell and the `generativity` crate (brands), typestate crates, RustBelt-style "tokens".

### 2.4 Refinement and existential types

**What exists.** `#[refined]` with a sealed `Frozen` inner type; `#[requires]`/`#[ensures]`; a list of instances proved in Lean (STATUS, P3).

**Add.** The work already planned in `docs/PROOF-PLAN.md` §4:
- a checked predicate fragment (comparisons, `&&`, `||`, `!`, literals, constants, `len()`) with closure-free expansion;
- `#[refined(provable, ...)]`, which makes leaving the fragment a compile error;
- `cargo tauten proofs draft`, which emits the Lean statement skeletons (F01 targets 1, 2, 3, 5) for each provable type in a kernel crate.

**Existentials.** A refined newtype is already the package "some raw value such that P". A brand is an existential lifetime. No new syntax.

**Enforceable.** C: construction only through `try_new`. L: fragment membership. P: per type, in kernel crates.

**Convention only.**
- No static predicate propagation: `Percent + Percent` is checked again at runtime.
- Refinements over two values ("index into *that* vec") need 2.3's bound handle.
- Predicates outside the fragment are "tested", never "proved".

**Before / after.**
```rust
// before
pub fn set_volume(v: u8) { assert!(v <= 100); ... }
// after
#[refined(provable, |v| *v <= 100)] pub struct Percent(u8);
pub fn set_volume(v: Percent) { ... }   // kernel crate: try_new proved (ok iff v <= 100)
```

**Prior art.**
- `nutype`: learn from its derive handling.
- Flux: liquid types as a rustc plugin on nightly. Watch it; do not gate on it.
- Creusot and Prusti: other proof stacks. Skip, per "one proof language".
- Kani: a plausible bounded rung for core-tier code Aeneas cannot translate. Neither project uses it, so defer.

## 3. The proof flows as one discipline

**Which tool for what.**

| Code | Tool | Label |
|---|---|---|
| Kernel: pure decisions, `no_std`, Aeneas-shaped | Charon/Aeneas → Lean; total and spec theorems | Proved |
| Protocol: sans-IO state machines (gripsack ring 1) | Lean over the extracted machine where it extracts; TLC for interleavings and crashes | Proved / Model-checked (bounded) |
| Core: library code with capability parameters | runtime contracts, property tests, cargo-mutants | Tested |
| Edge: IO, parsers | e2e tests; fuzzing for parsers (gripsack's pattern) | Tested / Fuzzed |
| Verus, TLAPS | no adapter by default; the schema leaves room for one | — |

**One claims model: `proofs/claims.toml` version 2.** Keep the current fields and add:
- `tool`: `aeneas-lean` or `tlc`; `verus`, `tlaps` and `kani` reserved.
- `kind`: `total`, `spec`, `refinement`, `contract`, `invariant` or `pure`.
- `evidence`: theorem names, or a TLA module plus config plus invariant.
- `calibration`: source mutants, or negative configs with the invariant each must violate. Both projects already write these.
- `discharges`: rule or obligation IDs. This generalises `subsumes`.
- Outcomes: today's six, plus `counterexample` and `timeout`.

**Fingerprint.** Narrow it per claim to: the committed extraction of the crate, the proof modules the theorem imports, and the toolchain pin. The drift check already ties the extraction to the source. Editing untranslated code, tests or `claims.toml` then no longer stales 151 claims.

**Mutants.** Cache results by (mutated source hash, proof module hash) and run them in parallel. Mutants dominate both long gates.

**Audit.** One Lean library, shipped in the image like Aeneas is, provides the axiom-closure check, the required-claims check, the total-shape check and the constructor audit. This replaces three diverged `Audit.lean` files (444, 399 and 187 lines) and writfold's copy `Total.lean`. It also puts the total-shape check under tauten's control, which adoption finding 7 asked for.

**How proofs discharge obligations.**

| Obligation | Discharged by |
|---|---|
| N001, P003, L001 in a kernel function | a `total` claim (exists today) |
| P001, P002, P004 in a kernel function | the same `total` claim. Aeneas models a panic as failure, so I believe the argument carries over; confirm on a fixture before enabling |
| A `#[refined(provable)]` type | its drafted `refinement` claim |
| `#[requires]`/`#[ensures]` on a kernel function | a `contract` claim, the P3 shape theorems |
| Kernel effect row `{}` | extraction succeeding (`pure`) |
| An `Evidence`-guarded transition | a `spec` theorem on the checking function |
| Check-then-act against external state | a TLC `invariant` claim, labelled bounded |
| Capability rules | nothing. A002 stays lint-enforced; no proof is about a threat model (`proofs/TRUST.md`) |

Caller credit for helpers with preconditions stays out. Writfold showed that rewriting four helpers to be total was the cheaper fix.

## 4. Packaging

**Crates.** Two new ones; resist more.

| Crate | Tier | Responsibility | Change |
|---|---|---|---|
| `tauten` | core | `cap`, `refine`, `contract`, new `origin` (evidence, bound handles, `scope`), new `effect` (marker traits) | Add modules. Check whether it can offer a `no_std` feature set so kernel crates can use `refine`/`origin`; `cap.rs` uses `std` today |
| `tauten-macros` | — | `#[refined]`, contracts, new `#[derive(Cap)]`, `#[evidence]`, `#[effects]` | Extend |
| `tauten-check-kernel` | kernel | proved decisions | Add effect-row comparison and the generalised discharge decision |
| `tauten-check` | core | rules, scanning, effect derivation | Move `proofs.rs` (614 lines) out |
| **`tauten-proofs`** (new) | core | claims v2, outcomes, fingerprints, mutant plan; pure | From `tauten-check/src/proofs.rs` |
| **`tauten-gate`** (new, binary in the image) | edge | extract, drift, build, audit, mutants, TLC runner; replaces the forked `gate.sh`, `mutants.py`, `extract.sh`, `gate.py` | New; a project keeps only mutant definitions and theorems |
| `cargo-tauten` | edge | CLI | New subcommands below |

Dependencies: `cargo-tauten` → `tauten-check`, `tauten-proofs` → `tauten-check-kernel`. `tauten` → `tauten-macros`. `tauten-gate` → `tauten-proofs`.

**Images.** Two, both public (the proofs image is private today, per `docs/OWNER-DECISIONS.md`).

- `ghcr.io/tauten-dev/proofs:<lean>-<aeneas>-<charon>-<n>`
  - Contents: today's toolchain image, the warmed Charon sysroot, the mathlib cache, the audit Lean library prebuilt, `tauten-gate`, and a headless JRE with `tla2tools.jar` 1.7.4.
  - Size: about 20 GB unpacked today (`compose.proofs.yaml` comment). Measuring and trimming it is part of milestone D1.
  - amd64 only.
- `ghcr.io/tauten-dev/dev:<rust>-<tauten>`
  - Contents: stable Rust 1.98, `cargo-tauten`, cargo-mutants, cargo-deny.
  - Size not measured.

Versioning: the tag is the pin tuple, and projects reference the digest. A `tauten-toolchain.lock` in each project records the digest and each tool's sha256, generalising `writfold/toolchains.lock.json`.

**CLI.**
- `check`, `explain`, `init`, `baseline` — unchanged.
- `obligations` — counts by state: fixed, proved, justified, open.
- `effects [--record]`.
- `proofs run [--only <fn>] | status | refresh | draft`.
- `doctor` — Docker, image digest, pins.

**What the agent gets.**
- A profile with one section per idea, plus the Aeneas shape rules lifted from `aeneas.md` (no iterator chains, one loop per function, no derives in translated modules). That prose is the knowledge agents currently lack.
- `proofs draft`, which writes the claim entry and a Lean skeleton.
- `proofs run --only f`, which builds one module in seconds instead of the full gate.
- JSON everywhere.
- No per-project Dockerfile to build. Cold image build is about 11 minutes today (plan 0056, line 711).

## 5. Milestones

Effort is in agent-days. No kill criterion uses a defect benchmark.

| # | Deliverable | Acceptance | Effort | Validates on | Kill / pivot |
|---|---|---|---|---|---|
| D0 | A worked example crate in tauten showing all four ideas (capability parameters, an evidence token, a provable refined type, an effect row) with one proved claim. APIs may be hand-written, no macros yet | A fresh agent session given only the draft profile adds a feature to it and passes `check` and the proof gate without help, in 2 of 3 tries | 1.5 | tauten (dogfood) | Agents cannot follow it after one profile revision: stop and simplify the ideas before building tooling |
| D1 | Shared gate: public proofs image with the audit library and `tauten-gate`; claims v2; narrowed fingerprint; mutant cache and parallelism | writfold-tauten and gripsack-f2 pass `cargo tauten proofs run` with their `gate.sh`, `extract.sh`, `Audit.lean` machinery and adapters deleted; same theorem and mutant verdicts as before | 4-5 | writfold, gripsack | Either project needs more than 1 day of migration or keeps a fork: the abstraction is wrong, so ship the image only |
| D2 | Incremental proof loop: `--only`, per-claim staleness | In writfold, a one-function kernel edit re-verifies in under 15 minutes (105 today); a no-op edit to untranslated code stales nothing | 2 | writfold | Not under 30 minutes: accept the coarse gate and move it to nightly |
| D3 | Obligations reframing: `cargo tauten obligations`; N001/P003/N002 become obligations (section 6); profile and `explain` texts updated | writfold reports proved/justified/open counts matching today's 142 subsumed and 527 baselined | 1.5 | writfold | — |
| D4 | Capabilities and effects: `Cap`, `#[derive(Cap)]`, the bundle, `cargo tauten effects`, `effects.lock`, the two capture checks | One real edge-to-core extraction (candidates in the adoption doc: writfold-kb record parsing or delta) lands with capability parameters and a recorded row; gate time rises under 10% | 4 | writfold; gripsack-fs for cap-std fit | The extraction takes more than 2 days or adds more than 15% lines: keep the API, drop the ratchet |
| D5 | Origin: `Evidence`, `#[evidence]`, bound handles | Gripsack's journal permission and `BoundOutputIndex`, or writfold's Pin/Bind, re-expressed with tauten's types and no proof regressions | 2-3 | gripsack-f2 | The types do not fit either hand-written pattern: document the pattern and ship no library type |
| D6 | Refinements: the predicate fragment, `provable`, `proofs draft` for refined types | Three provable types in a real kernel get drafted statements that prove with at most minor edits | 2-3 | tauten instances, then gripsack `ids.rs` | Drafted statements need rewriting each time: keep the fragment, drop drafting |
| D7 | TLC backend in claims: calibrated negatives as `invariant` claims | writfold's `spec/` (16 mutant configs) and gripsack's bug-finders run through `proofs run` and are labelled "bounded" | 2 | writfold, gripsack | — |
| D8 | Site and README revamp (sections 6 and 8); rustdoc to the standard in section 8; release 0.1.0 | Site builds from a pinned ref with link checks; docs.rs builds all crates | 2-3 | tauten | — |
| Later | dylint capture lint; Kani rung; Verus/TLAPS adapters | Only on a concrete need | — | — | — |

D0 and D1 are independent; do D0 first because it is the cheapest test of the direction. D3 can run in parallel with D1.

## 6. Repositioning

**Pitch.** From "a strictness profile for Rust written by AI agents" to something like: "A discipline for Rust: authority passed explicitly, effects visible in signatures, checked evidence, and proofs over the shipped code." Agents remain the audience, not the claim.

**README and STATUS.**
- Lead with the four ideas and the proof tier.
- Move the benchmark to one paragraph: "We measured defect reduction and did not find it: 2 escaped defects in 200 unassisted trials; 0 of 120 sampled findings were reachable defects. tauten makes no defect-reduction claim."
- Link the E1 report.
- Retire PLAN §5 and §9 (the benchmark kill criteria) and replace them with this document's milestones.

**Rules.**
- Keep as deny: the structural and discipline rules A001, A002, G001, T001-T003, K001-K003, U001, U002, X001-X004, S001, M001, C001, O001, E002.
- Reframe as obligations: N001, N002, P003 and L001. They were 115 of the 120 audited findings (N001 67, P003 34, N002 14) and none was a defect. They stay mandatory in kernel crates, where a proof or a checked operation discharges them. In core crates they become a ratcheted count at warn level, with a `strict` profile to deny. Wording changes from "can panic" to "unproved bound".
- Flag the E001 findings that are not errors: E001's known false positives (MAPPING, "Typed failure" row) should become an allowlist of non-error `Err` types.
- P001, P002 and S001 were not in the E1 sample, so there is no evidence either way. Keep them as discipline.

**Benchmark.** Publish as already recommended in OWNER-DECISIONS item 2. Do not run E2 unless the owner wants the defect question back.

## 7. Risks and open questions for the owner

1. **Verus and TLAPS.** You named them, but both projects have retired or archived them. I recommend no adapters. Confirm, or name the code that still needs them (writfold-domain has 130 baselined findings inside `verus!` files).
2. **Proof-shaped code is a cost.** Aeneas accepts a narrow Rust subset, and gripsack's gate went from 3 to 61 minutes as theorems grew. D2 must work, or the discipline is nightly-only.
3. **Publishing the proofs image** (about 20 GB, built from other projects' releases) needs your go-ahead.
4. **Effect rows are derived from signatures and are an upper bound with known holes.** Is that enough to call it "effect tracking" on the site? I would label it "effects visible in signatures".
5. **Scope of 0.1.0.** I suggest D0 through D3 plus D8, with D4-D7 as 0.2.

## 8. Website (tauten.dev)

Current pages (`~/workspace/tauten-dev.github.io/website/sitegen/pages_home.py`, `pages_docs.py`, `pages_ref.py`): `/`, `/start/`, `/enforcement/`, `/agents/`, `/benchmark/`, `/proofs/`, `/status/`, `/rules/` plus 37 rule pages, `/crates/` plus per-crate pages.

**Proposed inventory.** Each page is one screen in the home page's shape: a pitch line, one code or CLI panel, a short list, links.

| Page | Action | Content |
|---|---|---|
| `/` | Keep shape, rewrite | New pitch; install; one panel; the four ideas as a four-item list in place of only tiers; links |
| `/discipline/` | **Add**; absorbs `/enforcement/` | Four sections, each with one before/after snippet, a strength label (compiler, lint, proved, convention) and one honest-gap sentence. The full mapping table stays in the repo, linked |
| `/proofs/` | Keep, shorten | What "Proved" means in five lines; the claim list rendered from `claims.json`; link to `TRUST.md` |
| `/start/` | Keep; absorbs `/agents/` | Install, `init`, `check`, `proofs`; a link to the raw agent profile in the repo instead of a rendered copy |
| `/rules/` | Keep the table | Add an "obligation" or "rule" column |
| `/rules/<id>/` | Keep, generated | These are the targets of diagnostic links; keep them to the `explain` capture only |
| `/status/` | Shrink | What has been exercised and on which project, plus the benchmark paragraph from section 6 with a link to the report. Rule examples move to the rule pages |
| `/benchmark/` | **Cut**; redirect to `/status/` | The preregistration and reports live in the repo |
| `/enforcement/`, `/agents/` | **Merge** as above | Redirects |
| `/crates/`, `/crates/<name>/` | **Cut** | Replace with links to docs.rs and crates.io on the home page and `/start/` |

**API documentation moves to docs.rs**, which crates.io links for every published library crate. For that to be good enough:
- Crate-level docs in `tauten`: one paragraph per idea with a runnable doctest each. `cap.rs` already has this shape.
- `compile_fail` doctests for the guarantees: a forged `Authority`, a refined type built without `try_new`, stale evidence.
- Macro documentation written on the re-exports in `tauten`, since users land there and not on `tauten-macros`.
- `#![deny(missing_docs)]` on `tauten`, `tauten-macros`, `tauten-proofs` and `tauten-check-kernel`.
- `[package.metadata.docs.rs] all-features = true`, so the `fs` feature's `dir` appears.
- A per-crate README as the crates.io landing text.
- The site generator links to versioned URLs (`docs.rs/tauten/<latest_release>/…`) and its link check covers them.

Caveat: docs.rs documents libraries, not binaries. The `cargo-tauten` CLI reference therefore stays on `/start/` as captured `--help` and `explain` output, which the generator already does.

### Critical Files for Implementation
- /home/tarek/workspace/tauten/crates/tauten-check/src/proofs.rs
- /home/tarek/workspace/tauten/crates/cargo-tauten/templates/proofs/gate.py
- /home/tarek/workspace/writfold-tauten/docs/writfold/tauten-adoption.md
- /home/tarek/workspace/gripsack-f2-verus/plan/0056-verified-foundation.md
- /home/tarek/workspace/tauten/docs/MAPPING.md

---

# Supplement to the tauten discipline proposal (2026-11-19): wider design space, run cadence, free tests

This amends sections 2–5 of the proposal I already delivered, in response to the owner's two additions. Sections 1 and 6–8 stand. Section 5's milestone table is replaced in full at the end. Nothing was built or run; three one-day experiments (X1–X3) decide the bold options. Claims about Flux, Kani, Creusot, Lean tactics and rustdoc JSON are from memory, not checked today.

## Headline

Three of the bold options earn a place, each behind a one-day experiment:

1. **Charon/LLBC as a fact platform** for effects, capability flow and call graphs, run in the image only. Experiment X2.
2. **Solver power through Lean, not beside it.** Auto-generate the obligation theorems and try a tactic portfolio; contracts are erased from release builds only where proved. Experiment X1.
3. **Kani as the bounded rung** for code Aeneas cannot translate. Experiment X3, which replays the E1 audit mechanically.

I recommend against a block DSL ("tauten-lang"), tauten's own rustc driver, tauten's own verification-condition generator, and a shared IR across verifiers.

## Section 2 additions: beyond library, macros and lints

### 2.5 Bold options: cost, risk, smallest experiment

| Option | What it buys | Cost | Risk | Smallest experiment | Verdict |
|---|---|---|---|---|---|
| **Fact platform on Charon/LLBC** | Inferred, whole-program effect rows; capability flow through closures and struct fields; call graphs; panic sites per function | 4–6 days after X2 | Charon is nightly-pinned and may not load edge crates with heavy dependencies. `docs/PROOF-PLAN.md` §3 rejected it as the everyday checker for that reason; that still holds, so this is an image-only tier | **X2 (1 day):** run Charon without Aeneas on `writfold-kb` and `gripsack-exec` in the existing image. Record load success, time and share of functions with bodies. A ~200-line script computes transitive effect rows and compares them with today's A001 findings | **Do, if X2 passes** (both load in under 5 minutes) |
| rustc driver / dylint / `stable_mir` | The same facts, with full type information | Same build cost, plus breakage on every toolchain bump | Users need a matching nightly; tauten's promise is one `cargo check` on stable | None separate; it is the fallback if X2 fails | Only as fallback |
| rustdoc JSON for public signatures | Resolved types for `effects.lock` without `syn` guessing | 1 day | Nightly-only output format | Fold into D4 | Maybe; decide in D4 |
| **Lean-side automation with certificates** | "SMT at build time" with no new trusted solver. Lean's kernel checks the result; `bv_decide` checks a SAT certificate, `omega` and Aeneas's `scalar_tac` produce kernel-checked proofs | 3–4 days after X1 | The automation may stall on loops, which need invariants | **X1 (1 day):** take writfold's 149 total theorems, replace each proof with one portfolio tactic, count how many still close | **Do, if X1 closes at least 40%** |
| **Erasable static contracts** | `#[requires]`/`#[ensures]` checked in debug and test builds, compiled out of release builds only where a fresh claim proves them | 3 days, inside D6 | Public entry points still need a runtime check, because outside callers are unproved | Part of D6: one kernel function in the tauten instances crate | Do |
| **Kani (CBMC) rung** | Bounded, automatic checking of core-tier code: overflow, indexing, contracts | 3 days after X3 | Bounds need unwind limits; results are "bounded", not "proved" | **X3 (1 day):** replay the E1 sample. Of the 118 findings refuted by hand, how many does Kani discharge with no harness work? Also count how many of those functions Charon/Aeneas translate | Do, if X3 discharges at least a third |
| Flux | Static refinement propagation (`Percent + Percent`) | 1 day to try | Research tool on its own nightly | Annotate `gripsack-kernel/src/ids.rs`; see whether it runs on the pinned toolchain | Try after D6; reserve `tool = "flux"` |
| Creusot / Why3, Prusti | Proofs over iterator and closure code Aeneas rejects | A second spec language and trust base | Contradicts "one proof language" (gripsack plan 0056 §2.3) | None now | Revisit only if kernel shape rules prove too tight |
| Own VC generator over LLBC + Z3 | Full control | Weeks; it is a verifier | The Sovel trap | — | **No** |
| "tauten-lang" block macro | Sovel-like signatures | 1–2 weeks | rustfmt and rust-analyzer stop working inside the block. writfold already paid this for `verus!`: its checker needed `verus_syn` (`docs/writfold/quality-gate.md`) | In D0, write the example with attributes and once with a block; see which a fresh agent gets right | **No block DSL.** Attributes only: `#[effects(..)]`, `#[requires(..)]`, checked as declared-versus-inferred |
| Shared IR across tools or languages | — | High | Each verifier has its own semantics | — | **No.** Share two small formats instead: the fact relations and `claims.toml` |
| **Kernel shape rules + shared proved helpers** | The Aeneas subset as fast lints on stable, so an agent learns "this will not extract" in seconds; a small crate of total helpers with their Lean lemmas shipped once | 3 days | Low | Count lemmas duplicated across the three proof trees; 15 files already touch slice-get lemmas | Do (in D1 and D3) |

### How this upgrades the four ideas

- **Capabilities.** With facts, "captured authority" becomes a real check: closure and struct captures are visible in LLBC (to confirm in X2), so the `syn` heuristic becomes the fast approximation and the fact check the authority.
- **Effects.** Rows are inferred transitively from the call graph and compared with the declared or signature-derived row. Dependencies Charon can load are covered; the rest are listed as opaque.
- **Origin.** A taint query: values built by an unchecked constructor reaching a function that requires evidence.
- **Refinements.** Fragment predicates are linear integer arithmetic, which is what `omega` and `scalar_tac` decide, so most refinement claims should close with no hand proof. X1 measures this.

## Section 3 additions

### 3.1 Solver integration and erasure

- `cargo tauten proofs auto` generates, for each kernel function without a claim, the `total` theorem and tries the portfolio. Success records a claim with `origin = "auto"`. Failure reports the open goal as the diagnostic.
- Existing proofs are already automation-heavy: writfold's 8,912 proof lines use `simp` 1,004 times, `step` 375, `scalar_tac` 365 and `omega` 215. `bv_decide` and `grind` are unused in all three trees.
- Static contracts: debug and test builds keep the check. A release build drops it only for functions whose `contract` claim is `proved` and fresh; `cargo tauten check` refuses the erasure otherwise. Public kernel entry points keep a checked wrapper.
- Nothing from a solver is trusted on its word: every accepted result is a Lean proof under the existing axiom audit. Kani and TLC results carry the label "bounded" and never discharge a "proved" obligation.

### 3.2 Run cadence

A cargo profile cannot do this: profiles control code generation, not which tools run. Expose it as one subcommand with levels, plus cargo aliases written by `init`. The one real profile hook is erasure, above.

| Level | Command | When | Runs | Time |
|---|---|---|---|---|
| edit | `cargo tauten check` | every change | lints, structure, kernel shape rules, claim freshness by hash comparison; no Docker | one `cargo check` |
| feature | `cargo tauten ship` (= `verify --level feature`) | before the agent reports done | edit level as errors; re-extract; re-prove stale claims only; auto-proof for new kernel functions; mutants for changed functions; generated property tests; TLC configs marked `quick` | target under 10 minutes; unmeasured until D2 |
| full | `cargo tauten verify --level full` | merge or nightly | every claim from a clean state, every mutant, all TLC configs, Kani if adopted, fuzz replay, cargo-mutants on core | today 105 minutes for writfold, 61 for gripsack F2 |
| release | `cargo tauten verify --level release` | on a tag | full level in the image by digest; scheduled fuzzing; every claim `proved` at that commit; badge data written | full plus fuzz budget |

Per-project configuration in `[workspace.metadata.tauten.cadence]`:
- per level: which stages run, a time budget, `mutants = "changed" | "all" | "none"`, and which TLC configs;
- `max_stale`: how long a claim may stay stale on the main branch (commits or days) before the edit level fails.

Work that exceeds the feature budget is listed as deferred, not skipped silently. CI maps directly: a pull-request job runs `feature`, a nightly job runs `full`, the release workflow runs `release`.

### 3.3 Stale claims and fingerprints

Today a stale claim turns its subsumed findings back into errors, which blocks the edit loop until a 105-minute rerun. Change:

- **Edit level:** a stale claim is a `pending-proof` warning, and its findings are shown as pending, not as errors.
- **Feature level and above:** pending is an error.
- `claims.json` records `stale`; a stale claim never shows "Proved" on the site.

A claim is fresh when three hashes match:
1. **Function closure:** the extracted Lean text of the claimed function and its transitive callees, with recorded source line numbers stripped so reformatting does not stale it.
2. **Proof modules:** the theorem's file and the project proof files it imports.
3. **Toolchain:** image digest and audit library version.

Do not go finer (per-theorem constant dependencies) until D2 shows it is needed. One caveat: writfold's extraction is a single file (`proofs/Extracted/WritfoldCore.lean`), so `lake` rebuilds every proof module after any kernel edit. Fingerprints fix the bookkeeping and the mutant selection; the rebuild time is unknown and D2 must measure it. If it dominates, the remedy is splitting the kernel crate.

### 3.4 Free tests

| Idea | Value | Cost | Verdict |
|---|---|---|---|
| **Generated property tests** from fragment refinements and contracts: a generator per refined type, and a test "any input meeting `requires` does not panic and meets `ensures`" | High; it is the core-tier rung where proofs do not reach | 2–3 days | **Do (D6)** |
| **Proof mutants run against the tests too:** per mutant, "killed by proof / killed by tests / survived" | High; the runner and definitions exist. Gripsack did this by hand when retiring Verus ("every former Verus mutant killed by a unit test", plan 0056 §8) | 1 day | **Do (D1)** |
| **Contracts as debug-build oracles**, erased in release when proved; plus `#[oracle(reference_fn)]` for the differential-test pattern tauten and writfold both use | High | Inside D6 | **Do** |
| **Lean examples replayed in Rust:** run the non-vacuity examples through the compiled function and compare | Tests the Charon/Aeneas translation that `proofs/TRUST.md` lists as trusted | 2 days; needs the extracted functions to be executable in Lean, unconfirmed | Experiment after D6 |
| Kani counterexamples as unit tests (its concrete playback) | Real, automatic | Free once Kani is in | With the Kani rung |
| TLC counterexample traces as Rust tests | Needs a model-to-code mapping per spec | High | **No.** Keep calibrated negative configs |

## Section 4 additions

- **New crate `tauten-facts`** (only if X2 passes): reads LLBC, emits relations as JSON lines (`calls`, `param_type`, `captures`, `effect_source`, `panic_site`) and runs the queries. `cargo tauten facts` and `effects --inferred` call it inside the image.
- **Optional `tauten-kernel-std`:** total helpers with their lemmas in the audit library.
- **CLI:** add `ship`, `verify --level <edit|feature|full|release>`, `proofs auto`, `facts`.
- **Image:** the proofs image is 19.8 GB on disk locally (`docker image ls`). Adding Kani would grow it; decide after X3.
- **Agent profile**, one short block: "After each change run `cargo tauten check`. Before reporting done run `cargo tauten ship` and include its summary. Pending proofs are allowed while editing and not when shipping. Report anything listed as deferred. Never edit `claims.json` by hand."

## Section 5, replaced: ordered path

Effort in agent-days. No criterion depends on a defect benchmark.

| # | Deliverable | Acceptance | Effort | Validates on | Kill / pivot |
|---|---|---|---|---|---|
| D0 | Worked example crate with all four ideas and one proved claim; written with attributes and once as a block macro | A fresh agent with only the draft profile adds a feature and passes, 2 of 3 tries; record which surface it handled better | 1.5 | tauten | Agents cannot follow after one profile revision: simplify before building |
| X1 | Auto-tactic replay | Share of writfold's 149 total theorems closed by the portfolio | 1 | writfold | Under 40%: no `proofs auto`; keep drafting skeletons |
| X2 | Charon facts on edge crates | Both crates load in under 5 minutes; inferred rows agree with A001 findings | 1 | writfold, gripsack | Fails: signature-derived effects only; no fact platform |
| X3 | E1 replay with Kani | Share of the 118 refuted findings discharged automatically | 1 | tauten-bench | Under a third: no Kani rung |
| D1 | Shared gate: public image, audit library, `tauten-gate`, claims v2, mutants run against tests, kernel shape rules | Both projects pass with forked scripts deleted and unchanged verdicts | 5–6 | writfold, gripsack | More than 1 day of migration per project: ship the image only |
| D2 | Cadence and incremental proofs: `ship`, `verify --level`, three-hash fingerprints, pending-proof state, cadence config | In writfold a one-function kernel edit ships in under 15 minutes; an edit to untranslated code stales nothing | 3 | writfold | Over 30 minutes: split the kernel crate, or make proofs full-level only |
| D3 | Obligations reframing; `proofs auto` if X1 passed; `tauten-kernel-std` seed | writfold's counts reproduce (142 proved, 527 baselined); auto-proof closes new functions unaided | 2–4 | writfold | — |
| D4 | Capabilities and effects: `Cap`, bundle, `effects.lock`; inferred rows and the capture check from facts if X2 passed | One real edge-to-core extraction in writfold-kb with a recorded row; gate time up under 10% | 4–6 | writfold, gripsack-fs | Extraction over 2 days or over 15% more lines: keep the API, drop the ratchet |
| D5 | Origin: `Evidence`, bound handles | Gripsack's journal permission or `BoundOutputIndex` re-expressed with no proof regressions | 2–3 | gripsack-f2 | No fit: document the pattern, ship no type |
| D6 | Refinement fragment, `provable`, static erasable contracts, generated property tests, `#[oracle]` | Three provable types prove from drafts; one contract erased in release with a fresh claim; generated tests kill at least one existing mutant | 4–5 | tauten instances, gripsack `ids.rs` | Drafts need rewriting each time: keep the fragment and tests, drop drafting |
| D7 | TLC backend with `quick` and full configs | writfold's and gripsack's configs run through `verify` and are labelled bounded | 2 | both | — |
| D8 | Site and README revamp, rustdoc standard, 0.1.0 | Site builds from a pinned ref; docs.rs builds all crates | 2–3 | tauten | — |
| Later | Kani rung (if X3 passed); Flux trial; Lean-example replay | Each on a concrete need | — | — | — |

Order: D0 first, then X1–X3 in parallel (three days, and they decide the shape of D3, D4 and the Kani rung), then D1 and D2. D3 can overlap D1.

## Section 7 addition

6. **Image-only analysis tier.** The fact platform and proofs need a pinned nightly inside Docker. Is "fast checks on stable, deep checks in the image" acceptable as the product shape? I recommend yes.
