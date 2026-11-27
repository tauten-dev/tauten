---
rfc: "2026-11-27-scenario-environments-and-replay"
title: "Scenario environments, failure handling and replay"
status: draft
implementation: not-started
authors:
  - codex
created: 2026-11-27
updated: 2026-11-27
issue: "https://github.com/tauten-dev/tauten/issues/104"
milestone: later
supersedes: []
superseded_by: []
blocked_on:
  - "2026-11-27-executable-property-playground"
---

# RFC: Scenario environments, failure handling and replay

## Problem and scope

A generated executable test is misleading if candidates share dirty state, a
missing tool is reported as a product defect, or the saved seed cannot reproduce
the failure. This RFC specifies the proposed environment and replay contract for
[the executable playground](2026-11-27-executable-property-playground.md).
Qualification is tracked in [#104](https://github.com/tauten-dev/tauten/issues/104).

A native prototype exists and a Docker backend has been written locally, but live
Docker qualification and the complete contract below remain outstanding. This is
a proposal for owner review. Publishing these RFCs adds no runner, workflow or
required gate to the repository.

## Proof obligations and limits

No proved-kernel function, admission decision, checker or proof record changes.
The sandbox lifecycle, classification, shrinking and replay code remain tested
and unproved. The test matrix below must exercise them. This is not a security
proof, a determinism proof or an assertion that generated cases cover every state.
Any future role in proof/admission decisions needs its own accepted kernel-first
proposal. Report campaign results as tested evidence only.

## Environment lifecycle

Build a project-specific image once, then resolve its tag to an immutable local
image ID for the campaign and replay. There is no universally useful image that
already contains every project's binary, fixtures and runtime. Reuse Tauten's
tooling image where it fits; adapters can supply their own pinned image.

For every generated case, shrink attempt and replay:

1. Allocate a new case directory and empty HOME/XDG/TMP state.
2. For Docker, create a fresh container from the resolved image.
3. Materialize only the concrete case fixtures and execute setup hooks.
4. Run all scenario steps in that same environment, observing the actual target.
5. Capture bounded diagnostics and perform explicit cleanup on success or failure.

The setup hook runs again for each candidate. It must use only declared case data
and inputs; no ambient service lookups or unrecorded randomness. Setup failure is
an infrastructure error. Shrinking the fixture means changing serialized case
inputs, not reusing a mutated directory from the original run.

## Backends and boundaries

| Backend | Intended use | Proposed contract |
|---|---|---|
| Docker | Recommended isolated integration runs | Local daemon, immutable image ID, fresh container and case mount, networking disabled, bounded resources |
| Native | Explicit opt-in for trusted tools and quick trials | Fresh directory, cleared environment and separate HOME; host filesystem/network remain accessible |

The initial Docker contract uses a local rootful daemon and runs target processes
with the host user's UID/GID. Rootless user mapping and remote-daemon bind mounts
need explicit support and qualification before being claimed as supported.
Use a read-only image root, drop capabilities, set no-new-privileges, disable
networking, and bound CPU, memory and process counts. Mount only the case directory;
do not pass through the Docker socket, arbitrary host paths or ambient secrets.
Build/pull and fixture acquisition occur before the isolated campaign, not during
individual property checks. The initial image contract requires a shell and a
simple keepalive utility; validate required tools before generation.

These controls reduce cross-case contamination and host exposure. They do not make
Docker a security boundary for arbitrary hostile software, or normalize time,
entropy, kernels and scheduling. Native temporary directories are not isolation
from the host. In-process Rust hooks are trusted code and cannot be interrupted by
a subprocess deadline; a hard campaign wall-clock limit requires an outer worker
or CI timeout. Do not claim such supervision until implemented and tested.

## Execution and observations

Use argv arrays and explicit stdin/environment rather than concatenating generated
strings into a shell command. Shell scripts may be deliberate adapter fixtures,
but quoting and the invoked shell become part of that adapter's contract.

Apply per-command deadlines and captured-output limits. Return status, bounded
stdout/stderr and relevant resulting-file observations. Kill the native process
group on timeout and reap its leader; descendants that deliberately detach from
that group are outside the native guarantee. For Docker, remove the case container
and its anonymous volumes. Avoid broad cleanup commands affecting other containers.

Output limits are not a filesystem quota. The local implementation polls captured
output files; growth can overshoot a threshold between polls. A stricter disk bound
needs an explicitly qualified quota/tmpfs/streaming design.

Target exit codes are observations. A nonzero exit becomes a named product failure
only when the adapter's property says it is wrong. Distinguish Docker client/exec
errors from an actual target returning exit code 125, 126 or 127; exercise those
cases rather than assuming all such exits are infrastructure failures.

## Outcome classification

| Outcome | Meaning | Shrinking/reporting |
|---|---|---|
| Pass | The generated valid case met its checks | Count toward the successful-case budget |
| Reject | Generator or precondition did not yield a valid case | Do not count as pass; bound rejections |
| Property failure | A named law failed with expected/actual evidence | Shrink only candidates failing that same law |
| Infrastructure error | Setup, tool, deadline, output, runner or cleanup problem | Abort distinctly; never label it a minimized product defect |
| Non-reproduction | Repeated fresh execution did not preserve the failure | Report instability; do not certify a stable reproducer |

Require at least one successful case for a successful campaign, and expose requested
versus executed counts. A generation/rejection budget cannot turn zero executed
checks into success. If cleanup also fails after a product failure, preserve the
original observations but surface the infrastructure failure; do not return a clean
product-only result or continue with possibly contaminated state.

## Shrinking and replay record

Traverse Proptest's existing value tree within an explicit budget. Keep only a
candidate failing the same property ID as the original. A different failure is
not a successful reduction. Reject invalid candidates; abort on infrastructure
errors. Re-run the final candidate fresh and report reduction statistics and
whether the shrink budget was exhausted. No global-minimum claim is made.

Persist a versioned record containing:

- Original and reduced concrete cases, including generated fixture bytes or
  immutable fixture references whose content is checked on replay.
- Stable property ID, expected/actual observations, and the executed command trace.
- Adapter identity and target identity (binary hash or immutable image identity,
  plus all additional frontend/runtime dependencies that affect the test).
- Resolved backend, image ID where relevant, resource and execution settings.
- Seed, generator/library versions and campaign/shrink counts for provenance.

The seed is supplementary. Concrete replay must not regenerate a new case from the
seed and hope that a later library version makes the same choices. Record/schema
validation errors are infrastructure errors. An image ID detects a changed image
but does not preserve its bytes: durable replay also requires retaining the image
or a verified immutable source, plus matching fixtures and target artifacts.

By default replay refuses a changed target/adapter/environment. A deliberate
changed-target mode supports testing a repair and clearly reports the mismatch.
A caller-supplied adapter ID is not an automatic semantic fingerprint: the adapter
must change it whenever its execution or oracle semantics change.

This is diagnostic test output, not a new assurance ledger or signed receipt system.
Future Writfold consumption can use existing gate mechanisms after qualification.

## Qualification and acceptance

Run the following controls in CI and retain the actual results:

1. Correct executable passes; a planted defect and a no-op negative control fail.
2. Failure shrinks to a useful concrete case and reproduces fresh; a repaired
   target passes under an explicit identity-change override.
3. Generated cases, shrink attempts and replay cannot observe previous case files.
4. Missing tools/images, setup failures, target exits 125-127, deadlines, excessive
   output, invalid records and exhausted rejection budgets have distinct outcomes.
5. Cleanup is attempted after success, failure and runner errors; cleanup failure
   is surfaced; no case container remains after a completed Docker test.
6. A changed target/adapter/image is refused by default. Non-reproducing failures
   and a shrink candidate that fails a different property are handled honestly.
7. Live Docker tests exercise execution, isolation, shrinking, replay and cleanup.
   A mocked Docker command or ignored test is not evidence of backend support.
8. Document platform/daemon/UID limitations and test adapter-owned runtime and
   fixture identities rather than assuming a single binary hash captures them.

Do not make the harness an adopter gate before this qualification and owner
acceptance. Issue #104 owns this work; #105, #106 and #107 own bounded Strop,
Gripsack and Writfold trials. Measure cost and simplify or stop the shared design
if its upkeep exceeds the benefit compared with existing project tests.

## Decision log

- 2026-11-27: Drafted environment/failure/replay contract separately from the core
  experiment so backend qualification and remaining guarantees can be reviewed.
  Eleven native integration tests passed locally; live Docker tests and Tauten's
  full container gate were not run. No adopter integration has been completed.
