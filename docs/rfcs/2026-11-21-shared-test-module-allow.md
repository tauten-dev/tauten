---
rfc: "2026-11-21-shared-test-module-allow"
title: "Accept a reasoned file-level allow(dead_code) in a shared test module"
status: accepted
implementation: complete
authors:
  - astra
created: 2026-11-21
updated: 2026-11-21
issue: https://github.com/tauten-dev/tauten/issues/59
milestone: "0.0.6"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Accept a reasoned file-level allow(dead_code) in a shared test module

## Summary

TAU-X001 stops reporting one exact form: an inner
`#![allow(dead_code, reason = "...")]` in a file under `tests/` or `benches/`
that is not the root of any Cargo target. Every other file-level or
`cfg_attr` `allow` stays a TAU-X001 error, as does an unreasoned one, any
other lint, and anything in `src/`, `examples/` or the build script. The
rule's invariant and "Does not see" text say so; the carve-out is decided by
tauten's own scan of the file and Cargo's target list, not by the compiler or
the proved kernel, and is labelled that way.

## Motivation

A module compiled into several targets cannot carry an inner `#![expect]`:
it is unfulfilled in every target where the lint does not fire. Cargo's own
documented layout for shared integration-test helpers (`tests/common/mod.rs`
declared by several `tests/*.rs`) hits exactly this with `dead_code`. In
writfold (#59), `#![expect(dead_code)]` on one shared module goes unfulfilled
in 38 of its targets, and narrowing to items would take 146 attributes across
9 modules. The `mod`-declaration form (a reasoned `#[expect]` on `mod
support;` in each target where something is unused) works and is now the
advised repair, but it costs one attribute per declaring target that must
flip as usage changes; over more than a hundred targets that is real churn
for a lint with little signal in test helpers.

## Behaviour

- Not reported: `#![allow(dead_code, reason = "shared by several test targets")]`
  at the head of `tests/support/mod.rs` (or any non-root file under `tests/`
  or `benches/`).
- Still TAU-X001: the same attribute without a reason; any other lint, or a
  group containing `dead_code` (`unused`); `cfg_attr(.., allow(dead_code))`;
  the attribute in a test target root (`tests/one.rs`, `tests/it/main.rs`,
  any `[[test]]`/`[[bench]]` `path`), where an outer `expect` works; any file
  under `src/`, `examples/` or the build script.
- CHANGELOG: "TAU-X001 accepts a reasoned file-level `allow(dead_code)` in a
  non-root module under `tests/` or `benches/` (#59)."

## Proof obligations

None: no kernel decision changes. The proved `exception::decide` is not
involved (`dead_code` is not a profile lint). The file-level `allow` report
was, and remains, a check tauten makes by scanning the file. Tested and
unproved: the scanner's reason flag (unit tests, including a reason
containing commas, which X001 must not split), target-root detection from
`cargo metadata` (fixtures with a default root, a `tests/x/main.rs` root and
a shared module), and the report (fixture suite).

## Design

- `scan::lint_exceptions` records whether each attribute carries a `reason`.
- `SourceFile` gains `is_target_root`, true for the source path of every
  Cargo target of the package (lib, bin, proc-macro, test, bench, example,
  build script), from `cargo metadata`. Roots come from Cargo, not from path
  conventions, so custom `path =` targets are handled (cf. #55).
- In `exceptions`, the file-level report is skipped when all hold: inner
  attribute, `allow`, not conditional, lint is exactly `dead_code`, reason
  present, `kind == Test`, not a target root.

## Compatibility and reversibility

Only removes findings; a project baselined on the old findings sees its
baseline entries go unused (the ratchet already reports those). Reverting
re-adds the findings. `SourceFile` is a public struct of `tauten-check`
(0.0.x), so the new field is a minor break for direct library users.

## Alternatives

- Do nothing beyond the corrected advice: the per-target `mod` form works but
  scales badly; the owner chose the exemption.
- Accept only when two or more targets declare the module: closer to the
  reason for the carve-out, but needs resolving `mod` declarations across
  targets (path attributes, nested modules) for little gain.
- Accept any lint: an `allow` never goes stale, so the carve-out is kept to
  the one lint whose per-target firing is the problem.

## Evidence and tests

Fixtures: the accept fixture `acc-shared-test-module` gains a third target
using the file-level form; `rej-shared-test-allow` changes to the forms still
rejected (no reason; `unused`; in a target root). Unit tests for the scanner
flag and the condition. Quality gate.

## Rollout

0.0.6, after the corrected advice (same milestone). writfold can then drop
its nine baseline-blocking findings without changing its tests.

## Unresolved questions

None.

## Decision log

- 2026-11-21: proposed after triage of #59; the triage recommended corrected
  advice first and the exemption only through an RFC.
- 2026-11-21: accepted by the owner ("you have my blessing … file the rfcs
  and mark them as accepted"), with the narrow form above.
- 2026-11-21: implemented. `scan::LintException::reasoned`;
  `SourceFile::is_target_root` from every Cargo target's `src_path`; the
  exemption condition in `structure::exceptions`. Unit tests relax each
  condition alone (no reason, `unused`, `cfg_attr`, target root, `src/`,
  `examples/`) and check a reason containing commas; fixtures
  `acc-shared-test-module` (both the `mod`-declaration and the file-level
  form, clippy-clean) and `rej-shared-test-allow` (unreasoned module allow,
  reasoned allow in a target root). Not yet exercised: writfold's nine
  findings on the released binary.
