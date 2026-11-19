<!-- Planner spec for milestone D0. Revision 2 amends revision 1. -->

# D0 implementation spec: the discipline example (2026-11-19)

Plan only; nothing was edited or run. Items marked **verify** are assumptions the executor must check first.

Files read: `Cargo.toml`, `crates/tauten/{Cargo.toml,src/lib.rs,src/cap.rs,src/refine.rs}`, `crates/tauten-proof-instances/{Cargo.toml,src/lib.rs}`, `bench/hosts/ledger/**` (manifests, `proofs/claims.toml`, `proofs/Proofs/LedgerKernel.lean`, `proofs/gate.py`), `bench/README.md`, `crates/tauten-check/agents.md` (36 lines), `scripts/gate`, `compose.yaml`.

## 1. Location and layout

`examples/discipline/`, its own workspace modelled on `bench/hosts/ledger`. Add `"examples"` to the root workspace `exclude`.

Domain: **job leases**. A worker holds a lease on a slot until an expiry time and may renew it. State is one file in a directory.

| Crate | Tier | Contents | Lines |
|---|---|---|---|
| `lease-kernel` | kernel, `no_std`, zero dependencies | `renew`, `Slot`/`bind`, `Refusal` | ~90 |
| `lease-core` | core | `Ttl` (refined), `Table`, `Unexpired` evidence, `LeaseCaps` bundle, load/save through a directory capability | ~250 |
| `lease-cli` | edge | `main`: acquires authority, builds the bundle, `claim`/`renew`/`show` | ~120 |
| `proofs/` | — | written by `cargo tauten init --proofs lease-kernel --write` | — |

Also `README.md` (effect table, section 5) and `AGENTS.md` (section 4).

CI: add one step to `scripts/gate` after `scripts/bench-validate`: `cargo tauten check` and `cargo test` in `examples/discipline`. The proof gate is run locally and `proofs/claims.json` committed; it does not join `proofs.yml` in D0.

## 2. Library API added to `tauten`

**`no_std` decision: a feature split, not a new crate.** `default = ["std"]`; `cap` sits behind `std`; `refine`, `contract`, `error` and the new `origin` build with `default-features = false` (plus `alloc` for `String`/`Vec`/`Box`). A separate `tauten-core` would force the macros to emit two crate paths. In D0 only `origin` must be written core-only; the kernel crate takes **no** dependency, so the proof template runs unchanged and offline. The full split lands in D1.

**`tauten::cap` (std), hand-written:**
```rust
pub trait Cap {}                                   // marker; not sealed; dyn-compatible
impl<T: Cap + ?Sized> Cap for &T {}
impl<T: Cap + ?Sized> Cap for &mut T {}
impl Cap for SystemClock {}  impl Cap for ManualClock {}  impl Cap for Env {}
#[cfg(feature = "fs")] impl Cap for cap_std::fs::Dir {}
pub trait Clock: Cap { fn now_ms(&self) -> u64; }  // breaking: outside Clock impls must add `impl Cap`

pub trait HasClock { type Clock: Clock; fn clock(&self) -> &Self::Clock; }
#[cfg(feature = "fs")] pub trait HasDir { fn dir(&self) -> &cap_std::fs::Dir; }

#[cfg(feature = "fs")] pub struct ReadDir { inner: cap_std::fs::Dir }   // attenuated: no write methods
#[cfg(feature = "fs")] impl Cap for ReadDir {}
#[cfg(feature = "fs")] impl ReadDir {
    pub fn read(&self, name: &str) -> std::io::Result<Vec<u8>>;
    pub fn sub(&self, name: &str) -> std::io::Result<ReadDir>;
}
#[cfg(feature = "fs")] impl Authority {
    pub fn read_dir(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<ReadDir>;
}
```
Unforgeability comes from constructors, as today; `Cap` only names the set. Later: `#[derive(Cap)]` generates the bundle impls.

**Bundle, hand-written in `lease-core`:**
```rust
pub struct LeaseCaps<'a, C: Clock> { pub clock: &'a C, pub state: &'a cap_std::fs::Dir }
impl<C: Clock> Cap for LeaseCaps<'_, C> {}
impl<C: Clock> HasClock for LeaseCaps<'_, C> { /* … */ }
impl<C: Clock> HasDir for LeaseCaps<'_, C> { /* … */ }
```
`show` takes a `&ReadDir`, so the attenuated form is exercised.

