import Tauten.Ratchet

/-!
# Declared effects against the signature (`tauten_check_kernel::effects`, TAU-F001)

`tauten-check` classifies every parameter of a `pub fn` as an
`effects.Source` (expanding `#[derive(Cap)]` bundles into their fields),
then calls the extracted `derive` for the row the signature grants and
`agrees` / `missing` / `extra` for the finding.

Proved here, over the extracted functions:
* `derive_spec`: the derived row has a label exactly when some parameter
  grants it (`clock`, `env`, `net` from their sources; `fs_read` from `Dir`
  or `ReadDir`; `fs_write` from `Dir` only), plus `fail` and `suspend` from
  the signature's `Result` and `async`.
* `agrees_spec`, `f001_decision`: no finding exactly when the declared row
  equals the derived row, so a matching declaration is never reported (no
  false positive) and a differing one always is (no false negative).
* `derive_perm`, `derive_mono`: parameter order does not matter, and adding
  parameters never removes a label.
* `f001_error_iff`, `f001_silent_iff`: the verdict blocks exactly when the
  crate requires effects and the declaration is absent, or differs from a
  fully classified signature; an unclassified parameter can only warn.
* `missing_spec`, `extra_spec`: the finding names exactly the labels the
  signature grants but the declaration omits, and the reverse.

Trusted, not proved: the shell's classification of a parameter type as a
source (by type name; `docs/proofs/README.md`).
-/

open Aeneas Aeneas.Std Result
open tauten_check_kernel

namespace Tauten.Kernel

/-- Sources that grant each label. -/
def grantsClock : effects.Source → Bool
  | .Clock => true | .Env => false | .Dir => false | .ReadDir => false | .Net => false | .Plain => false
def grantsEnv : effects.Source → Bool
  | .Clock => false | .Env => true | .Dir => false | .ReadDir => false | .Net => false | .Plain => false
def grantsRead : effects.Source → Bool
  | .Clock => false | .Env => false | .Dir => true | .ReadDir => true | .Net => false | .Plain => false
def grantsWrite : effects.Source → Bool
  | .Clock => false | .Env => false | .Dir => true | .ReadDir => false | .Net => false | .Plain => false
def grantsNet : effects.Source → Bool
  | .Clock => false | .Env => false | .Dir => false | .ReadDir => false | .Net => true | .Plain => false

/-- One source added to a row. -/
def addN (r : effects.Row) (s : effects.Source) : effects.Row :=
  { r with
    clock := r.clock || grantsClock s
    env := r.env || grantsEnv s
    fs_read := r.fs_read || grantsRead s
    fs_write := r.fs_write || grantsWrite s
    net := r.net || grantsNet s }

/-- The row a signature grants, as a function on lists. -/
def deriveN (ss : List effects.Source) (fallible asynchronous : Bool) : effects.Row :=
  { clock := ss.any grantsClock
    env := ss.any grantsEnv
    fs_read := ss.any grantsRead
    fs_write := ss.any grantsWrite
    net := ss.any grantsNet
    fail := fallible
    suspend := asynchronous }

theorem add_spec (r : effects.Row) (s : effects.Source) :
    effects.add r s = ok (addN r s) := by
  rcases r with ⟨c, e, fr, fw, n, f, su⟩
  cases s <;> cases c <;> cases e <;> cases fr <;> cases fw <;> cases n <;>
    simp [effects.add, addN, grantsClock, grantsEnv, grantsRead, grantsWrite, grantsNet]

theorem add_at_spec (r : effects.Row) (ss : Slice effects.Source) (i : Usize)
    (hi : i.val < ss.val.length) :
    effects.add_at r ss i = ok (addN r ss.val[i.val]) := by
  unfold effects.add_at
  simp only [core.slice.Slice.get, core.slice.index.SliceIndexUsizeSlice,
    core.slice.index.Usize.get, Slice.getElem?_Usize_eq, bind_tc_ok, bind_ok,
    List.getElem?_eq_getElem hi]
  exact add_spec r _

theorem foldl_addN_fields (r : effects.Row) (l : List effects.Source) :
    l.foldl addN r =
      { clock := r.clock || l.any grantsClock
        env := r.env || l.any grantsEnv
        fs_read := r.fs_read || l.any grantsRead
        fs_write := r.fs_write || l.any grantsWrite
        net := r.net || l.any grantsNet
        fail := r.fail
        suspend := r.suspend } := by
  induction l generalizing r with
  | nil => cases r; simp
  | cons x t ih =>
    rw [List.foldl_cons, ih]
    simp [addN, Bool.or_assoc]

