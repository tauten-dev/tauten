import Extracted.TautenCheckKernel

/-!
# Shared proved helpers (`tauten_check_kernel::budget`, `::version`; A10, #14)

Small total functions that other kernels call instead of re-proving them:
budget arithmetic and format-version admission. (Byte equality and
membership are in `Tauten/Binding.lean`: `same_bytes_spec`,
`contains_bytes_spec`.) A consumer crate cites these claims through `inherit`.

Proved here, over the extracted functions:
* `charge_spec`: `charge left cost` returns a value exactly when
  `cost ≤ left`, leaving `left - cost`.
* `charges_never_exceed_initial`: what is left plus what was charged is the
  budget before, so a run of charges never spends more than it started with.
* `charge_repeated_no_overflow`: `charge_repeated left cost times` returns
  a value exactly when `cost * times ≤ left` in unbounded arithmetic,
  leaving `left - cost * times`; an overflowing product is refused, never
  wrapped.
* `version_admit_total`, `version_admit_spec`: `admit` never fails, and admits
  exactly a present version within `oldest..=newest`.
* `never_admits_newer`: a version newer than `newest` is reported `Newer`.
* `absent_is_invalid`: a missing version is reported `Absent`.

Trusted, not proved: the shell's reading of the version field.
-/

open Aeneas Aeneas.Std Result
open tauten_check_kernel

namespace Tauten.Kernel

/-! ## Budget -/

theorem charge_spec (left cost : Std.U64) :
    budget.charge left cost ⦃ r =>
      (cost.val ≤ left.val → ∃ v : Std.U64, r = some v ∧ v.val = left.val - cost.val) ∧
      (left.val < cost.val → r = none) ⦄ := by
  unfold budget.charge
  have h := U64.checked_sub_bv_spec left cost
  rcases hs : U64.checked_sub left cost with _ | z
  · rw [hs] at h
    simp only [WP.spec_ok]
    exact ⟨fun h' => absurd h' (by omega), fun _ => trivial⟩
  · rw [hs] at h
    obtain ⟨h1, h2, -⟩ := h
    simp only [WP.spec_ok]
    exact ⟨fun _ => ⟨z, rfl, h2⟩, fun h' => absurd h1 (by omega)⟩

theorem charges_never_exceed_initial (left cost : Std.U64) :
    budget.charge left cost ⦃ r => ∀ v : Std.U64, r = some v → v.val + cost.val = left.val ⦄ := by
  apply WP.spec_mono (charge_spec left cost)
  rintro r ⟨h1, h2⟩ v rfl
  by_cases h : cost.val ≤ left.val
  · obtain ⟨w, hw, hv⟩ := h1 h
    cases hw
    omega
  · cases h2 (by omega)

theorem charge_repeated_no_overflow (left cost times : Std.U64) :
    budget.charge_repeated left cost times ⦃ r =>
      (cost.val * times.val ≤ left.val →
        ∃ v : Std.U64, r = some v ∧ v.val = left.val - cost.val * times.val) ∧
      (left.val < cost.val * times.val → r = none) ⦄ := by
  unfold budget.charge_repeated
  have hm := U64.checked_mul_bv_spec cost times
  rcases hmul : U64.checked_mul cost times with _ | t
  · rw [hmul] at hm
    simp only [lift, bind_ok, WP.spec_ok]
    have hl : left.val < cost.val * times.val := by
      have := left.hBounds; scalar_tac
    exact ⟨fun h => absurd h (by omega), fun _ => trivial⟩
  · rw [hmul] at hm
    obtain ⟨-, ht, -⟩ := hm
    simp only [lift, bind_ok]
    have h := U64.checked_sub_bv_spec left t
    rcases hs : U64.checked_sub left t with _ | z
    · rw [hs] at h
      simp only [WP.spec_ok]
      exact ⟨fun h' => absurd h' (by omega), fun _ => trivial⟩
    · rw [hs] at h
      obtain ⟨h1, h2, -⟩ := h
      simp only [WP.spec_ok]
      exact ⟨fun _ => ⟨z, rfl, by rw [h2, ht]⟩, fun h' => absurd h1 (by omega)⟩

/-! ## Version admission -/

theorem version_admit_total (found : Option Std.U32) (oldest newest : Std.U32) :
    version.admit found oldest newest ⦃ _ => True ⦄ := by
  unfold version.admit
  cases found <;> simp only [WP.spec_ok]
  split_ifs <;> simp

theorem version_admit_spec (found : Option Std.U32) (oldest newest : Std.U32) :
    version.admit found oldest newest = ok .Admitted ↔
      ∃ v : Std.U32, found = some v ∧ oldest.val ≤ v.val ∧ v.val ≤ newest.val := by
  cases found with
  | none => simp [version.admit]
  | some v =>
    simp only [version.admit, Option.some.injEq, exists_eq_left']
    by_cases h1 : v > newest
    · simp only [h1, ite_true]
      have : ¬ v.val ≤ newest.val := by scalar_tac
      simp [this]
    · by_cases h2 : v < oldest
      · simp only [h1, h2, ite_true, ite_false]
        have : ¬ oldest.val ≤ v.val := by scalar_tac
        simp [this]
      · simp only [h1, h2, ite_false]
        have : oldest.val ≤ v.val ∧ v.val ≤ newest.val := by constructor <;> scalar_tac
        simp [this]

theorem never_admits_newer (v oldest newest : Std.U32) (h : newest.val < v.val) :
    version.admit (some v) oldest newest = ok .Newer := by
  have : v > newest := by scalar_tac
  simp [version.admit, this]

theorem absent_is_invalid (oldest newest : Std.U32) :
    version.admit none oldest newest = ok .Absent := by
  simp [version.admit]

end Tauten.Kernel
