import Extracted.TautenProofInstances
import Tauten.MacroShapes

/-!
# Contract instances (P3)

Over the Aeneas translation of `crates/tauten-proof-instances/src/contracts.rs`,
which is the real expansion of `#[tauten::requires(..)]` and
`#[tauten::ensures(..)]` with `old(..)`. In the expansion the body runs in an
immediately invoked closure; Aeneas translates that closure, and `body`
below is the translated closure applied to the arguments, that is, the
function's own body.

For each function:
- `*_shape`: the extracted function equals `Tauten.Macros.contractCall`
  (or `contractCallMut` for the `&mut` parameter) with the precondition,
  the postcondition and the exact `ContractViolation` the macro emits. The
  postcondition is stated over the parameters' values *at entry*: that is
  what `old(..)` must mean, and a snapshot taken after the body breaks it.
- the user-level claims (requires, ensures, pass-through, result), proved
  through the shape and a specification of the body.

What the four instances witness:
- `deposit`: requires, Err pass-through (overflow); the postcondition holds
  whenever the precondition does, so `Ensures` never occurs.
- `withdraw` (`&mut u64`): a refused precondition leaves the balance as it
  was; `old(*balance)` is the balance at entry, not after the body's write.
- `scale`: all three outcomes are reachable; `Ensures` exactly for a zero
  factor on a non-zero value.
- `share`: the body divides and would panic on a zero divisor; the wrapper
  returns `Requires` instead, so the body is not evaluated.
-/

open Aeneas Aeneas.Std Result
open tauten_proof_instances
open Tauten.Macros

namespace Tauten.Instances

/-- A contract violation of the precondition, as the caller sees it. -/
def IsRequires {T : Type} (r : core.result.Result T contracts.OpError) : Prop :=
  ∃ f c, r = .Err (.Contract (.Requires f c))

/-- A contract violation of the postcondition, as the caller sees it. -/
def IsEnsures {T : Type} (r : core.result.Result T contracts.OpError) : Prop :=
  ∃ f c, r = .Err (.Contract (.Ensures f c))

/-! ## `deposit(balance: u64, amount: u64)`
`#[requires(amount > 0)]`, `#[ensures(|after: &u64| *after > old(balance))]` -/

theorem deposit_shape (b a : Std.U64) :
    contracts.deposit b a
      = contractCall (decide (a.val ≠ 0))
          (.Contract (.Requires (toStr "deposit") (toStr "amount > 0")))
          (fun t : Std.U64 => decide (b.val < t.val))
          (.Contract (.Ensures (toStr "deposit")
            (toStr "| after : & u64 | * after > old(balance)")))
          (contracts.deposit.closure.Insts.CoreOpsFunctionFnTupleResultU64OpError.call (b, a) ()) := by
  unfold contracts.deposit contractCall
  by_cases h : a.val = 0
  · have : ¬ a > 0#u64 := by scalar_tac
    simp [this, h, tauten.error.ContractViolation.requires,
      contracts.OpError.Insts.CoreConvertFromContractViolation.from]
  · have : a > 0#u64 := by scalar_tac
    simp only [this, h, ite_true, decide_true, decide_not, Bool.not_false, tauten.contract.snapshot]
    simp only [bind_ok]
    congr 1
    funext r
    cases r with
    | Ok t =>
      by_cases hp : b.val < t.val
      · have : t > b := by scalar_tac
        simp [hp, this, tauten.contract.holds,
          contracts.deposit.closure_1.Insts.CoreOpsFunctionFnOnceTupleShared1U64Bool.call_once]
      · have : ¬ t > b := by scalar_tac
        simp [hp, this, tauten.contract.holds,
          contracts.deposit.closure_1.Insts.CoreOpsFunctionFnOnceTupleShared1U64Bool.call_once,
          tauten.error.ContractViolation.ensures,
          contracts.OpError.Insts.CoreConvertFromContractViolation.from]
    | Err e => simp

