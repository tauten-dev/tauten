import Extracted.TautenProbe

/-!
# P0 probe: refined constructors

`Percent` is the real `#[tauten::refined(|v| *v <= 100)]` expansion (the
predicate passed as a closure to `tauten::contract::holds`); `PercentHand`
is the hand-written closure-free equivalent. Both are proved the same way.
-/

open Aeneas Aeneas.Std Result
open tauten_probe

namespace Tauten.Probe

/-- The macro's constructor admits exactly the values with `v ≤ 100`, unchanged. -/
theorem percent_try_new_ok (v : Std.U8) (h : v.val ≤ 100) :
    percent.__tauten_percent.Percent.try_new v = ok (.Ok v) := by
  unfold percent.__tauten_percent.Percent.try_new tauten.contract.holds
  simp only [
    percent.__tauten_percent.Percent.try_new.closure.Insts.CoreOpsFunctionFnOnceTupleShared0U8Bool.call_once]
  have : (v ≤ 100#u8) := by scalar_tac
  simp [this]

theorem percent_try_new_err (v : Std.U8) (h : 100 < v.val) :
    ∃ e, percent.__tauten_percent.Percent.try_new v = ok (.Err e) := by
  unfold percent.__tauten_percent.Percent.try_new tauten.contract.holds
  simp only [
    percent.__tauten_percent.Percent.try_new.closure.Insts.CoreOpsFunctionFnOnceTupleShared0U8Bool.call_once]
  have : ¬ (v ≤ 100#u8) := by scalar_tac
  simp [this, tauten.error.RefineError.new]

/-- The macro's constructor returns `Ok` iff the predicate holds. -/
theorem percent_try_new_ok_iff (v : Std.U8) :
    (∃ p, percent.__tauten_percent.Percent.try_new v = ok (.Ok p)) ↔ v.val ≤ 100 := by
  constructor
  · rintro ⟨p, hp⟩
    by_contra h
    obtain ⟨e, he⟩ := percent_try_new_err v (by omega)
    rw [he] at hp
    simp at hp
  · intro h
    exact ⟨v, percent_try_new_ok v h⟩

/-- The closure-free hand expansion: the same statement. -/
theorem percent_hand_try_new_ok_iff (v : Std.U8) :
    (∃ p, percent_hand.inner.PercentHand.try_new v = ok (.Ok p)) ↔ v.val ≤ 100 := by
  unfold percent_hand.inner.PercentHand.try_new
  by_cases h : v.val ≤ 100
  · have : (v ≤ 100#u8) := by scalar_tac
    simp [this, h]
  · have : ¬ (v ≤ 100#u8) := by scalar_tac
    simp [this, h, tauten.error.RefineError.new]

end Tauten.Probe
