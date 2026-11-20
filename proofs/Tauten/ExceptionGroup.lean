import Tauten.Control
import Tauten.Exception

/-!
# Groups, conditions and manifest tables (`tauten_check_kernel::exception`)

`tauten_check::structure` calls the extracted `decide` for every
`#[expect]`/`#[allow]`, inner or outer, plain or inside `cfg_attr`, with the
profile rules the attribute may cover: one for a lint, every rule that may
own a member of a group (`warnings`, `clippy::all`, …). It calls `loosens`
for every manifest `[lints]` entry (TAU-X003).

Proved here, over the extracted functions:
* `decide_spec`: the decision is `Forbidden` for a conditional exception in a
  kernel crate, else the worst `excuse` over the covered rules.
* `conditional_never_allowed_in_kernel`.
* `group_never_weaker`: a group's decision is at least as bad as the decision
  for any lint it may cover.
* `decide_single`: for one plain lint the decision is `excuse`.
* `x003_iff`: an entry loosens the profile exactly when it lowers the lint
  below `deny` and a covered rule blocks in the tier.

Trusted, not proved: the shell's reading of attributes and manifests, and the
table of which rules a group may cover (an over-approximation: every clippy
rule for a clippy group, every rule for `warnings`, every rustc rule for a
rustc group).
-/

open Aeneas Aeneas.Std Result
open tauten_check_kernel

namespace Tauten.Kernel

/-! ## Models -/

def excuseN (allows : Bool) (t : tiers.Tier) (level : Option severity.Level) (cw : Bool) :
    exception.Excuse :=
  if allows = false ∨ (t = .Kernel ∧ level ≠ none) then .Forbidden
  else if cw = true ∧ level = some .Deny then .CrateWide
  else .Allowed

def rankN : exception.Excuse → Nat
  | .Allowed => 0
  | .CrateWide => 1
  | .Forbidden => 2

def worseN (a b : exception.Excuse) : exception.Excuse :=
  if rankN a < rankN b then b else a

def worstN (cs : List exception.Covered) (t : tiers.Tier) (cw : Bool) : exception.Excuse :=
  cs.foldl (fun a c => worseN a (excuseN c.allows t c.level cw)) .Allowed

def weakensN : exception.Setting → Bool
  | .Allow | .Warn | .Expect => true
  | .Deny | .Forbid => false

/-! ## Specs of the extracted functions -/

theorem excuse_eq (allows : Bool) (t : tiers.Tier) (level : Option severity.Level) (cw : Bool) :
    exception.excuse allows t level cw = ok (excuseN allows t level cw) := by
  rw [exception_spec]; rfl

theorem worse_spec (a b : exception.Excuse) : exception.worse a b = ok (worseN a b) := by
  cases a <;> cases b <;> simp [exception.worse, exception.rank, worseN, rankN] <;> decide

