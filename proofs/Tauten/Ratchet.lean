import Tauten.RatchetModel

/-!
# The baseline ratchet (`tauten_check_kernel::ratchet`)

`tauten_check::baseline` interns finding keys to indices (key order) and
calls the extracted functions for every ratchet decision: `apply` (which
findings the baseline covers, what is left per key), `stale` (TAU-R001),
`count_errors` (what `cargo tauten baseline` records) and `increases` (the
refusal to grow a baseline).

Structure: one refinement lemma per extracted function (`apply_spec`,
`stale_spec`, `count_errors_spec`, `increases_spec`, each by a loop
invariant) relates it to the list functions of `Tauten/RatchetModel.lean`;
the claims (section "Claims") are stated over the extracted functions and
proved from those lemmas. `apply_conserves` has its own loop invariant and
does not depend on what `take` decides. A mutant of one function therefore
breaks every claim proved through that function's lemma
(`proofs/mutants.py` counts a claim as broken when its proof depends on a
failed lemma).

Trusted, not proved (`docs/proofs/README.md`): interning (one index per
distinct key, in key order), the shell's use of the flags (report the
uncovered findings in order) and its rendering of one TAU-R001 per key
`stale` returns.
-/

open Aeneas Aeneas.Std Result
open tauten_check_kernel

set_option linter.unusedSimpArgs false

namespace Tauten.Kernel

theorem uscalar_max_lt (ty : UScalarTy) : UScalar.max ty < 2 ^ ty.numBits := by
  rw [UScalar.max_def]
  have := Nat.two_pow_pos ty.numBits
  omega

theorem uscalar_sat_add_val {ty : UScalarTy} (x y : UScalar ty) :
    (UScalar.saturating_add x y).val = min (UScalar.max ty) (x.val + y.val) := by
  show (BitVec.ofNat _ (min (UScalar.max ty) (x.val + y.val))).toNat = _
  rw [BitVec.toNat_ofNat]
  apply Nat.mod_eq_of_lt
  have := uscalar_max_lt ty
  omega

theorem uscalar_sat_sub_val {ty : UScalarTy} (x y : UScalar ty) :
    (UScalar.saturating_sub x y).val = x.val - y.val := by
  show (BitVec.ofNat _ (max 0 (x.val - y.val))).toNat = _
  rw [BitVec.toNat_ofNat]
  have hx : x.val < 2 ^ ty.numBits := x.bv.isLt
  rw [Nat.mod_eq_of_lt (by omega)]
  omega

