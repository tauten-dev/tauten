import Extracted.TautenCheckKernel

/-!
# Declared layers inside a tier (`tauten_check_kernel::layer`, TAU-T004)

`tauten_check::structure` decides every dependency edge between members of
a workspace that declares layers with the extracted `finding`.

Proved here:
* `may_use_spec`; the order over layer positions is reflexive
  (`may_use_refl`), transitive (`may_use_trans`), antisymmetric
  (`may_use_antisymm`) and total (`may_use_total`).
* `layer_finding_iff`: an edge is a finding exactly unless both crates have
  a known layer and the dependency's position is not later.
* `unknown_layer_is_finding`: an unknown or missing declaration on either
  side is never clean.

Trusted, not proved: the shell's reading of `layers` and `layer` from the
manifests into positions (unit-tested).
-/

open Aeneas Aeneas.Std Result
open tauten_check_kernel

namespace Tauten.Kernel

theorem may_use_spec (a b : Std.Usize) :
    layer.may_use a b = ok (decide (b.val ≤ a.val)) := by
  simp [layer.may_use]

theorem may_use_refl (a : Std.Usize) : layer.may_use a a = ok true := by
  simp [layer.may_use]

theorem may_use_trans (a b c : Std.Usize) (h1 : layer.may_use a b = ok true)
    (h2 : layer.may_use b c = ok true) : layer.may_use a c = ok true := by
  simp [layer.may_use] at *
  scalar_tac

theorem may_use_antisymm (a b : Std.Usize) (h1 : layer.may_use a b = ok true)
    (h2 : layer.may_use b a = ok true) : a = b := by
  simp [layer.may_use] at *
  scalar_tac

theorem may_use_total (a b : Std.Usize) :
    layer.may_use a b = ok true ∨ layer.may_use b a = ok true := by
  simp [layer.may_use]
  scalar_tac

/-- An edge is a finding exactly unless both layers are known and the
dependency's is not later. -/
theorem layer_finding_iff (o d : layer.Layer) :
    layer.finding o d ⦃ r => (r = true ↔
      ¬ ∃ a b, o = .Known a ∧ d = .Known b ∧ b.val ≤ a.val) ⦄ := by
  unfold layer.finding
  cases o <;> cases d <;> simp [layer.may_use]

theorem unknown_layer_is_finding (o d : layer.Layer)
    (h : o = .Unknown ∨ o = .Missing ∨ d = .Unknown ∨ d = .Missing) :
    layer.finding o d = ok true := by
  cases o <;> cases d <;> simp_all [layer.finding]

end Tauten.Kernel
