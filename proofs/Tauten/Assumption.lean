import Tauten.Registry

/-!
# A claim states exactly the assumptions it rests on, and they can hold
# (`tauten_check_kernel::assumption`)

The audit derives the named assumptions a claim's theorems take as
hypotheses (`derived`) and the ones its witness theorem proves can hold
together (`established`); the claim declares `assumes` (`declared`).

Proved here, over the extracted function:
* `assumptions_ok_iff`: accepted exactly when declared and derived name the
  same assumptions (each conserves the other) and, when any is declared,
  the witness establishes every declared one.
* `undeclared_assumption_refused`: a hypothesis the claim does not declare
  is refused.
* `unused_assumption_refused`: a declared assumption no theorem uses is
  refused.
* `unwitnessed_refused`: declared assumptions that no witness establishes
  (they could contradict each other, making every theorem vacuous) are
  refused.

Trusted, not proved: the audit's derivation of `derived` and `established`
from the Lean statements (`proofs/Audit.lean`, listed in `TRUST.md`).
-/

open Aeneas Aeneas.Std Result
open tauten_check_kernel

namespace Tauten.Kernel

/-- What `assumptions_ok` computes, as a proposition. -/
def AssumptionsP (dec der est : List registry.Entry) : Prop :=
  ConservesP dec der [] ∧ ConservesP der dec [] ∧ (dec = [] ∨ ConservesP dec est [])

theorem empty_slice_val : (Array.to_slice (Std.Array.empty Std.Usize)).val = [] := by
  simp [Array.to_slice, Std.Array.empty]

theorem assumptions_ok_iff (dec der est : Slice registry.Entry) :
    assumption.assumptions_ok dec der est ⦃ r =>
      (r = true ↔ AssumptionsP dec.val der.val est.val) ⦄ := by
  unfold assumption.assumptions_ok
  simp only [lift, bind_ok]
  apply WP.spec_bind (conserves_iff dec der _)
  intro b hb
  rw [empty_slice_val] at hb
  cases b
  · simp only [Bool.false_eq_true, ite_false, bind_ok, WP.spec_ok, false_iff]
    intro h; exact absurd (hb.mpr h.1) (by simp)
  · simp only [ite_true]
    apply WP.spec_bind (conserves_iff der dec _)
    intro c hc
    rw [empty_slice_val] at hc
    cases c
    · simp only [Bool.false_eq_true, ite_false, WP.spec_ok, false_iff]
      intro h; exact absurd (hc.mpr h.2.1) (by simp)
    · simp only [ite_true]
      have h1 := hb.mp rfl
      have h2 := hc.mp rfl
      by_cases he : dec.val = []
      · have : core.slice.Slice.is_empty dec = ok true := by
          simp [core.slice.Slice.is_empty, Slice.len, he]
        simp only [this, bind_ok, ite_true, WP.spec_ok, true_iff]
        exact ⟨h1, h2, Or.inl he⟩
      · have : core.slice.Slice.is_empty dec = ok false := by
          simp [core.slice.Slice.is_empty, Slice.len, he]
        simp only [this, bind_ok, Bool.false_eq_true, ite_false]
        apply WP.spec_mono (conserves_iff dec est _)
        intro r hr
        rw [empty_slice_val] at hr
        rw [hr]
        constructor
        · intro h; exact ⟨h1, h2, Or.inr h⟩
        · intro ⟨_, _, h⟩
          rcases h with h | h
          · exact absurd h he
          · exact h

/-- A hypothesis the claim does not declare is refused. -/
theorem undeclared_assumption_refused (dec der est : Slice registry.Entry) (d : registry.Entry)
    (hd : d ∈ der.val) (hn : ∀ x ∈ dec.val, x.id ≠ d.id) :
    assumption.assumptions_ok dec der est ⦃ r => r = false ⦄ := by
  apply WP.spec_mono (assumptions_ok_iff dec der est)
  intro r hr
  cases r
  · rfl
  · obtain ⟨_, h2, _⟩ := hr.mp rfl
    rcases h2 d hd with h | ⟨x, hx, hid, _⟩
    · simp at h
    · exact absurd hid (hn x hx)

/-- A declared assumption no theorem uses is refused. -/
theorem unused_assumption_refused (dec der est : Slice registry.Entry) (d : registry.Entry)
    (hd : d ∈ dec.val) (hn : ∀ x ∈ der.val, x.id ≠ d.id) :
    assumption.assumptions_ok dec der est ⦃ r => r = false ⦄ := by
  apply WP.spec_mono (assumptions_ok_iff dec der est)
  intro r hr
  cases r
  · rfl
  · obtain ⟨h1, _, _⟩ := hr.mp rfl
    rcases h1 d hd with h | ⟨x, hx, hid, _⟩
    · simp at h
    · exact absurd hid (hn x hx)

/-- A declared assumption the witness does not establish is refused. -/
theorem unwitnessed_refused (dec der est : Slice registry.Entry) (d : registry.Entry)
    (hd : d ∈ dec.val) (hn : ∀ x ∈ est.val, x.id ≠ d.id) :
    assumption.assumptions_ok dec der est ⦃ r => r = false ⦄ := by
  apply WP.spec_mono (assumptions_ok_iff dec der est)
  intro r hr
  cases r
  · rfl
  · obtain ⟨_, _, h3⟩ := hr.mp rfl
    rcases h3 with h | h
    · rw [h] at hd; simp at hd
    · rcases h d hd with h' | ⟨x, hx, hid, _⟩
      · simp at h'
      · exact absurd hid (hn x hx)

end Tauten.Kernel
