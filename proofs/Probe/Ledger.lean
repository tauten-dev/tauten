import Extracted.TautenProbe

/-!
# P0 probe: `checked_add` and `ok_or`, plain and under a contract

`ledger.apply` calls `u64::checked_add`/`checked_sub` and `Option::ok_or`;
`contract.deposit` is the real `#[requires]`/`#[ensures]`/`old` expansion
(the body in an immediately-invoked closure, the postcondition through
`tauten::contract::holds`). Both are modelled by the pinned Aeneas library
with bodies, so these theorems need no extra axiom.
-/

open Aeneas Aeneas.Std Result
open tauten_probe

namespace Tauten.Probe

/-- A deposit either adds exactly or reports overflow exactly when it overflows. -/
theorem apply_deposit_spec (b a : Std.U64) :
    (∃ c, ledger.apply b (.Deposit a) = ok (.Ok c) ∧ c.val = b.val + a.val) ∨
    (ledger.apply b (.Deposit a) = ok (.Err .Overflow) ∧ U64.max < b.val + a.val) := by
  have h := U64.checked_add_bv_spec b a
  unfold ledger.apply
  cases hc : U64.checked_add b a with
  | none => right; simp_all [lift]
  | some c => left; simp_all [lift]

/-- The precondition: a zero deposit is refused before the body runs. -/
theorem deposit_zero_refused (b : Std.U64) :
    ∃ v, contract.deposit b 0#u64 = ok (.Err (.Contract v)) := by
  unfold contract.deposit
  simp [tauten.error.ContractViolation.requires,
    error.LedgerError.Insts.CoreConvertFromContractViolation.from]

/-- With a positive amount and no overflow, the contract passes and the result is the sum. -/
theorem deposit_ok (b a : Std.U64) (ha : 0 < a.val) (hb : b.val + a.val ≤ U64.max) :
    ∃ c, contract.deposit b a = ok (.Ok c) ∧ c.val = b.val + a.val := by
  have h := U64.checked_add_bv_spec b a
  have hpos : a > 0#u64 := by scalar_tac
  unfold contract.deposit
  cases hc : U64.checked_add b a with
  | none => simp_all; omega
  | some c =>
    simp only [hc] at h
    have hlt : b.val < c.val := by omega
    refine ⟨c, ?_, h.2.1⟩
    simp [hpos, hc, lift, tauten.contract.snapshot, tauten.contract.holds,
      contract.deposit.closure.Insts.CoreOpsFunctionFnTupleResultU64LedgerError.call,
      contract.deposit.closure_1.Insts.CoreOpsFunctionFnOnceTupleShared1U64Bool.call_once,
      hlt]

end Tauten.Probe
