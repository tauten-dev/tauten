import Tauten.Control

/-!
# Whether a recorded result is still about the current inputs
# (`tauten_check_kernel::binding`)

A result is recorded with one binding (key, identifying bytes) per input it
depends on; the extracted `fresh` decides whether it may be reused.

Proved here, over the extracted functions:
* `same_bytes_spec`: `same_bytes a b` exactly when `a` and `b` are the same
  byte string (length included).
* `contains_bytes_spec`: `contains_bytes list item` exactly when `item` is one of
  the byte strings in `list`.
* `fresh_iff`: fresh exactly when every current input is recorded, every
  recorded input still exists, and every recorded binding has the same
  bytes as each current binding with its key.
* `missing_key_never_fresh`, `extra_key_never_fresh`,
  `changed_never_fresh`: a record that does not mention a current input
  (for example one written before that input was tracked), a record of an
  input that no longer exists, and any changed byte all make it stale.

Trusted, not proved: the shell's choice of inputs and of their identifying
bytes (for example a digest; its collision resistance is an assumption).
-/

open Aeneas Aeneas.Std Result
open tauten_check_kernel

namespace Tauten.Kernel

theorem vec_deref_val {T : Type} (v : alloc.vec.Vec T) : (alloc.vec.Vec.deref v).val = v.val := by
  simp [alloc.vec.Vec.deref]

/-- Byte-by-byte agreement on the first `k` positions. -/
def PrefixSame (a b : List Std.U8) (k : Nat) : Prop := ∀ j (h : j < k) (ha : j < a.length) (hb : j < b.length), a[j] = b[j]

