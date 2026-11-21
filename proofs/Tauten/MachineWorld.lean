import Tauten.Machine

/-!
# A partial extracted step against a world, through `rsys_inv_of_step`

A miniature for #60: a counter whose step can fail, a world that records
the last value written and may lose writes. The invariant "the world never
runs ahead of the machine" is proved with `rsys_inv_of_step`, without a
total model of the step. The examples below show it is not vacuous: a
success changes both state and world, a distinct input fails, and a world
whose adversary can corrupt the record reaches a configuration that breaks
the invariant.
-/

namespace Tauten.MachineWorld

open Aeneas Aeneas.Std Result Tauten.Machine

/-- Event `true` increments and writes the new value; `false` fails, as an
extracted function does on a refused input. -/
def counter : RMachine Nat Bool Nat where
  step s e := if e then ok (s + 1, s + 1) else fail .panic

/-- The world holds the last value written; the adversary may lose writes
(roll back to an older value). -/
def lossy : World Bool Nat Nat where
  deliver _ _ := True
  interpret _ c v' := v' = c
  adversary v v' := v' ≤ v

def start (x : Nat × Nat) : Prop := x = (0, 0)

/-- The world never runs ahead of the machine. -/
def Behind (x : Nat × Nat) : Prop := x.2 ≤ x.1

theorem counter_behind : ∀ x, RSysReachable counter lossy start x → Behind x := by
  apply rsys_inv_of_step counter lossy start Behind
  · intro x hx; simp [start] at hx; subst hx; simp [Behind]
  · intro s s' c v v' e _ _ hok hi
    cases e <;> simp [counter] at hok
    obtain ⟨rfl, rfl⟩ := hok
    simp [lossy] at hi; subst hi; simp [Behind]
  · intro s v v' hinv ha
    simp [lossy] at ha; simp [Behind] at *; omega

/-- A delivered success changes both the machine state and the world. -/
theorem counter_success_reachable : RSysReachable counter lossy start (1, 1) :=
  RSysReachable.step 0 1 1 0 1 true (RSysReachable.init _ rfl) trivial rfl rfl

/-- The refused input fails: it supplies no successful transition. -/
theorem counter_refuses : counter.step 0 false = fail .panic := rfl

/-- A world whose adversary can write ahead of the machine. -/
def corrupting : World Bool Nat Nat where
  deliver _ _ := True
  interpret _ c v' := v' = c
  adversary v v' := v' = v + 1

/-- Against the corrupting world the invariant fails in a reachable
configuration, so `counter_behind` depends on the adversary premise. -/
theorem corrupting_breaks_behind :
    RSysReachable counter corrupting start (0, 1) ∧ ¬ Behind (0, 1) :=
  ⟨RSysReachable.adversary 0 0 1 (RSysReachable.init _ rfl) rfl, by simp [Behind]⟩

end Tauten.MachineWorld
