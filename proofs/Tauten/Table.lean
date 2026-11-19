import Extracted.TautenCheckKernel

/-!
# Rule-table well-formedness (`tauten_check_kernel::table`)

`tauten_check::profile::Profile::parse` interns every rule ID (the rules'
own IDs, the rules API bans name, and the IDs the checker emits:
`EMITTED_BY_CODE`) to one index per distinct string, and every lint name
likewise, then calls the extracted `table.validate`. The claim
`validate_ok_iff` says exactly when a table is accepted: no two rules
share an ID, no lint is owned twice, every API ban names a rule of the
table, and every required ID is a rule's ID. The call never panics.

Structure: one lemma per extracted function, each loop by an invariant;
the claim is proved from them. Which error is reported first (the table's
reading order) is not stated here; a differential test in `tauten-check`
compares it with the reference implementation.

Trusted, not proved (`docs/proofs/README.md`): interning (equal strings
get equal indices, distinct strings distinct ones) and the shell's mapping
of the returned positions back to names.
-/

open Aeneas Aeneas.Std Result
open tauten_check_kernel

set_option linter.unusedSimpArgs false

namespace Tauten.Kernel

theorem table_uscalar_max_lt (ty : UScalarTy) : UScalar.max ty < 2 ^ ty.numBits := by
  rw [UScalar.max_def]
  have := Nat.two_pow_pos ty.numBits
  omega

theorem table_uscalar_sat_add_val {ty : UScalarTy} (x y : UScalar ty) :
    (UScalar.saturating_add x y).val = min (UScalar.max ty) (x.val + y.val) := by
  show (BitVec.ofNat _ (min (UScalar.max ty) (x.val + y.val))).toNat = _
  rw [BitVec.toNat_ofNat]
  apply Nat.mod_eq_of_lt
  have := table_uscalar_max_lt ty
  omega

