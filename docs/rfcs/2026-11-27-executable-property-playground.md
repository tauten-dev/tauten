---
rfc: "2026-11-27-executable-property-playground"
title: "Executable property-testing playground"
status: draft
implementation: not-started
authors:
  - codex
  - fable
created: 2026-11-27
updated: 2026-11-28
issue: "https://github.com/tauten-dev/tauten/issues/103"
milestone: "0.5.0"
supersedes: []
superseded_by: []
blocked_on:
  - "owner: kernel-first now, or a recorded exception for the unpublished playground (Unresolved questions, 1)"
---

# RFC: Executable property-testing playground

## Problem and scope

The owner wants to review small executable properties and useful counterexamples
instead of manually reviewing every behavior of agent-written code. A scenario
should prepare an environment, run a real executable through a sequence of
operations, check a stated law, and reduce a failing case into a replayable example.

On 2026-11-27 the owner authorized a reversible project-harness experiment in
Tauten, tracked in [#103](https://github.com/tauten-dev/tauten/issues/103). This
document proposes its design for review. The experiment is not authorization to
publish a production API or impose gates on other projects.

What exists if this is accepted: an unpublished runner in `playground/scenarios`
that is in this repository and runs in the gate, whose pass, fail, shrink and
replay verdicts are decided by the proved kernel; and one in-repository trial on
tauten's own CLI. What does not change: the checker, every existing claim and
badge, and every adopter's gate.

## Proof obligations

The 2026-11-27 draft said that no decision changes. That was wrong, and it is
the main correction of the 2026-11-28 review. A runner that prints "campaign
passed", keeps or discards a shrink candidate, or refuses a replay is making
decisions of exactly the kinds the kernel already proves: whether a set of
checks passed, whether a negative control failed for its named reason, and
whether a record is still about the current inputs. Writing them again as
unproved runner code would give tauten a second, weaker copy of its own
primitives. The labels were honest (everything was called tested); the
placement of the decisions was not what [AGENTS.md](../../AGENTS.md) requires.

**Reused, already proved (no new proof surface):**

| Runner decision | Kernel function | Existing theorems |
|---|---|---|
| The campaign's set of checks passed: every required control was observed once with the result it expects; an empty set never passes | `control::admit(required, observed)` | `admit_ok_iff`, `admit_infrastructure_refused` |
| A planted defect or no-op control counts only when it fails for its named property; a timeout, missing tool, zero cases or another failure satisfies nothing | `control::control_ok(expect, seen)` | `control_ok_spec`, `infrastructure_never_passes` |
| A saved reproducer is still about this target, adapter, environment and case | `binding::fresh(recorded, current)` | `fresh_iff`, `changed_never_fresh`, `missing_key_never_fresh`, `extra_key_never_fresh` |
| What may be said about the result | `badge::badge(established, fresh, Method::Test, _, _)`, which is `Tested` | `badge_spec`, `non_lean_never_proved` |

Each property under test is a `Control`: the correct target with
`Expect::Pass`, each planted defect and the no-op control with `Expect::Fail`.
A changed-target run is not a bypass of `fresh`. It is a new observation
against a new set of bindings, reported as such; the saved record is not marked
reproduced by it.

**New decision logic, kernel first.** Three small total functions, proposed as
`crates/tauten-check-kernel/src/campaign.rs`:

```rust
/// What one executed case showed. `Fail` carries the interned property.
pub enum Case { Pass, Reject, Fail(usize), Timeout, ToolMissing, Broken }
/// What to do with a shrink candidate.
pub enum Shrink { Keep, Discard, Abort }

/// One campaign's cases, folded into what its control saw.
pub fn seen(cases: &[Case], property: usize) -> control::Seen;
/// Whether a shrink candidate replaces the current reproducer.
pub fn keep(property: usize, candidate: Case) -> Shrink;
/// Whether the required set has a negative control at all.
pub fn calibrated(required: &[control::Control]) -> bool;
```

`seen` returns, first match wins: `Timeout` if any case timed out;
`ToolMissing` if any case lacked a tool; `FailOther` if any case is `Broken`
(setup, cleanup, record or runner error) or failed a different property;
`FailNamed` if a case failed `property`; `Empty` if no case passed (none ran,
or all were rejected); otherwise `Pass`. The re-execution of the final shrunk
case is one more `Case` given to `keep`; a reproducer is recorded only on
`Keep`, so a failure that does not reproduce is never certified.

- **Theorems** (`proofs/Tauten/Campaign.lean`):
  - `seen_pass_iff`: `seen cases p = Pass` exactly when some case is `Pass`
    and every case is `Pass` or `Reject`.
  - `seen_named_iff`: `seen cases p = FailNamed` exactly when some case is
    `Fail p` and every case is `Pass`, `Reject` or `Fail p`.
  - `rejects_never_pass`: if every case is `Reject` (including no cases),
    `seen cases p = Empty`.
  - `infrastructure_never_decides`: if any case is `Timeout`, `ToolMissing`
    or `Broken`, `seen cases p` is neither `Pass` nor `FailNamed`.
  - `keep_iff_same_property`: `keep p c = Keep` exactly when `c = Fail p`.
  - `infrastructure_aborts`: `keep p c = Abort` exactly when `c` is
    `Timeout`, `ToolMissing` or `Broken`.
  - `calibrated_iff_some_negative`: `calibrated req` exactly when some
    required control has `Expect::Fail`.
- **Mutants** (`proofs/mutants.py`), each with the theorems it must break:
  - `campaign-reject-counts-as-pass`: `seen_pass_iff`, `rejects_never_pass`.
  - `campaign-zero-cases-pass`: `seen_pass_iff`, `rejects_never_pass`.
  - `campaign-timeout-ignored`: `infrastructure_never_decides`.
  - `campaign-other-property-named`: `seen_named_iff`.
  - `campaign-shrink-keeps-other-property`: `keep_iff_same_property`.
  - `campaign-shrink-keeps-infrastructure`: `keep_iff_same_property`,
    `infrastructure_aborts`.
  - `campaign-no-negative-calibrated`: `calibrated_iff_some_negative`.
- A property whose required set is not `calibrated` is reported
  `uncalibrated`, never as passed. This is the tested-evidence counterpart of
  "a claim with no mutant is never proved".

**Tested and unproved**, to be listed in [proofs/TRUST.md](../../proofs/TRUST.md)
when the runner lands: Proptest's generation and shrink trees; the project's
generators and oracle; turning a process result into a `Case` (deadline hit,
spawn error, output limit, exit status as an observation); serialization of
the record; digests of the target and adapter; every OS interaction. Tested
by the control matrix under [Delivery and acceptance](#delivery-and-acceptance).
If the process-result mapping grows beyond a three-arm match it becomes the
`classify` function of [#26](https://github.com/tauten-dev/tauten/issues/26)
(`success_iff`, `infrastructure_never_success`), kernel first.

What the theorems do and do not give: the runner's bookkeeping cannot turn a
timeout, a missing tool, zero executed cases, a different failure or a stale
record into a pass or a certified reproducer. They say nothing about whether
the oracle states the right law, whether the cases cover the behaviour, or
whether the process facts were read correctly. The badge is `Tested`, and
`non_lean_never_proved` is the theorem that it can never read as `Proved`.

## Fit with the five primitives, and with what #30 already decided

| This proposal | Existing primitive or decision |
|---|---|
| Campaign verdict, planted defects, no-op control | Controls and badge |
| Reproducer identity, refusal on a changed target | Binding |
| Operation sequences and their replay | Machine (`tauten::machine::replay` already replays a tape exactly) |
| Process deadlines and output limits | [#26](https://github.com/tauten-dev/tauten/issues/26): supervision is tested under a named process-group assumption; only `classify` is a kernel matter |
| A per-case container | [#30](https://github.com/tauten-dev/tauten/issues/30) dropped "OS sandbox claims" and "a full process supervisor: edge code that can only be tested". See the [environment RFC](2026-11-27-scenario-environments-and-replay.md) for why the backend is sequenced after the trials |

The boundary with [#17](https://github.com/tauten-dev/tauten/issues/17) is the
badge: the explorer searches a `Machine` against a `World` exhaustively within
bounds and earns `Bounded`; this runner samples sequences against an executable
and earns `Tested`. Both write a counterexample as a replayable tape and both
use planted faults as negative controls. They must share the control and badge
decisions above, not grow two.

Where a project's logic is already a `Machine`, the preferred oracle is the
machine's own `step`: run the generated operation sequence through the proved
step function and through the executable, and compare observations. That tests
exactly the link the proofs do not reach (the driver refines the model), with
no shadow implementation to maintain.

## Reuse and ownership

| Layer | Proposal | Responsibility |
|---|---|---|
| Generation and shrink trees | Existing Rust Proptest library | Generate values/sequences and offer simpler candidates |
| Verdicts | `tauten-check-kernel` (`control`, `binding`, `badge`, proposed `campaign`) | Pass, named failure, shrink retention, replay freshness, badge |
| Tauten playground | Small unpublished Rust runner | Lifecycle, real processes, observations into `Case`, concrete replay record |
| Project adapter | Ordinary project-owned code | Valid generators, setup hooks, actions, independent oracle and named properties |

Proptest supplies the algorithms for shrinking values and sequences. The
runner's traversal asks `campaign::keep` about each candidate; it is not a
replacement property-testing engine. Rust is the initial adapter language,
not a restriction on the language of the executable under test.

No new scenario DSL, expression evaluator or universal property inference system
is proposed. Domain properties and setup remain ordinary project-owned code:
tauten ships no project-specific law.

## Scenario contract

An adapter provides:

1. A serializable case generated by a Proptest strategy. Include initial fixtures,
   operation sequences and any random choices that affect replay.
2. A setup hook that materializes that case in the fresh environment.
3. An executor that invokes the actual binary with bounded argv/stdin operations
   and observes structured output or resulting files.
4. A smaller, independent oracle that returns pass, a named property failure with
   expected/actual observations, or a rejected invalid case.
5. Target and adapter identities, plus explicit environment configuration.
6. For each property, at least one negative control: a planted defect or a
   no-op target that must fail that property.

State persists between operations of one scenario. The entire scenario starts
fresh for every generated case, shrink attempt and replay. Invalid candidates are
rejected, never counted as passes. Prefer generators and shrinkers that preserve
validity by construction: removing an operation must not leave a later operation
referring to a resource that was never created.

## Campaign, shrinking and replay

For each generated case, prepare, execute, observe and check, producing one
`Case`. On the first `Fail p`, traverse the supplied shrink tree within a
budget, keeping a candidate only on `campaign::keep(p, candidate) = Keep` and
stopping on `Abort`. Re-execute the final candidate in another fresh
environment before recording it. Report a smaller reproducer, not a globally
minimal one; expose budget exhaustion and non-reproduction.

Save the concrete case and generated fixtures, the bindings (target, adapter,
environment, case), expected/actual observations and command trace. A seed
helps explain how a case was found but is not a binding and is not sufficient
for replay across library versions. Replay asks `binding::fresh`.
The backends are specified in
[the environment RFC](2026-11-27-scenario-environments-and-replay.md).

## Alternatives

- **Do nothing; each project uses Proptest or Hypothesis directly.** This is
  the right answer for a project-local property and stays available. What it
  does not give is the part tauten already has proved: that an empty run, a
  timeout or an uncalibrated property cannot read as a pass.
- [Hypothesis stateful testing](https://hypothesis.readthedocs.io/en/latest/stateful.html)
  and [fast-check](https://fast-check.dev/docs/advanced/model-based-testing/)
  are the natural hosts for a Python or TypeScript suite. A project that
  chooses one keeps that work in its own tracker; tauten does not maintain
  adapters for other test runtimes ([#30](https://github.com/tauten-dev/tauten/issues/30)).
- [TSTL](https://github.com/agroce/tstl) is an existing scenario-language precedent.
  A DSL stays parked until ordinary adapters show a concrete gap.
- **Unproved verdict code in the runner (the 2026-11-27 draft).** Rejected in
  review: see Proof obligations.

## Delivery and acceptance

The runner is an unpublished package at `playground/scenarios` with its own
lockfile, tier `edge`, depending on `tauten-check-kernel` by path. It is
checked by tauten's own profile and its tests run in `docker compose run --rm
gate`. That one gate container, with a fresh directory and cleared environment
per case, is the first isolated backend; no per-case container is needed for
steps 1 to 3.

A local prototype was reported on 2026-11-27 (see Validation record). It is not
in the repository, so `implementation` is `not-started` and nothing in
[docs/STATUS.md](../STATUS.md) may cite it. #103 closes when the code is on
`main` and gated.

| Order | Issue | What ships | Milestone |
|---|---|---|---|
| 1 | [#103](https://github.com/tauten-dev/tauten/issues/103) | `campaign` kernel module, theorems and mutants through the proofs gate | 0.5.0 |
| 2 | [#103](https://github.com/tauten-dev/tauten/issues/103) | Runner on `main`: queue executable with a planted FIFO defect, an independent list model and a no-op control; verdicts only through the kernel | 0.5.0 |
| 3 | [#112](https://github.com/tauten-dev/tauten/issues/112) | In-repository trial on tauten's own CLI: ship-level reuse agrees with a full run under generated edits | 0.5.0 |
| 4 | [#105](https://github.com/tauten-dev/tauten/issues/105), [#106](https://github.com/tauten-dev/tauten/issues/106) | Two external trials (Strop, Gripsack): adopter evidence, never gates a tauten release | later |
| 5 | [#104](https://github.com/tauten-dev/tauten/issues/104) | Per-case Docker backend, only if a retained trial needs it | later |
| 6 | [#107](https://github.com/tauten-dev/tauten/issues/107) | Writfold trial: the same binding property as step 3, on an adopter | later |

Acceptance for steps 1 and 2:

- The proofs gate passes with the `campaign` claims and their mutants; the
  expected broken set equals the claims that break.
- The correct queue passes; the faulty mode yields a reduced case that
  reproduces; the no-op control fails a check of actual changed state.
- Removing the negative controls reports `uncalibrated`, not a pass.
- Deadline, missing tool, failed setup, failed cleanup, zero executed cases,
  exhausted rejection budget, a candidate failing a different property and a
  non-reproducing failure each produce their own result and none produces a
  pass or a recorded reproducer.
- A replay against a changed target is refused; an explicit changed-target
  run reports both identities and does not mark the record reproduced.

Step 3 justifies generality before any other repository is touched: the runner
must serve a second, real executable with a different shape of state. Each
external trial then records a named planted defect or historical regression, a
reduced reproducer, runtime, adapter size and fixture maintenance. Keep or
reject the shared integration on that evidence. Do not freeze a public API from
one toy example. Project-specific changes are reviewed in their own
repositories.

## Boundaries

Sequential generated testing does not implement #13's storage simulator or #17's
bounded world explorer. Deterministic scheduling, fault injection and exhaustive
state exploration are separate capabilities. A container does not by itself
control clock values, randomness or concurrent scheduling. Property quality
remains a review task: a generated input is useful only when the oracle tests
the intended behavior without repeating the implementation's mistake.

## Validation record

Reported by the drafting agent on 2026-11-27, from a local prototype that is
not in this repository and has not been reviewed: built with Rust 1.98.0;
eleven native integration tests, formatting checks and Clippy with warnings
denied passed; seed 7 reduced the planted FIFO defect from 16 operations to
push(0), push(-1), pop(); the correct mode passed 100 generated cases. The two
live Docker tests and tauten's container gate were not run. No adopter
integration exists. This is a report, not evidence under the "exercised"
definition of docs/STATUS.md.

## Unresolved questions

1. **Owner: kernel first now, or a recorded exception?** Recommended: add
   `campaign` to `tauten-check-kernel` before the runner uses it. It is about
   fifty lines, reuses the fold lemmas of `Control.lean`, and the alternative
   is the wire-first order AGENTS.md forbids. The cost is a new public module
   in a published crate for an experiment; if the experiment ends, the module
   is removed in a later 0.x release. The alternative needs the owner to accept
   an explicit exception: the same functions live in the playground, every
   result line says "verdict logic not yet proved", and they are extracted and
   proved before anything leaves `playground/`.
2. **Owner: is step 3 (tauten's own CLI) accepted as the first real trial,
   ahead of the external ones?**

## Decision log

- 2026-11-27: Owner authorized a bounded prototype and requested RFCs and issues
  for review. Proposed Proptest reuse with a small executable adapter layer;
  environment qualification and three adopter trials recorded separately.
- 2026-11-28: Planning review (Fable), draft unchanged in status. Proof
  obligations rewritten: the runner's verdicts are decided by the existing
  `control`, `binding` and `badge` functions plus a proposed `campaign` module
  with seven theorems and seven mutants; "no decision changes" withdrawn.
  `implementation` corrected from `in-progress` to `not-started` (no code in
  the repository). Outcome classification, shrink retention and the replay
  record moved here from the environment RFC; that RFC now covers backends
  only. Delivery reordered: runner in the gate, then a trial on tauten's own
  CLI (#112), then external trials, then the per-case Docker backend if a
  trial needs it. Hypothesis and fast-check adapters, a campaign settings file
  and a DSL recorded in #30.