**`tauten::origin` (core-only), hand-written:**
```rust
pub trait Proposition { type Subject: ?Sized; }
pub struct Evidence<'s, P: Proposition> { subject: &'s P::Subject, witness: P }   // no Clone, no Copy
impl<'s, P: Proposition> Evidence<'s, P> {
    pub fn mint(witness: P, subject: &'s P::Subject) -> Self;
    pub fn subject(&self) -> &'s P::Subject;
    pub fn redeem(self) -> P;
}
```
`P` is declared by the user as a struct with a private field, so only its module can build one and therefore mint:
```rust
pub struct Unexpired { slot: Slot, now_ms: u64 }                 // private fields
impl Proposition for Unexpired { type Subject = Table; }
impl Table {
    pub fn check_unexpired(&self, slot: Slot, now_ms: u64) -> Result<Evidence<'_, Unexpired>, LeaseError>;
    pub fn renew(&mut self, proof: Unexpired, ttl: Ttl) -> Result<(), LeaseError>;
}
// use: table.renew(ev.redeem(), ttl)
```
- **Verify:** that `table.renew(ev.redeem(), ttl)` compiles under two-phase borrows. If it does not, bind `let w = ev.redeem();` first.
- **Known hole, to document:** after `redeem` the witness no longer borrows the table, so freshness is compiler-enforced only up to the redeem. Convention: redeem in argument position only.
- Add two `compile_fail` doctests: mutating the table while evidence is alive; constructing `Unexpired` outside its module.
- Later: `#[evidence]` generates the proposition and the mint.

**Bound handle: pattern only, in `lease-kernel`, no library type:**
```rust
pub struct Slot { index: usize }                                 // private field; Clone + Copy by hand
pub fn bind(len: usize, candidate: usize) -> Option<Slot>;       // Some iff candidate < len
impl Slot { pub fn index(self) -> usize; }
```
It is bound to a length, not to an instance; say so in the README.

**Effect row for D0:** a doc table only (section 5). No type, no command.

## 3. The proved claim

```rust
pub enum Refusal { Expired, Overflow }
pub fn renew(expires_ms: u64, now_ms: u64, ttl_ms: u64) -> Result<u64, Refusal>
// Expired if now_ms > expires_ms; else now_ms.checked_add(ttl_ms).ok_or(Overflow)
```
All `u64`, no casts, no derives in the module, a flat match: the Aeneas limits listed in `docs/PROOF-PLAN.md` §1.

`proofs/Proofs/LeaseKernel.lean`, same shape as the ledger's `apply_spec`:
- **`renew_spec`** (the `total` theorem; no hypotheses): for all inputs, `renew e n t ⦃ r => if n.val > e.val then r = .Err .Expired else if n.val + t.val ≤ U64.max then ∃ x, r = .Ok x ∧ x.val = n.val + t.val else r = .Err .Overflow ⦄`.
- **`renew_ok_iff`**: the result is `Ok` iff `n.val ≤ e.val ∧ n.val + t.val ≤ U64.max`. Derived from `renew_spec`.

`claims.toml`: one claim on `renew`, both theorems, `total = renew_spec`, `subsumes = ["TAU-N001", "TAU-P003"]`. Mutants:
- `expiry-boundary`: `now_ms > expires_ms` → `now_ms >= expires_ms`;
- `renew-wraps`: `checked_add` → `Some(wrapping_add(..))`, as in the ledger.

`bind` and `Slot::index` have no claim; they contain no arithmetic or indexing.

## 4. Draft agent profile

For D0 it lives in `examples/discipline/AGENTS.md`. `crates/tauten-check/agents.md` is shipped by `init` and captured by the site, so it stays untouched until the trial passes. It then becomes a second section, "Discipline", after the existing rules block (D3).

Text:

> **Authority is a parameter.** Core code never calls `std::fs`, the clock or the environment. Take `&impl Clock`, a `&Dir`, or the crate's capability bundle. Only `main` calls `Authority::acquire`.
> Before: `fn is_stale(path: &Path)` reading `SystemTime::now()`. After: `fn is_stale(dir: &Dir, clock: &impl Clock, name: &str)`.
>
> **A signature shows the effects.** If a function needs a new capability, add it as a parameter and add the row to the README effect table. Do not store a capability in a struct unless that struct implements `Cap`.
>
> **Check, then act, with evidence.** A function that needs a fact about some state takes the proposition value that only the checking function can produce. Call the check, pass `evidence.redeem()` directly as the argument, never keep it in a variable.
> Before: `if table.is_unexpired(s, now) { table.renew(s) }`. After: `let ev = table.check_unexpired(s, now)?; table.renew(ev.redeem(), ttl)?`.
>
> **Valid by construction.** A value with a rule (`1..=3600`) is a `#[refined]` type built with `try_new` at the boundary. An index is a `Slot` from `bind`, never a bare `usize`.
>
> **Decisions live in the kernel and are proved.** Kernel code: `u64`/`usize` only, no casts, no iterators or closures, `checked_*` arithmetic. If you change a kernel function: `cargo fmt`, `sh proofs/refresh.sh`, update the theorem in `proofs/Proofs/`, then `cargo tauten proofs` must report every claim `proved`. Never edit `proofs/claims.json` or delete a mutant.
>
> **Done means:** `cargo tauten check` clean, `cargo test` green, `cargo tauten proofs` all `proved`.

## 5. Checks and effects output

No new or changed `cargo tauten` rules; D0 relies on A001, A002, G001, tiers and the existing proof subsumption. No `cargo tauten effects`. `README.md` carries a hand-written table:

| Function | Row |
|---|---|
| `lease_kernel::renew`, `bind` | `{}` (extracted; proved pure) |
| `lease_core::Table::check_unexpired`, `renew` | `{fail(LeaseError)}` |
| `lease_core::load` | `{fs(read), fail}` |
| `lease_core::save` | `{fs(write), fail}` |
| `lease_core::renew_now` | `{clock, fs(read, write), fail}` |

**Verify** that a kernel crate with zero dependencies passes T003 and K003 as the ledger kernel does.

## 6. Acceptance trial

**Request, verbatim:**
> "Add a grace period. A lease may be renewed up to `grace_ms` after it expires. `grace_ms` comes from the environment variable `LEASE_GRACE_MS`, default 0, valid range 0..=60000; an invalid value is an error at startup. `lease show` prints the grace period. Follow AGENTS.md."

It touches all four ideas: the `Env` capability, a new refined type, the evidence check and the proved kernel function, whose theorem and mutant text must be updated.

**Pass** means all of:
1. `cargo tauten check` reports 0 errors, with no new `#[expect]`, no baseline and no tier change.
2. `cargo test` is green, and so are the evaluator's hidden tests, written beforehand: renew at `expires + grace` succeeds; at `expires + grace + 1` it is refused; `LEASE_GRACE_MS=60001` fails startup; default is 0.
3. `cargo tauten proofs` reports every claim `proved`, with both mutants still present and killed.
4. No ambient call outside `lease-cli`; `LEASE_GRACE_MS` is read through `Env`.

Record 1+2+4 (discipline pass) and 3 (proof pass) separately. The milestone needs both in 2 of 3 tries.

**Running it:**
- Model Sonnet 5.5, `claude -p "$(cat prompt)" --output-format json --max-turns 80 --dangerously-skip-permissions`, as in `bench/README.md`.
- The agent needs the proof tools, so use a throwaway image built `FROM` the proofs toolchain image plus stable Rust 1.98, `cargo-tauten` and the Claude CLI. Give it a fresh copy of the example per try and no Docker socket.
- **Verify** how the template's `proofs/run.sh` behaves when the tools are already on `PATH`. If it insists on Docker, have the trial call `python3 proofs/gate.py` and say so in the trial's `AGENTS.md`.
- Three tries, sequentially.

**"Could not follow it"** is any of:
- adds `#[allow]`/`#[expect]` or moves code to the edge to pass;
- reads the environment or clock in core;
- passes a bare `usize`/`u64` where the refined type or evidence was required;
- edits `claims.json`, weakens the theorem to a hypothesis-carrying form, or deletes a mutant;
- stops with the proof gate failing;
- asks for help.

After a failed round, revise the profile once and rerun three tries. A second failure triggers the D0 kill criterion.

## 7. Ordered steps