theorem table_usize_succ (i : Usize) (h : i.val < Usize.max) :
    (core.num.Usize.saturating_add i 1#usize).val = i.val + 1 := by
  have := table_uscalar_sat_add_val (ty := .Usize) i 1#usize
  simp only [UScalar.max_USize_eq] at this
  simp only [core.num.Usize.saturating_add, this]
  simp; omega

theorem is_at_spec (s : Slice Usize) (i x : Usize) :
    table.is_at s i x ⦃ b => (b = true ↔ s.val[i.val]? = some x) ⦄ := by
  unfold table.is_at
  simp only [core.slice.Slice.get, core.slice.index.SliceIndexUsizeSlice,
    core.slice.index.Usize.get, bind_tc_ok, Slice.getElem?_Usize_eq]
  cases h : s.val[i.val]? <;> simp

/-- What `find_before s end x` returns: the first position before `end`
holding `x`, or nothing when no such position exists. -/
def FoundBefore (s : List Usize) (e : Nat) (x : Usize) : Option Usize → Prop
  | none => ∀ j, j < e → s[j]? ≠ some x
  | some i => i.val < e ∧ s[i.val]? = some x ∧ ∀ j, j < i.val → s[j]? ≠ some x

theorem find_before_spec (s : Slice Usize) (e x : Usize) :
    table.find_before s e x ⦃ r => FoundBefore s.val e.val x r ⦄ := by
  unfold table.find_before table.find_before_loop
  apply loop.spec_decr_nat
    (fun i : Usize => e.val - i.val)
    (fun i : Usize => i.val ≤ e.val ∧ ∀ j, j < i.val → s.val[j]? ≠ some x)
  · rintro i ⟨hi, hnot⟩
    unfold table.find_before_loop.body
    split
    · rename_i hlt
      apply WP.spec_bind (is_at_spec s i x)
      intro b hb
      split
      · rename_i htrue
        simp only [WP.spec_ok, FoundBefore]
        exact ⟨by scalar_tac, hb.mp htrue, hnot⟩
      · rename_i hfalse
        have hi2 := table_usize_succ i (by scalar_tac)
        simp only [lift, bind_tc_ok, bind_ok, WP.spec_ok]
        refine ⟨⟨by scalar_tac, ?_⟩, by scalar_tac⟩
        intro j hj
        rw [hi2] at hj
        rcases Nat.lt_succ_iff_lt_or_eq.mp hj with hj | hj
        · exact hnot j hj
        · subst hj
          intro h
          exact hfalse (hb.mpr h)
    · rename_i hge
      simp only [WP.spec_ok, FoundBefore]
      intro j hj
      exact hnot j (by scalar_tac)
  · exact ⟨by scalar_tac, fun j hj => absurd hj (by simp)⟩

theorem mem_iff_found (l : List Usize) (x : Usize) :
    x ∈ l ↔ ∃ j, j < l.length ∧ l[j]? = some x := by
  rw [List.mem_iff_getElem?]
  constructor
  · rintro ⟨j, hj⟩
    exact ⟨j, (List.getElem?_eq_some_iff.mp hj).1, hj⟩
  · rintro ⟨j, -, hj⟩
    exact ⟨j, hj⟩

theorem contains_spec (s : Slice Usize) (x : Usize) :
    table.contains s x ⦃ b => (b = true ↔ x ∈ s.val) ⦄ := by
  unfold table.contains
  apply WP.spec_bind (find_before_spec s _ x)
  intro r hr
  have hlen : (Slice.len s).val = s.val.length := Slice.len_val s
  cases r with
  | none =>
    simp only [WP.spec_ok, FoundBefore, hlen] at hr ⊢
    simp only [Bool.false_eq_true, false_iff, mem_iff_found, not_exists, not_and]
    exact hr
  | some i =>
    simp only [WP.spec_ok, FoundBefore, hlen] at hr ⊢
    simp only [true_iff, mem_iff_found]
    exact ⟨i.val, hr.1, hr.2.1⟩

theorem repeat_at_spec (s : Slice Usize) (j : Usize) (hj : j.val < s.val.length) :
    table.repeat_at s j ⦃ r => FoundBefore s.val j.val s.val[j.val] r ⦄ := by
  unfold table.repeat_at
  simp only [core.slice.Slice.get, core.slice.index.SliceIndexUsizeSlice,
    core.slice.index.Usize.get, bind_tc_ok, Slice.getElem?_Usize_eq,
    List.getElem?_eq_getElem hj, bind_ok]
  exact find_before_spec s j _

/-- No two positions below `n` hold the same value. -/
def DistinctUpTo (l : List Usize) (n : Nat) : Prop :=
  ∀ i j, i < j → j < n → l[i]? ≠ l[j]?

theorem first_duplicate_spec (s : Slice Usize) :
    table.first_duplicate s ⦃ r => match r with
      | none => s.val.Nodup
      | some (i, j) => i.val < j.val ∧ j.val < s.val.length ∧ s.val[i.val]? = s.val[j.val]? ⦄ := by
  unfold table.first_duplicate table.first_duplicate_loop
  have hlen : (Slice.len s).val = s.val.length := Slice.len_val s
  apply loop.spec_decr_nat
    (fun j : Usize => s.val.length - j.val)
    (fun j : Usize => j.val ≤ s.val.length ∧ DistinctUpTo s.val j.val)
  · rintro j ⟨hj, hdist⟩
    unfold table.first_duplicate_loop.body
    dsimp only
    split
    · rename_i hlt
      have hlt' : j.val < s.val.length := by scalar_tac
      apply WP.spec_bind (repeat_at_spec s j hlt')
      intro r hr
      cases r with
      | none =>
        have hj2 := table_usize_succ j (by scalar_tac)
        simp only [lift, bind_tc_ok, bind_ok, WP.spec_ok, FoundBefore] at hr ⊢
        refine ⟨⟨by scalar_tac, ?_⟩, by scalar_tac⟩
        intro i k hik hk
        rw [hj2] at hk
        rcases Nat.lt_succ_iff_lt_or_eq.mp hk with hk | hk
        · exact hdist i k hik hk
        · subst hk
          rw [List.getElem?_eq_getElem hlt']
          exact hr i hik
      | some i =>
        simp only [WP.spec_ok, FoundBefore] at hr ⊢
        refine ⟨hr.1, hlt', ?_⟩
        rw [hr.2.1, List.getElem?_eq_getElem hlt']
    · rename_i hge
      simp only [WP.spec_ok]
      rw [List.nodup_iff_getElem?_ne_getElem?]
      intro i k hik hk
      exact hdist i k hik (by scalar_tac)
  · exact ⟨by scalar_tac, fun i k _ hk => absurd hk (by simp)⟩

theorem first_duplicate_none_iff (s : Slice Usize) :
    table.first_duplicate s ⦃ r => (r = none ↔ s.val.Nodup) ⦄ := by
  apply WP.spec_mono (first_duplicate_spec s)
  intro r hr
  cases r with
  | none => simpa using hr
  | some p =>
    obtain ⟨i, j⟩ := p
    simp only [reduceCtorEq, false_iff]
    rw [List.nodup_iff_getElem?_ne_getElem?]
    intro h
    exact h i.val j.val hr.1 hr.2.1 hr.2.2

theorem absent_spec (s : Slice Usize) (x : Usize) :
    table.absent s x ⦃ b => (b = true ↔ x ∉ s.val) ⦄ := by
  unfold table.absent
  apply WP.spec_bind (contains_spec s x)
  intro b hb
  simp only [WP.spec_ok]
  cases b <;> simp_all

theorem missing_at_spec (present wanted : Slice Usize) (j : Usize)
    (hj : j.val < wanted.val.length) :
    table.missing_at present wanted j ⦃ b => (b = true ↔ wanted.val[j.val] ∉ present.val) ⦄ := by
  unfold table.missing_at
  simp only [core.slice.Slice.get, core.slice.index.SliceIndexUsizeSlice,
    core.slice.index.Usize.get, bind_tc_ok, Slice.getElem?_Usize_eq,
    List.getElem?_eq_getElem hj, bind_ok]
  exact absent_spec present _

theorem first_missing_spec (present wanted : Slice Usize) :
    table.first_missing present wanted ⦃ r => (r = none ↔ ∀ x ∈ wanted.val, x ∈ present.val) ⦄ := by
  unfold table.first_missing table.first_missing_loop
  have hlen : (Slice.len wanted).val = wanted.val.length := Slice.len_val wanted
  apply loop.spec_decr_nat
    (fun j : Usize => wanted.val.length - j.val)
    (fun j : Usize => j.val ≤ wanted.val.length ∧
      ∀ k (hk : k < wanted.val.length), k < j.val → wanted.val[k] ∈ present.val)
  · rintro j ⟨hj, hin⟩
    unfold table.first_missing_loop.body
    dsimp only
    split
    · rename_i hlt
      have hlt' : j.val < wanted.val.length := by scalar_tac
      apply WP.spec_bind (missing_at_spec present wanted j hlt')
      intro b hb
      split
      · rename_i htrue
        simp only [WP.spec_ok, reduceCtorEq, false_iff, not_forall]
        exact ⟨wanted.val[j.val], List.getElem_mem hlt', hb.mp htrue⟩
      · rename_i hfalse
        have hj2 := table_usize_succ j (by scalar_tac)
        simp only [lift, bind_tc_ok, bind_ok, WP.spec_ok]
        refine ⟨⟨by scalar_tac, ?_⟩, by scalar_tac⟩
        intro k hk hkj
        rw [hj2] at hkj
        rcases Nat.lt_succ_iff_lt_or_eq.mp hkj with hkj | hkj
        · exact hin k hk hkj
        · subst hkj
          by_contra hnot
          exact hfalse (hb.mpr hnot)
    · rename_i hge
      simp only [WP.spec_ok, true_iff]
      intro x hx
      obtain ⟨k, hk, hkx⟩ := List.getElem_of_mem hx
      rw [← hkx]
      exact hin k hk (by scalar_tac)
  · exact ⟨by scalar_tac, fun k _ hk => absurd hk (by simp)⟩

theorem owner_at_spec (s : Slice Usize) (k : Usize) :
    table.owner_at s k ⦃ _ => True ⦄ := by
  unfold table.owner_at
  simp only [core.slice.Slice.get, core.slice.index.SliceIndexUsizeSlice,
    core.slice.index.Usize.get, bind_tc_ok, Slice.getElem?_Usize_eq]
  cases s.val[k.val]? <;> simp

theorem duplicate_spec (ids lintRules lints : Slice Usize) :
    table.duplicate ids lintRules lints ⦃ r => (r = none ↔ ids.val.Nodup ∧ lints.val.Nodup) ⦄ := by
  unfold table.duplicate
  apply WP.spec_bind (first_duplicate_none_iff lints)
  intro l hl
  apply WP.spec_bind (first_duplicate_none_iff ids)
  intro r hr
  cases r with
  | none =>
    have hids : ids.val.Nodup := hr.mp rfl
    cases l with
    | none => simp only [WP.spec_ok, true_iff]; exact ⟨hids, hl.mp rfl⟩
    | some p =>
      obtain ⟨first, second⟩ := p
      simp at hl
      simp [hl]
  | some p =>
    obtain ⟨_, rule⟩ := p
    have hids : ¬ ids.val.Nodup := fun h => absurd (hr.mpr h) (by simp)
    dsimp only
    cases l with
    | none => simp [hids]
    | some p1 =>
      obtain ⟨first, second⟩ := p1
      dsimp only
      apply WP.spec_bind (owner_at_spec lintRules second)
      intro o _
      split <;> simp [hids]

/-- **Rule-table well-formedness.** The interned table is accepted iff no
two rules share an ID, no lint is owned twice, every API ban names a rule
of the table and every required ID is a rule's ID. `validate` never
panics. -/
theorem validate_ok_iff (ids lintRules lints apiRules required : Slice Usize) :
    table.validate ids lintRules lints apiRules required ⦃ r =>
      (r = core.result.Result.Ok () ↔
        ids.val.Nodup ∧ lints.val.Nodup ∧
        (∀ rule ∈ apiRules.val, rule ∈ ids.val) ∧
        (∀ id ∈ required.val, id ∈ ids.val)) ⦄ := by
  unfold table.validate
  apply WP.spec_bind (duplicate_spec ids lintRules lints)
  intro d hd
  cases d with
  | some invalid =>
    simp only [WP.spec_ok, reduceCtorEq, false_iff, not_and]
    intro h1 h2
    exact absurd (hd.mpr ⟨h1, h2⟩) (by simp)
  | none =>
    have hdup := hd.mp rfl
    simp only
    apply WP.spec_bind (first_missing_spec ids apiRules)
    intro a ha
    cases a with
    | some api =>
      simp only [WP.spec_ok, reduceCtorEq, false_iff, not_and]
      intro _ _ h _
      exact absurd (ha.mpr h) (by simp)
    | none =>
      have hapi := ha.mp rfl
      simp only
      apply WP.spec_bind (first_missing_spec ids required)
      intro m hm
      cases m with
      | some id =>
        simp only [WP.spec_ok, reduceCtorEq, false_iff, not_and]
        intro _ _ _ h
        exact absurd (hm.mpr h) (by simp)
      | none =>
        simp only [WP.spec_ok, true_iff]
        exact ⟨hdup.1, hdup.2, hapi, hm.mp rfl⟩

/-- A slice literal for the examples. -/
def usizes (l : List Usize) (h : l.length ≤ Usize.max := by simp; scalar_tac) : Slice Usize :=
  Slice.from l h

/-- Non-vacuity: a well-formed table (rules 0 and 1, lints 2 and 3 owned by
rule 0, an API ban naming rule 1, both IDs required) is accepted; the same
table is rejected with a repeated rule ID, with a lint owned twice, with an
API ban naming an unknown rule, and with a required ID missing (the case of
a table that lacks TAU-R001). -/
theorem validate_examples :
    table.validate (usizes [0#usize, 1#usize]) (usizes [0#usize, 0#usize])
      (usizes [2#usize, 3#usize]) (usizes [1#usize]) (usizes [0#usize, 1#usize])
      ⦃ r => r = core.result.Result.Ok () ⦄ ∧
    table.validate (usizes [0#usize, 0#usize]) (usizes [0#usize, 0#usize])
      (usizes [2#usize, 3#usize]) (usizes [1#usize]) (usizes [0#usize])
      ⦃ r => r ≠ core.result.Result.Ok () ⦄ ∧
    table.validate (usizes [0#usize, 1#usize]) (usizes [0#usize, 1#usize])
      (usizes [2#usize, 2#usize]) (usizes [1#usize]) (usizes [0#usize, 1#usize])
      ⦃ r => r ≠ core.result.Result.Ok () ⦄ ∧
    table.validate (usizes [0#usize, 1#usize]) (usizes [0#usize, 0#usize])
      (usizes [2#usize, 3#usize]) (usizes [4#usize]) (usizes [0#usize, 1#usize])
      ⦃ r => r ≠ core.result.Result.Ok () ⦄ ∧
    table.validate (usizes [0#usize, 1#usize]) (usizes [0#usize, 0#usize])
      (usizes [2#usize, 3#usize]) (usizes [1#usize]) (usizes [0#usize, 1#usize, 5#usize])
      ⦃ r => r ≠ core.result.Result.Ok () ⦄ := by
  refine ⟨?_, ?_, ?_, ?_, ?_⟩ <;>
  · apply WP.spec_mono (validate_ok_iff _ _ _ _ _)
    intro r hr
    simp only [usizes, Slice.from_val] at hr
    first
    | exact hr.mpr (by decide)
    | exact fun h => absurd (hr.mp h) (by decide)

end Tauten.Kernel
