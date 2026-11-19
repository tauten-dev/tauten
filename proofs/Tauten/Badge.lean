import Tauten.Ratchet

/-!
# The badge a claim earns (`tauten_check_kernel::badge`)

Proved here, over the extracted `badge`:
* `badge_proved_iff`: `Proved` exactly when the claim was established, is
  fresh, is a Lean theorem over the extracted function, and lists no
  assumption.
* `badge_proved_under_iff`: `ProvedUnder` exactly when the same holds but
  the claim lists assumptions.
* `non_lean_never_proved`, `model_only_never_proved`,
  `stale_never_established`: bounded, tested or reviewed claims never
  show as proved; a theorem about a hand-written model never shows as
  proved about the code; a stale claim shows only `Pending` or
  `NotEstablished`.
* `badge_monotone`: making any input worse (not established, not fresh,
  a weaker method, a model instead of the code, an added assumption)
  never raises the badge.
-/

open Aeneas Aeneas.Std Result
open tauten_check_kernel

namespace Tauten.Kernel

def badgeN (e f : Bool) (m : badge.Method) (r : badge.Relation) (a : Bool) : badge.Badge :=
  if !e then .NotEstablished else if !f then .Pending else
  match m, r with
  | .Lean, .Extracted => if a then .ProvedUnder else .Proved
  | .Lean, .ModelOnly => .ModelProved
  | .Bounded, _ => .Bounded
  | .Test, _ => .Tested
  | .Review, _ => .Reviewed

def badgeRankN : badge.Badge → Nat
  | .Proved => 7 | .ProvedUnder => 6 | .ModelProved => 5 | .Bounded => 4
  | .Tested => 3 | .Reviewed => 2 | .Pending => 1 | .NotEstablished => 0

/-- Methods, strongest highest. -/
def methodRank : badge.Method → Nat
  | .Lean => 3 | .Bounded => 2 | .Test => 1 | .Review => 0

theorem badge_spec (e f : Bool) (m : badge.Method) (r : badge.Relation) (a : Bool) :
    badge.badge e f m r a = ok (badgeN e f m r a) := by
  cases e <;> cases f <;> cases m <;> cases r <;> cases a <;> rfl

theorem badge_rank_spec (b : badge.Badge) : badge.rank b = ok ⟨BitVec.ofNat 8 (badgeRankN b)⟩ := by
  cases b <;> rfl

theorem badge_proved_iff (e f : Bool) (m : badge.Method) (r : badge.Relation) (a : Bool) :
    badge.badge e f m r a = ok .Proved ↔
      e = true ∧ f = true ∧ m = .Lean ∧ r = .Extracted ∧ a = false := by
  rw [badge_spec]
  cases e <;> cases f <;> cases m <;> cases r <;> cases a <;> simp [badgeN]

theorem badge_proved_under_iff (e f : Bool) (m : badge.Method) (r : badge.Relation) (a : Bool) :
    badge.badge e f m r a = ok .ProvedUnder ↔
      e = true ∧ f = true ∧ m = .Lean ∧ r = .Extracted ∧ a = true := by
  rw [badge_spec]
  cases e <;> cases f <;> cases m <;> cases r <;> cases a <;> simp [badgeN]

theorem non_lean_never_proved (e f : Bool) (m : badge.Method) (r : badge.Relation) (a : Bool)
    (hm : m ≠ .Lean) : badgeRankN (badgeN e f m r a) ≤ 4 := by
  cases e <;> cases f <;> cases m <;> cases r <;> cases a <;> simp_all [badgeN, badgeRankN]

theorem model_only_never_proved (e f : Bool) (m : badge.Method) (a : Bool) :
    badgeN e f m .ModelOnly a ≠ .Proved ∧ badgeN e f m .ModelOnly a ≠ .ProvedUnder := by
  cases e <;> cases f <;> cases m <;> cases a <;> simp [badgeN]

theorem stale_never_established (e : Bool) (m : badge.Method) (r : badge.Relation) (a : Bool) :
    badgeRankN (badgeN e false m r a) ≤ 1 := by
  cases e <;> cases m <;> cases r <;> cases a <;> simp [badgeN, badgeRankN]

/-- Worsening any input never raises the badge. -/
theorem badge_monotone (e f : Bool) (m m' : badge.Method) (r : badge.Relation) (a : Bool) :
    badgeRankN (badgeN false f m r a) ≤ badgeRankN (badgeN e f m r a) ∧
    badgeRankN (badgeN e false m r a) ≤ badgeRankN (badgeN e f m r a) ∧
    badgeRankN (badgeN e f m r true) ≤ badgeRankN (badgeN e f m r a) ∧
    badgeRankN (badgeN e f m .ModelOnly a) ≤ badgeRankN (badgeN e f m r a) ∧
    (methodRank m' ≤ methodRank m → badgeRankN (badgeN e f m' r a) ≤ badgeRankN (badgeN e f m r a)) := by
  cases e <;> cases f <;> cases m <;> cases m' <;> cases r <;> cases a <;>
    simp [badgeN, badgeRankN, methodRank]

end Tauten.Kernel