/-- The body: `balance.checked_add(amount).ok_or(Overflow)`. -/
theorem deposit_body (b a : Std.U64) :
    (∃ c, contracts.deposit.closure.Insts.CoreOpsFunctionFnTupleResultU64OpError.call (b, a) ()
        = ok (.Ok c) ∧ c.val = b.val + a.val ∧ b.val + a.val ≤ U64.max) ∨
    (contracts.deposit.closure.Insts.CoreOpsFunctionFnTupleResultU64OpError.call (b, a) ()
        = ok (.Err .Overflow) ∧ U64.max < b.val + a.val) := by
  have h := U64.checked_add_bv_spec b a
  unfold contracts.deposit.closure.Insts.CoreOpsFunctionFnTupleResultU64OpError.call
  cases hc : U64.checked_add b a with
  | none => right; simp_all [lift]
  | some c => left; simp_all [lift]

theorem deposit_requires_iff (b a : Std.U64) :
    (∃ r, contracts.deposit b a = ok r ∧ IsRequires r) ↔ a.val = 0 := by
  rw [deposit_shape]
  by_cases h : a.val = 0
  · simp [h, contract_requires, IsRequires]
  · rcases deposit_body b a with ⟨c, hc, hv, _⟩ | ⟨hc, _⟩
    · have : b.val < c.val := by omega
      simp [contractCall, h, hc, IsRequires, this]
    · simp [contractCall, h, hc, IsRequires]

/-- The postcondition never fails: with a positive amount, the sum is larger. -/
theorem deposit_never_ensures (b a : Std.U64) :
    ¬ ∃ r, contracts.deposit b a = ok r ∧ IsEnsures r := by
  rw [deposit_shape]
  by_cases h : a.val = 0
  · simp [h, contract_requires, IsEnsures]
  · rcases deposit_body b a with ⟨c, hc, hv, _⟩ | ⟨hc, _⟩
    · have : b.val < c.val := by omega
      simp [contractCall, h, hc, IsEnsures, this]
    · simp [contractCall, h, hc, IsEnsures]

/-- With a positive amount and no overflow: the body's sum. -/
theorem deposit_ok (b a : Std.U64) (ha : a.val ≠ 0) (hb : b.val + a.val ≤ U64.max) :
    ∃ c, contracts.deposit b a = ok (.Ok c) ∧ c.val = b.val + a.val := by
  rw [deposit_shape]
  rcases deposit_body b a with ⟨c, hc, hv, _⟩ | ⟨hc, hm⟩
  · have : b.val < c.val := by omega
    exact ⟨c, by simp [contractCall, ha, hc, this], hv⟩
  · omega

/-- With a positive amount and overflow: the body's `Err`, unchanged. -/
theorem deposit_overflow (b a : Std.U64) (ha : a.val ≠ 0) (hb : U64.max < b.val + a.val) :
    contracts.deposit b a = ok (.Err .Overflow) := by
  rw [deposit_shape]
  rcases deposit_body b a with ⟨c, hc, hv, hm⟩ | ⟨hc, hm⟩
  · omega
  · simp [contractCall, ha, hc]

/-! ## `withdraw(balance: &mut u64, amount: u64)`
`#[requires(amount > 0)]`, `#[ensures(|after: &u64| *after < old(*balance))]`.
Aeneas translates the `&mut` parameter as a value in and a value out
(`withdraw b a = ok (result, balance at exit)`), and the body closure,
which captures `balance` mutably, as `call_mut` plus a backward function
that gives back the balance it wrote. -/

/-- The body as the caller sees it: its result and the balance it leaves. -/
def withdrawBody (b a : Std.U64) :
    Result (core.result.Result Std.U64 contracts.OpError × Std.U64) := do
  let (r, c, back) ←
    contracts.withdraw.closure.Insts.CoreOpsFunctionFnMutTupleResultU64OpError.call_mut (b, a) ()
  ok (r, (back c).1)

