# Sovel idea → Rust mechanism

This is the enforcement table the website will reproduce unedited. "Status"
says what exists in this repository today; "Strength" says what does the
enforcing.

**Strength:** **C** = rustc type system · **L** = lint or tauten check (escapable
only by a visible, reasoned `#[expect]`, and not at all where marked
"no exceptions") · **P** = proved for kernel functions (proof tier, planned) ·
**V** = convention/review only.

| Sovel idea | Rust mechanism | Strength | Status | Honest gap |
|---|---|---|---|---|
| No ambient authority (`using`) | Banned fs/env/process/clock/rand/net/stdio APIs in `kernel`/`core` (TAU-A001, generated `disallowed-methods`); authority minted only in `edge` (TAU-A002) | L | **Exercised** on fixtures and writfold dry run | Not confinement: dependencies, `unsafe`, FFI, build scripts and re-exports under other paths keep ambient access. Claim "discipline", never "sandbox". Capability token crate not built yet. |
| Contextual forwarding | Explicit capability parameters; no globals (TAU-G001) | L | Exercised (globals) | More verbose than Sovel's `using`; no implicit forwarding. |
| Retained dependencies (`from`) | Lifetimes, borrow checker | C | Rust | Sovel's graph-ID brands are not covered. |
| Effect rows (`! {…}`) | Tiers as coarse effect classes; `Result` for `fail`; `async` for `suspend` | C for fail/suspend, L for tier | Tiers exercised | **Rust cannot express per-function effect rows or effect polymorphism.** |
| Typed failure | `Result` + error enums; `Result<_, ()>` and type-erased errors banned outside edge (TAU-E002); silent drops (TAU-E001) and unexplained discards (TAU-E003) rejected | C + L | Exercised | Banned-type list is finite. |
| No panic | unwrap/expect (P001), panic/unreachable (P002), indexing/slicing (P003), panics in `Result` fns (P004), exit/abort (P005) | L; P for kernel (planned) | Exercised | Syntactic only. Panics inside dependencies and allocation failure are not covered. |
| Checked arithmetic | `arithmetic_side_effects`, `integer_division` (N001); lossy casts (N002); kernel floats (N003) | L | Exercised | A runtime overflow check is still a panic unless `checked_*` is used. |
| Exhaustive matching | rustc exhaustiveness + no wildcard over enums (M001) | C + L | Exercised | Foreign `#[non_exhaustive]` enums need a wildcard (excused with a reason). |
| Nominal newtypes | `newtype!` (private field, no `Deref`/`From`) | C | **Planned (M2)** | — |
| Refinements | `#[refined(pred)]`: validated constructor only | C | **Planned (M2)** | Runtime check, no static predicate propagation. |
| `requires`/`ensures`/`old` | Always-on contract attributes; Kani under `cfg(kani)`; Lean for kernel | runtime; P for kernel | **Planned (M2/M3)** | Not static outside the kernel; `old` limited to `Copy` scalars. |
| Structured concurrency | `thread::spawn`/`tokio::spawn` banned in kernel/core (C001); await-holding-lock etc. (C002) | L | Exercised (C001) | Async cancellation safety not enforceable. |
| No implicit repair (alloc/clone/refcount) | `Rc`/`RefCell`/`Arc`/`Mutex` banned in kernel (K001) | L (kernel), V elsewhere | Exercised | An agent can still add `.clone()` to quiet the borrow checker. |
| Determinism | `HashMap`/`HashSet` banned in kernel (K001); pinned toolchain; `--locked` | L | Exercised | No bit-for-bit reproducibility claim. |
| Termination | `infinite_loop` outside edge (L001); Lean totality for kernel | L, P (planned) | Exercised (L001) | No general termination checking. |
| No stubs | `todo!`/`unimplemented!`/`dbg!` (S001, no exceptions); placeholder comments (S002, warning) | L | Exercised | S002 is a heuristic. |
| Visible exceptions | `#[allow]` banned in favour of `#[expect(.., reason)]` (X001); no crate-wide opt-outs (X002); loosening `[lints]` reported (X003); no exceptions in kernel or for no-exception rules (X004) | L | Exercised | An agent with write access can still add a reasoned `#[expect]` (visible in review; ratchet planned). |
| `unsafe` honesty | `unsafe_code` in kernel/core, no exceptions (U001); documented single-op blocks in edge (U002) | L | Exercised (U001) | No containment of C. |
| Tier structure | Every crate declares a tier (T001); dependencies point inward (T002); kernel deps allowlisted (T003); kernel is `no_std` (K003) | L | Exercised | — |
| Contracts/tests on public items | Presence check per `pub fn` | L | Planned | Presence, not quality (mutation testing covers quality). |
| Anti test-tampering | Protected-path manifest, test/`#[expect]` ratchet, `cargo-mutants` | L | Planned (mutants gate exists for tauten itself) | Needs owner/CI to own the manifest. |
| Repair-hint diagnostics | `--format json` stable schema; `explain` | tooling | Exercised | — |
| Proof tier | Charon/Aeneas → Lean for kernel crates, generalised from writfold | P | Planned (M3) | Only listed kernel theorems are "proved". |

Dropped as not portable to Rust: regions and arena brands, effect
polymorphism, hidden-authority capture tracking in closures, staged macros,
cleanup effects.
