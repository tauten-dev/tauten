import Aeneas

/-!
# What the macros' expansions reduce to (P3, plan section 2, binding 1)

`tauten-macros` itself (a proc-macro over `syn`/`quote`) is not translated.
Instead, each instance in `crates/tauten-proof-instances` is translated from
the macro's real expansion, and each instance theorem in
`Tauten/Refined.lean` and `Tauten/Contracts.lean` first proves that the
extracted function *equals* one of the shapes below at that instance's
predicate or conditions. The facts users rely on are proved here once, over
an arbitrary predicate, body and postcondition. If the macro changes what
it emits, the instance shape theorems stop holding and every claim proved
through them breaks (`proofs/mutants.py` checks this on mutants of
`crates/tauten-macros/src/lib.rs`).

This file mentions only Aeneas's types, never an extracted function.

- `refinedTryNew p err x`: `#[refined(p)]`'s `try_new`. Evaluate the
  predicate on the raw value; if it holds, `Ok` with the raw value itself,
  else `Err err`. The refined type is the raw type in the translation
  (Aeneas emits it as a reducible `def`), so the `Ok` value *is* the raw
  value: no clamping, no conversion.
- `contractCall pre reqErr post ensErr body`: `#[requires(pre)]` and
  `#[ensures(post)]` on a function whose parameters are passed by value.
  The precondition is decided first; if it is false, the result is
  `Err reqErr` and `body` is not used at all. Otherwise the body runs; an
  `Err` from it is returned unchanged, and an `Ok t` is returned if
  `post t`, else `Err ensErr`. `old(..)` snapshots are part of `post`: the
  instance theorems state `post` with the parameters' values at entry.
- `contractCallMut`: the same with a state `S` threaded through the body
  (a `&mut` parameter). If the precondition is false, the state is
  returned as it was at entry. A postcondition failure keeps the body's
  write: the wrapper reports the violation, it does not roll back.
-/

open Aeneas Aeneas.Std Result

namespace Tauten.Macros

/-- `#[refined(p)]`'s `try_new`, for an arbitrary total predicate `p`. -/
def refinedTryNew {T E : Type} (p : T → Bool) (err : E) (x : T) :
    Result (core.result.Result T E) :=
  if p x then ok (.Ok x) else ok (.Err err)

/-- F01-1, schematic: `try_new` returns `Ok` iff the predicate holds. -/
theorem refined_ok_iff {T E : Type} (p : T → Bool) (err : E) (x : T) :
    (∃ y, refinedTryNew p err x = ok (.Ok y)) ↔ p x = true := by
  unfold refinedTryNew
  cases h : p x <;> simp

/-- F01-2, schematic: an `Ok` value is the raw value, and it satisfies `p`. -/
theorem refined_ok_value {T E : Type} (p : T → Bool) (err : E) (x y : T)
    (h : refinedTryNew p err x = ok (.Ok y)) : y = x ∧ p x = true := by
  unfold refinedTryNew at h
  cases hp : p x <;> simp_all

/-- F01-3, schematic: `try_new` returns `Err` iff the predicate is false. -/
theorem refined_err_iff {T E : Type} (p : T → Bool) (err : E) (x : T) :
    (∃ e, refinedTryNew p err x = ok (.Err e)) ↔ p x = false := by
  unfold refinedTryNew
  cases h : p x <;> simp

/-- `try_new` never panics: it is `Ok x` or `Err err`. -/
theorem refined_total {T E : Type} (p : T → Bool) (err : E) (x : T) :
    refinedTryNew p err x = ok (.Ok x) ∨ refinedTryNew p err x = ok (.Err err) := by
  unfold refinedTryNew
  cases p x <;> simp

/-- The contract wrapper on a function with by-value parameters. -/
def contractCall {T E : Type} (pre : Bool) (reqErr : E) (post : T → Bool) (ensErr : E)
    (body : Result (core.result.Result T E)) : Result (core.result.Result T E) :=
  if pre then do
    let r ← body
    match r with
    | .Ok t => if post t then ok (.Ok t) else ok (.Err ensErr)
    | .Err e => ok (.Err e)
  else ok (.Err reqErr)

/-- A false precondition gives `Err reqErr` whatever the body would do,
including panic (`fail`) or diverge: the body is not evaluated. -/
theorem contract_requires {T E : Type} (reqErr ensErr : E) (post : T → Bool)
    (body : Result (core.result.Result T E)) :
    contractCall false reqErr post ensErr body = ok (.Err reqErr) := by
  simp [contractCall]

/-- With a true precondition the result is decided by the body and `post`. -/
theorem contract_ok {T E : Type} (reqErr ensErr : E) (post : T → Bool)
    (body : Result (core.result.Result T E)) (t : T) (h : body = ok (.Ok t)) :
    contractCall true reqErr post ensErr body
      = if post t then ok (.Ok t) else ok (.Err ensErr) := by
  simp [contractCall, h]

/-- An `Err` from the body is returned unchanged; no postcondition is checked. -/
theorem contract_err {T E : Type} (reqErr ensErr : E) (post : T → Bool)
    (body : Result (core.result.Result T E)) (e : E) (h : body = ok (.Err e)) :
    contractCall true reqErr post ensErr body = ok (.Err e) := by
  simp [contractCall, h]

/-- The contract wrapper on a function with a `&mut` parameter: the state
`s0` at entry, the body returning its result and the state at exit. -/
def contractCallMut {T E S : Type} (pre : Bool) (reqErr : E) (post : T → Bool) (ensErr : E)
    (s0 : S) (body : Result (core.result.Result T E × S)) :
    Result (core.result.Result T E × S) :=
  if pre then do
    let (r, s1) ← body
    match r with
    | .Ok t => if post t then ok (.Ok t, s1) else ok (.Err ensErr, s1)
    | .Err e => ok (.Err e, s1)
  else ok (.Err reqErr, s0)

/-- A false precondition gives `Err reqErr` with the state unchanged,
whatever the body would do. -/
theorem contract_mut_requires {T E S : Type} (reqErr ensErr : E) (post : T → Bool) (s0 : S)
    (body : Result (core.result.Result T E × S)) :
    contractCallMut false reqErr post ensErr s0 body = ok (.Err reqErr, s0) := by
  simp [contractCallMut]

/-- With a true precondition, the body's result and final state, checked by `post`. -/
theorem contract_mut_ok {T E S : Type} (reqErr ensErr : E) (post : T → Bool) (s0 s1 : S)
    (body : Result (core.result.Result T E × S)) (t : T) (h : body = ok (.Ok t, s1)) :
    contractCallMut true reqErr post ensErr s0 body
      = if post t then ok (.Ok t, s1) else ok (.Err ensErr, s1) := by
  simp [contractCallMut, h]

/-- An `Err` from the body is returned unchanged, with the body's final state. -/
theorem contract_mut_err {T E S : Type} (reqErr ensErr : E) (post : T → Bool) (s0 s1 : S)
    (body : Result (core.result.Result T E × S)) (e : E) (h : body = ok (.Err e, s1)) :
    contractCallMut true reqErr post ensErr s0 body = ok (.Err e, s1) := by
  simp [contractCallMut, h]

end Tauten.Macros
