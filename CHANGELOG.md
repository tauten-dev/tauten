# Changelog

## Unreleased

- `cargo tauten generated [--only NAME…]`: the generated-file stage (A11,
  #15). Each `[[workspace.metadata.tauten.generated]]` entry names a
  command and either the committed file its standard output must equal
  (`stdout`) or the files it writes (`outputs`, run in a scratch copy so the
  checkout is never written). Files are compared with the proved
  `same_bytes` and the stage is decided by the proved `control::admit`: one
  byte is `drift`, and an output the command did not produce or a failing
  command is `failed`, never `ok`. tauten declares `rules`
  (`docs/RULES.md`, now checked this way by the CI gate) and `translation`
  (`proofs/Extracted`, needs Docker).

- Which compiler messages become findings, whether a rule applies to a
  target (test targets included) and whether `check` passes are decided by
  the proved kernel (`compiler::kept`, `applies`, `passes`; T5, #32):
  `kept_iff`, `applies_spec`, `check_passes_iff`, `warnings_never_fail`.
  The pinned toolchain's real compiler output on `fixtures/cases` is
  recorded in `crates/cargo-tauten/tests/golden/` and replayed by a test;
  `scripts/record-golden` re-records it for a new toolchain
  (`TAUTEN_RECORD_COMPILER_JSON` keeps the raw messages).

- TAU-T001, TAU-T003 and TAU-K003 are decided by the proved kernel
  (`layout::t001`, `t003`, `k003`; T6, #33). `no_std` is no longer found by
  substring: only `#![no_std]` or `#![cfg_attr(not(test), no_std)]` satisfies
  TAU-K003; `no_std` under any other condition (a feature, nesting) and a
  library root that does not parse are findings. New fixtures
  `rej-conditional-no-std` and `acc-kernel-not-test`.

- Lint exceptions are decided by the proved kernel in more places (T4,
  #31). `exception::decide` takes how the attribute is written and every
  rule it may cover: an exception inside `cfg_attr` is never accepted in a
  kernel crate (`conditional_never_allowed_in_kernel`), and an exception for
  a lint group (`warnings`, `clippy::all`, `unused`, …) is decided as the
  worst over every profile rule the group may contain
  (`group_never_weaker`). TAU-X003 is `exception::loosens` (`x003_iff`) and
  now also sees groups in `[lints]`. The scanner reads exceptions inside
  `cfg_attr` (reported under TAU-X001 when they `allow`) and in `tests/`,
  `benches/`, `examples/` and the build script; in test targets a rule that
  does not apply to tests covers nothing. New reject fixtures:
  `rej-cfg-attr-allow`, `rej-group-allow`, `rej-test-exception`.

## 0.0.4 (2026-11-20)

- `cargo tauten proofs try --theorems T… | --mutants M…`: the developer
  loop. Runs extract, drift, build and audit, then only the mutants of the
  claims stating those theorems (or the named mutants), in parallel; prints
  each affected claim as `PARTIAL`, never writes `proofs/claims.json`, and
  says it is not a gate result. Needs `[gate] try` (the template writes it).
  On tauten itself, one theorem's two mutants take 78 s against about 25
  minutes for the full gate.

- `cargo tauten proofs` gates a snapshot of the committed `HEAD` (`git
  archive` into a private directory) instead of the live working tree, so
  the checkout may be edited while a long gate runs; uncommitted changes in
  the workspace are refused, and `proofs/claims.json` records the `commit`
  it is about. `--live` keeps the old behaviour (any edit during the run is
  drift). Outside Git, the live tree is gated as before.

- `tauten::machine`: a protocol as a pure step function (`Machine`), with
  `run` and `replay` (compare a recorded tape of events and commands). The
  Lean library `Tauten.Machine` (shipped in the proofs template as
  `Proofs/TautenMachine.lean`) proves the induction once: `inv_of_step`,
  `inv_run`, `rinv_of_step` for extracted steps that return a `Result`,
  `sys_inv_of_step` against a world that interprets commands and takes
  adversary steps, and `discharged_of_inv`. Worked instance:
  `tauten::publish` (write, sync, rename, sync the directory; remove the
  temporary file on failure), proved over its translation to leave no
  temporary file behind in any reachable terminal phase, with no
  filesystem assumption.

- Every rule states its `family` (safety, obligation, style), exactly what it
  decides (`invariant`), what it reads (`sees`), what it does not see
  (`dominion`), whether it is `complete`, and its reject `fixtures`
  (`docs/RULES.md`, `cargo tauten explain`). Proved laws: an incomplete rule
  never blocks (only TAU-S002 is incomplete), an input a rule could not read
  is never clean (a source file `syn` cannot parse is now reported under each
  source-scanning rule instead of being skipped), and a complete rule that
  blocks must name a reject fixture or the rule table is invalid. Nine new
  reject fixtures cover the rules that had none.

- `cargo tauten proofs extract [--crate <dir>]` and `cargo tauten check
  --extract` (TAU-K004): translate the proved kernel crates without building
  the proofs and report the first error with its location; code that
  translates only through opaque library functions (axioms) is refused too.
  Without the toolchain the result is tool-missing, never clean. Needs
  `[gate] extract` in `proofs/claims.toml` (the template writes it). The
  agent profile's kernel section is generated from `shapes.toml`, whose
  every entry is a fixture re-checked against the pinned toolchain
  (`scripts/extract-fixtures`): integer casts and derived `PartialEq` do
  translate at this pin; trait objects, floats, slices of borrowing
  structs, early returns across two loops, `match` on `get` in a loop and
  iterator chains do not. The planned syntax lint (old TAU-K004 plan) and
  `#[kernel]` macro are dropped.

- TAU-H001: release builds must check integer overflow
  (`overflow-checks = true` under `[profile.release]` in the workspace
  Cargo.toml, with no per-package or `.cargo/config.toml` override turning it
  off). A value tauten cannot read is never clean. Decided by the proved
  `tauten_check_kernel::manifest::hardening`; `cargo tauten init --write`
  adds the setting.

- Named assumptions are derived, not just declared. Tag a Lean proposition
  `@[tauten_assumption "ID"]` (template file `Proofs/TautenAssumption.lean`);
  the audit derives which assumptions each theorem's hypotheses use, and a
  claim's `assumes` must be exactly that list. A claim with assumptions names
  a `witness` theorem showing they can hold together; optional `necessity`
  theorems must pass like theorems. Decided by the proved
  `tauten_check_kernel::assumption::assumptions_ok`. Existing proofs
  directories: copy the template's `Audit.lean`, `gate.py` and
  `Proofs/TautenAssumption.lean`.

- TAU-R002: a recorded proof claim may not be weakened. `proofs/claims.toml`
  may not drop a theorem or mutant recorded in the committed
  `proofs/claims.json`, nor add an assumption, unless an `[[amend]]` entry
  (`file`, `function`, `kind`, `name`, `reason`) names the change;
  `cargo tauten proofs` refuses to re-record otherwise. Decided by the proved
  `tauten_check_kernel::registry::conserves`.

## 0.0.3 (2026-11-19)

- `cargo tauten proofs`: runs a project's proof gate (`proofs/claims.toml`)
  and prints one JSON outcome per claim (`proved`, `drift`,
  `build-failed`, `audit-failed`, `mutant-survived`, `tool-missing`);
  records them in `proofs/claims.json`.
- `cargo tauten init --proofs <crate>` writes the Charon/Aeneas/Lean proof
  template for a kernel crate.
- In kernel crates, TAU-N001/TAU-P003 findings inside a function with a
  current, total, proved claim are reported as subsumed by the proof
  (JSON field `subsumed_by`, severity `warning`), not as errors.
- Rule-table validation and proof subsumption are decided by
  `tauten-check-kernel` and proved in Lean (`docs/proofs/README.md`).
- **Breaking:** `#[refined]` accepts only an inner type that is
  `tauten::refine::Frozen` (primitives, `String`, and `Box`, `Option`, `Vec`,
  arrays, slices, tuples and `&'static` references of those). A `Cell` could
  be changed through `get()`, and a user-written `Clone` could break the
  predicate, after `try_new` checked it. Every refined type implements
  `tauten::refine::Refined`. Reported by an external review.
- Proof fingerprints include the workspace `Cargo.toml` (without `version`
  lines) and the toolchain pin.

## 0.0.2 (2026-11-18)

- `#[refined]` rejects derives and attributes that could construct a value
  without checking the predicate (`Default`, `Deserialize`, ...). Allowed:
  `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`.
- TAU-A002: `Authority::acquire` is banned outside the edge by resolved path,
  so aliases and function pointers are caught; cap-std's ambient constructors
  are banned under TAU-A001.
- TAU-A001's repair hint no longer names a type that does not exist.
- `#[ensures]` violation messages show `old(..)` as written.
- The ratchet's TAU-R001 names the crate's real tier.
- Rule tables missing a rule the checker emits are rejected.
- Releases use crates.io trusted publishing.

## 0.0.1 (2026-11-18)

First release.
