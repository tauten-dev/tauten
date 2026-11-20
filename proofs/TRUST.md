# What the proofs trust

One page. A "Proved" badge (docs/PROOF-PLAN.md, "The Proved badge rule")
means: a Lean theorem, checked by Lean's kernel with only Lean's standard
axioms, about the Charon/Aeneas translation of the Rust function that
production calls. Everything below is trusted, not proved.

## Toolchain (pinned; `ghcr.io/tauten-dev/proofs-toolchain:lean-4.35.0-aeneas-c5eb4a2-charon-8cae8fd`)

| Component | Pin | Trusted for |
|---|---|---|
| rustc / Charon | Charon `8cae8fd` on `nightly-2026-09-17` | Rust → LLBC: that the MIR it reads is what the shipped compiler (stable 1.98) builds from the same source, and that LLBC keeps its meaning |
| Aeneas and its Lean library | `nightly-2026.11.07-c5eb4a2` | LLBC → Lean: that the pure functional translation (and the `Result` monad: `fail` for panics and overflow, `div` for divergence) models the Rust behaviour; that its models of `core`/`alloc` (`checked_*`, `saturating_*`, `Slice.get`, `Vec.push`, ...) are right |
| Lean 4 kernel | v4.35.0 | checking the proofs |
| mathlib | `lake-manifest.json` | lemmas used by the proofs |
| Axioms | `propext`, `Classical.choice`, `Quot.sound` | nothing else; `Audit.lean` reads every theorem's transitive axiom closure (`sorryAx`, an Aeneas `axiom` for an opaque function or a native-evaluation axiom fails the build) |

## Shell steps (scripts, not proved)

- `proofs/extract.sh`: copies the crate and its path dependencies into a
  shadow workspace at the same relative paths, with the root's resolver,
  inherited `[workspace.package]` keys and `Cargo.lock`; then Charon and
  Aeneas with `-abort-on-error`. Trusted to translate the committed source.
