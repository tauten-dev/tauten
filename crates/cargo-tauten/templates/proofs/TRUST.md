# What the proofs trust

One page. A claim recorded as `proved` in `proofs/claims.json` means: Lean
theorems, checked by Lean's kernel with only Lean's standard axioms, about
the Charon/Aeneas translation of the claimed Rust function, with each
listed mutant of the Rust source breaking at least one of them.
Everything below is trusted, not proved.

## Toolchain (pinned; `ghcr.io/tauten-dev/proofs-toolchain:lean-4.35.0-aeneas-c5eb4a2-charon-8cae8fd`)

| Component | Pin | Trusted for |
|---|---|---|
| rustc / Charon | Charon `8cae8fd` on `nightly-2026-09-17` | Rust → LLBC: that the MIR it reads is what your compiler builds from the same source, and that LLBC keeps its meaning |
| Aeneas and its Lean library | `nightly-2026.11.07-c5eb4a2` | LLBC → Lean: that the translation (`fail` for panics and overflow, `div` for divergence) models the Rust behaviour; that its models of `core`/`alloc` are right |
| Lean 4 kernel | v4.35.0 | checking the proofs |
| mathlib | `lake-manifest.json` | lemmas used by the proofs |
| Axioms | `propext`, `Classical.choice`, `Quot.sound` | nothing else; `Audit.lean` reads every theorem's transitive axiom closure |

## Shell steps (scripts, not proved)

- `extract.sh`: copies the crate and its path dependencies into a shadow
  workspace and runs Charon and Aeneas with `-abort-on-error`.
- The drift check in `gate.py`: the theorems are about the current source
  only because the committed `Extracted/` equals a fresh translation.
- Mutants in `gate.py`: applied textually, re-translated and rebuilt; a
  build error is attributed to a theorem by file position, plus theorems
  that depend on a failed one (`sorryAx`). A miscounted mutant can only
  report a survivor or a kill; it cannot prove anything.
- `Audit.lean`'s total-shape check (`∀ args, f args ⦃ _ ⦄`, no hypotheses),
  which decides whether a claim may subsume lint findings.
- `Audit.lean`'s assumption derivation: the IDs tagged
  `@[tauten_assumption]` that a theorem's hypotheses mention, unfolding
  definitions of the `Proofs` library only, and the IDs a witness's
  conclusion establishes (a tagged proposition, a conjunction or an
  existential of them). A premise written without a tagged definition is
  not seen; reviewers read the statements. The decision on the derived
  lists is the proved `tauten_check_kernel::assumption::assumptions_ok`.
- `gate.py --extract-only` (`cargo tauten proofs extract`): the first
  located error is read from Charon's and Aeneas's output by pattern; an
  unrecognised message is still a failure, only less precisely located.
  `axiom` declarations in the fresh translation are reported as opaque
  library functions.
- `run.sh`, `compose.yaml`, `container.sh`: run the gate in the image on a
  copy of the checkout.
- `cargo tauten proofs`: reads the gate's `TAUTEN-PROOFS` lines, decides
  one outcome per claim and records it with a fingerprint (change
  detection, not tamper protection) of the crate and `proofs/`. Commit
  `claims.json` and review changes to it like a baseline.
- `cargo tauten proofs draft`: joins the translation's doc comments
  (Rust item, source file), the audit's `SUBJECT`/`TOTAL`/`PREMISES` lines
  and the `[[mutant]]` sites into a suggested patch. Tested, not proved,
  and never evidence: the applied entries are judged by the gate like
  hand-written ones.
- `cargo tauten check`: finds the function around a finding by line span
  (`syn`) and compares fingerprints before the proved tauten decision
  subsumes a TAU-N001/TAU-P003 finding.

## Not covered at all

Anything of the crate outside the claimed functions, and anything the
theorems do not state: a total theorem shows that the function never
panics, overflows or indexes out of bounds, not that its result is right
unless its postcondition says so. A proof of code against its own
specification does not show the specification is the right one.
