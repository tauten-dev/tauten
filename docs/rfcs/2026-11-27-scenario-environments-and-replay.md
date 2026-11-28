---
rfc: "2026-11-27-scenario-environments-and-replay"
title: "Scenario environments, failure handling and replay"
status: draft
implementation: not-started
authors:
  - codex
  - fable
created: 2026-11-27
updated: 2026-11-28
issue: "https://github.com/tauten-dev/tauten/issues/104"
milestone: later
supersedes: []
superseded_by: []
blocked_on:
  - "2026-11-27-executable-property-playground"
  - "a retained trial that needs per-case isolation (Pickup trigger)"
---

# RFC: Scenario environments, failure handling and replay

## Problem and scope

A generated executable test is misleading if candidates share dirty state. This
RFC specifies the environments a scenario runs in for
[the executable playground](2026-11-27-executable-property-playground.md):
what the native backend does and does not isolate, and a per-case Docker
backend. Qualification is tracked in
[#104](https://github.com/tauten-dev/tauten/issues/104).

Since the 2026-11-28 review this RFC covers backends only. Outcome
classification, shrink retention and the replay record are decisions, so they
moved to the playground RFC, where the proved kernel decides them. The
filename keeps its original title.

This is a proposal for owner review. It adds no runner, workflow or required
gate to the repository.

## Proof obligations and limits

None: no decision logic changes. A backend produces facts (a case ran, a
deadline was hit, a tool was missing, cleanup failed); the verdict over those
facts is `campaign::seen`, `control::admit` and `binding::fresh`, specified in
the playground RFC. A backend must never return a verdict of its own.

Everything here is tested and unproved, and is edge code that can only be
tested. That matters for whether to build it: #30 dropped "a full process
supervisor" for exactly that reason and dropped "OS sandbox claims" outright.
This is not a security proof, a determinism proof or an assertion that
generated cases cover every state. Report campaign results as tested evidence
only.

## Sequencing and pickup trigger

The 2026-11-27 draft qualified the Docker backend before any trial. The review
reverses that order, for three reasons:

1. No planned trial is known to need it. The queue example, the trial on
   tauten's own CLI ([#112](https://github.com/tauten-dev/tauten/issues/112))
   and the Strop headless trial ([#105](https://github.com/tauten-dev/tauten/issues/105))
   need a fresh directory and a cleared environment, which the native backend
   provides. The contract draft says the same of in-process boundary tests.
2. A cheaper isolated run already exists: the whole campaign inside the gate
   container (`docker compose run --rm gate`), one container per campaign and a
   fresh directory per case. That is how every other tauten test is isolated.
3. A per-case container with a rootful daemon, UID mapping, dropped
   capabilities and cleanup guarantees is weeks of tested-only edge code whose
   upkeep tauten would own for every adopter platform.

**Pickup trigger:** a retained trial records a concrete contamination or host
exposure that a fresh directory, a cleared environment and the gate container
did not prevent (for example state under a fixed absolute path, a listening
port, or a tool that writes outside its working directory). Until then the
contract below stays a draft and the Docker section is not built. The owner
authorized Docker environments in #103; this RFC asks to defer them, not to
drop them.

## Environment lifecycle

For every generated case, shrink attempt and replay:

1. Allocate a new case directory and empty HOME/XDG/TMP state.
2. For Docker, create a fresh container from the resolved image.
3. Materialize only the concrete case fixtures and execute setup hooks.
4. Run all scenario steps in that same environment, observing the actual target.
5. Capture bounded diagnostics and perform explicit cleanup on success or failure.

The setup hook runs again for each candidate. It must use only declared case data
and inputs; no ambient service lookups or unrecorded randomness. Shrinking the
fixture means changing serialized case inputs, not reusing a mutated directory
from the original run. A setup or cleanup failure is reported to the runner as
a `Broken` case; the backend does not decide what that means.

## Backends and boundaries

| Backend | Intended use | Proposed contract |
|---|---|---|
| Native, inside the gate container | Default for tauten's own trials | Fresh directory, cleared environment and separate HOME per case; the container bounds the campaign as a whole |
| Native, on the host | Explicit opt-in for trusted tools and quick trials | Same per-case state; host filesystem and network remain accessible |
| Docker per case | Deferred (see Pickup trigger) | Local daemon, immutable image ID, fresh container and case mount, networking disabled, bounded resources |

Native temporary directories are not isolation from the host. In-process Rust
hooks are trusted code and cannot be interrupted by a subprocess deadline; a
hard campaign wall-clock limit requires an outer worker or CI timeout. Do not
claim such supervision until implemented and tested.

For the deferred Docker backend: build a project-specific image once, then
resolve its tag to an immutable local image ID for the campaign and replay;
the image ID is one of the bindings `binding::fresh` compares. The initial
contract uses a local rootful daemon and runs target processes with the host
user's UID/GID. Rootless user mapping and remote-daemon bind mounts are not
supported and are recorded in #30. Use a read-only image root, drop
capabilities, set no-new-privileges, disable networking, and bound CPU, memory
and process counts. Mount only the case directory; do not pass through the
Docker socket, arbitrary host paths or ambient secrets. Build, pull and fixture
acquisition occur before the campaign, not during individual property checks.

These controls reduce cross-case contamination and host exposure. They do not make
Docker a security boundary for arbitrary hostile software, or normalize time,
entropy, kernels and scheduling. An image ID detects a changed image but does
not preserve its bytes: durable replay also requires retaining the image or a
verified immutable source.

## Execution and observations

Use argv arrays and explicit stdin/environment rather than concatenating generated
strings into a shell command. Shell scripts may be deliberate adapter fixtures,
but quoting and the invoked shell become part of that adapter's contract.

Apply per-command deadlines and captured-output limits. Return status, bounded
stdout/stderr and relevant resulting-file observations. Kill the native process
group on timeout and reap its leader; descendants that deliberately detach from
that group are outside the native guarantee, which is the named process-group
assumption of [#26](https://github.com/tauten-dev/tauten/issues/26). For
Docker, remove the case container and its anonymous volumes. Avoid broad
cleanup commands affecting other containers.

Output limits are not a filesystem quota. The reported prototype polls captured
output files; growth can overshoot a threshold between polls. A stricter disk bound
needs an explicitly qualified quota/tmpfs/streaming design.

Target exit codes are observations for the project's oracle. Distinguish Docker
client/exec errors from an actual target returning exit code 125, 126 or 127;
exercise those cases rather than assuming all such exits are infrastructure
failures.

## Qualification and acceptance

For the native backend, as part of #103's control matrix:

1. Generated cases, shrink attempts and replay cannot observe previous case files.
2. Cleanup is attempted after success, failure and runner errors, and a cleanup
   failure reaches the runner as `Broken`.
3. Deadlines, output limits and process-group cleanup are exercised, and the
   limits above are documented next to the runner.

For the Docker backend, only after the pickup trigger, as #104:

4. Live Docker tests exercise execution, isolation, shrinking, replay and cleanup
   in CI. A mocked Docker command or ignored test is not evidence of backend support.
5. Missing images, client errors and target exits 125-127 produce distinct facts;
   no case container remains after a completed test.
6. A changed image ID makes a saved record stale through `binding::fresh`.
7. Platform, daemon and UID limitations are documented.

Do not make the harness an adopter gate before this qualification and owner
acceptance. Measure cost and stop the Docker backend if its upkeep exceeds the
benefit over the gate container.

## Decision log

- 2026-11-27: Drafted environment/failure/replay contract separately from the core
  experiment so backend qualification and remaining guarantees can be reviewed.
  Eleven native integration tests were reported from a local prototype; live
  Docker tests and tauten's container gate were not run. No adopter integration
  has been completed.
- 2026-11-28: Planning review (Fable), draft unchanged in status. Narrowed to
  backends: outcome classification, shrink retention and the replay record
  moved to the playground RFC, where kernel functions decide them. The
  per-case Docker backend is sequenced after the trials behind a pickup
  trigger, with the gate container as the cheaper isolated run; the owner's
  authorization of Docker environments in #103 stands and the deferral is a
  recommendation for the owner. Rootless and remote-daemon support recorded
  in #30.