| # | Step | Acceptance |
|---|---|---|
| 1 | Verify the three assumptions (two-phase borrow on `renew(ev.redeem())`, T003/K003 with a zero-dependency kernel, `run.sh` in-image) | Each answered in a note; fallbacks chosen |
| 2 | `tauten`: `Cap`, blanket impls, `Clock: Cap`, `HasClock`/`HasDir`, `ReadDir`, `origin` | Existing tests pass; new doctests, including the two `compile_fail`; tauten self-check clean |
| 3 | `examples/discipline` workspace with the three crates and unit tests | `cargo tauten check` 0 errors; `cargo test` green; under 600 lines of Rust |
| 4 | `cargo tauten init --proofs lease-kernel --write`; write the two theorems, the claim and the two mutants | `cargo tauten proofs`: claim `proved`, both mutants killed; `claims.json` committed |
| 5 | `README.md` effect table and `AGENTS.md` | Table matches signatures by review |
| 6 | Wire the example into `scripts/gate`; exclude `examples` from the root workspace | `docker compose run --rm gate` green |
| 7 | Hidden tests, trial image, prompt file | A no-op "agent" fails the hidden tests; a hand-made reference solution passes all four criteria |
| 8 | Three tries; write `docs/dry-runs/discipline-d0.md` with per-try results and transcripts' failure points | 2 of 3 full passes, or a profile revision and one rerun |

---

# D0 implementation spec, revision 2: macros from the start (2026-11-19)

Plan only; nothing was edited or run. This replaces sections 2, 4, 5 and 7 of the D0 spec and amends 1, 3 and 6. The domain (job leases), layout, proved function `renew`, theorems, mutants and trial request are unchanged. Effort rises from about 1.5 to about 3.5 agent-days.

Additional files read: `crates/tauten-macros/{Cargo.toml,src/lib.rs}`, `bench/hosts/ledger/proofs/run.sh`.

## What the macros buy

Each one turns a mistake an agent would otherwise make silently into an error at `cargo check` time, on stable, with a span.

| Macro | Mistake it prevents |
|---|---|
| `#[derive(Cap)]` | Storing a non-capability, or a capability in an unmarked struct |
| `#[evidence(subject = T)]` | Building the proposition outside its module; deriving `Clone`/`Default` on it |
| `#[effects(..)]` | A signature that gained a capability without the declared row changing |
| `#[kernel]` | Writing kernel code Aeneas will reject, discovered only after a slow extraction |

No block DSL: every macro is an attribute or derive on ordinary Rust, so rustfmt and rust-analyzer keep working.

## 1. Layout (amended)

- `lease-kernel` now depends on `tauten` with `default-features = false`, for `#[kernel]` only.
- **Verify first:**
  - T003 allows `tauten` as a kernel dependency. The comment in `crates/tauten-proof-instances/Cargo.toml` says user kernel crates use the macros, but I did not read the allowlist.
  - The template's `proofs/extract.sh` copies a path dependency that lies outside the example workspace (`../../crates/tauten`).
  - The proofs container has network access to fetch `syn`/`quote`, which tauten's own `compose.proofs.yaml` needs for the same reason.
- `#[kernel]` expands to the item unchanged, so the extraction contains nothing from `tauten` and needs no Charon `include`.

## 2. Library and macro API

**Feature split, minimal, now required in D0.** In `tauten`: `default = ["std"]`; `cap` behind `std`; the crate is `no_std` plus `alloc` otherwise. `refine`, `contract`, `error`, `origin` and all macros work without `std`.

**`tauten::cap`**
```rust
pub trait Cap {}                                   // marker; dyn-compatible; not sealed
impl<T: Cap + ?Sized> Cap for &T {}
impl<T: Cap + ?Sized> Cap for &mut T {}
impl Cap for SystemClock {}  impl Cap for ManualClock {}  impl Cap for Env {}
#[cfg(feature = "fs")] impl Cap for cap_std::fs::Dir {}
pub trait Clock: Cap { fn now_ms(&self) -> u64; }
// ReadDir and Authority::read_dir as in revision 1
```
`HasClock`/`HasDir` are dropped: one concept fewer, and the bundle's fields are public.

