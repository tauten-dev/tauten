import Lean
import Probe

/-!
# Axiom audit and constructor enumeration (P0 probe)

Adapted from writfold's `proofs/Audit.lean`. Every theorem under
`Tauten.Probe` must depend only on Lean's standard three axioms, and the
required claims must exist.

Constructor enumeration (docs/PROOF-PLAN.md section 2, binding 3): every
extracted definition whose result is `Result T` with `T` mentioning a refined
type must be on a reviewed allowlist. The `derive(Default)` control shows the
check is not vacuous: the derived `Default::default` of the plain struct is
found as a constructor.
-/
open Lean Elab Command Meta

def allowedAxioms : List Name := [``propext, ``Classical.choice, ``Quot.sound]

def claims : List Name := [
  ``Tauten.Probe.rank_spec,
  ``Tauten.Probe.may_depend_on_spec,
  ``Tauten.Probe.may_depend_on_derived_agrees,
  ``Tauten.Probe.kernel_depends_only_on_kernel,
  ``Tauten.Probe.percent_try_new_ok_iff,
  ``Tauten.Probe.percent_hand_try_new_ok_iff,
  ``Tauten.Probe.apply_deposit_spec,
  ``Tauten.Probe.deposit_zero_refused,
  ``Tauten.Probe.deposit_ok,
]

def auditedTheorems (env : Environment) : Array Name :=
  let audited : Name := `Tauten.Probe
  (env.constants.fold (init := (#[] : Array Name)) fun acc decl info =>
    if audited.isPrefixOf decl && decl.isInternal = false && info.isThm then acc.push decl
    else acc).qsort (fun a b => a.toString < b.toString)

def auditAxioms : CommandElabM Unit := do
  let env ← getEnv
  let sorted := auditedTheorems env
  if sorted.isEmpty then
    throwError "axiom audit: no theorems under Tauten.Probe"
  for decl in sorted do
    let axioms := (← collectAxioms decl).qsort (fun a b => a.toString < b.toString)
    logInfo m!"AXIOMS {decl}: {axioms.toList}"
    for ax in axioms do
      unless allowedAxioms.contains ax do
        throwError "axiom audit: {decl} depends on {ax}, which is not a standard Lean axiom"
  for claim in claims do
    unless sorted.contains claim do
      throwError "axiom audit: required claim {claim} is missing or is not a theorem"
  logInfo m!"AXIOM_AUDIT ok: {sorted.size} theorems, {claims.length} required claims, axioms ⊆ {allowedAxioms}"

run_cmd auditAxioms

/-- Extracted definitions (under `tauten_probe`) whose result is `Result T`
with `T` mentioning `ty`, read from the unreduced signature. No
`isInternal` filter: the macro's module `__tauten_<name>` starts with `_`,
which Lean counts as internal, and would hide every refined constructor. -/
def constructorsOf (env : Environment) (ty : Name) : Array Name :=
  (env.constants.fold (init := (#[] : Array Name)) fun acc decl info =>
    if (`tauten_probe).isPrefixOf decl && info.isDefinition then
      let rec target : Expr → Expr
        | .forallE _ _ b _ => target b
        | e => e
      match (target info.type).getAppFn, (target info.type).getAppArgs with
      | .const ``Aeneas.Std.Result _, #[t] =>
        if (t.find? fun e => e.isConstOf ty).isSome then acc.push decl else acc
      | _, _ => acc
    else acc).qsort (fun a b => a.toString < b.toString)

def refinedType : Name := `tauten_probe.percent.__tauten_percent.Percent

/-- The reviewed constructors of the refined type: the checked constructor,
`TryFrom` (which calls it) and `Clone` (which copies an existing value). -/
def allowedConstructors : List Name := [
  `tauten_probe.percent.__tauten_percent.Percent.try_new,
  `tauten_probe.percent.__tauten_percent.Percent.Insts.CoreConvertTryFromU8RefineError.try_from,
  `tauten_probe.percent.__tauten_percent.Percent.Insts.CoreCloneClone.clone,
]

def auditConstructors : CommandElabM Unit := do
  let env ← getEnv
  let found := constructorsOf env refinedType
  logInfo m!"CONSTRUCTORS {refinedType}: {found.toList}"
  for decl in found do
    unless allowedConstructors.contains decl do
      throwError "constructor audit: {decl} builds a {refinedType} and is not reviewed"
  let control := constructorsOf env `tauten_probe.control.Plain
  logInfo m!"CONSTRUCTORS control.Plain: {control.toList}"
  unless control.contains `tauten_probe.control.Plain.Insts.CoreDefaultDefault.default do
    throwError "constructor audit is vacuous: the derived Default of the control was not found"
  logInfo m!"CONSTRUCTOR_AUDIT ok"

run_cmd auditConstructors