theorem worst_spec (cs : Slice exception.Covered) (t : tiers.Tier) (cw : Bool) :
    exception.worst cs t cw ⦃ r => r = worstN cs.val t cw ⦄ := by
  unfold exception.worst exception.worst_loop
  apply WP.spec_mono (fold_loop cs
    (fun a c => worseN a (excuseN c.allows t c.level cw)) _ ?_ ?_
    .Allowed 0#usize (by simp))
  · intro r hr; rw [hr]; simp [worstN]
  · intro a i h
    have hlt : i < Slice.len cs := by scalar_tac
    simp only [exception.worst_loop.body, hlt, ite_true, get_ok cs i h, bind_ok, lift,
      excuse_eq, worse_spec]
    simp
  · intro a i h
    have hlt : ¬ i < Slice.len cs := by scalar_tac
    simp only [exception.worst_loop.body, hlt, ite_false, WP.spec_ok]

/-- A conditional exception in a kernel crate is forbidden; otherwise the
worst `excuse` over the covered rules stands. -/
theorem decide_spec (f : exception.Form) (cs : Slice exception.Covered) (t : tiers.Tier) :
    exception.decide f cs t ⦃ r =>
      r = if f.conditional = true ∧ t = .Kernel then .Forbidden else worstN cs.val t f.crate_wide ⦄ := by
  unfold exception.decide
  cases t <;> cases hc : f.conditional
  · simp only [bind_ok, Bool.false_eq_true, ite_false]
    apply WP.spec_mono (worst_spec cs _ _); intro r hr; simp [hr, hc]
  · simp only [bind_ok, ite_true, WP.spec_ok]; simp
  all_goals
    simp only [bind_ok, Bool.false_eq_true, ite_false, ite_true]
    apply WP.spec_mono (worst_spec cs _ _); intro r hr; simp [hr, hc]

/-- Proofs and checks see one configuration of a kernel crate, so an
exception that is in force only under another one is never accepted there. -/
theorem conditional_never_allowed_in_kernel (f : exception.Form) (cs : Slice exception.Covered)
    (hc : f.conditional = true) :
    exception.decide f cs .Kernel = ok .Forbidden := by
  simp [exception.decide, hc]

theorem rank_worse_left (a b : exception.Excuse) : rankN a ≤ rankN (worseN a b) := by
  unfold worseN; split <;> omega

theorem rank_worse_right (a b : exception.Excuse) : rankN b ≤ rankN (worseN a b) := by
  unfold worseN; split
  · omega
  · cases a <;> cases b <;> simp_all [rankN]

theorem foldl_worse_ge (cs : List exception.Covered) (t : tiers.Tier) (cw : Bool)
    (a : exception.Excuse) :
    rankN a ≤ rankN (cs.foldl (fun a c => worseN a (excuseN c.allows t c.level cw)) a) := by
  induction cs generalizing a with
  | nil => simp
  | cons c rest ih => exact le_trans (rank_worse_left _ _) (ih _)

theorem foldl_worse_mem (cs : List exception.Covered) (t : tiers.Tier) (cw : Bool)
    (a : exception.Excuse) (c : exception.Covered) (hc : c ∈ cs) :
    rankN (excuseN c.allows t c.level cw) ≤
      rankN (cs.foldl (fun a c => worseN a (excuseN c.allows t c.level cw)) a) := by
  induction cs generalizing a with
  | nil => simp at hc
  | cons d rest ih =>
    rcases List.mem_cons.mp hc with rfl | h
    · exact le_trans (rank_worse_right _ _) (foldl_worse_ge _ _ _ _)
    · exact ih _ h

/-- A group is never weaker than any lint it may cover: its decision is at
least as bad as the decision for each covered rule on its own. -/
theorem group_never_weaker (f : exception.Form) (cs : Slice exception.Covered) (t : tiers.Tier)
    (c : exception.Covered) (hc : c ∈ cs.val) :
    exception.decide f cs t ⦃ r => rankN (excuseN c.allows t c.level f.crate_wide) ≤ rankN r ⦄ := by
  apply WP.spec_mono (decide_spec f cs t)
  intro r hr
  rw [hr]
  split
  · cases c.allows <;> cases t <;> rcases c.level with _ | (_ | _) <;> cases f.crate_wide <;>
      simp [excuseN, rankN]
  · exact foldl_worse_mem _ _ _ _ _ hc

/-- For one unconditional lint the decision is the single-rule `excuse`. -/
theorem decide_single (cw : Bool) (c : exception.Covered) (t : tiers.Tier) (cs : Slice exception.Covered)
    (h : cs.val = [c]) :
    exception.decide ⟨cw, false⟩ cs t ⦃ r => r = excuseN c.allows t c.level cw ⦄ := by
  apply WP.spec_mono (decide_spec _ cs t)
  intro r hr
  rw [hr, h]
  simp only [Bool.false_eq_true, false_and, ite_false, worstN, List.foldl_cons, List.foldl_nil]
  unfold worseN
  split
  · rfl
  · rename_i hn
    cases e : excuseN c.allows t c.level cw <;> simp_all [rankN]

/-! ## TAU-X003 -/

theorem weakens_spec (s : exception.Setting) : exception.weakens s = ok (weakensN s) := by
  cases s <;> rfl

theorem loosens_spec (s : exception.Setting) (ls : Slice severity.Level) :
    exception.loosens s ls ⦃ r => r = (weakensN s && ls.val.any (fun l => decide (l = .Deny))) ⦄ := by
  unfold exception.loosens
  rw [weakens_spec]
  cases hw : weakensN s
  · simp
  · simp only [bind_ok, ite_true, Bool.true_and]
    unfold exception.loosens_loop
    apply WP.spec_mono (fold_loop ls (fun a l => a || decide (l = .Deny)) _ ?_ ?_ false 0#usize
      (by simp))
    · intro r hr; rw [hr, foldl_or]; simp
    · intro a i h
      have hlt : i < Slice.len ls := by scalar_tac
      simp only [exception.loosens_loop.body, hlt, ite_true, get_ok ls i h, bind_ok, lift]
      cases a
      · cases ls.val[i.val] <;> simp [severity.is_blocking]
      · simp
    · intro a i h
      have hlt : ¬ i < Slice.len ls := by scalar_tac
      simp only [exception.loosens_loop.body, hlt, ite_false, WP.spec_ok]

/-- TAU-X003 fires exactly when the entry lowers the lint below `deny` and
some covered rule blocks in the tier. -/
theorem x003_iff (s : exception.Setting) (ls : Slice severity.Level) :
    exception.loosens s ls ⦃ r => (r = true ↔ weakensN s = true ∧ severity.Level.Deny ∈ ls.val) ⦄ := by
  apply WP.spec_mono (loosens_spec s ls)
  intro r hr
  rw [hr]
  simp

end Tauten.Kernel
