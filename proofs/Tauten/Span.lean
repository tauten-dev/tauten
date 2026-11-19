import Tauten.Ratchet

/-!
# Which function a finding is in (`tauten_check_kernel::span`)

A proof subsumes findings only inside the function its claim names, so the
function a finding is attributed to must be right. The shell lists every
function's first and last line; the extracted `innermost` picks one.

Proved here (`innermost_spec`): the result is `none` exactly when no span
covers the line; otherwise it is a covering span, no span covering the line
is narrower, and among equally narrow covering spans it is the first.

Trusted, not proved: the shell's line spans (from `syn`; tested).
-/

open Aeneas Aeneas.Std Result
open tauten_check_kernel

namespace Tauten.Kernel

/-- `s` covers `line`. -/
def covN (s : Std.U32 × Std.U32) (line : Std.U32) : Prop := s.1.val ≤ line.val ∧ line.val ≤ s.2.val

instance (s : Std.U32 × Std.U32) (line : Std.U32) : Decidable (covN s line) :=
  inferInstanceAs (Decidable (_ ∧ _))

/-- Lines after the first. -/
def widN (s : Std.U32 × Std.U32) : Nat := s.2.val - s.1.val

theorem covers_spec (s : Std.U32 × Std.U32) (line : Std.U32) :
    span.covers s line = ok (decide (covN s line)) := by
  rcases s with ⟨a, b⟩
  by_cases h : a.val ≤ line.val <;> simp [span.covers, covN, h]

theorem width_spec (s : Std.U32 × Std.U32) :
    ∃ w, span.width s = ok w ∧ w.val = widN s := by
  rcases s with ⟨a, b⟩
  refine ⟨core.num.U32.saturating_sub b a, rfl, ?_⟩
  simp only [core.num.U32.saturating_sub, uscalar_sat_sub_val, widN]

/-- After looking at the first `k` spans, `best` is the narrowest covering
one among them (the first on a tie), or `none` if none of them covers. -/
def SpanInv (l : List (Std.U32 × Std.U32)) (line : Std.U32) (best : Option Std.Usize) (k : Nat) : Prop :=
  (∀ b, best = some b → b.val < k ∧ ∃ sb, l[b.val]? = some sb ∧ covN sb line ∧
      ∀ j sj, j < k → l[j]? = some sj → covN sj line →
        widN sb ≤ widN sj ∧ (widN sj = widN sb → b.val ≤ j)) ∧
  (best = none → ∀ j sj, j < k → l[j]? = some sj → ¬ covN sj line)

