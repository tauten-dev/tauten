<!-- Planner survey of recurring patterns across the owner's projects, 2026-11-19. -->

# Patterns to lift into tauten: evidence from the workspace

Read-only research; nothing was built or run. I read the main checkouts plus `writfold-tauten` and `gripsack-f2-verus`; the other `gripsack-f2-*` and `writfold-*` worktrees were skipped as near-duplicates. Counts are from grep and are approximate. Paths are relative to `~/workspace/`. `kctx`, `griplint-*` and `gripfetch-conformance` are Python and yielded only the conformance-suite pattern.

Two facts change the proposal:
- **The proof tree is forked six times, not three.** `proofs/{extract.sh,gate.sh,refresh.sh,Dockerfile,mutants.py,Audit.lean}` exist in writfold, gripsack-f2-verus, strop, weft, splicery and tauten, plus the `cargo-tauten` template. All use the same `lean-toolchain`.
- **Capability plumbing barely exists outside gripsack.** `cap_std` appears in 28 gripsack files and nowhere else. The only `Clock` trait is `cadence/crates/core/src/ports.rs:13`. Ambient `std::fs` use is over 1,000 lines each in writfold, gripsack and strop.

## 1. Pattern table

| Pattern | Where | Occurrences | Same shape? | tauten form | Saves an agent |
|---|---|---|---|---|---|
| Forked proof gate | `*/proofs/` in the six projects | 6 forks; 445 mutants in total | Yes. writfold and gripsack `extract.sh` are identical after substituting the crate name; the `mutants.py` runner tail differs by 15 lines between writfold and splicery | Gate stage (`tauten-gate`) plus `mutants.toml` | Copying about 900 lines per project; hard-coded crate names |
| `start_from` coverage check | `strop/proofs/gate.sh:11-27` only | 1 | — | Gate stage for all | A module silently shipping unproved |
| Byte-equality and membership loops with Lean specs | `gripsack-kernel/src/retention.rs:96`, `weft-core/src/scope.rs:23`, `splicery-splice/src/splice.rs:322`, `writfold-core/src/bytes.rs:5`; `contains` in writfold `plan.rs`, weft `sets.rs`, tauten `table.rs` | 4 `same_bytes` and 4 `*_loop_spec` proofs; 3 `contains_spec` | Yes, identical code and near-identical proofs | `tauten-kernel-std` crate plus Lean lib | Re-proving a 30-line loop invariant |
| Hand-written enum `same` and `convenience.rs` | gripsack `convenience.rs` (1,907 lines, 100 impls, 82 of them `fmt::Debug`); `splicery-splice` and `weft-core` `convenience.rs`; 17 `fn same` in gripsack-kernel | 3 kernels | Yes | Derive (`Same`) plus attribute | About 2,000 lines; forgetting a variant pair |
| Typed-ID macros | writfold `identities!` (3 copies) and `digest_types!`; gripsack `hash_domain!`, `digest!`, `identity!`, `occasion_id!`; cadence `typed_name!`, `typed_counter!`; strop `coordinate!`; `weft-domain/src/ids.rs` has 9 hand-copied blocks | 11 local macros in 4 projects; about 59 plain newtypes | Four families: digest, grammar string, index, counter | `tauten::id!` family | Reinventing hex parsing and serde each time |
| Refined values | cadence `BasisPoints`, `ViewSlot` (`core/src/id.rs:242,282`); gripsack `FileMode` | About 58 `new/parse -> Result<Self>` | Yes | Existing `#[refined]` with a custom error | — |
| Evidence and bound handles | `gripsack-policy/src/graph.rs:29`; main-checkout `journal_protocol.rs:72` and `journal/mutation.rs:29`; `gripsack-fetch/src/limits.rs:87`; `activation/ledger.rs:45`; `splicery-domain` `ReadCapture::new`; weft `pub(crate) fn at` | About 7 real tokens in 3 projects | Three sub-shapes | `Evidence<'s,P>`, `Bound<B>`, `#[evidence]` | Leaving a constructor public |
| Sans-IO machines | gripsack-kernel `recover`, `apply`, `gc`, `rollback`, `activation`, `journal_run`: each has `Command`/`Event`/`Phase`/`Outcome` and `State::new()` | 6, one project | Yes within gripsack | Template and profile section, no macro yet | Naming and shape |
| Atomic write and lock | `writfold-kb/src/write/atomic.rs`; `gripsack-fs/src/lib.rs:126` (typestate); `strop-core/src/buffer/io.rs:198`; `cadence/crates/data/src/files.rs` | 4 | Same skeleton, different substance | `tauten::durable` (std, cap-std) | A missed parent-directory fsync |
| Seeded test RNG and reference model | `struct Rng(u64)` or `Lcg` in writfold (6), strop (10), tauten (2); `tauten-check/src/baseline/reference.rs` | 18 copies | Yes (xorshift) | `tauten-test`: `Rng`, `differential!` | Boilerplate; inconsistent seeds |
| TLC with calibrated negatives | `gripsack/scripts/check_models.sh`, `splicery/scripts/run_model.py`, `weft/scripts/run_model.py`, `cadence/tools/run_models.py`, writfold `spec/*.mutant-*.cfg` | 5 | Same idea, four manifest formats | Gate stage (D7) | — |
| Receipts and evidence | `splicery/scripts/evidence.py` and `weft/scripts/evidence.py` (13 differing lines); `writfold/scripts/writfold_evidence.py`; `writfold/qualification/gate-receipt.schema.json`; `toolchains.lock.json` in 3 projects; `release-evidence/` | 3 to 4 | Core is the same: source manifest, `run_logged`, write-new | `cargo tauten verify --receipt` | About 300 lines of Python per project |
| Lint tables | `[workspace.lints]` identical in writfold, splicery, weft; cadence a superset; gripsack and strop use only `clippy -D warnings` in the Dockerfile; `writfold-core/src/lib.rs:20-36` restates lints because the shadow workspace drops them | 4 plus 2 | Yes | `cargo tauten init` writes it; `extract.sh` carries `workspace.lints` | — |
| Error enums | thiserror in writfold (12), gripsack (22), strop (27); hand-rolled in cadence, weft, splicery; `io::Error::other` 391 times in gripsack | — | No | Nothing | — |
| `#[must_use]` | cadence 105, tauten 75, others 5 or fewer | — | No | An existing lint only | — |