/-- The body: subtract and store, or `Insufficient` with the balance untouched. -/
theorem withdraw_body (b a : Std.U64) :
    (∃ n, withdrawBody b a = ok (.Ok n, n) ∧ n.val = b.val - a.val ∧ a.val ≤ b.val) ∨
    (withdrawBody b a = ok (.Err .Insufficient, b) ∧ b.val < a.val) := by
  have h := U64.checked_sub_bv_spec b a
  unfold withdrawBody contracts.withdraw.closure.Insts.CoreOpsFunctionFnMutTupleResultU64OpError.call_mut
  cases hc : U64.checked_sub b a with
  | none =>
    right
    simp only [hc] at h
    refine ⟨?_, h⟩
    simp [hc, lift, core.option.Option.ok_or, core.result.Result.Insts.CoreOpsTry.branch,
      core.result.Result.Insts.CoreOpsTry_traitFromResidualResult.from_residual] <;> rfl
  | some n =>
    left
    simp only [hc] at h
    refine ⟨n, ?_, h.2.1, h.1⟩
    simp [hc, lift, core.option.Option.ok_or, core.result.Result.Insts.CoreOpsTry.branch] <;> rfl

/-- `old(*balance)` is `b`, the balance at entry, not the balance the body
wrote. -/
theorem withdraw_shape (b a : Std.U64) :
    contracts.withdraw b a
      = contractCallMut (decide (a.val ≠ 0))
          (.Contract (.Requires (toStr "withdraw") (toStr "amount > 0")))
          (fun t : Std.U64 => decide (t.val < b.val))
          (.Contract (.Ensures (toStr "withdraw")
            (toStr "| after : & u64 | * after < old(* balance)")))
          b (withdrawBody b a) := by
  unfold contracts.withdraw contractCallMut
  by_cases h : a.val = 0
  · have : ¬ a > 0#u64 := by scalar_tac
    simp [this, h, tauten.error.ContractViolation.requires,
      contracts.OpError.Insts.CoreConvertFromContractViolation.from]
  · have : a > 0#u64 := by scalar_tac
    have hd : decide (a.val ≠ 0) = true := by simp [h]
    simp only [this, hd, ite_true, tauten.contract.snapshot, withdrawBody, bind_tc_eq, bind_ok,
      Std.bind_assoc]
    congr 1
    funext x
    obtain ⟨r, c, back⟩ := x
    cases r with
    | Ok t =>
      by_cases hp : t.val < b.val
      · have : t < b := by scalar_tac
        simp [hp, this, tauten.contract.holds,
          contracts.withdraw.closure_1.Insts.CoreOpsFunctionFnOnceTupleShared1U64Bool.call_once] <;> rfl
      · have : ¬ t < b := by scalar_tac
        simp [hp, this, tauten.contract.holds,
          contracts.withdraw.closure_1.Insts.CoreOpsFunctionFnOnceTupleShared1U64Bool.call_once,
          tauten.error.ContractViolation.ensures,
          contracts.OpError.Insts.CoreConvertFromContractViolation.from] <;> rfl
    | Err e => simp <;> rfl

/-- A zero amount is refused, and the balance is left as it was. -/
theorem withdraw_requires_iff (b a : Std.U64) :
    (∃ r b', contracts.withdraw b a = ok (r, b') ∧ IsRequires r) ↔ a.val = 0 := by
  rw [withdraw_shape]
  by_cases h : a.val = 0
  · simp [h, contract_mut_requires, IsRequires]
  · rcases withdraw_body b a with ⟨n, hn, hv, hle⟩ | ⟨hn, _⟩
    · have : n.val < b.val := by omega
      simp [contractCallMut, h, hn, IsRequires, this]
    · simp [contractCallMut, h, hn, IsRequires]

theorem withdraw_requires_keeps_balance (b : Std.U64) :
    contracts.withdraw b 0#u64
      = ok (.Err (.Contract (.Requires (toStr "withdraw") (toStr "amount > 0"))), b) := by
  rw [withdraw_shape]; simp [contract_mut_requires]

/-- The postcondition never fails: a positive withdrawal lowers the balance. -/
theorem withdraw_never_ensures (b a : Std.U64) :
    ¬ ∃ r b', contracts.withdraw b a = ok (r, b') ∧ IsEnsures r := by
  rw [withdraw_shape]
  by_cases h : a.val = 0
  · simp [h, contract_mut_requires, IsEnsures]
  · rcases withdraw_body b a with ⟨n, hn, hv, hle⟩ | ⟨hn, _⟩
    · have : n.val < b.val := by omega
      simp [contractCallMut, h, hn, IsEnsures, this]
    · simp [contractCallMut, h, hn, IsEnsures]

