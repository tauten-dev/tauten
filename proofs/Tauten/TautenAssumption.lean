import Lean

/-!
# Named assumptions (`@[tauten_assumption "ID"]`)

Tag a proposition about the outside world with the ID it has in
`proofs/assumptions.toml`:

    @[tauten_assumption "FS.rename-atomic"]
    def RenameAtomic (fs : FsModel) : Prop := ...

A theorem that takes such a proposition as a hypothesis is proved only under
that assumption. The audit (`proofs/Audit.lean`) derives, for every theorem,
the IDs its hypotheses mention (`PREMISES`) and, for a theorem without
tagged hypotheses, the IDs its conclusion mentions (`ESTABLISHES`: a
witness that the assumptions are satisfiable). `cargo tauten proofs` checks
both against `assumes` and `witness` in `proofs/claims.toml`.
-/

open Lean

syntax (name := tauten_assumption) "tauten_assumption " str : attr

initialize tautenAssumptionAttr : ParametricAttribute String ←
  registerParametricAttribute {
    name := `tauten_assumption
    descr := "a named assumption about the outside world (proofs/assumptions.toml)"
    getParam := fun _ stx => match stx with
      | `(attr| tauten_assumption $s:str) => pure s.getString
      | _ => throwError "expected @[tauten_assumption \"ID\"]"
  }
