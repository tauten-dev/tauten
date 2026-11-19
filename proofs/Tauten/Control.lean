import Tauten.Ratchet

/-!
# Whether a set of checks passed (`tauten_check_kernel::control`)

The shell reads each gate result (a theorem's audit, a mutant's run, a
model-checker configuration, a reject fixture) into an `Observation` of a
required `Control`; the extracted `admit` decides whether the set passed.

Proved here, over the extracted functions:
* `control_ok_spec`, `infrastructure_never_passes`: a control is satisfied
  only by a pass when it expects a pass, and only by a failure for a named
  reason when it is a negative control; a timeout, a missing tool, an empty
  run, a skipped control or a failure for another reason satisfies none.
* `admit_ok_iff`: admitted exactly when the inventory is non-empty, every
  observation is of a required control and occurs exactly once, and every
  required control was observed with the result it expects.
* `admit_infrastructure_refused`: if a required control's only observation
  is an infrastructure result, the set is refused.
* `named_iff`: a failure is named exactly when a declared reason is among
  the broken ones.

`fold_loop` is the shared lemma: a `while i < len { s = f(s, xs[i]); i += 1 }`
loop computes a left fold.

Trusted, not proved: the shell's reading of gate output into observations.
-/

open Aeneas Aeneas.Std Result
open tauten_check_kernel

namespace Tauten.Kernel