theorem derive_loop_spec (ss : Slice effects.Source) (r : effects.Row) (i : Usize)
    (hi : i.val ≤ ss.val.length) :
    effects.derive_loop ss r i ⦃ out => out = (ss.val.drop i.val).foldl addN r ⦄ := by
  unfold effects.derive_loop
  apply loop.spec_decr_nat
    (fun p : effects.Row × Usize => ss.val.length - p.2.val)
    (fun p : effects.Row × Usize => p.2.val ≤ ss.val.length ∧
      (ss.val.drop p.2.val).foldl addN p.1 = (ss.val.drop i.val).foldl addN r)
  · rintro ⟨row, k⟩ ⟨hk, hinv⟩
    unfold effects.derive_loop.body
    dsimp only at hk hinv ⊢
    split
    · rename_i hlt
      have hlt' : k.val < ss.val.length := by scalar_tac
      rw [add_at_spec row ss k hlt']
      have hk2 := usize_sat_add_one k (by scalar_tac)
      simp only [bind_ok, lift, bind_tc_ok, WP.spec_ok]
      refine ⟨⟨by omega, ?_⟩, by omega⟩
      rw [hk2, ← hinv, List.drop_eq_getElem_cons hlt', List.foldl_cons]
    · rename_i hge
      try simp only [WP.spec_ok]
      have : k.val = ss.val.length := by scalar_tac
      rw [← hinv, this]
      simp
  · exact ⟨hi, rfl⟩

/-- The derived row: a label exactly when some parameter grants it, plus
`fail` and `suspend` from the signature. -/
theorem derive_spec (ss : Slice effects.Source) (fallible asynchronous : Bool) :
    effects.derive ss fallible asynchronous ⦃ r => r = deriveN ss.val fallible asynchronous ⦄ := by
  unfold effects.derive effects.empty
  simp only [bind_ok]
  apply WP.spec_mono (derive_loop_spec ss _ 0#usize (by simp))
  intro out h
  subst h
  simp [foldl_addN_fields, deriveN]

/-- `missing`, field by field. -/
theorem missing_spec (d g : effects.Row) :
    effects.missing d g = ok
      { clock := g.clock && !d.clock, env := g.env && !d.env,
        fs_read := g.fs_read && !d.fs_read, fs_write := g.fs_write && !d.fs_write,
        net := g.net && !d.net, fail := g.fail && !d.fail, suspend := g.suspend && !d.suspend } := by
  rcases d with ⟨c, e, fr, fw, n, f, su⟩
  rcases g with ⟨c', e', fr', fw', n', f', su'⟩
  cases c' <;> cases e' <;> cases fr' <;> cases fw' <;> cases n' <;> cases f' <;> cases su' <;>
    simp [effects.missing]

/-- `extra` is `missing` with the roles swapped. -/
theorem extra_spec (d g : effects.Row) :
    effects.extra d g = ok
      { clock := d.clock && !g.clock, env := d.env && !g.env,
        fs_read := d.fs_read && !g.fs_read, fs_write := d.fs_write && !g.fs_write,
        net := d.net && !g.net, fail := d.fail && !g.fail, suspend := d.suspend && !g.suspend } := by
  unfold effects.extra
  exact missing_spec g d

theorem is_empty_spec (r : effects.Row) :
    effects.is_empty r = ok (!(r.clock || r.env || r.fs_read || r.fs_write || r.net || r.fail || r.suspend)) := by
  rcases r with ⟨c, e, fr, fw, n, f, su⟩
  cases c <;> cases e <;> cases fr <;> cases fw <;> cases n <;> cases f <;> cases su <;>
    simp [effects.is_empty]

/-- Field-by-field equality of rows. -/
def rowEq (d g : effects.Row) : Bool :=
  d.clock == g.clock && d.env == g.env && d.fs_read == g.fs_read && d.fs_write == g.fs_write &&
    d.net == g.net && d.fail == g.fail && d.suspend == g.suspend

theorem rowEq_iff (d g : effects.Row) : rowEq d g = true ↔ d = g := by
  rcases d with ⟨c, e, fr, fw, n, f, su⟩
  rcases g with ⟨c', e', fr', fw', n', f', su'⟩
  simp [rowEq, and_assoc]

/-- No finding exactly when the declaration equals the derived row: no
false positive, no false negative. -/
theorem agrees_spec (d g : effects.Row) :
    effects.agrees d g = ok (rowEq d g) := by
  unfold effects.agrees
  rw [missing_spec, extra_spec]
  simp only [bind_ok, is_empty_spec]
  rcases d with ⟨c, e, fr, fw, n, f, su⟩
  rcases g with ⟨c', e', fr', fw', n', f', su'⟩
  simp only [rowEq]
  -- Seven independent fields: case on each pair, kernel-checked.
  cases c <;> cases c' <;> cases e <;> cases e' <;> simp <;>
    cases fr <;> cases fr' <;> simp <;> cases fw <;> cases fw' <;> simp <;>
    cases n <;> cases n' <;> simp <;> cases f <;> cases f' <;> simp <;>
    cases su <;> cases su' <;> simp

/-- The end-to-end decision for one function: given the parameter sources,
the declaration is accepted exactly when it equals `deriveN`. -/
theorem f001_decision (ss : Slice effects.Source) (fallible asynchronous : Bool) (d : effects.Row) :
    (do let g ← effects.derive ss fallible asynchronous; effects.agrees d g)
      ⦃ b => b = true ↔ d = deriveN ss.val fallible asynchronous ⦄ := by
  apply WP.spec_bind (derive_spec ss fallible asynchronous)
  intro g hg
  subst hg
  rw [agrees_spec]
  simp only [WP.spec_ok]
  exact rowEq_iff _ _

/-- Non-vacuity: a directory parameter grants writing, a read-only one does
not, and a declaration that omits a granted label is refused. -/
theorem f001_examples :
    (deriveN [.Dir] false false).fs_write = true ∧
    (deriveN [.ReadDir, .Plain] true false).fs_write = false ∧
    (deriveN [.ReadDir, .Plain] true false).fail = true ∧
    effects.agrees (deriveN [.ReadDir] false false) (deriveN [.Dir] false false) = ok false := by
  refine ⟨by decide, by decide, by decide, ?_⟩
  rw [agrees_spec]
  simp [rowEq, deriveN, grantsClock, grantsEnv, grantsRead, grantsWrite, grantsNet]

/-- Order does not matter: permuting the parameters derives the same row. -/
theorem derive_perm {ss ts : List effects.Source} (h : ss.Perm ts) (fallible asynchronous : Bool) :
    deriveN ss fallible asynchronous = deriveN ts fallible asynchronous := by
  have hany : ∀ p : effects.Source → Bool, ss.any p = ts.any p := by
    intro p
    apply Bool.eq_iff_iff.mpr
    simp only [List.any_eq_true]
    exact ⟨fun ⟨x, hx, px⟩ => ⟨x, h.mem_iff.mp hx, px⟩,
      fun ⟨x, hx, px⟩ => ⟨x, h.mem_iff.mpr hx, px⟩⟩
  simp [deriveN, hany]

/-- Monotone: every label granted by a parameter list is still granted
when parameters are added. -/
theorem derive_mono {ss ts : List effects.Source} (h : ∀ x, x ∈ ss → x ∈ ts)
    (p : effects.Source → Bool) :
    ss.any p = true → ts.any p = true := by
  simp only [List.any_eq_true]
  exact fun ⟨x, hx, px⟩ => ⟨x, h x hx, px⟩

/-- The verdict as a function. -/
def verdictN (declared : Option effects.Row) (derived : effects.Row) (required resolved : Bool) :
    effects.Verdict :=
  match declared with
  | none => if required then .Error else .Silent
  | some d => if rowEq d derived then .Silent else if required && resolved then .Error else .Warning

theorem verdict_spec (declared : Option effects.Row) (derived : effects.Row) (required resolved : Bool) :
    effects.verdict declared derived required resolved = ok (verdictN declared derived required resolved) := by
  cases declared with
  | none => cases required <;> simp [effects.verdict, verdictN]
  | some d =>
    simp only [effects.verdict, verdictN, agrees_spec, bind_ok]
    cases rowEq d derived <;> cases required <;> cases resolved <;> simp

/-- A blocking finding exactly when the crate requires effects and the
function either declares none, or declares a row that differs from its
fully classified signature. -/
theorem f001_error_iff (declared : Option effects.Row) (derived : effects.Row) (required resolved : Bool) :
    effects.verdict declared derived required resolved = ok .Error ↔
      required = true ∧ (declared = none ∨ (resolved = true ∧ declared ≠ some derived)) := by
  rw [verdict_spec]
  cases declared with
  | none => cases required <;> simp [verdictN]
  | some d =>
    have h := rowEq_iff d derived
    cases hr : rowEq d derived <;> cases required <;> cases resolved <;> simp_all [verdictN]

/-- Silent exactly when the declaration matches, or there is none and none
is required: a matching declaration is never reported. -/
theorem f001_silent_iff (declared : Option effects.Row) (derived : effects.Row) (required resolved : Bool) :
    effects.verdict declared derived required resolved = ok .Silent ↔
      declared = some derived ∨ (declared = none ∧ required = false) := by
  rw [verdict_spec]
  cases declared with
  | none => cases required <;> simp [verdictN]
  | some d =>
    have h := rowEq_iff d derived
    cases hr : rowEq d derived <;> cases required <;> cases resolved <;> simp_all [verdictN]

end Tauten.Kernel
