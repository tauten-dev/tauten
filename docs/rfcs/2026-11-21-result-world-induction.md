---
rfc: "2026-11-21-result-world-induction"
title: "Prove a world invariant directly over successful extracted steps"
status: accepted
implementation: not-started
authors:
  - astra
created: 2026-11-21
updated: 2026-11-21
issue: https://github.com/tauten-dev/tauten/issues/60
milestone: "0.2"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Prove a world invariant directly over successful extracted steps

## Motivation

Tauten ships [`RSysReachable`](https://github.com/tauten-dev/tauten/blob/v0.1.0/proofs/Tauten/Machine.lean#L164-L189) for an Aeneas result-returning step and a nondeterministic world. There is a convenient world-invariant theorem for a **total** machine. Using it via `rsys_of_model` requires agreement with a second machine on **every** state and event, not just the ones on a reachable successful transition.

[Gripsack's publication adapter](https://github.com/gripsack-dev/gripsack/blob/c62afc5c05087e9e261b68fce1744f0f6301eeef/proofs/Protocol/Publication.lean#L95-L140) defines `next`, a total `model` and `extracted_agrees`, then uses that bridge in `reachable_inv`. Its agreement proof is legitimate and useful elsewhere; it is not evidence of a missing safety guarantee. A different extracted step that actually can fail should not need to be total merely to use an invariant induction principle. This is a Lean library question, not a new storage model, executable checker, or dependency on a shared runner.

## Proof obligations first

No change to Rust, `tauten-check-kernel`, the `World` or `RSysReachable` constructors, or what the gate admits. Propose **one** theorem in `Tauten.Machine`, proved by induction on the existing derivation. Its entire statement, up to binder names:

```lean
theorem rsys_inv_of_step (m : RMachine S E C) (w : World E C W)
    (init Inv : S × W → Prop)
    (h0 : ∀ x, init x → Inv x)
    (hstep : ∀ s s' c v v' e, Inv (s, v) → w.deliver v e →
      m.step s e = ok (s', c) → w.interpret v c v' → Inv (s', v'))
    (hadv : ∀ s v v', Inv (s, v) → w.adversary v v' → Inv (s, v')) :
    ∀ x, RSysReachable m w init x → Inv x
```

Each premise corresponds to a constructor. `fail` and divergence do not produce a successful machine transition. That says **nothing** about host effects before failure: the adopter must model them in its world. An adversary-only transition preserves the machine state, not necessarily the world. No restart, eventual success, crash recovery or filesystem law follows from this theorem.

## Behaviour and evidence

- Before: use `induction h` directly or prove total model agreement. After: an adopter may instead `apply rsys_inv_of_step` and discharge the initial, delivered-success and adversary cases. Both existing proof paths keep their names and statements.
- First build a small world against the **existing recursor**. Exhibit an actual delivered success that changes state and world, a permitted world-only step, and a distinct input for which the extracted-style step fails. Give it a nontrivial invariant; admitting a corrupting adversary step must invalidate the claimed invariant, and the failing input must not supply a success witness. Do not use `False` initialization or posit an always-successful model. Reuse the same case to evaluate the proposed lemma.
- Add the theorem to the normal axiom audit and keep `proofs/Tauten/Machine.lean` and the shipped `Proofs/TautenMachine.lean` template identical. Build the consumer example from that template and pass the full proof gate including existing exact mutants. This is a generic Lean library lemma, **not** a new proved Rust claim; do not manufacture a mutant or proof badge for a host adapter. If an extracted-code claim is changed, its own calibrated witnesses are required as usual.

## Alternatives, rollout and decision

A short, compiling example using `RSysReachable` induction alone is the no-new-API alternative. Prefer it if the named lemma does not save expressing the constructor plumbing in a real proof. `rinv_of_step` has no world; requiring `rsys_of_model` excludes genuinely partial steps; building a recovery framework addresses a different problem ([#18](https://github.com/tauten-dev/tauten/issues/18)).

For 0.2, ship the additive theorem: it is the missing fourth corner beside `inv_of_step`, `rinv_of_step` and `sys_inv_of_step`, and its premises match the `RSysReachable` constructors one to one, so it costs one three-case induction. The direct-induction miniature stays in the documentation as the no-new-API alternative. Ship the audited source and matching template, not a new crate, format or command. Old pins and proof records stay valid; an adopter can substitute direct induction if it cannot update. No Gripsack migration, tag or host proof is required for Tauten's release.

## Decision log

- 2026-11-21: drafted for [#60](https://github.com/tauten-dev/tauten/issues/60), following the concrete proof-only use in [Gripsack #366](https://github.com/gripsack-dev/gripsack/issues/366). Owner decision pending.
- 2026-11-21: **accepted, amended** (the owner delegated 0.2 grooming decisions to the executing agent after a Fable review): decide for the lemma instead of comparing first; keep the direct-induction miniature as the documented alternative; the lemma joins the `Audit.lean` axiom audit and the existing template parity test. The adopter trial is a follow-up ([#69](https://github.com/tauten-dev/tauten/issues/69)); `implementation` becomes `partial` after landing, `complete` once an adopter reports.