theorem usize_sat_add_one (i : Usize) (h : i.val < Usize.max) :
    (core.num.Usize.saturating_add i 1#usize).val = i.val + 1 := by
  have := uscalar_sat_add_val (ty := .Usize) i 1#usize
  simp only [UScalar.max_USize_eq] at this
  simp only [core.num.Usize.saturating_add, this]
  simp; omega

theorem u32_sat_sub_one (x : U32) :
    (core.num.U32.saturating_sub x 1#u32).val = x.val - 1 := by
  simp only [core.num.U32.saturating_sub, uscalar_sat_sub_val]
  simp

theorem u32_sat_add_one (x : U32) :
    (core.num.U32.saturating_add x 1#u32).val = min (x.val + 1) U32.max := by
  simp only [core.num.U32.saturating_add, uscalar_sat_add_val]
  simp [Nat.min_comm]

theorem list_vals_set_opt_some (l : List U32) (n : Nat) (x : U32) :
    vals (l.set_opt n (some x)) = (vals l).set n x.val := by
  unfold vals
  induction l generalizing n with
  | nil => simp
  | cons hd tl ih =>
    cases n with
    | zero => simp
    | succ n => simp [ih]

theorem list_set_opt_none (l : List U32) (n : Nat) : l.set_opt n none = l := by
  induction l generalizing n with
  | nil => simp
  | cons hd tl ih =>
    cases n with
    | zero => simp
    | succ n => simp [ih]

theorem list_set_opt_same (l : List U32) (n : Nat) (x : U32) (h : l[n]? = some x) :
    l.set_opt n (some x) = l := by
  induction l generalizing n with
  | nil => simp at h
  | cons hd tl ih =>
    cases n with
    | zero => simp at h; simp [h]
    | succ n => simp at h; simp [ih n h]

theorem vals_set_opt_some (s : Slice U32) (k : Usize) (x : U32) :
    vals (s.set_opt k (some x)).val = (vals s.val).set k.val x.val := by
  rw [Slice.set_opt_val_eq, list_vals_set_opt_some]

theorem vals_set_opt_none (s : Slice U32) (k : Usize) : s.set_opt k none = s := by
  apply Slice.ext
  rw [Slice.set_opt_val_eq, list_set_opt_none]

theorem vals_getD (l : List U32) (k : Nat) : (vals l).getD k 0 = (l[k]?.map (·.val)).getD 0 := by
  simp [vals, List.getD_eq_getElem?_getD]

theorem count_at_spec (s : Slice U32) (k : Usize) :
    ratchet.count_at s k ⦃ c => c.val = (vals s.val).getD k.val 0 ⦄ := by
  unfold ratchet.count_at
  simp only [core.slice.Slice.get, core.slice.index.SliceIndexUsizeSlice,
    core.slice.index.Usize.get, bind_tc_ok, vals_getD, Slice.getElem?_Usize_eq]
  cases h : s.val[k.val]? <;> simp

theorem take_spec (s : Slice U32) (f : ratchet.Finding) :
    ratchet.take s f ⦃ r => r.1 = (stepN (vals s.val) f).1 ∧
      vals r.2.val = (stepN (vals s.val) f).2 ⦄ := by
  unfold ratchet.take
  cases he : f.error
  · simp [stepN, he]
  · simp only [↓reduceIte, core.slice.Slice.get_mut, core.slice.index.Usize.get_mut,
      Slice.getElem?_Usize_eq, bind_tc_ok]
    cases h : s.val[f.key.val]? with
    | none =>
      have hd : (vals s.val)[f.key.val]?.getD 0 = 0 := by simp [vals, h]
      simp [list_set_opt_none, stepN, he, hd]
    | some slot =>
      have hd : (vals s.val)[f.key.val]?.getD 0 = slot.val := by simp [vals, h]
      by_cases hp : slot > 0#u32
      · have hp' : 0 < slot.val := by scalar_tac
        simp [hp, lift, stepN, he, hd, hp', list_vals_set_opt_some, u32_sat_sub_one]
      · have hp' : ¬ 0 < slot.val := by scalar_tac
        have hs := list_set_opt_same s.val f.key.val slot h
        simp [hp, stepN, he, hd, hp', hs]

theorem take_at_spec (s : Slice U32) (fs : Slice ratchet.Finding) (i : Usize)
    (hi : i.val < fs.val.length) :
    ratchet.take_at s fs i ⦃ r => r.1 = (stepN (vals s.val) fs.val[i.val]).1 ∧
      vals r.2.val = (stepN (vals s.val) fs.val[i.val]).2 ⦄ := by
  unfold ratchet.take_at
  simp only [core.slice.Slice.get, core.slice.index.Usize.get, Slice.getElem?_Usize_eq,
    bind_tc_ok, bind_ok, List.getElem?_eq_getElem hi]
  exact take_spec s _

theorem apply_loop_spec (fs : Slice ratchet.Finding) (left : alloc.vec.Vec U32)
    (covered : alloc.vec.Vec Bool) (i : Usize) (target : List Bool × List Nat)
    (hi : i.val ≤ fs.val.length) (hcov : covered.val.length = i.val)
    (h1 : covered.val ++ (applyN (vals left.val) (fs.val.drop i.val)).1 = target.1)
    (h2 : (applyN (vals left.val) (fs.val.drop i.val)).2 = target.2) :
    ratchet.apply_loop fs left covered i ⦃ r => r.1.val = target.1 ∧ vals r.2.val = target.2 ⦄ := by
  unfold ratchet.apply_loop
  apply loop.spec_decr_nat
    (fun p : alloc.vec.Vec U32 × alloc.vec.Vec Bool × Usize => fs.val.length - p.2.2.val)
    (fun p : alloc.vec.Vec U32 × alloc.vec.Vec Bool × Usize =>
      p.2.2.val ≤ fs.val.length ∧ p.2.1.val.length = p.2.2.val ∧
      p.2.1.val ++ (applyN (vals p.1.val) (fs.val.drop p.2.2.val)).1 = target.1 ∧
      (applyN (vals p.1.val) (fs.val.drop p.2.2.val)).2 = target.2)
  · rintro ⟨left, covered, i⟩ ⟨hi, hcov, h1, h2⟩
    unfold ratchet.apply_loop.body
    dsimp only at hi hcov h1 h2 ⊢
    split
    · rename_i hlt
      have hlt' : i.val < fs.val.length := by scalar_tac
      simp only [lift, alloc.vec.Vec.deref_mut, bind_tc_ok, bind_ok]
      apply WP.spec_bind (take_at_spec _ fs i hlt')
      rintro ⟨hit, s1⟩ ⟨hhit, hs1⟩
      have hlen : covered.val.length < Usize.max := by scalar_tac
      apply WP.spec_bind (alloc.vec.Vec.push_spec _ _ hlen)
      intro covered1 hcov1
      have hi2 := usize_sat_add_one i (by scalar_tac)
      simp only [lift, bind_tc_ok, bind_ok, WP.spec_ok]
      rw [List.drop_eq_getElem_cons hlt'] at h1 h2
      simp only [applyN] at h1 h2
      dsimp only at hhit hs1
      have hl : (left.slice : Slice U32).val = left.val := rfl
      have hs1' : ({ slice := s1 } : alloc.vec.Vec U32).val = s1.val := rfl
      refine ⟨⟨by omega, by simp [hcov1, hcov, hi2], ?_, ?_⟩, by omega⟩
      · rw [hcov1, hi2, ← h1, hhit, hs1', hs1, hl]; simp
      · rw [hi2, ← h2, hs1', hs1, hl]
    · rename_i hge
      try simp only [WP.spec_ok]
      have : i.val = fs.val.length := by scalar_tac
      rw [this, List.drop_length] at h1 h2
      simp only [applyN, List.append_nil] at h1 h2
      exact ⟨h1, h2⟩
  · exact ⟨hi, hcov, h1, h2⟩

theorem apply_spec (acc : alloc.vec.Vec U32) (fs : Slice ratchet.Finding) :
    ratchet.apply acc fs ⦃ r => r.1.val = (applyN (vals acc.val) fs.val).1 ∧
      vals r.2.val = (applyN (vals acc.val) fs.val).2 ⦄ := by
  unfold ratchet.apply
  apply apply_loop_spec <;> simp

theorem stale_loop_spec (s : Slice U32) (keys : alloc.vec.Vec Usize) (k : Usize)
    (hk : k.val ≤ s.val.length)
    (hkeys : keys.val.map (·.val) = (List.range k.val).filter (fun j => 0 < (vals s.val).getD j 0)) :
    ratchet.stale_loop s keys k ⦃ r => r.val.map (·.val) = staleN (vals s.val) ⦄ := by
  unfold ratchet.stale_loop
  apply loop.spec_decr_nat
    (fun p : alloc.vec.Vec Usize × Usize => s.val.length - p.2.val)
    (fun p : alloc.vec.Vec Usize × Usize => p.2.val ≤ s.val.length ∧
      p.1.val.map (·.val) = (List.range p.2.val).filter (fun j => 0 < (vals s.val).getD j 0))
  · rintro ⟨keys, k⟩ ⟨hk, hkeys⟩
    unfold ratchet.stale_loop.body
    dsimp only at hk hkeys ⊢
    split
    · rename_i hlt
      have hlt' : k.val < s.val.length := by scalar_tac
      apply WP.spec_bind (count_at_spec s k)
      intro c hc
      have hk2 := usize_sat_add_one k (by scalar_tac)
      split
      · rename_i hpos
        have hlen : keys.val.length < Usize.max := by
          have := congrArg List.length hkeys
          simp only [List.length_map] at this
          have := List.length_filter_le (fun j => 0 < (vals s.val).getD j 0) (List.range k.val)
          simp only [List.length_range] at this
          scalar_tac
        apply WP.spec_bind (alloc.vec.Vec.push_spec _ _ hlen)
        intro keys1 hkeys1
        simp only [lift, bind_tc_ok, bind_ok, WP.spec_ok]
        refine ⟨⟨by omega, ?_⟩, by omega⟩
        rw [hkeys1, hk2, List.range_succ, List.filter_append, List.map_append, hkeys]
        have : 0 < (vals s.val)[k.val]?.getD 0 := by
          rw [← List.getD_eq_getElem?_getD]; scalar_tac
        simp [this]
      · rename_i hpos
        simp only [lift, bind_tc_ok, bind_ok, WP.spec_ok]
        refine ⟨⟨by omega, ?_⟩, by omega⟩
        rw [hk2, List.range_succ, List.filter_append, hkeys]
        have : ¬ 0 < (vals s.val)[k.val]?.getD 0 := by
          rw [← List.getD_eq_getElem?_getD]; scalar_tac
        simp [this]
    · rename_i hge
      try simp only [WP.spec_ok]
      have : k.val = s.val.length := by scalar_tac
      rw [hkeys, this]
      simp [staleN, vals]
  · exact ⟨hk, hkeys⟩

theorem stale_spec (s : Slice U32) :
    ratchet.stale s ⦃ r => r.val.map (·.val) = staleN (vals s.val) ⦄ := by
  unfold ratchet.stale
  apply stale_loop_spec <;> simp

theorem zeros_spec (n : Usize) :
    ratchet.zeros n ⦃ r => r.val = List.replicate n.val 0#u32 ⦄ := by
  unfold ratchet.zeros ratchet.zeros_loop
  apply loop.spec_decr_nat
    (fun p : alloc.vec.Vec U32 × Usize => n.val - p.2.val)
    (fun p : alloc.vec.Vec U32 × Usize => p.2.val ≤ n.val ∧ p.1.val = List.replicate p.2.val 0#u32)
  · rintro ⟨counts, i⟩ ⟨hi, hc⟩
    unfold ratchet.zeros_loop.body
    dsimp only at hi hc ⊢
    split
    · rename_i hlt
      have hlen : counts.val.length < Usize.max := by rw [hc]; simp; scalar_tac
      apply WP.spec_bind (alloc.vec.Vec.push_spec _ _ hlen)
      intro counts1 hc1
      have hi2 := usize_sat_add_one i (by scalar_tac)
      simp only [lift, bind_tc_ok, bind_ok, WP.spec_ok]
      refine ⟨⟨by scalar_tac, ?_⟩, by scalar_tac⟩
      rw [hc1, hc, hi2, List.replicate_succ']
    · rename_i hge
      try simp only [WP.spec_ok]
      have : i.val = n.val := by scalar_tac
      rw [hc, this]
  · simp

theorem add_error_spec (s : Slice U32) (f : ratchet.Finding) :
    ratchet.add_error s f ⦃ r => vals r.val =
      if f.error = true then
        (vals s.val).set f.key.val (min ((vals s.val).getD f.key.val 0 + 1) U32.max)
      else vals s.val ⦄ := by
  unfold ratchet.add_error
  cases he : f.error
  · simp
  · simp only [↓reduceIte, core.slice.Slice.get_mut, core.slice.index.Usize.get_mut,
      Slice.getElem?_Usize_eq, bind_tc_ok, bind_ok]
    cases h : s.val[f.key.val]? with
    | none =>
      have hk : ¬ f.key.val < s.val.length := by
        intro hl; rw [List.getElem?_eq_getElem hl] at h; simp at h
      have hset : (vals s.val).set f.key.val (min ((vals s.val)[f.key.val]?.getD 0 + 1) U32.max) =
          vals s.val := List.set_eq_of_length_le (by simp [vals]; omega)
      simp [vals_set_opt_none, hset]
    | some slot =>
      have hd : (vals s.val)[f.key.val]?.getD 0 = slot.val := by simp [vals, h]
      simp [lift, hd]
      rw [list_vals_set_opt_some, u32_sat_add_one]

theorem add_error_at_spec (s : Slice U32) (fs : Slice ratchet.Finding) (i : Usize)
    (hi : i.val < fs.val.length) :
    ratchet.add_error_at s fs i ⦃ r => vals r.val =
      if fs.val[i.val].error = true then
        (vals s.val).set fs.val[i.val].key.val
          (min ((vals s.val).getD fs.val[i.val].key.val 0 + 1) U32.max)
      else vals s.val ⦄ := by
  unfold ratchet.add_error_at
  simp only [core.slice.Slice.get, core.slice.index.Usize.get, Slice.getElem?_Usize_eq,
    bind_tc_ok, bind_ok, List.getElem?_eq_getElem hi]
  exact add_error_spec s _

/-- The saturated error counts of the first `i` findings. -/
def countUpTo (n i : Nat) (fs : List ratchet.Finding) : List Nat :=
  (List.range n).map (fun k => min (errorsOf (fs.take i) k) U32.max)

theorem countUpTo_step (n i : Nat) (fs : List ratchet.Finding) (hi : i < fs.length) :
    countUpTo n (i + 1) fs =
      if fs[i].error = true then
        (countUpTo n i fs).set fs[i].key.val
          (min ((countUpTo n i fs).getD fs[i].key.val 0 + 1) U32.max)
      else countUpTo n i fs := by
  have htake : ∀ k, errorsOf (fs.take (i + 1)) k =
      errorsOf (fs.take i) k + (if fs[i].error = true ∧ fs[i].key.val = k then 1 else 0) := by
    intro k
    rw [List.take_add_one, List.getElem?_eq_getElem hi]
    simp only [errorsOf, Option.toList_some, List.countP_append, List.countP_singleton]
    congr 1
    by_cases h : fs[i].error = true ∧ fs[i].key.val = k
    · simp [h.1, h.2]
    · have : ¬ (fs[i].error && fs[i].key.val == k) = true := by
        simpa [Bool.and_eq_true, beq_iff_eq] using h
      simp only [this, h]; rfl
  apply List.ext_getElem
  · split <;> simp [countUpTo]
  · intro k h1 h2
    have hk : k < n := by simpa [countUpTo] using h1
    split
    · rename_i he
      rw [List.getElem_set]
      simp only [countUpTo, List.getElem_map, List.getElem_range, htake k, he, true_and]
      split
      · rename_i hkey
        subst hkey
        rw [List.getD_eq_getElem?_getD, List.getElem?_map, List.getElem?_range hk]
        simp; omega
      · rename_i hkey
        have : ¬ k = fs[i].key.val := fun h => hkey h.symm
        simp [this]
    · rename_i he
      simp [countUpTo, htake k, he]

theorem count_errors_loop_spec (fs : Slice ratchet.Finding) (counts : alloc.vec.Vec U32)
    (i : Usize) (n : Nat) (hi : i.val ≤ fs.val.length)
    (hc : vals counts.val = countUpTo n i.val fs.val) :
    ratchet.count_errors_loop fs counts i ⦃ r => vals r.val = countUpTo n fs.val.length fs.val ⦄ := by
  unfold ratchet.count_errors_loop
  apply loop.spec_decr_nat
    (fun p : alloc.vec.Vec U32 × Usize => fs.val.length - p.2.val)
    (fun p : alloc.vec.Vec U32 × Usize => p.2.val ≤ fs.val.length ∧
      vals p.1.val = countUpTo n p.2.val fs.val)
  · rintro ⟨counts, i⟩ ⟨hi, hc⟩
    unfold ratchet.count_errors_loop.body
    dsimp only at hi hc ⊢
    split
    · rename_i hlt
      have hlt' : i.val < fs.val.length := by scalar_tac
      simp only [lift, alloc.vec.Vec.deref_mut, bind_tc_ok, bind_ok]
      apply WP.spec_bind (add_error_at_spec _ fs i hlt')
      intro s1 hs1
      have hi2 := usize_sat_add_one i (by scalar_tac)
      simp only [lift, bind_tc_ok, bind_ok, WP.spec_ok]
      have hl : (counts.slice : Slice U32).val = counts.val := rfl
      have hs1' : ({ slice := s1 } : alloc.vec.Vec U32).val = s1.val := rfl
      refine ⟨⟨by omega, ?_⟩, by omega⟩
      rw [hs1', hs1, hi2, countUpTo_step _ _ _ hlt', hl, hc]
    · rename_i hge
      try simp only [WP.spec_ok]
      have : i.val = fs.val.length := by scalar_tac
      rw [hc, this]
  · exact ⟨hi, hc⟩

theorem count_errors_spec (n : Usize) (fs : Slice ratchet.Finding) :
    ratchet.count_errors n fs ⦃ r => vals r.val = countN n.val fs.val ⦄ := by
  unfold ratchet.count_errors
  apply WP.spec_bind (zeros_spec n)
  intro z hz
  have h := count_errors_loop_spec fs z 0#usize n.val (by simp)
    (by rw [hz]; simp [countUpTo, errorsOf, vals, List.map_const'])
  simpa [countUpTo, countN, List.take_length] using h

theorem grows_at_spec (old : Slice U32) (next : Slice ratchet.Entry) (i : Usize)
    (hi : i.val < next.val.length) :
    ratchet.grows_at old next i ⦃ b =>
      b = decide ((vals old.val).getD next.val[i.val].key.val 0 < next.val[i.val].count.val) ⦄ := by
  unfold ratchet.grows_at
  simp only [core.slice.Slice.get, core.slice.index.Usize.get, Slice.getElem?_Usize_eq,
    bind_tc_ok, bind_ok, List.getElem?_eq_getElem hi]
  apply WP.spec_bind (count_at_spec old _)
  intro c hc
  simp only [WP.spec_ok, ← hc]
  scalar_tac

/-- The flag of one entry of a new baseline. -/
def growsN (old : List Nat) (e : ratchet.Entry) : Bool := decide (old.getD e.key.val 0 < e.count.val)

theorem increases_loop_spec (old : Slice U32) (next : Slice ratchet.Entry)
    (grew : alloc.vec.Vec Bool) (i : Usize) (hi : i.val ≤ next.val.length)
    (hg : grew.val = (next.val.take i.val).map (growsN (vals old.val))) :
    ratchet.increases_loop old next grew i ⦃ r => r.val = next.val.map (growsN (vals old.val)) ⦄ := by
  unfold ratchet.increases_loop
  apply loop.spec_decr_nat
    (fun p : alloc.vec.Vec Bool × Usize => next.val.length - p.2.val)
    (fun p : alloc.vec.Vec Bool × Usize => p.2.val ≤ next.val.length ∧
      p.1.val = (next.val.take p.2.val).map (growsN (vals old.val)))
  · rintro ⟨grew, i⟩ ⟨hi, hg⟩
    unfold ratchet.increases_loop.body
    dsimp only at hi hg ⊢
    split
    · rename_i hlt
      have hlt' : i.val < next.val.length := by scalar_tac
      apply WP.spec_bind (grows_at_spec old next i hlt')
      intro b hb
      have hlen : grew.val.length < Usize.max := by rw [hg]; simp; scalar_tac
      apply WP.spec_bind (alloc.vec.Vec.push_spec _ _ hlen)
      intro grew1 hg1
      have hi2 := usize_sat_add_one i (by scalar_tac)
      simp only [lift, bind_tc_ok, bind_ok, WP.spec_ok]
      refine ⟨⟨by omega, ?_⟩, by omega⟩
      rw [hg1, hg, hi2, hb]
      simp only [List.take_add_one, List.getElem?_eq_getElem hlt', List.map_append]
      simp [growsN]
    · rename_i hge
      try simp only [WP.spec_ok]
      have : i.val = next.val.length := by scalar_tac
      rw [hg, this, List.take_length]
  · exact ⟨hi, hg⟩

theorem increases_spec (old : Slice U32) (next : Slice ratchet.Entry) :
    ratchet.increases old next ⦃ r => r.val = next.val.map (growsN (vals old.val)) ⦄ := by
  unfold ratchet.increases
  apply increases_loop_spec <;> simp

/-! ## Claims -/

-- The proof is deliberately generic (`split` and `simp` over every branch)
-- so that a mutant that changes what `take` decides, but not the length,
-- leaves it standing.
set_option linter.unusedTactic false in
set_option linter.unnecessarySeqFocus false in
/-- `take_at` keeps the length of the counts, whatever it decides. Proved
from the code, not from `take_spec`. -/
theorem take_at_length (s : Slice U32) (fs : Slice ratchet.Finding) (i : Usize) :
    ratchet.take_at s fs i ⦃ r => r.2.val.length = s.val.length ⦄ := by
  unfold ratchet.take_at ratchet.take
  simp only [core.slice.Slice.get, core.slice.index.Usize.get, core.slice.Slice.get_mut,
    core.slice.index.Usize.get_mut, bind_tc_ok, bind_ok, lift]
  cases fs[i]? <;> simp <;> (repeat' split) <;> simp [Slice.set_opt_val_eq, List.length_set_opt]

/-- **R-conserve.** `apply` returns exactly one flag per finding, so every
finding is either covered or reported and none is lost or duplicated, and
one count per key. Proved by its own loop invariant, independently of what
`take` decides. -/
theorem apply_conserves (acc : alloc.vec.Vec U32) (fs : Slice ratchet.Finding) :
    ratchet.apply acc fs ⦃ r => r.1.val.length = fs.val.length ∧
      r.2.val.length = acc.val.length ⦄ := by
  unfold ratchet.apply ratchet.apply_loop
  apply loop.spec_decr_nat
    (fun p : alloc.vec.Vec U32 × alloc.vec.Vec Bool × Usize => fs.val.length - p.2.2.val)
    (fun p : alloc.vec.Vec U32 × alloc.vec.Vec Bool × Usize =>
      p.2.2.val ≤ fs.val.length ∧ p.2.1.val.length = p.2.2.val ∧
      p.1.val.length = acc.val.length)
  · rintro ⟨left, covered, i⟩ ⟨hi, hcov, hleft⟩
    unfold ratchet.apply_loop.body
    dsimp only at hi hcov hleft ⊢
    split
    · rename_i hlt
      simp only [lift, alloc.vec.Vec.deref_mut, bind_tc_ok, bind_ok]
      apply WP.spec_bind (take_at_length _ fs i)
      rintro ⟨hit, s1⟩ hs1
      have hlen : covered.val.length < Usize.max := by scalar_tac
      apply WP.spec_bind (alloc.vec.Vec.push_spec _ _ hlen)
      intro covered1 hcov1
      have hi2 := usize_sat_add_one i (by scalar_tac)
      simp only [lift, bind_tc_ok, bind_ok, WP.spec_ok]
      have hl : (left.slice : Slice U32).val = left.val := rfl
      have hs1' : ({ slice := s1 } : alloc.vec.Vec U32).val = s1.val := rfl
      dsimp only at hs1
      refine ⟨⟨by scalar_tac, by simp [hcov1, hcov, hi2], ?_⟩, by scalar_tac⟩
      rw [hs1', hs1, hl, hleft]
    · rename_i hge
      try simp only [WP.spec_ok]
      exact ⟨by scalar_tac, hleft⟩
  · simp

/-- **R-cover (decision).** The finding at position `i` is covered iff it is
an error and fewer errors with its key come before it than the baseline
accepts for that key. So a warning is never covered, a finding is covered
only while its key has accepted count left, and the first `accepted[k]`
errors of key `k` are the ones covered. -/
theorem apply_covered_iff (acc : alloc.vec.Vec U32) (fs : Slice ratchet.Finding) :
    ratchet.apply acc fs ⦃ r => ∀ i (hi : i < fs.val.length),
      r.1.val[i]? = some (fs.val[i].error &&
        decide (errorsOf (fs.val.take i) fs.val[i].key.val <
          (vals acc.val).getD fs.val[i].key.val 0)) ⦄ := by
  apply WP.spec_mono (apply_spec acc fs)
  rintro r ⟨h1, -⟩ i hi
  have hl := (applyN_length (vals acc.val) fs.val).1
  have hc := applyN_covered (vals acc.val) fs.val i hi
  rw [h1, List.getElem?_eq_getElem (by omega), ← hc, List.getD_eq_getElem?_getD,
    List.getElem?_eq_getElem (by omega)]
  rfl

/-- **R-cover (count).** Per key, the number of covered findings is the
accepted count or the number of errors with that key, whichever is
smaller. -/
theorem apply_cover_count (acc : alloc.vec.Vec U32) (fs : Slice ratchet.Finding) :
    ratchet.apply acc fs ⦃ r => ∀ k,
      coveredOf fs.val r.1.val k = min ((vals acc.val).getD k 0) (errorsOf fs.val k) ⦄ := by
  apply WP.spec_mono (apply_spec acc fs)
  rintro r ⟨h1, -⟩ k
  rw [h1, applyN_coveredOf]

/-- **No over-suppression.** Per key, the baseline never covers more
findings than it accepts. -/
theorem apply_suppressed_le_accepted (acc : alloc.vec.Vec U32) (fs : Slice ratchet.Finding) :
    ratchet.apply acc fs ⦃ r => ∀ k, coveredOf fs.val r.1.val k ≤ (vals acc.val).getD k 0 ⦄ := by
  apply WP.spec_mono (apply_spec acc fs)
  rintro r ⟨h1, -⟩ k
  rw [h1, applyN_coveredOf]
  exact Nat.min_le_left _ _

/-- What is left of each key after `apply`: the accepted count minus the
errors with that key, truncated at zero. -/
theorem apply_left_spec (acc : alloc.vec.Vec U32) (fs : Slice ratchet.Finding) :
    ratchet.apply acc fs ⦃ r => r.2.val.length = acc.val.length ∧
      ∀ k, (vals r.2.val).getD k 0 = (vals acc.val).getD k 0 - errorsOf fs.val k ⦄ := by
  apply WP.spec_mono (apply_spec acc fs)
  rintro r ⟨-, h2⟩
  have hl := (applyN_length (vals acc.val) fs.val).2
  refine ⟨?_, fun k => by rw [h2, applyN_getD]⟩
  have := congrArg List.length h2
  simp only [vals, List.length_map] at this hl
  omega

theorem deref_val (v : alloc.vec.Vec U32) : (alloc.vec.Vec.deref v).val = v.val := by
  simp [alloc.vec.Vec.deref]

/-- **R-stale.** After `apply`, `stale` lists exactly the keys whose accepted
count exceeds the errors now, each once and in increasing order, so the
shell reports exactly one TAU-R001 per such key. -/
theorem ratchet_stale_iff (acc : alloc.vec.Vec U32) (fs : Slice ratchet.Finding) :
    ratchet.apply acc fs ⦃ r => ratchet.stale (alloc.vec.Vec.deref r.2) ⦃ ks =>
      (∀ k, k ∈ ks.val.map (·.val) ↔ errorsOf fs.val k < (vals acc.val).getD k 0) ∧
      (ks.val.map (·.val)).Pairwise (· < ·) ⦄ ⦄ := by
  apply WP.spec_mono (apply_spec acc fs)
  rintro r ⟨-, h2⟩
  apply WP.spec_mono (stale_spec _)
  intro ks hks
  rw [hks, deref_val, h2]
  refine ⟨fun k => ?_, ?_⟩
  · simp only [staleN, List.mem_filter, List.mem_range, decide_eq_true_eq, applyN_getD]
    constructor
    · rintro ⟨-, h⟩; omega
    · intro h
      refine ⟨?_, by omega⟩
      have hk := getD_pos_lt (show 0 < (vals acc.val).getD k 0 by omega)
      have := (applyN_length (vals acc.val) fs.val).2
      omega
  · exact List.Pairwise.filter _ List.pairwise_lt_range

theorem coveredOf_map_error (fs : List ratchet.Finding) (k : Nat) :
    coveredOf fs (fs.map (·.error)) k = errorsOf fs k := by
  induction fs with
  | nil => simp [coveredOf, errorsOf]
  | cons f fs ih =>
    simp only [coveredOf, errorsOf] at ih ⊢
    simp [List.countP_cons, ih]

theorem errorsOf_take_lt (fs : List ratchet.Finding) (i : Nat) (hi : i < fs.length)
    (he : fs[i].error = true) :
    errorsOf (fs.take i) fs[i].key.val < errorsOf fs fs[i].key.val := by
  have h := congrArg (fun l => errorsOf l fs[i].key.val) (List.take_append_drop i fs)
  simp only [errorsOf, List.countP_append] at h
  rw [List.drop_eq_getElem_cons hi] at h
  simp only [errorsOf, List.countP_cons, he, Bool.true_and, beq_self_eq_true, ↓reduceIte] at h ⊢
  omega

/-- **R-exact.** With every finding's key interned (the shell's job), the
check passes, i.e. every error is covered and no key is stale, iff the
baseline accepts exactly the errors present now, key by key. -/
theorem ratchet_passes_iff (acc : alloc.vec.Vec U32) (fs : Slice ratchet.Finding)
    (hkeys : ∀ f ∈ fs.val, f.key.val < acc.val.length) :
    ratchet.apply acc fs ⦃ r => ratchet.stale (alloc.vec.Vec.deref r.2) ⦃ ks =>
      ((∀ i (hi : i < fs.val.length), fs.val[i].error = true → r.1.val[i]? = some true) ∧
        ks.val = []) ↔
      ∀ k < acc.val.length, (vals acc.val).getD k 0 = errorsOf fs.val k ⦄ ⦄ := by
  apply WP.spec_mono (apply_spec acc fs)
  rintro r ⟨h1, h2⟩
  apply WP.spec_mono (stale_spec _)
  intro ks hks
  rw [deref_val, h2] at hks
  have hlen := applyN_length (vals acc.val) fs.val
  have hstale : ks.val = [] ↔ ∀ k, ¬ errorsOf fs.val k < (vals acc.val).getD k 0 := by
    rw [← List.map_eq_nil_iff (f := (·.val)), hks, staleN, List.filter_eq_nil_iff]
    simp only [List.mem_range, decide_eq_true_eq, applyN_getD]
    constructor
    · intro h k hk
      have hk' := getD_pos_lt (show 0 < (vals acc.val).getD k 0 by omega)
      exact h k (by simp only [vals, List.length_map] at hlen hk' ⊢; omega) (by omega)
    · intro h k _ hk; exact h k (by omega)
  rw [hstale]
  constructor
  · rintro ⟨hall, hnone⟩ k hk
    -- every error is covered, so the flags are the error flags
    have hflags : r.1.val = fs.val.map (·.error) := by
      apply List.ext_getElem?
      intro i
      by_cases hi : i < fs.val.length
      · rw [List.getElem?_map, List.getElem?_eq_getElem hi]
        simp only [Option.map_some]
        cases he : fs.val[i].error
        · have hc := applyN_covered (vals acc.val) fs.val i hi
          rw [he, Bool.false_and, List.getD_eq_getElem?_getD,
            List.getElem?_eq_getElem (by omega)] at hc
          rw [h1, List.getElem?_eq_getElem (by omega)]
          simpa using hc
        · exact hall i hi he
      · rw [List.getElem?_eq_none (by rw [h1]; omega), List.getElem?_eq_none (by simp; omega)]
    have hc := applyN_coveredOf (vals acc.val) fs.val k
    rw [← h1, hflags, coveredOf_map_error] at hc
    have := hnone k
    omega
  · intro hexact
    refine ⟨fun i hi he => ?_, fun k hk => ?_⟩
    · have hc := applyN_covered (vals acc.val) fs.val i hi
      have hkey := hkeys _ (List.getElem_mem hi)
      rw [List.getD_eq_getElem?_getD, ← h1] at hc
      have hlt := errorsOf_take_lt fs.val i hi he
      rw [← hexact _ (by simpa [vals] using hkey)] at hlt
      rw [List.getElem?_eq_getElem (by rw [h1]; omega)] at hc ⊢
      simp only [Option.getD_some, he, Bool.true_and, hlt, decide_true] at hc
      rw [hc]
    · by_cases hin : k < acc.val.length
      · rw [hexact k hin] at hk; omega
      · rw [List.getD_eq_getElem?_getD, List.getElem?_eq_none (by simp [vals]; omega)] at hk
        simp at hk

/-- **R-headroom.** From a passing state (the baseline accepts exactly the
errors present), one more error finding is not covered, whatever its key. -/
theorem ratchet_headroom (acc : alloc.vec.Vec U32) (fs fs' : Slice ratchet.Finding)
    (g : ratchet.Finding) (hg : g.error = true) (hfs' : fs'.val = fs.val ++ [g])
    (hpass : ∀ k < acc.val.length, (vals acc.val).getD k 0 = errorsOf fs.val k) :
    ratchet.apply acc fs' ⦃ r => r.1.val[fs.val.length]? = some false ⦄ := by
  apply WP.spec_mono (apply_covered_iff acc fs')
  intro r hr
  have hi : fs.val.length < fs'.val.length := by simp [hfs']
  rw [hr _ hi]
  have hget : fs'.val[fs.val.length] = g := by simp [hfs']
  have htake : fs'.val.take fs.val.length = fs.val := by simp [hfs']
  rw [hget, htake]
  simp only [hg, Bool.true_and, Option.some.injEq, decide_eq_false_iff_not, not_lt]
  by_cases hin : g.key.val < acc.val.length
  · rw [hpass _ hin]
  · rw [List.getD_eq_getElem?_getD, List.getElem?_eq_none (by simp [vals]; omega)]
    simp

/-- The error counts a new baseline records: one count per key, the number
of error findings with that key, saturating at `u32::MAX`. -/
theorem count_errors_counts (n : Usize) (fs : Slice ratchet.Finding) :
    ratchet.count_errors n fs ⦃ c => c.val.length = n.val ∧
      ∀ k < n.val, (vals c.val).getD k 0 = min (errorsOf fs.val k) U32.max ⦄ := by
  apply WP.spec_mono (count_errors_spec n fs)
  intro c hc
  refine ⟨?_, fun k hk => ?_⟩
  · have := congrArg List.length hc
    simpa [vals, countN] using this
  · rw [hc, countN, List.getD_eq_getElem?_getD, List.getElem?_map, List.getElem?_range hk]
    simp

/-- One flag per entry of the new baseline: set iff the entry accepts more
than the old baseline does for its key (0 for a key the old one lacks). -/
theorem increases_flags (old : Slice U32) (next : Slice ratchet.Entry) :
    ratchet.increases old next ⦃ g => g.val.length = next.val.length ∧
      ∀ i (hi : i < next.val.length), g.val[i]? =
        some (decide ((vals old.val).getD next.val[i].key.val 0 < next.val[i].count.val)) ⦄ := by
  apply WP.spec_mono (increases_spec old next)
  intro g hg
  refine ⟨by simp [hg], fun i hi => ?_⟩
  rw [hg, List.getElem?_map, List.getElem?_eq_getElem hi]
  rfl

/-- **R-shrink.** No entry grows iff the new baseline accepts, key by key,
at most what the old one does: `cargo tauten baseline` refuses exactly the
regenerations that would let a finding back in. -/
theorem increases_none_iff (old : Slice U32) (next : Slice ratchet.Entry) :
    ratchet.increases old next ⦃ g => (∀ b ∈ g.val, b = false) ↔
      ∀ e ∈ next.val, e.count.val ≤ (vals old.val).getD e.key.val 0 ⦄ := by
  apply WP.spec_mono (increases_spec old next)
  intro g hg
  rw [hg]
  simp [growsN]

/-- **R-shrink (regeneration).** A baseline regenerated from a passing run
(the accepted counts equal the errors present) never grows: every entry of
it carries the new count of its key, and none exceeds the old one. -/
theorem regenerated_baseline_never_grows (acc : Slice U32) (fs : Slice ratchet.Finding)
    (n : Usize) (hpass : ∀ k < acc.val.length, (vals acc.val).getD k 0 = errorsOf fs.val k)
    (hn : n.val = acc.val.length) :
    ratchet.count_errors n fs ⦃ c => ∀ next : Slice ratchet.Entry,
      (∀ e ∈ next.val, e.key.val < n.val ∧ e.count.val = (vals c.val).getD e.key.val 0) →
      ratchet.increases acc next ⦃ g => ∀ b ∈ g.val, b = false ⦄ ⦄ := by
  apply WP.spec_mono (count_errors_spec n fs)
  intro c hc next hnext
  apply WP.spec_mono (increases_spec acc next)
  intro g hg
  rw [hg]
  simp only [List.mem_map, growsN, forall_exists_index, and_imp]
  rintro b e he rfl
  obtain ⟨hk, hcount⟩ := hnext e he
  have hp := hpass _ (show e.key.val < acc.val.length by omega)
  rw [List.getD_eq_getElem?_getD] at hp
  rw [hcount, hc, countN]
  simp only [List.getD_eq_getElem?_getD, List.getElem?_map, List.getElem?_range hk, hp]
  simp

/-! ## Non-vacuity: the ratchet really covers, really goes stale and really
refuses growth. -/

/-- Baseline `[1, 2]` (keys 0 and 1); findings: two errors of key 0, a
warning of key 0, one error of key 1. The first error of key 0 and the error
of key 1 are covered; the second error of key 0 and the warning are
reported; key 1 has one accepted finding left and is stale. -/
theorem ratchet_example :
    ratchet.apply (alloc.vec.Vec.from [1#u32, 2#u32] (by simp; scalar_tac))
      (Slice.from [⟨0#usize, true⟩, ⟨0#usize, true⟩, ⟨0#usize, false⟩, ⟨1#usize, true⟩]
        (by simp; scalar_tac)) ⦃ r =>
      r.1.val = [true, false, false, true] ∧ vals r.2.val = [0, 1] ∧
      ratchet.stale (alloc.vec.Vec.deref r.2) ⦃ ks => ks.val.map (·.val) = [1] ⦄ ⦄ := by
  apply WP.spec_mono (apply_spec _ _)
  rintro r ⟨h1, h2⟩
  have e1 : r.1.val = [true, false, false, true] := by
    rw [h1]; simp [applyN, stepN, vals]
  have e2 : vals r.2.val = [0, 1] := by
    rw [h2]; simp [applyN, stepN, vals]
  refine ⟨e1, e2, ?_⟩
  apply WP.spec_mono (stale_spec _)
  intro ks hks
  rw [hks, deref_val, e2]
  decide

/-- Old baseline `[1]`; a new one accepting 2 findings of key 0 grows. -/
theorem increase_example :
    ratchet.increases (Slice.from [1#u32] (by simp; scalar_tac))
      (Slice.from [⟨0#usize, 2#u32⟩] (by simp; scalar_tac))
      ⦃ g => g.val = [true] ⦄ := by
  apply WP.spec_mono (increases_spec _ _)
  intro g hg
  rw [hg]
  simp [growsN, vals]

end Tauten.Kernel