**`#[derive(Cap)]`** (in `tauten-macros`, re-exported as `tauten::Cap`)
```rust
#[derive(Cap)]
pub struct LeaseCaps<'a, C: Clock> { pub clock: &'a C, pub state: &'a cap_std::fs::Dir }
```
Emits `impl Cap`, plus one compile-time assertion per field that its type is `Cap`. A non-capability field is a compile error pointing at the field.

**`tauten::origin` and `#[evidence]`**
```rust
pub trait Proposition { type Subject: ?Sized; }
pub struct Evidence<'s, P: Proposition> { /* private: subject: &'s P::Subject, witness: P */ }
impl<'s, P: Proposition> Evidence<'s, P> {
    #[doc(hidden)] pub fn __mint(witness: P, subject: &'s P::Subject) -> Self;
    pub fn subject(&self) -> &'s P::Subject;
    pub fn redeem(self) -> P;
}

#[evidence(subject = Table)]
pub struct Unexpired { slot: Slot, now_ms: u64 }
```
The attribute follows `#[refined]`'s private-module technique. It generates:
- the struct inside a private module, so no struct literal compiles anywhere;
- `impl Proposition for Unexpired { type Subject = Table; }`;
- `pub(super) fn evidence(subject: &Table, slot: Slot, now_ms: u64) -> Evidence<'_, Unexpired>`, callable only from the declaring module, where the checking function lives;
- a by-reference getter per field;
- a compile error for `Clone`, `Copy`, `Default` or `Deserialize` derives, reusing the list in `check_refined_attrs`.

Use, unchanged from revision 1:
```rust
impl Table {
    pub fn check_unexpired(&self, slot: Slot, now_ms: u64) -> Result<Evidence<'_, Unexpired>, LeaseError>;
    pub fn renew(&mut self, proof: Unexpired, ttl: Ttl) -> Result<(), LeaseError>;
}
table.renew(ev.redeem(), ttl)?;
```
Two items carry over from revision 1. **Verify** that this call compiles under two-phase borrows. The documented hole remains: after `redeem` the witness no longer borrows the table.

**`#[effects(..)]`**
```rust
#[effects(clock, fs_read, fs_write, fail)]
pub fn renew_now<C: Clock>(caps: &LeaseCaps<'_, C>, slot: Slot, ttl: Ttl) -> Result<(), LeaseError>
```
- Vocabulary: `clock`, `env`, `fs_read`, `fs_write`, `net`, `fail`, `suspend`; `#[effects()]` means none.
- The macro validates the labels and appends a rustdoc line, `Effects: {clock, fs_read, fs_write, fail}`, so docs.rs shows the row. It is otherwise a pass-through.
- The comparison with the signature is a `cargo tauten check` rule (section 5), because a proc macro cannot resolve types.

**`#[kernel]`** on a `fn`, an `impl` block or an inline `mod`. It emits a spanned `compile_error!` for:
- `as` casts;
- closures;
- `for` loops and iterator adaptor calls (`iter`, `map`, `filter`, `fold`, …);
- `dyn` and `impl Trait`;
- `#[derive]` on items inside;
- float literals and types;
- any macro call other than `matches!`.

These are the Aeneas limits listed in `docs/PROOF-PLAN.md` §1. It cannot see nested borrows or loop shape; the extraction still decides those. An out-of-line `mod x;` cannot be covered by an attribute macro, so the example puts `#[kernel]` on each item.

**Bound handle** stays hand-written inside `#[kernel]` items: `Slot { index: usize }` with a private field, `bind(len, candidate) -> Option<Slot>`, `Slot::index(&self)`. It has no derives and is not `Copy`, to stay inside the shape rules.

## 3. Proved claim (amended)

Unchanged, except that `renew`, `bind` and `impl Slot` carry `#[kernel]`. Add one `compile_fail` test showing `#[kernel]` rejecting an `as` cast.

## 4. Agent profile (replaces the draft)

Location and coexistence as before: `examples/discipline/AGENTS.md` for the trial, merged into `crates/tauten-check/agents.md` as a "Discipline" section after it passes.