/-- A positive amount within the balance: the new balance, returned and stored. -/
theorem withdraw_ok (b a : Std.U64) (ha : a.val ≠ 0) (hb : a.val ≤ b.val) :
    ∃ n, contracts.withdraw b a = ok (.Ok n, n) ∧ n.val = b.val - a.val := by
  rw [withdraw_shape]
  rcases withdraw_body b a with ⟨n, hn, hv, hle⟩ | ⟨hn, hm⟩
  · have : n.val < b.val := by omega
    exact ⟨n, by simp [contractCallMut, ha, hn, this], hv⟩
  · omega

/-- More than the balance: the body's `Err`, unchanged, balance untouched. -/
theorem withdraw_insufficient (b a : Std.U64) (ha : a.val ≠ 0) (hb : b.val < a.val) :
    contracts.withdraw b a = ok (.Err .Insufficient, b) := by
  rw [withdraw_shape]
  rcases withdraw_body b a with ⟨n, hn, hv, hle⟩ | ⟨hn, hm⟩
  · omega
  · simp [contractCallMut, ha, hn]

/-! ## `scale(value: u32, factor: u32)`
`#[requires(factor <= 1000)]`, `#[ensures(|scaled: &u32| *scaled >= old(value))]` -/

theorem scale_shape (v k : Std.U32) :
    contracts.scale v k
      = contractCall (decide (k.val ≤ 1000))
          (.Contract (.Requires (toStr "scale") (toStr "factor <= 1000")))
          (fun t : Std.U32 => decide (v.val ≤ t.val))
          (.Contract (.Ensures (toStr "scale")
            (toStr "| scaled : & u32 | * scaled >= old(value)")))
          (contracts.scale.closure.Insts.CoreOpsFunctionFnTupleResultU32OpError.call (v, k) ()) := by
  unfold contracts.scale contractCall
  by_cases h : k.val ≤ 1000
  · have : k <= 1000#u32 := by scalar_tac
    simp only [this, h, ite_true, decide_true, tauten.contract.snapshot]
    simp only [bind_ok]
    congr 1
    funext r
    cases r with
    | Ok t =>
      by_cases hp : v.val ≤ t.val
      · have : t >= v := by scalar_tac
        simp [hp, this, tauten.contract.holds,
          contracts.scale.closure_1.Insts.CoreOpsFunctionFnOnceTupleShared1U32Bool.call_once]
      · have : ¬ t >= v := by scalar_tac
        simp [hp, this, tauten.contract.holds,
          contracts.scale.closure_1.Insts.CoreOpsFunctionFnOnceTupleShared1U32Bool.call_once,
          tauten.error.ContractViolation.ensures,
          contracts.OpError.Insts.CoreConvertFromContractViolation.from]
    | Err e => simp
  · have : ¬ k <= 1000#u32 := by scalar_tac
    simp [this, h, tauten.error.ContractViolation.requires,
      contracts.OpError.Insts.CoreConvertFromContractViolation.from]

/-- The body: `value.checked_mul(factor).ok_or(Overflow)`. -/
theorem scale_body (v k : Std.U32) :
    (∃ c, contracts.scale.closure.Insts.CoreOpsFunctionFnTupleResultU32OpError.call (v, k) ()
        = ok (.Ok c) ∧ c.val = v.val * k.val ∧ v.val * k.val ≤ U32.max) ∨
    (contracts.scale.closure.Insts.CoreOpsFunctionFnTupleResultU32OpError.call (v, k) ()
        = ok (.Err .Overflow) ∧ U32.max < v.val * k.val) := by
  have h := U32.checked_mul_bv_spec v k
  unfold contracts.scale.closure.Insts.CoreOpsFunctionFnTupleResultU32OpError.call
  cases hc : U32.checked_mul v k with
  | none => right; simp_all [lift]
  | some c => left; simp_all [lift]

