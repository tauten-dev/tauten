import Aeneas

/-!
# Step machines: reachability, invariants, worlds, discharge

A protocol written as a pure step function `step : S → E → S × C` (state,
event, command) is proved once here, generically:

* `Reachable` and `inv_of_step`: an invariant that holds initially and is
  kept by every step holds in every reachable state.
* `run` / `inv_run`: the same along a finite trace of events, with the
  commands it emits.
* `RMachine`, `RReachable`, `rinv_of_step`: the same for a step function as
  Aeneas extracts it, returning `Result (S × C)`; the per-step obligation is
  a Hoare triple over the extracted function.
* `World` and `SysReachable` (`RSysReachable` for an extracted step): the
  machine run against a world that interprets its commands, delivers its
  events, and may take adversary steps (a crash, a delay) at any time;
  `sys_inv_of_step` is the induction for the pair, `rsys_inv_of_step` the
  same for an extracted step that may fail, and `rsys_of_model` transfers
  results from a total pure model.
* `Discharged` and `discharged_of_inv`: in every reachable terminal
  configuration, nothing the machine acquired is still held.

A project states its invariant and proves one lemma per step; everything
else is these theorems. This file is shipped identically in the proofs
template (`Proofs/TautenMachine.lean`).
-/

namespace Tauten.Machine

open Aeneas Aeneas.Std Result

/-- A pure step machine. -/
structure Machine (S E C : Type) where
  step : S → E → S × C

variable {S E C W : Type}

/-- The states reachable from an initial state by any sequence of events. -/
inductive Reachable (m : Machine S E C) (init : S → Prop) : S → Prop
  | init (s : S) : init s → Reachable m init s
  | step (s : S) (e : E) : Reachable m init s → Reachable m init (m.step s e).1

/-- An invariant that holds initially and is kept by every step holds in
every reachable state. -/
theorem inv_of_step (m : Machine S E C) (init : S → Prop) (Inv : S → Prop)
    (h0 : ∀ s, init s → Inv s) (hstep : ∀ s e, Inv s → Inv (m.step s e).1) :
    ∀ s, Reachable m init s → Inv s := by
  intro s h
  induction h with
  | init s hs => exact h0 s hs
  | step s e _ ih => exact hstep s e ih