## 2. Top 8 to ship

**1. `tauten-gate` with `mutants.toml` and a `start_from` coverage stage** (edge binary; D1). Mutants today are a Python dataclass (`writfold/proofs/mutants.py:27-40`):
```python
Mutant("same-ignores-last-byte", "bytes.rs", "    while i < left.len() {...", "    while i + 1 < left.len() {...", ("Writfold.Core.same_loop_spec",)),
```
After:
```toml
[[mutant]]
name = "same-ignores-last-byte"
file = "bytes.rs"
before = "while i < left.len() {"
after  = "while i + 1 < left.len() {"
fails  = ["Writfold.Core.same_loop_spec"]
```
The kernel crate comes from `[package.metadata.tauten] tier = "kernel"`. Strop's `Patch` (Lean-side patches) and `PAIRED` fields need optional keys.

**2. `tauten-kernel-std`** (kernel, `no_std`, no dependencies; D3 seed).
```rust
pub fn same_bytes(a: &[u8], b: &[u8]) -> bool;
pub fn contains_bytes(haystack: &[Vec<u8>], needle: &[u8]) -> bool;
```
Lean: `Tauten.Std.same_bytes_spec : same_bytes a b ⦃ r => r ↔ a.val = b.val ⦄`, shipped in the audit library. Before: the same function at `gripsack-kernel/src/retention.rs:96-108` and the proof at `splicery/proofs/Kernel/Render.lean:184` onward, repeated four times. Risk (inferred): Charon must translate a dependency crate inside the shadow workspace; today every kernel has zero dependencies by rule. Fallback: `cargo tauten proofs vendor-std` copies the source and lemma.

**3. `#[derive(Same)]` and `#[kernel_type]`** (kernel, `no_std`). Before, `gripsack-kernel/src/journal.rs:36`:
```rust
pub fn same(self, other: PublicationAction) -> bool {
    matches!((self, other), (Self::Stage, Self::Stage) | (Self::SyncFile, Self::SyncFile) | ...)
}
```
plus a hand-written `impl fmt::Debug` in `convenience.rs:224`. After:
```rust
#[tauten::kernel_type(conveniences(Debug, Clone, Hash))]
pub enum PublicationAction { Stage, SyncFile, PublishName, SyncNamespace, Complete, Refused }
```
It expands `same` in the flat `matches!` shape Aeneas accepts and emits the trait impls where Charon will not reach them. **Unverified:** whether Charon can exclude macro-emitted impls inside a `start_from` module. Run a one-day fixture experiment (call it X4) first. If it fails, ship `derive(Same)` only.

**4. `tauten::id!` family** (`no_std`; `serde` feature in std).
```rust
tauten::id!(digest  pub struct PlanDigest([u8; 32]));           // parse lower-hex 64, Display, from_bytes
tauten::id!(index   pub struct DeclId(usize));                  // pub(crate) at(), index()
tauten::id!(counter pub struct MonoTick(u64));                  // INITIAL, next() -> Result
tauten::id!(string  pub struct RootId(String), grammar = is_root_id, error = IdentityError);
```
Before: `weft-domain/src/ids.rs:3-12`, repeated nine times; and the 64-hex parser written twice in gripsack (`gripsack-ir/.../identity.rs:17`, `gripsack-store/src/jobs.rs:10`). Kernel variants emit `same` and no derives.

**5. `tauten-test`** (dev-only).
```rust
pub struct Rng(u64);  // new(seed), next(), below(n), pick(&[T])
tauten_test::differential!(seeds 1..=3000, |rng| gen(rng), kernel_fn, reference_fn);
```
Before: `writfold-kb/tests/verdict.rs:12-20`, `strop-engine/src/editor/conformance.rs:16-28`, `tauten-check/src/baseline/tests.rs:95`. It also gives the proposal's `#[oracle(reference_fn)]` a runtime to target.