> **Authority is a parameter.** Core code never calls `std::fs`, the clock or the environment. Take `&impl Clock`, a `&Dir`, or a `#[derive(Cap)]` bundle. Only `main` calls `Authority::acquire`.
>
> **Declare effects.** Every `pub fn` in a core crate has `#[effects(..)]`. If you add a capability parameter, add its label. Never remove a label to silence the check.
>
> **Check, then act, with evidence.** A fact about some state is a `#[evidence(subject = T)]` struct. Only its module's checking function creates it. Pass `ev.redeem()` directly as the argument; never store it.
>
> **Valid by construction.** A value with a rule is a `#[refined]` type built with `try_new` at the boundary. An index is a `Slot` from `bind`.
>
> **Decisions live in the kernel.** Every kernel item has `#[kernel]`; fix what it rejects, do not remove it. After a kernel change: `cargo fmt`, `sh proofs/refresh.sh`, update the theorem, and `cargo tauten proofs` must report every claim `proved`. Never edit `claims.json` or delete a mutant.
>
> **Done means:** `cargo tauten check` clean, `cargo test` green, `cargo tauten proofs` all `proved`.

## 5. Checks

One new rule, **TAU-F001 "Declared effects match the signature"**, in `tauten-check`'s `syn` scan:
- The row derived from a signature:
  - a `Clock` bound or `SystemClock`/`ManualClock` → `clock`;
  - `Env` → `env`;
  - `Dir` → `fs_read`, `fs_write`;
  - `ReadDir` → `fs_read`;
  - a `#[derive(Cap)]` struct in the same crate → the union of its fields;
  - a `Result` return → `fail`;
  - `async` → `suspend`.
- A finding when the declared and derived rows differ, or when a `pub fn` has no `#[effects]`.
- Matching is by type name, so it is a heuristic; the rule's `explain` text says so.
- Level: warning by default. It becomes an error only when the crate sets `effects = "required"` under `[package.metadata.tauten]`, which `lease-core` does. Existing adopters are unaffected.
- One reject fixture and one accept fixture under `fixtures/cases`.

`cargo tauten effects` stays out of D0. The README table is dropped: the attributes are the single source.

## 6. Trial (amended)

- `proofs/run.sh` requires Docker and exits with `tools: missing` otherwise (read, not run). Inside the trial image the agent calls `python3 proofs/gate.py` directly, and the trial's `AGENTS.md` says so in place of `cargo tauten proofs`.
- Add to "could not follow it":
  - removing a `#[kernel]` or `#[effects]` attribute, or a label, to get past an error;
  - setting `effects` back to its default in the manifest.
- Record, per try, how many compile errors came from each macro and whether the agent fixed each on the first attempt. That is the direct measure of whether the macros help.

## 7. Ordered steps

| # | Step | Acceptance |
|---|---|---|
| 1 | Verify: T003 allowlist; `extract.sh` with an outside path dependency; container network; two-phase borrow on `renew(ev.redeem())` | Each answered in a note; fallback chosen (fallback for the first three: kernel without `#[kernel]`, zero dependencies) |
| 2 | `tauten` feature split; `Cap`, `Clock: Cap`, `ReadDir`, `origin` | Builds with and without `std`; existing tests and self-check pass |
| 3 | `#[derive(Cap)]`, `#[evidence]`, `#[effects]`, `#[kernel]` in `tauten-macros`, re-exported from `tauten` | trybuild: one pass and at least one compile-fail case per macro (non-`Cap` field; literal construction and `Clone` on evidence; unknown effect label; `as` and a closure under `#[kernel]`) |
| 4 | TAU-F001 with fixtures; `docs/RULES.md` regenerated | Fixture suite green; drift check green |
| 5 | `examples/discipline` with the three crates and unit tests | `cargo tauten check` 0 errors with `effects = "required"`; `cargo test` green; under 600 lines of Rust |
| 6 | `cargo tauten init --proofs lease-kernel --write`; theorems, claim, two mutants | Claim `proved`, both mutants killed; `claims.json` committed |
| 7 | `AGENTS.md`; wire the example into `scripts/gate`; exclude `examples` from the root workspace | `docker compose run --rm gate` green |
| 8 | Hidden tests, trial image, prompt | A no-op agent fails; a hand-made reference solution passes all four criteria |
| 9 | Three tries; write `docs/dry-runs/discipline-d0.md` | 2 of 3 full passes, or one profile revision and a rerun |

Steps 2–4 change published crates. Keep them on a branch and unreleased until step 9 passes.