theorem scale_requires_iff (v k : Std.U32) :
    (∃ r, contracts.scale v k = ok r ∧ IsRequires r) ↔ 1000 < k.val := by
  rw [scale_shape]
  by_cases h : k.val ≤ 1000
  · have hk : ¬ 1000 < k.val := by omega
    simp only [hk, iff_false]
    rcases scale_body v k with ⟨c, hc, hv, _⟩ | ⟨hc, _⟩
    · by_cases hp : v.val ≤ c.val <;> simp [contractCall, h, hc, IsRequires, hp]
    · simp [contractCall, h, hc, IsRequires]
  · have hk : 1000 < k.val := by omega
    simp [h, hk, contract_requires, IsRequires]

/-- The postcondition fails exactly for a zero factor on a non-zero value. -/
theorem scale_ensures_iff (v k : Std.U32) :
    (∃ r, contracts.scale v k = ok r ∧ IsEnsures r) ↔ k.val = 0 ∧ v.val ≠ 0 := by
  rw [scale_shape]
  by_cases h : k.val ≤ 1000
  · rcases scale_body v k with ⟨c, hc, hv, _⟩ | ⟨hc, hm⟩
    · by_cases hp : v.val ≤ c.val
      · have : ¬ (k.val = 0 ∧ v.val ≠ 0) := by
          intro ⟨h0, h1⟩; simp [h0] at hv; omega
        simp [contractCall, h, hc, IsEnsures, hp, this]
      · have : k.val = 0 ∧ v.val ≠ 0 := by
          constructor
          · by_contra h0
            have : v.val * 1 ≤ v.val * k.val := Nat.mul_le_mul_left _ (by omega)
            omega
          · intro h0; simp [h0] at hv; omega
        simp [contractCall, h, hc, IsEnsures, hp, this]
    · have : ¬ (k.val = 0 ∧ v.val ≠ 0) := by
        intro ⟨h0, _⟩; simp [h0] at hm
      simp [contractCall, h, hc, IsEnsures, this]
  · have : ¬ (k.val = 0 ∧ v.val ≠ 0) := by omega
    simp [h, contract_requires, IsEnsures, this]

/-- Within the bound, without overflow, and not a violation: the product. -/
theorem scale_ok (v k : Std.U32) (hk : k.val ≤ 1000) (hm : v.val * k.val ≤ U32.max)
    (hp : k.val ≠ 0 ∨ v.val = 0) :
    ∃ c, contracts.scale v k = ok (.Ok c) ∧ c.val = v.val * k.val := by
  rw [scale_shape]
  rcases scale_body v k with ⟨c, hc, hv, _⟩ | ⟨hc, hm'⟩
  · have : v.val ≤ c.val := by
      rcases hp with hp | hp
      · have : v.val * 1 ≤ v.val * k.val := Nat.mul_le_mul_left _ (by omega)
        omega
      · omega
    exact ⟨c, by simp [contractCall, hk, hc, this], hv⟩
  · omega

/-- Within the bound with overflow: the body's `Err`, unchanged. -/
theorem scale_overflow (v k : Std.U32) (hk : k.val ≤ 1000) (hm : U32.max < v.val * k.val) :
    contracts.scale v k = ok (.Err .Overflow) := by
  rw [scale_shape]
  rcases scale_body v k with ⟨c, hc, hv, hm'⟩ | ⟨hc, _⟩
  · omega
  · simp [contractCall, hk, hc]

/-! ## `share(total: u32, divisor: u32)`, `#[requires(divisor != 0)]`
No postcondition: the shape's `post` is constantly true (its error is
then never produced). -/

