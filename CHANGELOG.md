# Changelog

## Unreleased

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