**6. Origin types** (kernel, `no_std`; D5).
```rust
pub struct Evidence<'s, P> { _s: PhantomData<&'s ()>, _p: PhantomData<P> }  // private ctor via #[evidence]
pub struct Bound<B> { position: usize, _b: PhantomData<B> }
```
Before, `gripsack-policy/src/graph.rs:29-48`: `BoundOutputIndex { position }` with `bind_output_index(names, declared, candidate) -> Option<_>`. After: `#[evidence] fn bind_output_index(...) -> Option<Bound<Output>>`.

**7. `tauten::durable`** (std, `fs` feature, on `cap_std::fs::Dir`). Adopt gripsack's typestate and add writfold's probe:
```rust
Staged::create(dir, name, bytes)?.sync_file()?.publish(Replace | CreateNew)?.sync_parent()?;
pub fn with_probe(self, probe: fn(Step)) -> Self;   // crash-cut tests
pub struct DirLock; impl DirLock { pub fn acquire(dir: &Dir, name: &str, wait: Duration) -> Result<Self, LockHeld>; }
```
Before: `strop-core/src/buffer/io.rs:198-222` and `writfold-kb/src/write/atomic.rs:81-93`.

**8. Receipts and the TLC negative runner** (edge; D2 and D7). One `tla.toml` (`config`, `expect = "clean" | "invariant:Name" | "temporal:Name"`), keeping gripsack's exit-code 12/13 discipline from `scripts/check_models.sh`. One receipt schema generalised from `writfold/qualification/gate-receipt.schema.json`, written by `verify --level`.

## 3. What not to abstract

- **Error enums.** thiserror, hand-rolled Debug-as-Display (`weft-domain/src/lib.rs:30`) and `io::Error::other` strings are three policies. Do not ship a derive.
- **Receipt and witness structs that are data.** `strop-workspace/src/operation.rs:160` `PublicationWitness`, writfold `StageWitness` ("Rehydration is data, not a grant") and `stale::Checked` have public fields and serde. They are observations, not proofs-of-check, so `Evidence` does not fit.
- **`strop-picker` `Evidence<'a>`** (`query/plan/admission.rs:45`) is optional lookup context that happens to share the name.
- **Crash exploration.** gripsack's `MemFs` and `CrashPoint` model (`gripsack-host/src/storage/mem/explore.rs`) and writfold's `Probe` function pointer are different mechanisms. Share only the `Step` hook.
- **Generational IDs.** `strop-core/src/id.rs:21` `Id<K>` and cadence `ViewId` are runtime staleness checks tied to an arena, not typed IDs.
- **writfold `identities!`** carries Verus specs inside `verus!`. It is archived-layer code; do not target it.
- **Canonical encoding and hashing.** Each `canonical_*` in `gripsack-store/src/hash.rs` and `writfold-kb/src/gate/*` defines a different preimage. Share only hex encoding (about 37 helper functions across 7 projects).
- **A sans-IO machine macro.** Six instances in one project. Document the `Command`/`Event`/`Phase` shape and wait for a second project.
- **Release process** (`SHARED-BUILDBOT-CI-PLAN.md`, `release-evidence/`, `playbook/`). These are owner policy and per-release records. tauten should emit receipts they can consume, not encode Buildbot.

## 4. Milestone slots

| Item | Slot |
|---|---|
| 1 gate, `mutants.toml`, coverage stage; shadow workspace carries lints | D1. Change acceptance to "all five consumer forks deleted", or name which two |
| 2 `tauten-kernel-std` | D3 (already named there); the dependency-translation check belongs in D1 |
| 3 `derive(Same)` / `kernel_type` | New experiment X4 (1 day), then D1's kernel shape rules |
| 4 `id!` family | D6, validated on `gripsack-kernel/src/ids.rs` and `weft-domain/src/ids.rs` |
| 5 `tauten-test` | D0 for `Rng`; D6 for `#[oracle]` |
| 6 Origin types | D5 unchanged; add splicery `ReadCapture` as a third validation case |
| 7 `durable` | New milestone D4b after D4, since it needs the cap-std bundle; validate on writfold-kb |
| 8 Receipts, TLC | Receipts in D2 (`verify --level`); TLC in D7, with five runners to replace, not two |
| Lint table in `init` | D0 |

Inference, not observed: D4's capability work will cost more than estimated, because no project other than gripsack injects fs, clock or env today.

### Critical Files for Implementation
- ~/workspace/gripsack-f2-verus/crates/gripsack-kernel/src/convenience.rs
- ~/workspace/strop/proofs/gate.sh
- ~/workspace/writfold/proofs/mutants.py
- ~/workspace/tauten/crates/tauten-macros/src/lib.rs
- ~/workspace/gripsack-f2-verus/crates/gripsack-fs/src/lib.rs
