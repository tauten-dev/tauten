import Extracted.TautenCheckKernel

/-!
# The ratchet, as a function on lists (no extracted function)

`Tauten/Ratchet.lean` proves that the extracted `ratchet` functions compute
these functions and states every claim over the extracted functions. The
definitions here mention only the extracted *types* (`Finding`, `Entry`),
never an extracted function, so a mutant of the kernel's Rust code cannot
break this file.
-/

open Aeneas Aeneas.Std Result
open tauten_check_kernel

namespace Tauten.Kernel

/-- The values of a list of counts. -/
def vals (l : List U32) : List Nat := l.map (·.val)

/-- The number of error findings with key `k`. -/
def errorsOf (fs : List ratchet.Finding) (k : Nat) : Nat :=
  fs.countP (fun f => f.error && f.key.val == k)

/-- One finding against the counts left: covered when it is an error and its
key has count left, which then drops by one. -/
def stepN (left : List Nat) (f : ratchet.Finding) : Bool × List Nat :=
  if f.error = true ∧ 0 < left.getD f.key.val 0 then
    (true, left.set f.key.val (left.getD f.key.val 0 - 1))
  else (false, left)

/-- All findings in order: the covered flags and the counts left. -/
def applyN : List Nat → List ratchet.Finding → List Bool × List Nat
  | left, [] => ([], left)
  | left, f :: fs =>
    ((stepN left f).1 :: (applyN (stepN left f).2 fs).1, (applyN (stepN left f).2 fs).2)

/-- Number of covered findings with key `k`. -/
def coveredOf (fs : List ratchet.Finding) (flags : List Bool) (k : Nat) : Nat :=
  (fs.zip flags).countP (fun p => p.2 && p.1.key.val == k)

theorem getD_set_eq (l : List Nat) (j k v : Nat) :
    (l.set j v).getD k 0 = if j = k ∧ j < l.length then v else l.getD k 0 := by
  simp only [List.getD_eq_getElem?_getD, List.getElem?_set]
  by_cases hjk : j = k
  · subst hjk
    by_cases hl : j < l.length
    · simp [hl]
    · simp [hl]
  · simp [hjk]

theorem getD_pos_lt {l : List Nat} {k : Nat} (h : 0 < l.getD k 0) : k < l.length := by
  by_contra hk
  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_none (Nat.le_of_not_lt hk)] at h
  simp at h

theorem stepN_fst (l : List Nat) (f : ratchet.Finding) :
    (stepN l f).1 = (f.error && decide (0 < l.getD f.key.val 0)) := by
  unfold stepN
  cases he : f.error <;> by_cases hp : 0 < l.getD f.key.val 0 <;>
    simp only [hp, Bool.false_eq_true, false_and, true_and, ↓reduceIte, Bool.true_and,
      Bool.false_and, decide_true, decide_false]

theorem stepN_length (l : List Nat) (f : ratchet.Finding) : (stepN l f).2.length = l.length := by
  unfold stepN; split <;> simp

/-- What one step leaves for key `k`. -/
theorem stepN_getD (l : List Nat) (f : ratchet.Finding) (k : Nat) :
    (stepN l f).2.getD k 0 =
      l.getD k 0 - (if f.error = true ∧ f.key.val = k then 1 else 0) := by
  unfold stepN
  split
  · rename_i h
    rw [getD_set_eq]
    by_cases hk : f.key.val = k
    · subst hk; simp [h.1, getD_pos_lt h.2]
    · simp [hk]
  · rename_i h
    by_cases hk : f.error = true ∧ f.key.val = k
    · obtain ⟨he, hk⟩ := hk
      subst hk
      have : l.getD f.key.val 0 = 0 := by
        by_contra h0; exact h ⟨he, Nat.pos_of_ne_zero h0⟩
      rw [this]; simp
    · simp [hk]

theorem applyN_length (l : List Nat) (fs : List ratchet.Finding) :
    (applyN l fs).1.length = fs.length ∧ (applyN l fs).2.length = l.length := by
  induction fs generalizing l with
  | nil => simp [applyN]
  | cons f fs ih =>
    have := ih (stepN l f).2
    simp only [applyN, List.length_cons, this, stepN_length, and_self]

