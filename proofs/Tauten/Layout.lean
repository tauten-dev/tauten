import Tauten.Control

/-!
# Workspace layout rules (`tauten_check_kernel::layout`)

`tauten_check::structure` decides TAU-T001 (declared tier), TAU-T003 (the
kernel's dependency allowlist) and TAU-K003 (`no_std` kernel library roots)
with the extracted functions.

Proved here:
* `t001_iff`: a finding exactly when the tier is unknown or missing.
* `t003_iff`: a finding exactly for a kernel crate whose dependency is not
  on its allowlist; other tiers never.
* `k003_iff`: a finding exactly for a kernel crate whose root is not
  unconditionally `no_std` or `cfg_attr(not(test), no_std)`.
* `conditional_no_std_is_finding`, `unparsed_root_is_finding`: any other
  condition, and a root that did not parse, are findings.

Trusted, not proved: the shell's reading of the manifest and the crate root
into these facts (unit-tested), including which `cfg_attr` predicate counts
as exactly `not(test)`.
-/

open Aeneas Aeneas.Std Result
open tauten_check_kernel

namespace Tauten.Kernel

theorem t001_iff (d : layout.Declared) :
    layout.t001 d ⦃ r => (r = true ↔ ∀ t, d ≠ .Known t) ⦄ := by
  unfold layout.t001
  cases d <;> simp

theorem t003_iff (t : tiers.Tier) (dep : Std.Usize) (allowed : Slice Std.Usize) :
    layout.t003 t dep allowed ⦃ r => (r = true ↔ t = .Kernel ∧ dep ∉ allowed.val) ⦄ := by
  unfold layout.t003
  cases t
  · apply WP.spec_bind (member_spec allowed dep)
    intro b hb
    subst hb
    simp [memberN]
  · simp
  · simp

/-- TAU-K003 fires exactly for a kernel crate whose root is not
unconditionally `no_std` (or `no_std` whenever built as a library). -/
theorem k003_iff (t : tiers.Tier) (f : layout.NoStd) :
    layout.k003 t f ⦃ r =>
      (r = true ↔ t = .Kernel ∧ f ≠ .Unconditional ∧ f ≠ .NotTest) ⦄ := by
  unfold layout.k003
  cases t <;> cases f <;> simp

theorem conditional_no_std_is_finding :
    layout.k003 .Kernel .Conditional ⦃ r => r = true ⦄ := by
  simp [layout.k003]

theorem unparsed_root_is_finding :
    layout.k003 .Kernel .Unparsed ⦃ r => r = true ⦄ := by
  simp [layout.k003]

end Tauten.Kernel
