---
rfc: "2026-11-27-component-contract-testing"
title: "Behavioral contracts between components"
status: draft
implementation: not-started
authors:
  - codex
created: 2026-11-27
updated: 2026-11-27
issue: "https://github.com/tauten-dev/tauten/issues/109"
milestone: later
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Behavioral contracts between components

## Problem and scope

Two components can type-check and pass their own tests while disagreeing about
identity, freshness, errors, side effects or operation order. The owner requested
an RFC and issues for later triage on 2026-11-27, then asked which ideas from the
Verus discussion could improve Tauten. This is a design proposal, not an accepted
API or an implementation report.

Track shared experimentation in [#109](https://github.com/tauten-dev/tauten/issues/109)
and the first Strop boundary trial in [#110](https://github.com/tauten-dev/tauten/issues/110).
This complements the executable-property playground [#103](https://github.com/tauten-dev/tauten/issues/103)
and draft designs in [#108](https://github.com/tauten-dev/tauten/pull/108).

## Proof obligations and limits

No kernel function, checker decision, proof admission, claim format or badge changes.
The proposed contract suites, observations, fixtures and replay adapters are tested
and unproved. Passing finite cases is evidence about the exercised boundary and
configuration, not a composition theorem or universal interface compatibility.

Aeneas/Lean remains the proof route for pure production kernels. A theorem must
name its real production subject and assumptions. External inputs, callers,
observation adapters, scheduling and effects are not discharged by a theorem about
a comparison helper. A future change to an admission decision must first specify
its kernel function and Lean obligations through the normal RFC process.

The historical [Strop v0.52 review](https://github.com/stropdev/strop/blob/main/docs/rfcs/2026-11-09-independent-critique-0.52.md)
reported tautological guard proofs, missing production callers and literal-true
premises while distinguishing useful edit-map proofs. Gripsack's
[foundation RFC](https://github.com/gripsack-dev/gripsack/blob/main/docs/rfcs/2026-11-08-verified-foundation.md)
identified the unproved links between decision kernels and protocols. These are
historical findings and design lessons, not claims about present defects. A change
of prover alone does not establish that the contract is useful or connected to
production behavior.

## Existing Tauten abstractions

As inspected at main commit bac9fdd1f4f2390f8c7fe2d95a6bc878e6149f42:

- Tauten already exports requires/ensures, refined/newtype wrappers, evidence and
  Permit/Stamped carriers. Reuse these rather than introduce duplicate syntax.
- The contract macros require Result-returning synchronous functions. Preconditions
  run before the body; ensures checks only Ok results. False generated checks return
  ContractViolation in every build profile. Arbitrary predicate code is not thereby
  proved pure, total or non-panicking.
- old(expr) already exists, restricted to Copy snapshots. This is not a general
  observation of mutable collections, files or external state.
- A postcondition failure is detected after execution. Returning ContractViolation
  does not roll back side effects. Err results likewise have no general unchanged-
  state guarantee from the existing ensures expansion.
- Effects/capabilities constrain classes of authority; a permission to write does
  not establish which particular fields or files remained unchanged.
- Subject identity matters in addition to revision: [#96](https://github.com/tauten-dev/tauten/issues/96)
  tracks the current stamp-binding gap. This proposal does not silently repair it.

Sources: [macro expansion](../../crates/tauten-macros/src/lib.rs),
[contract helpers](../../crates/tauten/src/contract.rs), and
[evidence carriers](../../crates/tauten/src/origin.rs).

## Contract contents

For caller A using provider B, review these obligations together:

| Part | Meaning |
|---|---|
| Caller obligations | Requests meet the provider's specified domain and ordering requirements |
| Provider guarantees | Each permitted request produces allowed results and observable effects |
| Caller assumptions | The caller handles the provider's permitted outcomes, not only its current usual response |
| Frame conditions | Explicitly named state outside the operation's allowed footprint stays unchanged |
| Scope and assumptions | Identity/incarnation, revision, units, serialization, time and external-system assumptions are explicit |

Start from what consumers actually need and what the provider intends to promise.
Avoid encoding incidental implementation details. Review errors separately: an
operation may promise no change, partial change, or an unknown outcome requiring
reconciliation. A Result type alone does not choose that policy. Cancellation,
retry and durability are contract dimensions only when the selected boundary needs
them; do not infer them from successful return values.

## Three complementary checks

| Check | Runs | Evidence sought |
|---|---|---|
| Provider conformance | Real B against independently stated laws | B honors the promises used by its consumers |
| Consumer conformance | Real A against controlled contract-permitted outcomes | A meets its obligations and handles allowed variability/errors |
| Composition | Real A connected to real B through production entry points | Wiring, conversion and ordering preserve the intended behavior |

A fake or mock is useful for forcing rare allowed responses in consumer checks.
Run applicable provider laws against maintained fakes as well as real providers.
Fake-only success is not evidence about the real provider. Keep the oracle smaller
than the implementation and do not call the same production decision function to
calculate both expected and actual results.

Consumer response generation samples permitted behavior; it is not a symbolic
proof of all outcomes. Assert useful variation explicitly so a generator that only
returns success cannot accidentally claim error-handling coverage.

## Proposed test abstraction

Begin with ordinary Rust functions over explicit observations. Conceptually:

~~~rust
// Illustrative test structure, not a new published Tauten API.
let before = fixture.observe();
let outcome = production_entry_point(&request);
let after = fixture.observe();
contract.check(&before, &request, &outcome, &after);
~~~

The project owns request generation, observation and semantic laws. A small shared
helper may coordinate fixture reset, named failures, case serialization and replay.
Evaluate Proptest and proptest-state-machine before writing custom generation or
shrinking machinery. Most internal Rust boundaries can run in-process. Use the
qualified environment work in #104 only when the test requires external effects;
pure component trials need not wait for live Docker qualification.

An observation is a chosen semantic view of real state, not a shadow implementation
of the operation. A buffer view might contain subject identity, revision and text;
an ownership view might contain exact owned/unowned bytes. State explicitly what
is omitted. Cross-check the observer through real reads or a separate observation
path where feasible; a reference model and observer can share the same mistake.

Keep success effects, failure effects and frame conditions in the same transition
law where that makes review easier. Prefer project-local functions initially.
Convenience macros or a general Contract/View trait require evidence from two
boundaries and a separate decision about their semantics and costs.

## Negative requests, generation and replay

Distinguish three things:

1. Valid requests expected to succeed.
2. Requests the product promises to reject safely, such as a stale revision. These
   are valid test scenarios and must not be filtered out as generator-invalid.
3. Malformed test setup or impossible generated dependencies. Reject these with a
   bounded rejection budget; they do not count as passed cases.

Reset state for each case, shrink candidate and replay. Preserve operation
prerequisites during shrinking while retaining deliberate stale/wrong-subject cases.
Retain reductions failing the same law. Setup/tool/observation/cleanup errors and
zero exercised cases cannot produce a passing campaign. Record the concrete inputs,
operation sequence, observations, law identity and target/adapter identity using
#103's replay direction rather than creating a second ledger.

## First trial: Strop prepared edits to buffer application

Locate the actual preparation caller and application provider before adapting any
tests. The conceptual interface apply(expected_revision, edits) -> Applied | Stale
is explanatory; no exact Strop API is asserted here. Confirm the intended behavior
and add subject/incarnation binding where the real interface needs it.

Candidate laws for #110:

- A valid edit on the current subject/revision produces the intended buffer and
  revision transition; a no-op implementation must fail this law.
- Stale or wrong-subject requests refuse with the documented unchanged state.
- Span units and Unicode boundaries agree across preparation and application.
- Freshness check and mutation occur within the required serialized operation;
  an earlier comparison does not close a check/use race.
- The real caller handles refusal by invalidating/recomputing its prepared edit.

Exercise current, stale and equal-revision/different-subject cases separately.
Use a deterministic intervening edit between prepare and apply where supported.
Do not claim exhaustive concurrency coverage from that one schedule. If concurrency
inside the operation is part of the contract, it needs an explicit scheduling or
proof approach rather than sleep-based timing.

This differs from #105's headless edit/undo trial: that checks an executable product
flow; #110 checks caller/provider responsibilities at the internal boundary. Reuse
fixtures without treating one as proof of the other's scope. Strop code changes
belong in a separately reviewed Strop change.

## Tool choices and inspiration

The matching [HN discussion](https://news.ycombinator.com/item?id=43745987) is from
22 April 2025. Current primary documentation, checked 27 November 2026, informs
these options; historical comments are not the tool specification.

| Tool or idea | Relevance and proposed disposition |
|---|---|
| Proptest/state-machine | Reuse for executable laws and operation sequences; already fits the playground direction |
| Kani contracts | Strong inspiration for checking a provider contract before using that contract as a caller abstraction; optional isolated evaluation, not adoption in this RFC |
| Creusot/Pearlite | Learn from explicit semantic views of types and pure specifications; retain the existing Lean proof route |
| Verus/Prusti | Learn from modular pre/postconditions and caller obligations; no second required verification stack |
| contracts crate | Study trait-level ergonomics and invariant declarations; existing Tauten error-returning contracts make a second runtime-contract dependency unnecessary for this trial |
| no-panic | Link-time and build-configuration-dependent; does not establish behavioral contracts. Keep the existing #30 disposition |
| Pact | Existing option for genuine HTTP/message boundaries; does not replace provider side-effect tests and is not needed for the first internal Rust trial |

Kani exposes requires/ensures/modifies, proof_for_contract and stub_verified.
Its abstraction can return any value satisfying the contract, helping detect a
caller that assumes more than the provider promises. Its function-contract API
is experimental. Any later evaluation must pin the toolchain, exercise the actual
production function, check provider and caller harness selection, preserve reachability
and unwinding checks, and record assumptions, input restrictions and unsupported
features. The documentation also warns about panicking modifies expressions; such
limitations need qualification before reliance. A harness result is not automatically
a Tauten Lean proof or a proof of the full application.

A common-looking annotation does not make contracts interchangeable between engines.
Integer semantics, old-state capture, mutation footprints, panics, quantifiers and
async behavior differ. Do not build an attribute translator that silently treats
runtime checking, sampling and symbolic/formal verification as equivalent.

References:
- [Proptest state machines](https://proptest-rs.github.io/proptest/proptest/state-machine.html)
- [Kani contracts](https://model-checking.github.io/kani/crates/doc/kani/contracts/index.html)
- [Kani loop unwinding](https://model-checking.github.io/kani/tutorial-loop-unwinding.html)
- [Creusot Pearlite and views](https://guide.creusot.rs/pearlite)
- [Verus unverified callers](https://verus-lang.github.io/verus/guide/call-from-unverified-code.html)
- [Prusti](https://github.com/prusti/prusti)
- [contracts crate](https://docs.rs/contracts/latest/contracts/)
- [no-panic caveats](https://github.com/dtolnay/no-panic#caveats)
- [Pact interaction versus functional checks](https://docs.pact.io/consumer/contract_tests_not_functional_tests)

## Acceptance and stopping rule

Review a small set of concrete laws and their real production subjects. Demonstrate
provider, consumer and composition checks with a useful reduced replayable historical
regression or targeted failing control. Record runtime, adapter size, maintenance
cost and the exact observations/schedules covered. Do not start a general mutation
programme as part of this work; that topic remains separate.

Keep the first tests project-local if shared machinery provides no benefit. Generalize
only after a second boundary demonstrates reuse. No mandatory adopter gate or public
API is approved here, and no optional verifier is added to Tauten's tooling image.

## Decision log

- 2026-11-27: Owner requested an RFC/issues for later triage and a review of ideas
  from the Verus discussion. Drafted the three-check contract approach, Strop pilot,
  reuse of existing Tauten primitives, and optional tool inspirations. No component
  contract tests or new tool integration were implemented or executed for this RFC.