theorem same_bytes_loop_spec (a b : Slice Std.U8) (hlen : a.val.length = b.val.length)
    (same : Bool) (i : Std.Usize) (hi : i.val ≤ a.val.length)
    (inv : same = true ↔ PrefixSame a.val b.val i.val) :
    binding.same_bytes_loop a b same i ⦃ r => (r = true ↔ PrefixSame a.val b.val a.val.length) ⦄ := by
  unfold binding.same_bytes_loop
  apply loop.spec_decr_nat
    (fun p : Bool × Std.Usize => a.val.length - p.2.val)
    (fun p : Bool × Std.Usize => p.2.val ≤ a.val.length ∧ (p.1 = true ↔ PrefixSame a.val b.val p.2.val))
  · rintro ⟨s, k⟩ ⟨hk, hinv⟩
    unfold binding.same_bytes_loop.body
    dsimp only at hk hinv ⊢
    split
    · rename_i hlt
      have hlt' : k.val < a.val.length := by scalar_tac
      have hltb : k.val < b.val.length := by omega
      rw [get_ok a k hlt', get_ok b k hltb]
      have hk2 := usize_sat_add_one k (by scalar_tac)
      simp only [bind_ok, lift]
      cases s
      · simp only [Bool.false_eq_true, ite_false, bind_ok, WP.spec_ok]
        refine ⟨⟨by omega, ?_⟩, by omega⟩
        rw [hk2]
        constructor
        · intro h; cases h
        · intro hp
          exfalso
          have : PrefixSame a.val b.val k.val := fun j hj ha hb => hp j (by omega) ha hb
          exact absurd (hinv.mpr this) (by simp)
      · simp only [ite_true, bind_ok, WP.spec_ok]
        refine ⟨⟨by omega, ?_⟩, by omega⟩
        rw [hk2]
        have hp := hinv.mp rfl
        constructor
        · intro h j hj ha hb
          by_cases hjk : j = k.val
          · subst hjk; simpa using h
          · exact hp j (by omega) ha hb
        · intro h
          simpa using h k.val (by omega) hlt' hltb
    · rename_i hge
      try simp only [WP.spec_ok]
      have : k.val = a.val.length := by scalar_tac
      rw [← this]; exact hinv
  · exact ⟨hi, inv⟩

/-- Same bytes, length included. -/
theorem same_bytes_spec (a b : Slice Std.U8) :
    binding.same_bytes a b ⦃ r => r = decide (a.val = b.val) ⦄ := by
  unfold binding.same_bytes
  by_cases hl : a.val.length = b.val.length
  · have hne : ¬ (Slice.len a != Slice.len b) = true := by
      simp only [bne_iff_ne, ne_eq, Decidable.not_not]; scalar_tac
    simp only [hne, ite_false]
    apply WP.spec_mono (same_bytes_loop_spec a b hl true 0#usize (by simp)
      ⟨fun _ j hj => by simp at hj, fun _ => rfl⟩)
    intro r hr
    have : a.val = b.val ↔ PrefixSame a.val b.val a.val.length := by
      constructor
      · intro h j _ ha hb; simp [h]
      · intro h; apply List.ext_getElem hl; intro j ha hb; exact h j ha ha hb
    cases r <;> simp_all
  · have hne : (Slice.len a != Slice.len b) = true := by
      simp only [bne_iff_ne, ne_eq]; intro h; apply hl; scalar_tac
    simp only [hne, ite_true, WP.spec_ok]
    have : a.val ≠ b.val := fun h => hl (by rw [h])
    simp [this]

/-- `contains_bytes list item` exactly when some entry of `list` is the byte
string `item` (A10, #14). -/
theorem contains_bytes_spec (l : Slice (alloc.vec.Vec Std.U8)) (item : Slice Std.U8) :
    binding.contains_bytes l item ⦃ r => r = l.val.any (fun x => decide (x.val = item.val)) ⦄ := by
  unfold binding.contains_bytes binding.contains_bytes_loop
  apply WP.spec_mono (fold_loop l (fun a x => a || decide (x.val = item.val)) _ ?_ ?_
    false 0#usize (by simp))
  · intro r hr; rw [hr, foldl_or]; simp
  · intro s i h
    have hlt : i < Slice.len l := by scalar_tac
    simp only [binding.contains_bytes_loop.body, hlt, ite_true, get_ok l i h, bind_ok, lift]
    cases s
    · simp only [Bool.false_eq_true, ite_false]
      apply WP.spec_bind (same_bytes_spec _ item)
      intro b hb
      simp [hb, vec_deref_val]
    · simp
  · intro s i h
    have hlt : ¬ i < Slice.len l := by scalar_tac
    simp only [binding.contains_bytes_loop.body, hlt, ite_false, WP.spec_ok]

/-! ## Models -/

def hasKeyN (l : List binding.Binding) (key : Std.Usize) : Bool := l.any (fun b => decide (b.key = key))

def agreesN (l : List binding.Binding) (key : Std.Usize) (bytes : List Std.U8) : Bool :=
  l.all (fun b => decide (b.key ≠ key) || decide (b.bytes.val = bytes))

def coveredN (rec cur : List binding.Binding) : Bool :=
  cur.all (fun c => hasKeyN rec c.key && agreesN rec c.key c.bytes.val)

def presentN (rec cur : List binding.Binding) : Bool := rec.all (fun r => hasKeyN cur r.key)

theorem has_key_spec (l : Slice binding.Binding) (key : Std.Usize) :
    binding.has_key l key ⦃ r => r = hasKeyN l.val key ⦄ := by
  unfold binding.has_key binding.has_key_loop
  apply WP.spec_mono (fold_loop l (fun a b => a || decide (b.key = key)) _ ?_ ?_ false 0#usize (by simp))
  · intro r hr; rw [hr, foldl_or]; simp [hasKeyN]
  · intro s i h
    have hlt : i < Slice.len l := by scalar_tac
    simp only [binding.has_key_loop.body, hlt, ite_true, get_ok l i h, bind_ok, lift]
    cases s <;> simp
  · intro s i h
    have hlt : ¬ i < Slice.len l := by scalar_tac
    simp only [binding.has_key_loop.body, hlt, ite_false, WP.spec_ok]

theorem agrees_spec' (l : Slice binding.Binding) (key : Std.Usize) (bytes : Slice Std.U8) :
    binding.agrees l key bytes ⦃ r => r = agreesN l.val key bytes.val ⦄ := by
  unfold binding.agrees binding.agrees_loop
  apply WP.spec_mono (fold_loop l
    (fun a b => a && (decide (b.key ≠ key) || decide (b.bytes.val = bytes.val))) _ ?_ ?_
    true 0#usize (by simp))
  · intro r hr; rw [hr, foldl_and]; simp [agreesN]
  · intro s i h
    have hlt : i < Slice.len l := by scalar_tac
    simp only [binding.agrees_loop.body, hlt, ite_true, get_ok l i h, bind_ok, lift]
    cases s
    · simp
    · simp only [ite_true, Bool.true_and]
      by_cases hk : (l.val[i.val].key).val = key.val
      · have hk' : l.val[i.val].key = key := by scalar_tac
        rw [if_neg (by simp; scalar_tac)]
        apply WP.spec_bind (same_bytes_spec _ bytes)
        intro b hb
        simp [hb, vec_deref_val]
        try (intro h; exfalso; apply h; scalar_tac)
      · rw [if_pos (by simp; scalar_tac)]
        simp
        try (left; scalar_tac)
  · intro s i h
    have hlt : ¬ i < Slice.len l := by scalar_tac
    simp only [binding.agrees_loop.body, hlt, ite_false, WP.spec_ok]

theorem has_and_agrees (rec : Slice binding.Binding) (key : Std.Usize) (v : alloc.vec.Vec Std.U8) :
    (do let b ← binding.has_key rec key
        if b = true then binding.agrees rec key (alloc.vec.Vec.deref v) else ok false) ⦃ r =>
      r = (hasKeyN rec.val key && agreesN rec.val key v.val) ⦄ := by
  apply WP.spec_bind (has_key_spec rec key)
  intro b hb
  subst hb
  cases h : hasKeyN rec.val key
  · simp [WP.spec_ok]
  · simp only [ite_true, Bool.true_and]
    apply WP.spec_mono (agrees_spec' rec key _)
    intro r hr
    rw [hr, vec_deref_val]

theorem covered_spec (rec cur : Slice binding.Binding) :
    binding.covered rec cur ⦃ r => r = coveredN rec.val cur.val ⦄ := by
  unfold binding.covered binding.covered_loop
  apply WP.spec_mono (fold_loop cur
    (fun a c => a && (hasKeyN rec.val c.key && agreesN rec.val c.key c.bytes.val)) _ ?_ ?_
    true 0#usize (by simp))
  · intro r hr; rw [hr, foldl_and]; simp [coveredN]
  · intro s i h
    have hlt : i < Slice.len cur := by scalar_tac
    simp only [binding.covered_loop.body, hlt, ite_true, get_ok cur i h, bind_ok, lift]
    cases s
    · simp
    · simp only [ite_true, Bool.true_and]
      apply WP.spec_bind (has_and_agrees rec _ _)
      intro b hb
      simp [hb]
  · intro s i h
    have hlt : ¬ i < Slice.len cur := by scalar_tac
    simp only [binding.covered_loop.body, hlt, ite_false, WP.spec_ok]

theorem present_spec (rec cur : Slice binding.Binding) :
    binding.present rec cur ⦃ r => r = presentN rec.val cur.val ⦄ := by
  unfold binding.present binding.present_loop
  apply WP.spec_mono (fold_loop rec (fun a r => a && hasKeyN cur.val r.key) _ ?_ ?_
    true 0#usize (by simp))
  · intro r hr; rw [hr, foldl_and]; simp [presentN]
  · intro s i h
    have hlt : i < Slice.len rec := by scalar_tac
    simp only [binding.present_loop.body, hlt, ite_true, get_ok rec i h, bind_ok, lift]
    cases s
    · simp
    · simp only [ite_true, Bool.true_and]
      apply WP.spec_bind (has_key_spec cur _)
      intro b hb
      simp [hb]
  · intro s i h
    have hlt : ¬ i < Slice.len rec := by scalar_tac
    simp only [binding.present_loop.body, hlt, ite_false, WP.spec_ok]

/-- What `fresh` computes, as a proposition. -/
def FreshP (rec cur : List binding.Binding) : Prop :=
  (∀ c ∈ cur, ∃ r ∈ rec, r.key = c.key) ∧
  (∀ r ∈ rec, ∀ c ∈ cur, r.key = c.key → r.bytes.val = c.bytes.val) ∧
  (∀ r ∈ rec, ∃ c ∈ cur, c.key = r.key)

/-- Fresh exactly when the keys match both ways and every pair of bindings
with one key has the same bytes. -/
theorem fresh_iff (rec cur : Slice binding.Binding) :
    binding.fresh rec cur ⦃ r => (r = true ↔ FreshP rec.val cur.val) ⦄ := by
  unfold binding.fresh
  apply WP.spec_bind (covered_spec rec cur)
  intro b hb
  have hcov : b = true ↔ (∀ c ∈ cur.val, ∃ r ∈ rec.val, r.key = c.key) ∧
      (∀ r ∈ rec.val, ∀ c ∈ cur.val, r.key = c.key → r.bytes.val = c.bytes.val) := by
    subst hb
    simp only [coveredN, hasKeyN, agreesN, List.all_eq_true, List.any_eq_true, Bool.and_eq_true,
      decide_eq_true_eq, Bool.or_eq_true, ne_eq]
    constructor
    · intro h
      refine ⟨fun c hc => (h c hc).1, fun r hr c hc hk => ?_⟩
      rcases (h c hc).2 r hr with h1 | h1
      · exact absurd hk h1
      · exact h1
    · intro ⟨h1, h2⟩ c hc
      refine ⟨h1 c hc, fun r hr => ?_⟩
      by_cases hk : r.key = c.key
      · exact Or.inr (h2 r hr c hc hk)
      · exact Or.inl hk
  cases b
  · simp only [Bool.false_eq_true, ite_false, WP.spec_ok]
    simp only [FreshP]
    constructor
    · intro h; cases h
    · intro ⟨h1, h2, _⟩; exact absurd (hcov.mpr ⟨h1, h2⟩) (by simp)
  · simp only [ite_true]
    apply WP.spec_mono (present_spec rec cur)
    intro r hr
    subst hr
    obtain ⟨h1, h2⟩ := hcov.mp rfl
    simp only [FreshP, presentN, hasKeyN, List.all_eq_true, List.any_eq_true, decide_eq_true_eq]
    constructor
    · intro h; exact ⟨h1, h2, h⟩
    · intro ⟨_, _, h3⟩; exact h3

/-- A current input the record does not mention makes it stale. -/
theorem missing_key_never_fresh (rec cur : Slice binding.Binding) (c : binding.Binding)
    (hc : c ∈ cur.val) (hm : ∀ r ∈ rec.val, r.key ≠ c.key) :
    binding.fresh rec cur ⦃ r => r = false ⦄ := by
  apply WP.spec_mono (fresh_iff rec cur)
  intro r hr
  cases r
  · rfl
  · obtain ⟨h1, _, _⟩ := hr.mp rfl
    obtain ⟨r', hr', hk⟩ := h1 c hc
    exact absurd hk (hm r' hr')

/-- A recorded input that no longer exists makes it stale. -/
theorem extra_key_never_fresh (rec cur : Slice binding.Binding) (r0 : binding.Binding)
    (hr0 : r0 ∈ rec.val) (hm : ∀ c ∈ cur.val, c.key ≠ r0.key) :
    binding.fresh rec cur ⦃ r => r = false ⦄ := by
  apply WP.spec_mono (fresh_iff rec cur)
  intro r hr
  cases r
  · rfl
  · obtain ⟨_, _, h3⟩ := hr.mp rfl
    obtain ⟨c, hc, hk⟩ := h3 r0 hr0
    exact absurd hk (hm c hc)

/-- A changed input makes it stale. -/
theorem changed_never_fresh (rec cur : Slice binding.Binding) (r0 c : binding.Binding)
    (hr0 : r0 ∈ rec.val) (hc : c ∈ cur.val) (hk : r0.key = c.key) (hb : r0.bytes.val ≠ c.bytes.val) :
    binding.fresh rec cur ⦃ r => r = false ⦄ := by
  apply WP.spec_mono (fresh_iff rec cur)
  intro r hr
  cases r
  · rfl
  · obtain ⟨_, h2, _⟩ := hr.mp rfl
    exact absurd (h2 r0 hr0 c hc hk) hb

end Tauten.Kernel
