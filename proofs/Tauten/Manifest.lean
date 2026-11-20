import Tauten.Control

/-!
# Required workspace build settings (`tauten_check_kernel::manifest`)

Proved here, over the extracted function:
* `hardening_iff`: a finding exactly when the rule is required and the fact
  is not `Yes`.
* `unparsed_never_clean`: a required setting tauten could not read is
  always a finding.

Trusted, not proved: the shell's reading of `Cargo.toml` and
`.cargo/config.toml` into a fact (unit-tested), and that no environment
variable (`CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS`) overrides them at build
time.
-/

open Aeneas Aeneas.Std Result
open tauten_check_kernel

namespace Tauten.Kernel

theorem hardening_iff (required : Bool) (fact : manifest.Fact) :
    manifest.hardening required fact ⦃ r =>
      (r = true ↔ required = true ∧ fact ≠ manifest.Fact.Yes) ⦄ := by
  unfold manifest.hardening
  cases fact <;> simp

theorem unparsed_never_clean :
    manifest.hardening true manifest.Fact.Unparsed ⦃ r => r = true ⦄ := by
  apply WP.spec_mono (hardening_iff true manifest.Fact.Unparsed)
  intro r hr
  exact hr.mpr ⟨rfl, by simp⟩

end Tauten.Kernel
