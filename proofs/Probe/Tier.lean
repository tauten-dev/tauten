import Extracted.TautenProbe

/-!
# P0 probe: the tier relation

Over the Aeneas translation of `tauten_probe::tier` (proofs/probe). `rank`
replaces the derived `PartialOrd`; both forms are translated and agree.
-/

open Aeneas Aeneas.Std Result
open tauten_probe

namespace Tauten.Probe

/-- The intended order of tiers, innermost first. -/
def rankN : tier.Tier → Nat
  | .Kernel => 0
  | .Core => 1
  | .Edge => 2

/-- `rank` returns the intended position and never fails. -/
theorem rank_spec (t : tier.Tier) : ∃ r, tier.rank t = ok r ∧ r.val = rankN t := by
  cases t <;> exact ⟨_, rfl, rfl⟩

/-- `may_depend_on` agrees with `rank`: inward or same-tier dependencies only. -/
theorem may_depend_on_spec (t dep : tier.Tier) :
    tier.may_depend_on t dep = ok (decide (rankN dep ≤ rankN t)) := by
  cases t <;> cases dep <;> simp [tier.may_depend_on, tier.rank, rankN] <;> decide

/-- The shipped derived-`PartialOrd` form decides the same relation. -/
theorem may_depend_on_derived_agrees (t dep : tier.Tier) :
    tier.may_depend_on_derived t dep = tier.may_depend_on t dep := by
  cases t <;> cases dep <;>
  simp [tier.may_depend_on_derived, core.cmp.PartialOrd.le.trait_default,
    core.cmp.PartialOrd.le.default, core.cmp.PartialOrd.le_body,
    tier.Tier.Insts.CoreCmpPartialOrdTier.partial_cmp, tier.Tier.Insts.CoreCmpOrd.cmp,
    tier.Tier.read_discriminant, tier.may_depend_on, tier.rank] <;> decide

/-- A kernel crate may depend only on kernel crates. -/
theorem kernel_depends_only_on_kernel (dep : tier.Tier) :
    tier.may_depend_on .Kernel dep = ok true ↔ dep = .Kernel := by
  cases dep <;> simp [may_depend_on_spec, rankN]

end Tauten.Probe