/-- Run a trace of events; the final state and the commands, in order. -/
def run (m : Machine S E C) : S → List E → S × List C
  | s, [] => (s, [])
  | s, e :: es =>
    let (s', c) := m.step s e
    let (s'', cs) := run m s' es
    (s'', c :: cs)

theorem run_nil (m : Machine S E C) (s : S) : run m s [] = (s, []) := rfl

theorem run_cons (m : Machine S E C) (s : S) (e : E) (es : List E) :
    run m s (e :: es) = ((run m (m.step s e).1 es).1, (m.step s e).2 :: (run m (m.step s e).1 es).2) := rfl

/-- One command per event. -/
theorem run_length (m : Machine S E C) (s : S) (es : List E) :
    (run m s es).2.length = es.length := by
  induction es generalizing s with
  | nil => rfl
  | cons e es ih => simp [run_cons, ih]

/-- Running a trace from a reachable state ends in a reachable state. -/
theorem run_reachable (m : Machine S E C) (init : S → Prop) (s : S) (es : List E)
    (h : Reachable m init s) : Reachable m init (run m s es).1 := by
  induction es generalizing s with
  | nil => exact h
  | cons e es ih => rw [run_cons]; exact ih _ (Reachable.step s e h)

/-- An inductive invariant holds after any trace. -/
theorem inv_run (m : Machine S E C) (Inv : S → Prop)
    (hstep : ∀ s e, Inv s → Inv (m.step s e).1) (s : S) (es : List E) (h : Inv s) :
    Inv (run m s es).1 := by
  induction es generalizing s with
  | nil => exact h
  | cons e es ih => rw [run_cons]; exact ih _ (hstep s e h)

/-! ## The extracted form: a step that returns a `Result` -/

/-- A step function as Aeneas extracts it. -/
structure RMachine (S E C : Type) where
  step : S → E → Result (S × C)

/-- The states reachable through successful steps. -/
inductive RReachable (m : RMachine S E C) (init : S → Prop) : S → Prop
  | init (s : S) : init s → RReachable m init s
  | step (s : S) (e : E) (s' : S) (c : C) :
      RReachable m init s → m.step s e = ok (s', c) → RReachable m init s'

/-- The induction for extracted steps: the per-step obligation is a Hoare
triple over the extracted function. -/
theorem rinv_of_step (m : RMachine S E C) (init : S → Prop) (Inv : S → Prop)
    (h0 : ∀ s, init s → Inv s) (hstep : ∀ s e, Inv s → m.step s e ⦃ r => Inv r.1 ⦄) :
    ∀ s, RReachable m init s → Inv s := by
  intro s h
  induction h with
  | init s hs => exact h0 s hs
  | step s e s' c _ hok ih =>
    have := hstep s e ih
    rw [hok] at this
    simpa [WP.spec_ok] using this

/-- An extracted step that never fails and agrees with a pure model gives
the pure model's reachable states. -/
theorem rreachable_of_model (m : RMachine S E C) (p : Machine S E C)
    (hmodel : ∀ s e, m.step s e = ok (p.step s e)) (init : S → Prop) :
    ∀ s, RReachable m init s → Reachable p init s := by
  intro s h
  induction h with
  | init s hs => exact Reachable.init s hs
  | step s e s' c _ hok ih =>
    rw [hmodel] at hok
    have heq : p.step s e = (s', c) := by simpa using hok
    have := Reachable.step s e ih
    rwa [heq] at this

/-! ## A machine against a world -/

/-- The world a machine runs in: it delivers events, interprets commands,
and may take adversary steps (a crash, a lost message) at any time. -/
structure World (E C W : Type) where
  deliver : W → E → Prop
  interpret : W → C → W → Prop
  adversary : W → W → Prop

/-- Reachable (machine state, world) configurations: the machine takes a
delivered event and the world interprets the command, or the world moves
on its own. -/
inductive SysReachable (m : Machine S E C) (w : World E C W) (init : S × W → Prop) :
    S × W → Prop
  | init (x : S × W) : init x → SysReachable m w init x
  | step (s : S) (v v' : W) (e : E) :
      SysReachable m w init (s, v) → w.deliver v e →
      w.interpret v (m.step s e).2 v' → SysReachable m w init ((m.step s e).1, v')
  | adversary (s : S) (v v' : W) :
      SysReachable m w init (s, v) → w.adversary v v' → SysReachable m w init (s, v')

/-- The induction for a machine and its world: the invariant must hold
initially, survive every delivered step with its command's effect, and
survive every adversary step. -/
theorem sys_inv_of_step (m : Machine S E C) (w : World E C W) (init : S × W → Prop)
    (Inv : S × W → Prop) (h0 : ∀ x, init x → Inv x)
    (hstep : ∀ s v v' e, Inv (s, v) → w.deliver v e →
      w.interpret v (m.step s e).2 v' → Inv ((m.step s e).1, v'))
    (hadv : ∀ s v v', Inv (s, v) → w.adversary v v' → Inv (s, v')) :
    ∀ x, SysReachable m w init x → Inv x := by
  intro x h
  induction h with
  | init x hx => exact h0 x hx
  | step s v v' e _ hd hi ih => exact hstep s v v' e ih hd hi
  | adversary s v v' _ ha ih => exact hadv s v v' ih ha

/-- Reachable configurations of an extracted step against a world: a
step counts only when the extracted function succeeds. -/
inductive RSysReachable (m : RMachine S E C) (w : World E C W) (init : S × W → Prop) :
    S × W → Prop
  | init (x : S × W) : init x → RSysReachable m w init x
  | step (s s' : S) (c : C) (v v' : W) (e : E) :
      RSysReachable m w init (s, v) → w.deliver v e → m.step s e = ok (s', c) →
      w.interpret v c v' → RSysReachable m w init (s', v')
  | adversary (s : S) (v v' : W) :
      RSysReachable m w init (s, v) → w.adversary v v' → RSysReachable m w init (s, v')

/-- The induction for an extracted step and its world: the invariant must
hold initially, survive every delivered step that the extracted function
completes (`ok`) with its command's effect, and survive every adversary
step. A failing step is not a transition, so no total model is needed;
effects a host performs before failing belong in the world. -/
theorem rsys_inv_of_step (m : RMachine S E C) (w : World E C W)
    (init Inv : S × W → Prop) (h0 : ∀ x, init x → Inv x)
    (hstep : ∀ s s' c v v' e, Inv (s, v) → w.deliver v e →
      m.step s e = ok (s', c) → w.interpret v c v' → Inv (s', v'))
    (hadv : ∀ s v v', Inv (s, v) → w.adversary v v' → Inv (s, v')) :
    ∀ x, RSysReachable m w init x → Inv x := by
  intro x h
  induction h with
  | init x hx => exact h0 x hx
  | step s s' c v v' e _ hd hok hi ih => exact hstep s s' c v v' e ih hd hok hi
  | adversary s v v' _ ha ih => exact hadv s v v' ih ha

/-- An extracted step that never fails and agrees with a pure model
reaches only the pure model's configurations, so everything proved about
the model holds for the extracted code. -/
theorem rsys_of_model (m : RMachine S E C) (p : Machine S E C)
    (hmodel : ∀ s e, m.step s e = ok (p.step s e)) (w : World E C W) (init : S × W → Prop) :
    ∀ x, RSysReachable m w init x → SysReachable p w init x := by
  intro x h
  induction h with
  | init x hx => exact SysReachable.init x hx
  | step s s' c v v' e _ hd hok hi ih =>
    rw [hmodel] at hok
    have heq : p.step s e = (s', c) := by simpa using hok
    have := SysReachable.step s v v' e ih hd (by rw [heq]; exact hi)
    rwa [heq] at this
  | adversary s v v' _ ha ih => exact SysReachable.adversary s v v' ih ha

/-- In every reachable terminal configuration nothing acquired is held. -/
def Discharged (m : Machine S E C) (w : World E C W) (init : S × W → Prop)
    (terminal : S → Prop) (held : S × W → Prop) : Prop :=
  ∀ s v, SysReachable m w init (s, v) → terminal s → ¬ held (s, v)

/-- Discharge from an inductive invariant that implies it. -/
theorem discharged_of_inv (m : Machine S E C) (w : World E C W) (init : S × W → Prop)
    (terminal : S → Prop) (held : S × W → Prop) (Inv : S × W → Prop)
    (h0 : ∀ x, init x → Inv x)
    (hstep : ∀ s v v' e, Inv (s, v) → w.deliver v e →
      w.interpret v (m.step s e).2 v' → Inv ((m.step s e).1, v'))
    (hadv : ∀ s v v', Inv (s, v) → w.adversary v v' → Inv (s, v'))
    (himp : ∀ s v, Inv (s, v) → terminal s → ¬ held (s, v)) :
    Discharged m w init terminal held := by
  intro s v h ht
  exact himp s v (sys_inv_of_step m w init Inv h0 hstep hadv (s, v) h) ht

end Tauten.Machine