/-- What is left of each key: the accepted count minus the errors, truncated. -/
theorem applyN_getD (l : List Nat) (fs : List ratchet.Finding) (k : Nat) :
    (applyN l fs).2.getD k 0 = l.getD k 0 - errorsOf fs k := by
  induction fs generalizing l with
  | nil => simp [applyN, errorsOf]
  | cons f fs ih =>
    simp only [applyN, ih, stepN_getD, errorsOf, List.countP_cons]
    by_cases hk : f.error = true ∧ f.key.val = k
    · obtain ⟨he, hk⟩ := hk
      simp [he, hk, Nat.sub_sub, Nat.add_comm]
    · have : ¬ (f.error && f.key.val == k) = true := by
        simpa [Bool.and_eq_true, beq_iff_eq] using hk
      simp [hk, this]

/-- The decision for the finding at position `i`: covered iff it is an error
and fewer errors of its key come before it than the accepted count. -/
theorem applyN_covered (l : List Nat) (fs : List ratchet.Finding) (i : Nat)
    (hi : i < fs.length) :
    (applyN l fs).1.getD i false =
      (fs[i].error && decide (errorsOf (fs.take i) fs[i].key.val < l.getD fs[i].key.val 0)) := by
  induction fs generalizing l i with
  | nil => simp at hi
  | cons f fs ih =>
    cases i with
    | zero =>
      simp only [applyN, List.getD_cons_zero, List.getElem_cons_zero, List.take_zero,
        errorsOf, List.countP_nil]
      rw [stepN_fst]
      simp
    | succ i =>
      have hi' : i < fs.length := by simpa using hi
      simp only [applyN, List.getD_cons_succ, List.getElem_cons_succ, List.take_succ_cons]
      rw [ih _ i hi']
      congr 1
      rw [stepN_getD]
      simp only [errorsOf, List.countP_cons]
      by_cases hk : f.error = true ∧ f.key.val = fs[i].key.val
      · obtain ⟨he, hk⟩ := hk
        simp only [he, hk, and_self, ↓reduceIte, Bool.true_and, beq_self_eq_true]
        simp only [decide_eq_decide]
        omega
      · have : ¬ (f.error && f.key.val == fs[i].key.val) = true := by
          simpa [Bool.and_eq_true, beq_iff_eq] using hk
        simp [hk, this]

/-- Per key, the covered findings are the accepted count or the errors,
whichever is smaller. -/
theorem applyN_coveredOf (l : List Nat) (fs : List ratchet.Finding) (k : Nat) :
    coveredOf fs (applyN l fs).1 k = min (l.getD k 0) (errorsOf fs k) := by
  induction fs generalizing l with
  | nil => simp [applyN, coveredOf, errorsOf]
  | cons f fs ih =>
    have hrest := ih (stepN l f).2
    simp only [coveredOf] at hrest ⊢
    simp only [applyN, List.zip_cons_cons, List.countP_cons, hrest]
    rw [stepN_getD]
    simp only [errorsOf, List.countP_cons]
    by_cases hk : f.error = true ∧ f.key.val = k
    · obtain ⟨he, hk⟩ := hk
      subst hk
      by_cases hpos : 0 < l.getD f.key.val 0
      · have h1 : (stepN l f).1 = true := by rw [stepN_fst]; simp only [he, hpos, decide_true, Bool.true_and]
        simp only [h1, he, and_self, ↓reduceIte, Bool.true_and, beq_self_eq_true]
        omega
      · have h1 : (stepN l f).1 = false := by
          rw [stepN_fst]; simp only [he, hpos, decide_false, Bool.true_and]
        simp only [h1, he, and_self, ↓reduceIte, Bool.true_and, Bool.false_and,
          beq_self_eq_true, Bool.false_eq_true]
        omega
    · have : ¬ (f.error && f.key.val == k) = true := by
        simpa [Bool.and_eq_true, beq_iff_eq] using hk
      have hcov : ¬ ((stepN l f).1 && f.key.val == k) = true := by
        unfold stepN; split
        · rename_i h; simpa [h.1] using hk
        · simp
      simp [hk, this, hcov]

/-- The keys with count left, in increasing order. -/
def staleN (left : List Nat) : List Nat :=
  (List.range left.length).filter (fun k => 0 < left.getD k 0)

/-- Saturating count of the error findings per key, for keys `0..n`. -/
def countN (n : Nat) (fs : List ratchet.Finding) : List Nat :=
  (List.range n).map (fun k => min (errorsOf fs k) U32.max)

end Tauten.Kernel
