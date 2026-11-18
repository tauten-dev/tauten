import Extracted.TautenCheckKernel

/-!
# Lint exceptions (`tauten_check_kernel::exception`)

`tauten_check::structure` calls the extracted `excuse` for every
`#[expect]`/`#[allow]` of a profile lint: `Forbidden` is reported as
TAU-X004, `CrateWide` as TAU-X002.
-/

open Aeneas Aeneas.Std Result
open tauten_check_kernel

namespace Tauten.Kernel

deriving instance DecidableEq for tiers.Tier
deriving instance DecidableEq for severity.Level

/-- An exception is permitted iff the rule admits exceptions and the crate is
not a kernel crate in which the rule has a level. Never fails. -/
theorem permitted_spec (allows : Bool) (t : tiers.Tier) (level : Option severity.Level) :
    exception.permitted allows t level =
      ok (decide (allows = true ∧ ¬ (t = .Kernel ∧ level ≠ none))) := by
  cases allows <;> cases t <;> rcases level with _ | (_ | _) <;> simp [exception.permitted]

/-- The whole decision: forbidden when the rule admits no exceptions or the
crate is a kernel crate in which the rule has a level; otherwise reported as
crate-wide when the exception is an inner attribute and the rule blocks
(`deny`) in the tier; otherwise allowed. Never fails. -/
theorem exception_spec (allows : Bool) (t : tiers.Tier) (level : Option severity.Level)
    (crateWide : Bool) :
    exception.excuse allows t level crateWide =
      ok (if allows = false ∨ (t = .Kernel ∧ level ≠ none) then .Forbidden
          else if crateWide = true ∧ level = some .Deny then .CrateWide
          else .Allowed) := by
  cases allows <;> cases t <;> rcases level with _ | (_ | _) <;> cases crateWide <;>
    simp [exception.excuse, exception.permitted, severity.is_blocking]

/-- A kernel crate never excuses a rule that has a level in the kernel tier,
whatever the rule's `exceptions` flag and whatever the attribute's scope. -/
theorem kernel_never_excuses_levelled_rule (allows : Bool) (l : severity.Level)
    (crateWide : Bool) :
    exception.excuse allows .Kernel (some l) crateWide = ok .Forbidden := by
  cases allows <;> cases l <;> cases crateWide <;>
    simp [exception.excuse, exception.permitted, severity.is_blocking]

/-- A rule with `exceptions = false` is never excused, in any tier. -/
theorem no_exceptions_never_excused (t : tiers.Tier) (level : Option severity.Level)
    (crateWide : Bool) :
    exception.excuse false t level crateWide = ok .Forbidden := by
  cases t <;> rcases level with _ | (_ | _) <;> cases crateWide <;>
    simp [exception.excuse, exception.permitted, severity.is_blocking]

end Tauten.Kernel
