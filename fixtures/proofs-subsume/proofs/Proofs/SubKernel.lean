import Extracted.SubKernel
import Proofs.TautenAssumption

/-!
# Theorems about `sub-kernel` (the proof-subsumption fixture)

`add_capped` and `first_or_zero` are proved total (`∀ args, f args ⦃ _ ⦄`,
no hypotheses): their arithmetic and indexing never fail, so `cargo tauten
check` reports TAU-N001/TAU-P003 inside them as subsumed. `double_spec`
needs a hypothesis, so the audit does not list it as total, its claim is
`audit-failed`, and the finding in `double` stays an error.
-/

open Aeneas Aeneas.Std Result
open sub_kernel

namespace Proofs.SubKernel

/-- `add_capped` never fails and returns the sum, capped at `u32::MAX`. -/
theorem add_capped_spec (a b : U32) :
    add_capped a b ⦃ r => r.val = min (a.val + b.val) U32.max ⦄ := by
  have hmax : (core.num.U32.MAX : U32).val = 4294967295 := by
    simp [core.num.U32.MAX, U32.rMax]
  have hb : b.val ≤ (core.num.U32.MAX : U32).val := by rw [hmax]; scalar_tac
  unfold add_capped
  step as ⟨ i, hi ⟩
  simp only [U32.rMax] at hi
  split
  · rename_i h
    simp only [WP.spec_ok, U32.max_eq]
    scalar_tac
  · rename_i h
    step as ⟨ r, hr ⟩
    simp only [U32.max_eq]
    scalar_tac

/-- `first_or_zero` never fails and returns the first value, or 0. -/
theorem first_or_zero_spec (values : Slice U32) :
    first_or_zero values ⦃ r => r = values.val.headD 0#u32 ⦄ := by
  unfold first_or_zero
  step as ⟨ b, hb ⟩
  split
  · rename_i h
    simp only [WP.spec_ok]
    have : values.val = [] := by simp_all [Slice.length]
    simp [this]
  · rename_i h
    have hlen : 0 < values.length := by
      rcases Nat.eq_zero_or_pos values.length with h0 | h0
      · simp_all
      · exact h0
    step as ⟨ x, hx ⟩
    cases hv : values.val with
    | nil => simp_all [Slice.length]
    | cons y ys => simp_all

/-- `double` doubles a value up to 1000. Not total: it needs a hypothesis. -/
theorem double_spec (a : U32) (h : a.val ≤ 1000) :
    double a ⦃ r => r.val = 2 * a.val ⦄ := by
  unfold double
  step as ⟨ r, hr ⟩
  scalar_tac

/-- The counter source never goes backwards. -/
@[tauten_assumption "FIXTURE.counter-monotone"]
def CounterMonotone (read : Nat → U32) : Prop := ∀ i j, i ≤ j → (read i).val ≤ (read j).val

/-- Under `FIXTURE.counter-monotone`, `latest` of an earlier and a later
reading is the later one. -/
theorem latest_is_later_reading (read : Nat → U32) (h : CounterMonotone read)
    (i j : Nat) (hij : i ≤ j) :
    latest (read i) (read j) ⦃ r => r = read j ⦄ := by
  have := h i j hij
  unfold latest
  split
  · simp
  · rename_i hn
    simp only [WP.spec_ok]
    exfalso; apply hn; scalar_tac

/-- The assumption can hold: a constant counter never goes backwards. -/
theorem constant_counter_is_monotone : CounterMonotone (fun _ => 0#u32) := by
  intro i j _
  simp

end Proofs.SubKernel