- The drift check (`diff -ru` in `gate.sh`, file comparison in the
  template's `gate.py`): the theorems are about the current source only
  because the committed `proofs/Extracted/` equals a fresh translation.
- `proofs/mutants.py` / the template's `gate.py`: apply each mutation
  textually, re-translate, rebuild, and attribute build errors to theorems
  by file position plus the `sorryAx` reporter. A miscounted mutant can
  only make the gate stricter or report a survivor; it cannot prove
  anything.
- `proofs/container.sh` and the Compose service: copy the read-only
  checkout into the image's `/work`.
- `cargo tauten proofs`: runs the gate command, reads its `TAUTEN-PROOFS`
  result lines, decides one outcome per claim
  (`tauten_check::proofs::outcome`, unit-tested) and records them, with a
  fingerprint (FNV-1a 64 over the claim's crate directory, the workspace
  crates it depends on and `proofs/`,
  change detection only) of the sources it saw, in `proofs/claims.json`.
  That file is reviewed and committed like a baseline; editing it by hand
  is as visible as editing a baseline.
- `cargo tauten check`, proof subsumption: finds the function enclosing a
  finding with `syn` (innermost `fn` by line span), compares fingerprints,
  and calls the proved `subsume::subsumes`. A finding is subsumed only for
  a claim whose recorded outcome is `proved`, whose `total` theorem the
  audit confirmed has the shape `∀ args, f args ⦃ _ ⦄` with no hypotheses,
  which lists the rule under `subsumes`, and whose fingerprint matches.
- TAU-R002, claim conservation: `cargo tauten check` and `cargo tauten
  proofs` compare `proofs/claims.toml` with the committed
  `proofs/claims.json`, intern each (file, function, kind, name) entry and
  call the proved `registry::conserves`: theorems and mutants may not
  shrink, assumptions may not grow, except where an `[[amend]]` entry with a
  reason names the change. Conservation is relative to the committed
  record: whoever reviews a commit that rewrites `claims.json` (or deletes
  it) is trusted to see a weakening in its diff, as for a baseline.
- Named assumptions (A8): `Audit.lean` (tauten's own, and the template's)
  derives each theorem's tagged premises and a witness's established
  assumptions (audit code, not proved); `cargo tauten proofs` decides with
  the proved `assumption::assumptions_ok`. tauten's publish claims (A7)
  rest on the filesystem assumptions in `proofs/assumptions.toml`
  (`FS.rename-atomic`, `FS.sync-file`, `FS.sync-dir`,
  `FS.exclusive-create`): that a real filesystem behaves like a `Profile`
  with those flags set is exactly what is assumed, and the model of
  `proofs/Tauten/Durable.lean` (two names, durable and pending entries,
  crashes keeping either) is trusted to describe one.

## What each scanner does not see (A2)

Every rule in `crates/tauten-check/rules.toml` states what it reads
(`sees`) and what it does not see (`dominion`); `docs/RULES.md` lists them
per rule. In short:

| Reads | Rules | Does not see |
|---|---|---|
| `resolved` (rustc and clippy) | the lint rules (P, N, E, M, S001, X001, U, C002, O001, L001, K002, D001) and the ban-list rules (A000, A001, A002, G001, C001, K001, P005) | code expanded from macros defined in other crates (clippy skips it); `cfg`-disabled code; build scripts; for ban lists, APIs not on the list and dependency functions that wrap a banned one |
| `source` (`syn`) | X001 (inner and `cfg_attr` allows), X002, X004, K003, F001, S002 | attributes and items produced by macros. Exceptions are read in `src/`, `tests/`, `benches/`, `examples/` and the build script, including inside `cfg_attr`; a lint group counts as every profile rule it may contain. F001, A002 and S002 read `src/` only. A file `syn` cannot parse is reported under each rule that reads it, never treated as clean |
| `manifest` | T001, T002, T003, X003, H001 | dev- and build-dependencies (T002, T003); settings from the environment or Cargo configs outside the workspace (H001) |
| `record` | R001, R002 | the committed record itself; its diff is reviewed |
| `translation` | K004 | kernel crates not listed in `[gate] crates` |

The two laws over these labels are proved (`tauten_check_kernel::rule`): a
rule marked incomplete (today only S002, a text heuristic) never blocks, and
an input a rule could not read is never clean. The labels themselves are
reviewed text, not derived.

## Which decisions are proved

The decision every rule ends in is a proved kernel function; the facts it
is given are not. Proved (`proofs/claims.toml`): tier order and dependency
direction (T002), severity and finding severity, lint exceptions with
groups and conditions (X002, X004, `exception::decide`), manifest lint
tables (X003, `exception::loosens`), declared tier (T001), kernel
dependency allowlist (T003), `no_std` (K003), overflow checks (H001),
which compiler messages are findings, rule applicability to test targets
and the exit verdict (`compiler::kept`, `applies`, `passes`), the baseline
ratchet (R001), claim conservation (R002), proof subsumption, outcomes,
badges and admission of a set of checks. Compiler-enforced: the
capability, evidence, permit and refinement types (compile-fail tests in
`crates/tauten/tests/ui`). Tested, not proved: every scanner that turns
source or manifests into those facts, and the rendering.

## What tauten's own proofs additionally trust

- Interning in the shell (`baseline::Interned`, `profile::validate`):
  equal strings get equal indices. Covered by differential tests against
  the pre-kernel implementations.
- The one-to-one conversions between `tauten_check` and kernel enums.
- For the macros (P3): that the listed instances represent the generator
  ("proved for listed instances", never "macros verified"); the privacy of
  the generated module (rustc); the completeness of the derive/attribute
  allowlist (compile-fail tests); that predicates and bodies outside the
  instances do not panic or diverge.

## Not covered at all

Scanning (`syn` visitors), the conversion of cargo's JSON messages
(checked against the pinned toolchain's recorded output in
`crates/cargo-tauten/tests/golden/`), rendering, workspace loading, async
contracts, and A002 (authority) are tested, not proved. The scanners'
tests are themselves calibrated by mutation: the CI gate runs
`cargo-mutants` over `scan.rs`, `effects.rs` and `manifest.rs` of
`tauten-check` and requires every viable mutant to be killed by a test
(147 mutants, 125 caught, 22 unviable, 0 surviving on 2026-11-20; one
equivalent mutant is excluded with its reason in `.cargo/mutants.toml`).
A killed mutant shows a test notices the change, not that the scanner
reads every Rust construct: the per-rule `dominion` text above is the
statement of what each one does not see. No theorem is about a threat model: a proof of a
decision against its own specification does not show the specification is
the right one.
