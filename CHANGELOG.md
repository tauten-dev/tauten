# Changelog

## Unreleased

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
