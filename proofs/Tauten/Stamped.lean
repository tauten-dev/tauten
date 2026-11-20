import Extracted.TautenProofInstances

/-!
# A stamped kernel action refuses a stale stamp (A5)

`tauten_proof_instances::stamped::bump` takes a `Stamped` fact
(`#[evidence(.., stamped)]`) and acts only if the counter is still at the
version the fact was checked at (`Stamped::verify`).

Proved here, over the translation (the macro's expansion and
`tauten::origin` included):
* `stale_refused`: a stamp whose version differs from the counter's returns
  `Stale` and leaves the counter unchanged.
* `fresh_accepted`: a stamp at the counter's version is accepted, and the
  counter's version moves on (so the same stamp is stale afterwards).

This is the lemma shape a project proves for its own stamped kernel
actions; the version comparison itself is the runtime-checked part.
-/

open Aeneas Aeneas.Std Result
open tauten_proof_instances

namespace Tauten.Instances

theorem stale_refused (c : stamped.Counter)
    (st : tauten.origin.Stamped stamped.__tauten_evidence_belowlimit.BelowLimit stamped.Counter)
    (h : st.version ≠ c.version) :
    stamped.bump c st = ok (core.result.Result.Err (), c) := by
  have h' : ¬ c.version = st.version := fun e => h e.symm
  simp [stamped.bump, tauten.origin.Stamped.verify,
    stamped.Counter.Insts.TautenOriginVersioned.version, h',
    core.result.Result.Insts.CoreOpsTry.branch,
    core.result.Result.Insts.CoreOpsTry_traitFromResidualResult.from_residual,
    core.convert.FromSame]

theorem fresh_accepted (c : stamped.Counter)
    (st : tauten.origin.Stamped stamped.__tauten_evidence_belowlimit.BelowLimit stamped.Counter)
    (h : st.version = c.version) :
    stamped.bump c st ⦃ r => r.1 = core.result.Result.Ok () ∧
      r.2.version = core.num.U64.wrapping_add c.version 1#u64 ⦄ := by
  simp [stamped.bump, tauten.origin.Stamped.verify,
    stamped.Counter.Insts.TautenOriginVersioned.version, h,
    core.result.Result.Insts.CoreOpsTry.branch,
    tauten.origin.Evidence.__mint, tauten.origin.Evidence.redeem,
    stamped.__tauten_evidence_belowlimit.BelowLimit.impl.value, lift]

end Tauten.Instances