theorem consider_spec (ss : Slice (Std.U32 × Std.U32)) (line : Std.U32) (best : Option Std.Usize)
    (k : Std.Usize) (hk : k.val < ss.val.length) (inv : SpanInv ss.val line best k.val) :
    span.consider ss line best k ⦃ r => SpanInv ss.val line r (k.val + 1) ⦄ := by
  obtain ⟨hsome, hnone⟩ := inv
  have hget : ss.val[k.val]? = some ss.val[k.val] := List.getElem?_eq_getElem hk
  unfold span.consider
  simp only [core.slice.Slice.get, core.slice.index.SliceIndexUsizeSlice,
    core.slice.index.Usize.get, Slice.getElem?_Usize_eq, bind_tc_ok, bind_ok, hget, covers_spec]
  set s := ss.val[k.val] with hs
  by_cases hc : covN s line
  · simp only [hc, decide_true, ite_true]
    cases best with
    | none =>
      simp only [WP.spec_ok]
      refine ⟨?_, by simp⟩
      intro b hb
      cases hb
      refine ⟨by omega, s, hget, hc, ?_⟩
      intro j sj hj hsj hcj
      by_cases hjk : j = k.val
      · subst hjk; rw [hget] at hsj; cases hsj; omega
      · exact absurd hcj (hnone rfl j sj (by omega) hsj)
    | some b =>
      obtain ⟨hbk, sb, hsb, hcb, hmin⟩ := hsome b rfl
      simp only [hsb]
      obtain ⟨wb, hwb, hwbv⟩ := width_spec sb
      obtain ⟨wk, hwk, hwkv⟩ := width_spec s
      simp only [hwb, hwk, bind_ok]
      by_cases hle : wb ≤ wk
      · simp only [hle, ite_true, WP.spec_ok]
        refine ⟨?_, by simp⟩
        intro b' hb'
        cases hb'
        refine ⟨by omega, sb, hsb, hcb, ?_⟩
        intro j sj hj hsj hcj
        by_cases hjk : j = k.val
        · subst hjk; rw [hget] at hsj; cases hsj
          have : wb.val ≤ wk.val := by scalar_tac
          omega
        · exact hmin j sj (by omega) hsj hcj
      · simp only [hle, ite_false, WP.spec_ok]
        have hlt : wk.val < wb.val := by scalar_tac
        refine ⟨?_, by simp⟩
        intro b' hb'
        cases hb'
        refine ⟨by omega, s, hget, hc, ?_⟩
        intro j sj hj hsj hcj
        by_cases hjk : j = k.val
        · subst hjk; rw [hget] at hsj; cases hsj; omega
        · have := hmin j sj (by omega) hsj hcj
          omega
  · simp only [hc, decide_false, Bool.false_eq_true, ite_false, WP.spec_ok]
    refine ⟨fun b hb => ?_, fun hb j sj hj hsj => ?_⟩
    · obtain ⟨hbk, sb, hsb, hcb, hmin⟩ := hsome b hb
      refine ⟨by omega, sb, hsb, hcb, ?_⟩
      intro j sj hj hsj hcj
      by_cases hjk : j = k.val
      · subst hjk; rw [hget] at hsj; cases hsj; exact absurd hcj hc
      · exact hmin j sj (by omega) hsj hcj
    · by_cases hjk : j = k.val
      · subst hjk; rw [hget] at hsj; cases hsj; exact hc
      · exact hnone hb j sj (by omega) hsj

theorem innermost_loop_spec (ss : Slice (Std.U32 × Std.U32)) (line : Std.U32)
    (best : Option Std.Usize) (i : Std.Usize) (hi : i.val ≤ ss.val.length)
    (inv : SpanInv ss.val line best i.val) :
    span.innermost_loop ss line best i ⦃ r => SpanInv ss.val line r ss.val.length ⦄ := by
  unfold span.innermost_loop
  apply loop.spec_decr_nat
    (fun p : Option Std.Usize × Std.Usize => ss.val.length - p.2.val)
    (fun p : Option Std.Usize × Std.Usize => p.2.val ≤ ss.val.length ∧ SpanInv ss.val line p.1 p.2.val)
  · rintro ⟨b, k⟩ ⟨hk, hinv⟩
    unfold span.innermost_loop.body
    dsimp only at hk hinv ⊢
    split
    · rename_i hlt
      have hlt' : k.val < ss.val.length := by scalar_tac
      apply WP.spec_bind (consider_spec ss line b k hlt' hinv)
      intro r hr
      have hk2 := usize_sat_add_one k (by scalar_tac)
      simp only [bind_ok, lift, bind_tc_ok, WP.spec_ok]
      refine ⟨⟨by omega, ?_⟩, by omega⟩
      rw [hk2]; exact hr
    · rename_i hge
      try simp only [WP.spec_ok]
      have : k.val = ss.val.length := by scalar_tac
      rw [← this]; exact hinv
  · exact ⟨hi, inv⟩

/-- The narrowest covering span, first on a tie; `none` exactly when no
span covers the line. -/
theorem innermost_spec (ss : Slice (Std.U32 × Std.U32)) (line : Std.U32) :
    span.innermost ss line ⦃ r => SpanInv ss.val line r ss.val.length ⦄ := by
  unfold span.innermost
  refine innermost_loop_spec ss line none 0#usize (by simp) ⟨?_, ?_⟩
  · intro b hb
    simp at hb
  · intro _ j sj hj
    exact absurd hj (by simp)

/-- Non-vacuity: a nested function is chosen over its enclosing one, and a
line outside every function is attributed to none. -/
theorem innermost_examples :
    covN (8#u32, 13#u32) 11#u32 ∧ covN (10#u32, 12#u32) 11#u32 ∧
    widN (10#u32, 12#u32) < widN (8#u32, 13#u32) ∧ ¬ covN (1#u32, 3#u32) 5#u32 := by
  simp [covN, widN]

end Tauten.Kernel