theorem share_shape (t d : Std.U32) :
    contracts.share t d
      = contractCall (decide (d.val ≠ 0))
          (.Contract (.Requires (toStr "share") (toStr "divisor != 0")))
          (fun _ : Std.U32 => true)
          (.Contract (.Requires (toStr "share") (toStr "divisor != 0")))
          (contracts.share.closure.Insts.CoreOpsFunctionFnTupleResultU32OpError.call (t, d) ()) := by
  unfold contracts.share contractCall
  by_cases h : d.val = 0
  · have : ¬ d != 0#u32 := by simp; scalar_tac
    simp [this, h, tauten.error.ContractViolation.requires,
      contracts.OpError.Insts.CoreConvertFromContractViolation.from]
  · have : (d != 0#u32) = true := by simp; scalar_tac
    simp only [this, h, ite_true, decide_not, decide_false, Bool.not_false]
    congr 1
    funext r
    cases r <;> simp

/-- The body divides: it panics on a zero divisor. -/
theorem share_body_zero (t : Std.U32) :
    contracts.share.closure.Insts.CoreOpsFunctionFnTupleResultU32OpError.call (t, 0#u32) ()
      = fail .divisionByZero := by
  simp [contracts.share.closure.Insts.CoreOpsFunctionFnTupleResultU32OpError.call,
    HDiv.hDiv, UScalar.div]

/-- A zero divisor is refused with `Requires` although the body would
panic: the precondition is checked before, and instead of, the body. -/
theorem share_requires_skips_body (t : Std.U32) :
    contracts.share t 0#u32
      = ok (.Err (.Contract (.Requires (toStr "share") (toStr "divisor != 0")))) ∧
    contracts.share.closure.Insts.CoreOpsFunctionFnTupleResultU32OpError.call (t, 0#u32) ()
      = fail .divisionByZero := by
  refine ⟨?_, share_body_zero t⟩
  rw [share_shape]; simp [contract_requires]

theorem share_requires_iff (t d : Std.U32) :
    (∃ r, contracts.share t d = ok r ∧ IsRequires r) ↔ d.val = 0 := by
  rw [share_shape]
  by_cases h : d.val = 0
  · simp [h, contract_requires, IsRequires]
  · obtain ⟨q, hq, _⟩ := UScalar.div_spec t h
    have hb : contracts.share.closure.Insts.CoreOpsFunctionFnTupleResultU32OpError.call (t, d) ()
        = ok (.Ok q) := by
      simp [contracts.share.closure.Insts.CoreOpsFunctionFnTupleResultU32OpError.call, hq]
    simp [contractCall, h, hb, IsRequires]

/-- A non-zero divisor: the quotient. -/
theorem share_ok (t d : Std.U32) (h : d.val ≠ 0) :
    ∃ q, contracts.share t d = ok (.Ok q) ∧ q.val = t.val / d.val := by
  rw [share_shape]
  obtain ⟨q, hq, hv⟩ := UScalar.div_spec t h
  have hb : contracts.share.closure.Insts.CoreOpsFunctionFnTupleResultU32OpError.call (t, d) ()
      = ok (.Ok q) := by
    simp [contracts.share.closure.Insts.CoreOpsFunctionFnTupleResultU32OpError.call, hq]
  exact ⟨q, by simp [contractCall, h, hb], hv⟩

/-! ## Witnesses -/

theorem contract_examples :
    contracts.deposit 5#u64 0#u64
      = ok (.Err (.Contract (.Requires (toStr "deposit") (toStr "amount > 0")))) ∧
    contracts.scale 5#u32 0#u32
      = ok (.Err (.Contract (.Ensures (toStr "scale")
          (toStr "| scaled : & u32 | * scaled >= old(value)")))) ∧
    contracts.scale 1#u32 1001#u32
      = ok (.Err (.Contract (.Requires (toStr "scale") (toStr "factor <= 1000")))) ∧
    contracts.withdraw 10#u64 8#u64 = ok (.Ok 2#u64, 2#u64) ∧
    contracts.withdraw 7#u64 8#u64 = ok (.Err .Insufficient, 7#u64) := by
  refine ⟨?_, ?_, ?_, ?_, ?_⟩
  · rw [deposit_shape]; simp [contract_requires]
  · rw [scale_shape]
    rcases scale_body 5#u32 0#u32 with ⟨c, hc, hv, _⟩ | ⟨hc, hm⟩
    · simp at hv; simp [contractCall, hc, hv]
    · simp at hm
  · rw [scale_shape]; simp [contract_requires]
  · rw [withdraw_shape]
    rcases withdraw_body 10#u64 8#u64 with ⟨n, hn, hv, _⟩ | ⟨hn, hm⟩
    · simp at hv
      have : n = 2#u64 := by scalar_tac
      subst this
      simp [contractCallMut, hn]
    · simp at hm
  · exact withdraw_insufficient _ _ (by simp) (by simp)

end Tauten.Instances
