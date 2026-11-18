import Extracted.TautenCheckKernel

/-!
# The tier relation (`tauten_check_kernel::tiers`)

Over the committed Aeneas translation of `crates/tauten-check-kernel`.
`tauten_check::Tier::may_depend_on` calls the extracted `may_depend_on`, and
TAU-T002 reports exactly the workspace dependencies it refuses. Every
theorem is proved directly against the extracted definitions (not through
another theorem), so a mutant of the Rust source breaks each claim it
falsifies.
-/

open Aeneas Aeneas.Std Result
open tauten_check_kernel

namespace Tauten.Kernel

/-- The intended order of tiers, innermost first. -/
def rankN : tiers.Tier → Nat
  | .Kernel => 0
  | .Core => 1
  | .Edge => 2

/-- `rank` never fails and returns the intended position. -/
theorem rank_spec (t : tiers.Tier) : ∃ r, tiers.rank t = ok r ∧ r.val = rankN t := by
  cases t <;> exact ⟨_, rfl, rfl⟩

/-- `may_depend_on` never fails and agrees with the intended rank:
a tier may depend on its own tier and on tiers further in. -/
theorem may_depend_on_spec (t dep : tiers.Tier) :
    tiers.may_depend_on t dep = ok (decide (rankN dep ≤ rankN t)) := by
  cases t <;> cases dep <;> simp [tiers.may_depend_on, tiers.rank, rankN]

/-- Reflexive: every tier may depend on its own tier. -/
theorem may_depend_on_refl (t : tiers.Tier) : tiers.may_depend_on t t = ok true := by
  cases t <;> simp [tiers.may_depend_on, tiers.rank]

/-- Transitive. -/
theorem may_depend_on_trans (a b c : tiers.Tier)
    (hab : tiers.may_depend_on a b = ok true) (hbc : tiers.may_depend_on b c = ok true) :
    tiers.may_depend_on a c = ok true := by
  cases a <;> cases b <;> cases c <;>
    simp_all [tiers.may_depend_on, tiers.rank]

/-- Total: of two tiers, one may always depend on the other. -/
theorem may_depend_on_total (a b : tiers.Tier) :
    tiers.may_depend_on a b = ok true ∨ tiers.may_depend_on b a = ok true := by
  cases a <;> cases b <;> simp [tiers.may_depend_on, tiers.rank]

/-- Antisymmetric: two tiers that may depend on each other are the same tier.
With the three above, `may_depend_on` is a total order. -/
theorem may_depend_on_antisymm (a b : tiers.Tier)
    (hab : tiers.may_depend_on a b = ok true) (hba : tiers.may_depend_on b a = ok true) :
    a = b := by
  cases a <;> cases b <;> simp_all [tiers.may_depend_on, tiers.rank]

/-- A kernel crate may depend only on kernel crates. -/
theorem kernel_depends_only_on_kernel (dep : tiers.Tier) :
    tiers.may_depend_on .Kernel dep = ok true ↔ dep = .Kernel := by
  cases dep <;> simp [tiers.may_depend_on, tiers.rank]

end Tauten.Kernel
