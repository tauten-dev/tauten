import Tauten.Control

/-!
# Nothing quietly removed or weakened (`tauten_check_kernel::registry`)

A registry (the theorems and mutants of a claim, a baseline, required
checks, diagnostic codes) lists entries `(id, strength)`. A change from
`old` to `new` is conserved when every old entry is named by an amendment
or kept with at least its strength. A may-not-grow list swaps the first two
arguments.

Proved here, over the extracted functions:
* `conserves_iff`: conserved exactly when every old entry is amended or kept
  in the new list with at least its strength.
* `conserves_refl`: an unchanged list is conserved, with no amendment.
* `conserves_trans`: two conserved changes compose, their amendments
  accumulated (stated on `ConservesP`, which `conserves_iff` ties to the
  code).
* `amendment_is_explicit`: when an entry was removed or weakened and the
  change is accepted, that entry's identifier is among the amendments.

Trusted, not proved: the shell's reading of the committed record and of the
amendment list; conservation is relative to the committed record, and
reviewing that record's diff is trusted.
-/

open Aeneas Aeneas.Std Result
open tauten_check_kernel

namespace Tauten.Kernel

/-! ## Models -/

def keptN (l : List registry.Entry) (id : Std.Usize) (s : Std.U32) : Bool :=
  l.any (fun e => decide (e.id = id) && decide (s ≤ e.strength))

def conservesN (old new : List registry.Entry) (am : List Std.Usize) : Bool :=
  old.all (fun o => memberN am o.id || keptN new o.id o.strength)

/-- What `conserves` computes, as a proposition. -/
def ConservesP (old new : List registry.Entry) (am : List Std.Usize) : Prop :=
  ∀ o ∈ old, o.id ∈ am ∨ ∃ n ∈ new, n.id = o.id ∧ o.strength.val ≤ n.strength.val

theorem kept_spec (l : Slice registry.Entry) (id : Std.Usize) (s : Std.U32) :
    registry.kept l id s ⦃ r => r = keptN l.val id s ⦄ := by
  unfold registry.kept registry.kept_loop
  apply WP.spec_mono (fold_loop l
    (fun a e => a || (decide (e.id = id) && decide (s ≤ e.strength))) _ ?_ ?_
    false 0#usize (by simp))
  · intro r hr; rw [hr, foldl_or]; simp [keptN]
  · intro a i h
    have hlt : i < Slice.len l := by scalar_tac
    simp only [registry.kept_loop.body, hlt, ite_true, get_ok l i h, bind_ok, lift]
    cases a
    · simp only [Bool.false_eq_true, ite_false, Bool.false_or]
      split <;> rename_i hk <;> simp [hk]
    · simp
  · intro a i h
    have hlt : ¬ i < Slice.len l := by scalar_tac
    simp only [registry.kept_loop.body, hlt, ite_false, WP.spec_ok]

theorem member_or_kept (am : Slice Std.Usize) (new : Slice registry.Entry) (o : registry.Entry) :
    (do let b ← control.member am o.id
        if b = true then ok true else registry.kept new o.id o.strength) ⦃ r =>
      r = (memberN am.val o.id || keptN new.val o.id o.strength) ⦄ := by
  apply WP.spec_bind (member_spec am o.id)
  intro b hb
  subst hb
  cases h : memberN am.val o.id
  · simp only [Bool.false_eq_true, ite_false, Bool.false_or]
    exact kept_spec new o.id o.strength
  · simp [WP.spec_ok]

theorem conserves_spec (old new : Slice registry.Entry) (am : Slice Std.Usize) :
    registry.conserves old new am ⦃ r => r = conservesN old.val new.val am.val ⦄ := by
  unfold registry.conserves registry.conserves_loop
  apply WP.spec_mono (fold_loop old
    (fun a o => a && (memberN am.val o.id || keptN new.val o.id o.strength)) _ ?_ ?_
    true 0#usize (by simp))
  · intro r hr; rw [hr, foldl_and]; simp [conservesN]
  · intro a i h
    have hlt : i < Slice.len old := by scalar_tac
    simp only [registry.conserves_loop.body, hlt, ite_true, get_ok old i h, bind_ok, lift]
    cases a
    · simp
    · simp only [ite_true, Bool.true_and]
      apply WP.spec_bind (member_or_kept am new _)
      intro b hb
      simp [hb]
  · intro a i h
    have hlt : ¬ i < Slice.len old := by scalar_tac
    simp only [registry.conserves_loop.body, hlt, ite_false, WP.spec_ok]

theorem conservesN_iff (old new : List registry.Entry) (am : List Std.Usize) :
    conservesN old new am = true ↔ ConservesP old new am := by
  simp only [conservesN, keptN, memberN, ConservesP, List.all_eq_true, Bool.or_eq_true,
    List.any_eq_true, Bool.and_eq_true, decide_eq_true_eq]
  constructor
  · intro h o ho
    rcases h o ho with ⟨x, hx, rfl⟩ | h2
    · exact Or.inl hx
    · exact Or.inr h2
  · intro h o ho
    rcases h o ho with h1 | h2
    · exact Or.inl ⟨o.id, h1, rfl⟩
    · exact Or.inr h2

/-- Conserved exactly when every old entry is amended or kept with at least
its strength. -/
theorem conserves_iff (old new : Slice registry.Entry) (am : Slice Std.Usize) :
    registry.conserves old new am ⦃ r => (r = true ↔ ConservesP old.val new.val am.val) ⦄ := by
  apply WP.spec_mono (conserves_spec old new am)
  intro r hr
  rw [hr]
  exact conservesN_iff _ _ _

/-- An unchanged list is conserved without any amendment. -/
theorem conserves_refl (l : Slice registry.Entry) (am : Slice Std.Usize) :
    registry.conserves l l am ⦃ r => r = true ⦄ := by
  apply WP.spec_mono (conserves_iff l l am)
  intro r hr
  exact hr.mpr (fun o ho => Or.inr ⟨o, ho, rfl, le_refl _⟩)

/-- Two conserved changes compose; their amendments accumulate. -/
theorem conserves_trans (a b c : List registry.Entry) (am1 am2 : List Std.Usize)
    (h1 : ConservesP a b am1) (h2 : ConservesP b c am2) : ConservesP a c (am1 ++ am2) := by
  intro o ho
  rcases h1 o ho with hx | ⟨n, hn, hid, hs⟩
  · exact Or.inl (List.mem_append_left _ hx)
  · rcases h2 n hn with hy | ⟨m, hm, hid2, hs2⟩
    · exact Or.inl (List.mem_append_right _ (hid ▸ hy))
    · exact Or.inr ⟨m, hm, hid2.trans hid, le_trans hs hs2⟩

/-- An accepted change that removed or weakened an entry names it in an
amendment. -/
theorem amendment_is_explicit (old new : Slice registry.Entry) (am : Slice Std.Usize)
    (o : registry.Entry) (ho : o ∈ old.val)
    (hlost : ∀ n ∈ new.val, n.id = o.id → n.strength.val < o.strength.val) :
    registry.conserves old new am ⦃ r => r = true → o.id ∈ am.val ⦄ := by
  apply WP.spec_mono (conserves_iff old new am)
  intro r hr hok
  rcases hr.mp hok o ho with h | ⟨n, hn, hid, hs⟩
  · exact h
  · exact absurd hs (Nat.not_le.mpr (hlost n hn hid))

end Tauten.Kernel