/-- A `while` loop over a slice whose step is a fold step computes the fold. -/
theorem fold_loop {α β : Type} (xs : Slice α) (f : β → α → β)
    (body : β × Std.Usize → Result (ControlFlow (β × Std.Usize) β))
    (hstep : ∀ s (i : Std.Usize) (h : i.val < xs.val.length),
      body (s, i) ⦃ r => r = ControlFlow.cont (f s xs.val[i.val], core.num.Usize.saturating_add i 1#usize) ⦄)
    (hdone : ∀ s (i : Std.Usize), xs.val.length ≤ i.val → body (s, i) ⦃ r => r = ControlFlow.done s ⦄)
    (s : β) (i : Std.Usize) (hi : i.val ≤ xs.val.length) :
    loop body (s, i) ⦃ r => r = (xs.val.drop i.val).foldl f s ⦄ := by
  apply loop.spec_decr_nat
    (fun p : β × Std.Usize => xs.val.length - p.2.val)
    (fun p : β × Std.Usize => p.2.val ≤ xs.val.length ∧
      (xs.val.drop p.2.val).foldl f p.1 = (xs.val.drop i.val).foldl f s)
  · rintro ⟨t, k⟩ ⟨hk, hinv⟩
    dsimp only at hk hinv ⊢
    by_cases hlt : k.val < xs.val.length
    · apply WP.spec_mono (hstep t k hlt)
      intro r hr
      subst hr
      dsimp only
      have hk2 := usize_sat_add_one k (by scalar_tac)
      refine ⟨⟨by omega, ?_⟩, by omega⟩
      rw [hk2, ← hinv, List.drop_eq_getElem_cons hlt, List.foldl_cons]
    · apply WP.spec_mono (hdone t k (by omega))
      intro r hr
      subst hr
      have : k.val = xs.val.length := by omega
      dsimp only
      rw [← hinv, this]
      try simp
  · exact ⟨hi, rfl⟩

theorem foldl_or {α : Type} (p : α → Bool) (l : List α) (b : Bool) :
    l.foldl (fun a x => a || p x) b = (b || l.any p) := by
  induction l generalizing b with
  | nil => simp
  | cons x t ih => simp [List.foldl_cons, ih, Bool.or_assoc]

theorem foldl_and {α : Type} (p : α → Bool) (l : List α) (b : Bool) :
    l.foldl (fun a x => a && p x) b = (b && l.all p) := by
  induction l generalizing b with
  | nil => simp
  | cons x t ih => simp [List.foldl_cons, ih, Bool.and_assoc]

/-! ## Models -/

def control_okN : control.Expect → control.Seen → Bool
  | .Pass, .Pass => true
  | .Fail, .FailNamed => true
  | _, _ => false

def memberN (ids : List Std.Usize) (id : Std.Usize) : Bool := ids.any (fun x => decide (x = id))

def namedN (declared broken : List Std.Usize) : Bool := declared.any (fun x => memberN broken x)

def obsCountN (obs : List control.Observation) (id : Std.Usize) : Nat :=
  obs.countP (fun o => decide (o.id = id))

def requiredN (req : List control.Control) (id : Std.Usize) : Bool :=
  req.any (fun c => decide (c.id = id))

def metN (obs : List control.Observation) (id : Std.Usize) (e : control.Expect) : Bool :=
  obs.any (fun o => decide (o.id = id) && control_okN e o.seen)

def observationsN (req : List control.Control) (obs : List control.Observation) : Bool :=
  obs.all (fun o => decide (obsCountN obs o.id = 1) && requiredN req o.id)

def controlsN (req : List control.Control) (obs : List control.Observation) : Bool :=
  req.all (fun c => metN obs c.id c.expect)

/-! ## The extracted functions compute the models -/

theorem control_ok_spec (e : control.Expect) (s : control.Seen) :
    control.control_ok e s = ok (control_okN e s) := by
  cases e <;> cases s <;> rfl

/-- Slice access at an in-bounds index, for the loop bodies. -/
theorem get_ok {α : Type} (xs : Slice α) (i : Std.Usize) (h : i.val < xs.val.length) :
    core.slice.Slice.get (core.slice.index.SliceIndexUsizeSlice α) xs i = ok (some xs.val[i.val]) := by
  simp only [core.slice.Slice.get, core.slice.index.SliceIndexUsizeSlice,
    core.slice.index.Usize.get, Slice.getElem?_Usize_eq, List.getElem?_eq_getElem h]

theorem member_spec (ids : Slice Std.Usize) (id : Std.Usize) :
    control.member ids id ⦃ r => r = memberN ids.val id ⦄ := by
  unfold control.member control.member_loop
  apply WP.spec_mono (fold_loop ids (fun a x => a || decide (x = id)) _ ?_ ?_ false 0#usize (by simp))
  · intro r hr; rw [hr, foldl_or]; simp [memberN]
  · intro s i h
    have hlt : i < Slice.len ids := by scalar_tac
    simp only [control.member_loop.body, hlt, ite_true, get_ok ids i h, bind_ok, lift]
    cases s <;> simp
  · intro s i h
    have hlt : ¬ i < Slice.len ids := by scalar_tac
    simp only [control.member_loop.body, hlt, ite_false, WP.spec_ok]

theorem named_spec (declared broken : Slice Std.Usize) :
    control.named declared broken ⦃ r => r = namedN declared.val broken.val ⦄ := by
  unfold control.named control.named_loop
  apply WP.spec_mono
    (fold_loop declared (fun a x => a || memberN broken.val x) _ ?_ ?_ false 0#usize (by simp))
  · intro r hr; rw [hr, foldl_or]; simp [namedN]
  · intro s i h
    have hlt : i < Slice.len declared := by scalar_tac
    simp only [control.named_loop.body, hlt, ite_true, get_ok declared i h, bind_ok, lift]
    cases s
    · simp only [Bool.false_eq_true, ite_false, Bool.false_or]
      apply WP.spec_bind (member_spec broken _)
      intro r hr
      simp [hr]
    · simp
  · intro s i h
    have hlt : ¬ i < Slice.len declared := by scalar_tac
    simp only [control.named_loop.body, hlt, ite_false, WP.spec_ok]

/-- The counting fold, while it cannot saturate. -/
theorem count_fold (id : Std.Usize) (l : List control.Observation) (c : Std.Usize)
    (h : c.val + l.length ≤ Usize.max) :
    (l.foldl (fun c o => if o.id = id then core.num.Usize.saturating_add c 1#usize else c) c).val =
      c.val + l.countP (fun o => decide (o.id = id)) := by
  induction l generalizing c with
  | nil => simp
  | cons x t ih =>
    simp only [List.foldl_cons, List.countP_cons, List.length_cons] at h ⊢
    by_cases hx : x.id = id
    · simp only [hx, ite_true, decide_true]
      have h1 := usize_sat_add_one c (by omega)
      rw [ih _ (by omega), h1]
      omega
    · simp only [hx, ite_false, decide_false]
      rw [ih _ (by omega)]
      simp

theorem count_id_spec (obs : Slice control.Observation) (id : Std.Usize) :
    control.count_id obs id ⦃ r => r.val = obsCountN obs.val id ⦄ := by
  unfold control.count_id control.count_id_loop
  apply WP.spec_mono (fold_loop obs
    (fun c o => if o.id = id then core.num.Usize.saturating_add c 1#usize else c) _ ?_ ?_
    0#usize 0#usize (by simp))
  · intro r hr
    rw [hr, count_fold id _ _ (by simpa using obs.property)]
    try simp [obsCountN]
  · intro s i h
    have hlt : i < Slice.len obs := by scalar_tac
    simp only [control.count_id_loop.body, hlt, ite_true, get_ok obs i h, bind_ok, lift]
    split <;> simp_all
  · intro s i h
    have hlt : ¬ i < Slice.len obs := by scalar_tac
    simp only [control.count_id_loop.body, hlt, ite_false, WP.spec_ok]

theorem required_id_spec (req : Slice control.Control) (id : Std.Usize) :
    control.required_id req id ⦃ r => r = requiredN req.val id ⦄ := by
  unfold control.required_id control.required_id_loop
  apply WP.spec_mono (fold_loop req (fun a c => a || decide (c.id = id)) _ ?_ ?_ false 0#usize (by simp))
  · intro r hr; rw [hr, foldl_or]; simp [requiredN]
  · intro s i h
    have hlt : i < Slice.len req := by scalar_tac
    simp only [control.required_id_loop.body, hlt, ite_true, get_ok req i h, bind_ok, lift]
    cases s <;> simp
  · intro s i h
    have hlt : ¬ i < Slice.len req := by scalar_tac
    simp only [control.required_id_loop.body, hlt, ite_false, WP.spec_ok]

theorem met_spec (obs : Slice control.Observation) (id : Std.Usize) (e : control.Expect) :
    control.met obs id e ⦃ r => r = metN obs.val id e ⦄ := by
  unfold control.met control.met_loop
  apply WP.spec_mono (fold_loop obs (fun a o => a || (decide (o.id = id) && control_okN e o.seen))
    _ ?_ ?_ false 0#usize (by simp))
  · intro r hr; rw [hr, foldl_or]; simp [metN]
  · intro s i h
    have hlt : i < Slice.len obs := by scalar_tac
    simp only [control.met_loop.body, hlt, ite_true, get_ok obs i h, bind_ok, lift]
    cases s
    · simp only [Bool.false_eq_true, ite_false]
      split <;> rename_i hid <;> simp [hid, control_ok_spec]
    · simp
  · intro s i h
    have hlt : ¬ i < Slice.len obs := by scalar_tac
    simp only [control.met_loop.body, hlt, ite_false, WP.spec_ok]

theorem once_and_required (req : Slice control.Control) (obs : Slice control.Observation)
    (x : Std.Usize) :
    (do let n ← control.count_id obs x
        if n = 1#usize then control.required_id req x else ok false) ⦃ b =>
      b = (decide (obsCountN obs.val x = 1) && requiredN req.val x) ⦄ := by
  apply WP.spec_bind (count_id_spec obs x)
  intro n hn
  by_cases h1 : n = 1#usize
  · have hc : obsCountN obs.val x = 1 := by rw [← hn, h1]; rfl
    rw [if_pos h1]
    apply WP.spec_mono (required_id_spec req x)
    intro b hb
    simp [hb, hc]
  · have hc : obsCountN obs.val x ≠ 1 := by
      intro hc; apply h1; scalar_tac
    rw [if_neg h1]
    simp [WP.spec_ok, hc]

theorem observations_ok_spec (req : Slice control.Control) (obs : Slice control.Observation) :
    control.observations_ok req obs ⦃ r => r = observationsN req.val obs.val ⦄ := by
  unfold control.observations_ok control.observations_ok_loop
  apply WP.spec_mono (fold_loop obs
    (fun a o => a && (decide (obsCountN obs.val o.id = 1) && requiredN req.val o.id)) _ ?_ ?_
    true 0#usize (by simp))
  · intro r hr; rw [hr, foldl_and]; simp [observationsN]
  · intro s i h
    have hlt : i < Slice.len obs := by scalar_tac
    simp only [control.observations_ok_loop.body, hlt, ite_true, get_ok obs i h, bind_ok, lift]
    cases s
    · simp
    · simp only [ite_true, Bool.true_and]
      apply WP.spec_bind (once_and_required req obs _)
      intro b hb
      simp [hb]
  · intro s i h
    have hlt : ¬ i < Slice.len obs := by scalar_tac
    simp only [control.observations_ok_loop.body, hlt, ite_false, WP.spec_ok]

theorem controls_ok_spec (req : Slice control.Control) (obs : Slice control.Observation) :
    control.controls_ok req obs ⦃ r => r = controlsN req.val obs.val ⦄ := by
  unfold control.controls_ok control.controls_ok_loop
  apply WP.spec_mono (fold_loop req (fun a c => a && metN obs.val c.id c.expect) _ ?_ ?_
    true 0#usize (by simp))
  · intro r hr; rw [hr, foldl_and]; simp [controlsN]
  · intro s i h
    have hlt : i < Slice.len req := by scalar_tac
    simp only [control.controls_ok_loop.body, hlt, ite_true, get_ok req i h, bind_ok, lift]
    cases s
    · simp
    · simp only [ite_true, Bool.true_and]
      apply WP.spec_bind (met_spec obs _ _)
      intro b hb
      simp [hb]
  · intro s i h
    have hlt : ¬ i < Slice.len req := by scalar_tac
    simp only [control.controls_ok_loop.body, hlt, ite_false, WP.spec_ok]

/-- What `admit` computes. -/
def admitN (req : List control.Control) (obs : List control.Observation) : Bool :=
  !req.isEmpty && observationsN req obs && controlsN req obs

theorem admit_spec (req : Slice control.Control) (obs : Slice control.Observation) :
    control.admit req obs ⦃ r => r = admitN req.val obs.val ⦄ := by
  unfold control.admit
  simp only [core.slice.Slice.is_empty, bind_ok]
  by_cases he : req.length = 0
  · have : req.val = [] := by simpa [Slice.length] using he
    simp [he, admitN, this]
  · have : req.val ≠ [] := by intro h0; apply he; simp [Slice.length, h0]
    simp only [he, decide_false, Bool.false_eq_true, ite_false]
    apply WP.spec_bind (observations_ok_spec req obs)
    intro b hb
    cases b
    · simp [admitN, ← hb, this]
    · simp only [ite_true]
      apply WP.spec_mono (controls_ok_spec req obs)
      intro r hr
      simp [admitN, ← hb, hr, this]

/-- Admitted exactly when the inventory is non-empty, every observation is
of a required control and occurs once, and every required control was
observed with the result it expects. -/
theorem admit_ok_iff (req : Slice control.Control) (obs : Slice control.Observation) :
    control.admit req obs ⦃ r => (r = true ↔
      req.val ≠ [] ∧
      (∀ o ∈ obs.val, obsCountN obs.val o.id = 1 ∧ ∃ c ∈ req.val, c.id = o.id) ∧
      (∀ c ∈ req.val, ∃ o ∈ obs.val, o.id = c.id ∧ control_okN c.expect o.seen = true)) ⦄ := by
  apply WP.spec_mono (admit_spec req obs)
  intro r hr
  subst hr
  simp [admitN, observationsN, controlsN, metN, requiredN, List.all_eq_true, List.any_eq_true,
    List.isEmpty_iff, and_assoc]

/-- Infrastructure results satisfy no control, and the two kinds of
control are not interchangeable. -/
theorem infrastructure_never_passes (e : control.Expect) :
    control_okN e .FailOther = false ∧ control_okN e .Timeout = false ∧
    control_okN e .ToolMissing = false ∧ control_okN e .Empty = false ∧
    control_okN e .NotRun = false ∧ control_okN .Pass .FailNamed = false ∧
    control_okN .Fail .Pass = false := by
  cases e <;> simp [control_okN]

/-- Two observations of one control in an admitted set are the same. -/
theorem unique_of_count (obs : List control.Observation) (o o' : control.Observation)
    (ho : o ∈ obs) (ho' : o' ∈ obs) (hid : o'.id = o.id) (hc : obsCountN obs o.id = 1) :
    o' = o := by
  unfold obsCountN at hc
  rw [List.countP_eq_length_filter, List.length_eq_one_iff] at hc
  obtain ⟨x, hx⟩ := hc
  have h1 : o ∈ obs.filter (fun p => decide (p.id = o.id)) := List.mem_filter.mpr ⟨ho, by simp⟩
  have h2 : o' ∈ obs.filter (fun p => decide (p.id = o.id)) := List.mem_filter.mpr ⟨ho', by simp [hid]⟩
  rw [hx] at h1 h2
  simp at h1 h2
  rw [h1, h2]

/-- If a required control's observation is an infrastructure result (a
timeout, a missing tool, an empty or skipped run, or an unnamed failure),
the set is refused, whatever else was observed. -/
theorem admit_infrastructure_refused (req : Slice control.Control) (obs : Slice control.Observation)
    (c : control.Control) (o : control.Observation) (hc : c ∈ req.val) (ho : o ∈ obs.val)
    (hid : o.id = c.id) (hinfra : o.seen = .FailOther ∨ o.seen = .Timeout ∨
      o.seen = .ToolMissing ∨ o.seen = .Empty ∨ o.seen = .NotRun) :
    control.admit req obs ⦃ r => r = false ⦄ := by
  apply WP.spec_mono (admit_ok_iff req obs)
  intro r hr
  cases r with
  | false => rfl
  | true =>
    obtain ⟨_, hobs, hctl⟩ := hr.mp rfl
    obtain ⟨o', ho', hid', hok⟩ := hctl c hc
    have hcount := (hobs o ho).1
    have : o' = o := unique_of_count obs.val o o' ho ho' (by rw [hid', hid]) hcount
    subst this
    rcases hinfra with h | h | h | h | h <;> rw [h] at hok <;> cases c.expect <;> simp_all [control_okN]

/-- A failure is named exactly when a declared reason is among the broken. -/
theorem named_iff (declared broken : Slice Std.Usize) :
    control.named declared broken ⦃ r => (r = true ↔ ∃ x ∈ declared.val, x ∈ broken.val) ⦄ := by
  apply WP.spec_mono (named_spec declared broken)
  intro r hr
  subst hr
  simp [namedN, memberN]

/-- Non-vacuity: a theorem that passed and a mutant that failed for its
named reason are admitted; the same mutant timing out is not. -/
theorem admit_examples :
    admitN [⟨1#usize, .Pass⟩, ⟨2#usize, .Fail⟩] [⟨2#usize, .FailNamed⟩, ⟨1#usize, .Pass⟩] = true ∧
    admitN [⟨1#usize, .Pass⟩, ⟨2#usize, .Fail⟩] [⟨2#usize, .Timeout⟩, ⟨1#usize, .Pass⟩] = false ∧
    admitN [] [] = false := by
  refine ⟨?_, ?_, rfl⟩ <;>
    simp [admitN, observationsN, controlsN, metN, requiredN, obsCountN, control_okN]

end Tauten.Kernel
