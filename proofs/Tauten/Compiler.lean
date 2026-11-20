import Tauten.Control

/-!
# Compiler output and the exit verdict (`tauten_check_kernel::compiler`)

Proved here, over the extracted functions:
* `kept_iff`: a compiler message is a finding exactly when it is an error
  or a warning, not a summary line, and either its crate declares a tier or
  the profile made it an error.
* `applies_spec`: a rule applies to a target exactly when it has a level in
  the tier and the target is not a test target or the rule applies in tests.
* `check_passes_iff`: the check passes exactly when no finding is an error;
  `warnings_never_fail`: a list of warnings passes.

Trusted, not proved: the shell's conversion of cargo's JSON messages
(`clippy.rs`, checked against recorded output of the pinned toolchain in
`crates/cargo-tauten/tests/golden/`) and of a lint code to its rule.
-/

open Aeneas Aeneas.Std Result
open tauten_check_kernel

namespace Tauten.Kernel

deriving instance DecidableEq for severity.Severity
deriving instance DecidableEq for compiler.MsgLevel

theorem kept_iff (l : compiler.MsgLevel) (summary tiered : Bool) (s : severity.Severity) :
    compiler.kept l summary tiered s ⦃ r =>
      (r = true ↔ l ≠ .Other ∧ summary = false ∧ (tiered = true ∨ s = .Error)) ⦄ := by
  unfold compiler.kept
  cases l <;> cases summary <;> cases tiered <;> cases s <;> simp

theorem applies_spec (level : Option severity.Level) (inTests test : Bool) :
    compiler.applies level inTests test ⦃ r =>
      (r = true ↔ level ≠ none ∧ (inTests = true ∨ test = false)) ⦄ := by
  unfold compiler.applies
  rcases level with _ | (_ | _) <;> cases inTests <;> cases test <;> simp

theorem passes_spec (fs : Slice severity.Severity) :
    compiler.passes fs ⦃ r => r = fs.val.all (fun s => decide (s ≠ .Error)) ⦄ := by
  unfold compiler.passes compiler.passes_loop
  apply WP.spec_mono (fold_loop fs (fun a s => a && decide (s ≠ .Error)) _ ?_ ?_
    true 0#usize (by simp))
  · intro r hr; rw [hr, foldl_and]; simp
  · intro a i h
    have hlt : i < Slice.len fs := by scalar_tac
    simp only [compiler.passes_loop.body, hlt, ite_true, get_ok fs i h, bind_ok, lift]
    cases a
    · simp
    · cases fs.val[i.val] <;> simp
  · intro a i h
    have hlt : ¬ i < Slice.len fs := by scalar_tac
    simp only [compiler.passes_loop.body, hlt, ite_false, WP.spec_ok]

/-- The check passes exactly when no finding is an error. -/
theorem check_passes_iff (fs : Slice severity.Severity) :
    compiler.passes fs ⦃ r => (r = true ↔ severity.Severity.Error ∉ fs.val) ⦄ := by
  apply WP.spec_mono (passes_spec fs)
  intro r hr
  rw [hr]
  simp only [List.all_eq_true, decide_eq_true_eq]
  constructor
  · intro h he; exact h _ he rfl
  · intro h x hx hxe; subst hxe; exact h hx

/-- Warnings never fail the check. -/
theorem warnings_never_fail (fs : Slice severity.Severity)
    (h : ∀ s ∈ fs.val, s = .Warning) :
    compiler.passes fs ⦃ r => r = true ⦄ := by
  apply WP.spec_mono (check_passes_iff fs)
  intro r hr
  exact hr.mpr (fun he => by cases h _ he)

end Tauten.Kernel
