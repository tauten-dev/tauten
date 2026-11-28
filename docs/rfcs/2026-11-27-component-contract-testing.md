---
rfc: "2026-11-27-component-contract-testing"
title: "Behavioral contracts between components"
status: draft
implementation: not-started
authors:
  - codex
  - fable
created: 2026-11-27
updated: 2026-11-28
issue: "https://github.com/tauten-dev/tauten/issues/109"
milestone: later
supersedes: []
superseded_by: []
blocked_on:
  - "https://github.com/tauten-dev/tauten/issues/23"
  - "https://github.com/tauten-dev/tauten/issues/96"
  - "a second boundary, in a different project, that shows reuse"
---

# RFC: Behavioral contracts between components

## Problem and scope

Two components can type-check and pass their own tests while disagreeing about
identity, freshness, errors, side effects or operation order. The owner requested
an RFC and issues for later triage on 2026-11-27, then asked which ideas from the
Verus discussion could improve Tauten. This is a design proposal, not an accepted
API or an implementation report.

Track the design in [#109](https://github.com/tauten-dev/tauten/issues/109) and
the first boundary trial in [#110](https://github.com/tauten-dev/tauten/issues/110).
It builds on the executable-property playground
([#103](https://github.com/tauten-dev/tauten/issues/103)).

**What the 2026-11-28 review changed.** As first drafted, this RFC shipped
nothing in tauten and decided nothing: contracts were project-local test code,
no API was proposed until a second boundary, and the first trial lived in
another repository. That is a testing method, which is useful, but it is not a
tauten design: it fails "decided by the proved kernel or the compiler" by
having no decision at all. The review keeps the method and the inventory, and
adds the one thing tauten can decide: **whether a consumer may rely on a law**.
That decision already exists in the kernel. Until tested claims have a place in
the record ([#23](https://github.com/tauten-dev/tauten/issues/23)), there is
nothing to wire, so this RFC waits in `later`.

## Proof obligations

No new kernel function is needed for the composition decision; it is two
proved functions applied to new inputs.

| Decision | Kernel function | Existing theorems |
|---|---|---|
| Every law a consumer relies on is established by the provider, at no less than the strength the consumer states | `registry::conserves(relied, provided, &[])`, with `Entry.id` the interned law and `Entry.strength = badge::rank(badge)` | `conserves_iff`, `conserves_trans` |
| A law cited from another crate's shipped record is never stronger than what that record holds | `badge::inherit(inherited, version_matches, recorded)` | `inherited_badge_le`, `inherit_needs_version_and_record` |
| A law's own checks passed: conformance passed, and a planted violation failed for that law | `control::admit`, and the `campaign` functions of the [playground RFC](2026-11-27-executable-property-playground.md) | `admit_ok_iff`, `infrastructure_never_passes` |
| What may be said about a law established by tests | `badge::badge(.., Method::Test, ..)`, which is `Tested` | `non_lean_never_proved` |

- **Kernel:** none new. If wiring shows that the composed badge of a boundary
  needs its own function (the weaker of the provider's and the consumer's
  badge), that is new decision logic: `badge::weakest(a, b) -> Badge` with
  `weakest_le_left`, `weakest_le_right`, `weakest_is_one_of`, and a mutant
  `weakest-returns-stronger` that must break the first two. It is specified
  here so it is not added to the shell instead.
- **Mutants:** the existing `removal-accepted` and `inherit-strengthens`
  mutants must still break their claims.
- **Tested and unproved:** the laws themselves, the observation functions, the
  generators, the fakes, and the mapping from a test run to a `Case`. A
  passing finite campaign is evidence about the exercised boundary and
  configuration. It is not a composition theorem. The kernel decision above
  proves only that the bookkeeping is right: a consumer is never reported as
  covered by a law the provider did not establish, or by a stronger badge than
  the provider earned.

Aeneas/Lean remains the proof route for pure production kernels, and a
compiler-enforced type remains the preferred route wherever one fits. For the
identity and freshness laws that motivated this RFC it does fit: a subject-bound
`Stamped` ([#96](https://github.com/tauten-dev/tauten/issues/96)) makes "a
request bound to a different subject is refused" a property of the carrier, not
something each boundary re-tests. Tests are for what the types and the proofs
cannot reach.

The historical [Strop v0.52 review](https://github.com/stropdev/strop/blob/main/docs/rfcs/2026-11-09-independent-critique-0.52.md)
reported tautological guard proofs, missing production callers and literal-true
premises while distinguishing useful edit-map proofs. Gripsack's
[foundation RFC](https://github.com/gripsack-dev/gripsack/blob/main/docs/rfcs/2026-11-08-verified-foundation.md)
identified the unproved links between decision kernels and protocols. These are
historical findings and design lessons, not claims about present defects.

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

This inventory is the most useful part of the first draft and is true of the
shipped crates whatever happens to the rest. It is handed to
[#97](https://github.com/tauten-dev/tauten/issues/97) so that MAPPING.md states
these limits. An `ensures` clause over `Err` results or an unchanged-state
clause is not proposed here; it is recorded in #30 with its trigger.

Sources: [macro expansion](../../crates/tauten-macros/src/lib.rs),
[contract helpers](../../crates/tauten/src/contract.rs), and
[evidence carriers](../../crates/tauten/src/origin.rs).

## Behaviour

Once #23 gives `claims.toml` a claim kind for tested laws, a provider records a
law like any other claim, with its controls; a consumer names the laws it
relies on. `cargo tauten proofs` reports, per boundary, whether each relied-on
law is established and fresh, and the weaker of the two badges. A law the
provider never established, a stale provider record, or a consumer that states
a stronger badge than the provider earned is a refusal that names the law.
Nothing here is ever reported as `Proved` unless both sides are Lean claims
over extracted code. The exact keys and output lines are deliberately not
fixed in this draft: they belong to #23's schema, and fixing them twice would
produce the second inventory #23 was deferred to avoid.

Until then nothing user-visible changes.

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
reconciliation. A Result type alone does not choose that policy.

## Three complementary checks

| Check | Runs | Evidence sought | As controls |
|---|---|---|---|
| Provider conformance | Real B against independently stated laws | B honors the promises used by its consumers | `Expect::Pass` on the real provider; `Expect::Fail` on a provider with a planted violation of that law |
| Consumer conformance | Real A against controlled contract-permitted outcomes | A meets its obligations and handles allowed variability/errors | `Expect::Pass` on the real consumer; `Expect::Fail` on a consumer that ignores a rejection |
| Composition | Real A connected to real B through production entry points | Wiring, conversion and ordering preserve the intended behavior | `Expect::Pass`; a miswired composition as `Expect::Fail` |

A fake or mock is useful for forcing rare allowed responses in consumer checks.
Run applicable provider laws against maintained fakes as well as real providers.
Fake-only success is not evidence about the real provider. Keep the oracle smaller
than the implementation and do not call the same production decision function to
calculate both expected and actual results.

Consumer response generation samples permitted behavior; it is not a symbolic
proof of all outcomes. Assert useful variation explicitly so a generator that only
returns success cannot accidentally claim error-handling coverage. A law with
no negative control is `uncalibrated`, as in the playground RFC.

## Test structure

Begin with ordinary Rust functions over explicit observations. Conceptually:

~~~rust
// Illustrative test structure, not a new published Tauten API.
let before = fixture.observe();
let outcome = production_entry_point(&request);
let after = fixture.observe();
contract.check(&before, &request, &outcome, &after);
~~~

The project owns request generation, observation and semantic laws; tauten
ships no project-specific law. Most internal Rust boundaries run in-process and
need neither the scenario runner's process handling nor a container.

An observation is a chosen semantic view of real state, not a shadow implementation
of the operation. State explicitly what is omitted. Cross-check the observer
through real reads or a separate observation path where feasible; a reference
model and observer can share the same mistake.

A shared `Contract` or `View` trait, or convenience macros, are not proposed.
They are recorded in #30 with their trigger.

## Negative requests, generation and replay

Distinguish three things:

1. Valid requests expected to succeed.
2. Requests the product promises to reject safely, such as a stale revision. These
   are valid test scenarios and must not be filtered out as generator-invalid.
3. Malformed test setup or impossible generated dependencies. Reject these with a
   bounded rejection budget; they do not count as passed cases.

Reset state for each case, shrink candidate and replay. Preserve operation
prerequisites during shrinking while retaining deliberate stale/wrong-subject cases.
Verdicts, shrink retention and the replay record are those of the playground RFC;
this RFC adds no second set.

## First trial: Strop prepared edits to buffer application

Tracked in [#110](https://github.com/tauten-dev/tauten/issues/110) as adopter
evidence: it never gates a tauten release, and Strop code changes are reviewed
in Strop. Locate the actual preparation caller and application provider before
adapting any tests. The conceptual interface
apply(expected_revision, edits) -> Applied | Stale is explanatory; no exact
Strop API is asserted here.

Candidate laws:

- A valid edit on the current subject/revision produces the intended buffer and
  revision transition; a no-op implementation must fail this law.
- Stale or wrong-subject requests refuse with the documented unchanged state.
- Span units and Unicode boundaries agree across preparation and application.
- Freshness check and mutation occur within the required serialized operation;
  an earlier comparison does not close a check/use race.
- The real caller handles refusal by invalidating/recomputing its prepared edit.

Two of these have a stronger route than a test and the trial should say which
it used: the stale and wrong-subject laws are what a subject-bound `Stamped`
(#96) carries, and the span laws are the `bind_span` and `validate_batch`
proofs of [#24](https://github.com/tauten-dev/tauten/issues/24). The trial is
most useful after #96, as evidence for or against that carrier on a real
boundary.

Exercise current, stale and equal-revision/different-subject cases separately.
Do not claim exhaustive concurrency coverage from one deterministic schedule.

## Tool choices and inspiration

Design input only; none is adopted. The owner settled "no adapters for now" for
other verifiers on 2026-11-19, and #30 lists second-verifier adapters and
link-time no-panic checks as dropped. An evaluation of Kani's verified stubs
would reopen that decision and needs its trigger from #30, not this RFC.

| Tool or idea | What is taken from it |
|---|---|
| Proptest and proptest-state-machine | Reused for laws over operation sequences, through the playground runner |
| Kani contracts (`proof_for_contract`, `stub_verified`) | The idea that a provider's contract is checked before a caller may assume it, and that the caller is then checked against anything the contract permits. That is the composition decision above |
| Creusot/Pearlite | Explicit semantic views of a type as the thing a law is stated over |
| Verus/Prusti | Modular pre/postconditions and caller obligations |
| Pact | For genuine HTTP or message boundaries in a project that has them; not a tauten dependency |

A common-looking annotation does not make contracts interchangeable between engines.
Integer semantics, old-state capture, mutation footprints, panics, quantifiers and
async behavior differ. Do not build an attribute translator that silently treats
runtime checking, sampling and symbolic/formal verification as equivalent.

References:
- [Proptest state machines](https://proptest-rs.github.io/proptest/proptest/state-machine.html)
- [Kani contracts](https://model-checking.github.io/kani/crates/doc/kani/contracts/index.html)
- [Creusot Pearlite and views](https://guide.creusot.rs/pearlite)
- [Verus unverified callers](https://verus-lang.github.io/verus/guide/call-from-unverified-code.html)
- [Pact interaction versus functional checks](https://docs.pact.io/consumer/contract_tests_not_functional_tests)

## Alternatives

- **Keep it a method only (the first draft).** Costs nothing and is where every
  project starts. It leaves tauten with no decision, so it is guidance for the
  agent profile, not an RFC.
- **A contract DSL or a shared `Contract` trait now.** No second boundary
  exists to shape it; parked in #30.
- **A second verifier.** Dropped in #30; see above.
- **Types and proofs first (#96, #24), tests for the rest.** This is the
  recommended order and the reason this RFC is blocked on #96.

## Rollout

1. `later`, adopter evidence: the Strop trial (#110), project-local, after #96.
   It reports which laws needed tests at all.
2. With #23: a tested law is a claim with controls; a consumer names the laws
   it relies on; the composition decision above is wired. This RFC is revised
   with the concrete keys at that point and needs acceptance before any of it
   is built.
3. Only after a second boundary in a different project: reconsider shared
   helpers.

## Unresolved questions

1. **Owner:** is the composition decision (step 2) wanted at all, or should
   boundary testing stay guidance only? If guidance only, this RFC should be
   rejected and the method recorded in the agent profile.

## Decision log

- 2026-11-27: Owner requested an RFC/issues for later triage and a review of ideas
  from the Verus discussion. Drafted the three-check contract approach, Strop pilot,
  reuse of existing Tauten primitives, and optional tool inspirations. No component
  contract tests or new tool integration were implemented or executed for this RFC.
- 2026-11-28: Planning review (Fable), draft unchanged in status. Found that the
  draft made no tauten decision. Added the composition decision on existing
  proved functions (`registry::conserves` over law strengths, `badge::inherit`)
  and specified `badge::weakest` in case it is needed; expressed the three
  checks as controls; blocked the RFC on #23, #96 and a second boundary. Handed
  the inventory of contract-macro limits to #97. Recorded in #30: a shared
  `Contract`/`View` trait and macros, `ensures` over `Err` and unchanged-state
  clauses, and a Kani evaluation. Dropped the optional Kani evaluation from
  this RFC because the owner's 2026-11-19 decision and #30 already cover it.
