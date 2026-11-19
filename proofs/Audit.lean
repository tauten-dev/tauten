import Lean
import Tauten

/-!
# Axiom audit, constructor enumeration and coverage of the extraction

Adapted from writfold's `proofs/Audit.lean` and the P0 probe. Covers
`tauten_check_kernel` (P1, P2, P4) and `tauten_proof_instances` (P3, the
macros' expansion of its instances).

1. Every theorem under `Tauten` depends only on Lean's standard three axioms
   (`propext`, `Classical.choice`, `Quot.sound`). This reads the transitive
   axiom closure (`collectAxioms`); an Aeneas `axiom` for an opaque function,
   a `sorry` (`sorryAx`) or a native-evaluation axiom would fail it.
2. Every required claim exists and is a theorem. `proofs/mutants.py` also
   requires every theorem a mutant names to be in this list.
3. Constructor enumeration (Plyward F01 target 5; plan section 2, binding
   3): for each refined type of `tauten_proof_instances`, every extracted
   function whose result is `Result T` with `T` mentioning the type is
   listed and must be `try_new`, `TryFrom::try_from` or the derived `clone`
   (each stated in a claim: `try_from` is `try_new`, `clone` is the
   identity). A generated or derived impl that builds a refined value any
   other way (a `Default`, say) fails the build. The control, the plain
   newtype `Tag` with `derive(Default)`, must show `new` and the derived
   `default`, or the enumeration is not seeing derived impls.
4. Every function extracted from either crate is reviewed (listed
   below) and is covered by a claim: it is mentioned in the statement of a
   required claim, or it is called (transitively, through the extracted
   definitions) by a function that is. Loop bodies and the ratchet's
   per-finding helpers are covered this way by the theorems about `apply`,
   `stale`, `count_errors` and `increases`. A function added to
   `start_from` without a theorem fails the build.

Pitfall (P0): Lean's `Name.isInternal` treats the macro's `__tauten_<name>`
module as internal, so no filter here uses it on extracted names.
-/
open Lean Elab Command Meta

def allowedAxioms : List Name := [``propext, ``Classical.choice, ``Quot.sound]

def claims : List Name := [
  ``Tauten.Kernel.rank_spec,
  ``Tauten.Kernel.may_depend_on_spec,
  ``Tauten.Kernel.may_depend_on_refl,
  ``Tauten.Kernel.may_depend_on_trans,
  ``Tauten.Kernel.may_depend_on_total,
  ``Tauten.Kernel.may_depend_on_antisymm,
  ``Tauten.Kernel.kernel_depends_only_on_kernel,
  ``Tauten.Kernel.is_blocking_spec,
  ``Tauten.Kernel.severity_spec,
  ``Tauten.Kernel.permitted_spec,
  ``Tauten.Kernel.exception_spec,
  ``Tauten.Kernel.kernel_never_excuses_levelled_rule,
  ``Tauten.Kernel.no_exceptions_never_excused,
  -- the baseline ratchet (P2)
  ``Tauten.Kernel.apply_conserves,
  ``Tauten.Kernel.apply_covered_iff,
  ``Tauten.Kernel.apply_cover_count,
  ``Tauten.Kernel.apply_suppressed_le_accepted,
  ``Tauten.Kernel.apply_left_spec,
  ``Tauten.Kernel.ratchet_stale_iff,
  ``Tauten.Kernel.ratchet_passes_iff,
  ``Tauten.Kernel.ratchet_headroom,
  ``Tauten.Kernel.count_errors_counts,
  ``Tauten.Kernel.increases_flags,
  ``Tauten.Kernel.increases_none_iff,
  ``Tauten.Kernel.regenerated_baseline_never_grows,
  ``Tauten.Kernel.ratchet_example,
  ``Tauten.Kernel.increase_example,
  -- rule-table well-formedness (P4)
  ``Tauten.Kernel.validate_ok_iff,
  ``Tauten.Kernel.validate_examples,
  -- the macros' generated code, per instance (P3)
  ``Tauten.Instances.percent_try_new_shape,
  ``Tauten.Instances.percent_try_new_ok_iff,
  ``Tauten.Instances.percent_try_new_sound,
  ``Tauten.Instances.percent_try_new_err_iff,
  ``Tauten.Instances.percent_try_from_spec,
  ``Tauten.Instances.percent_derived_spec,
  ``Tauten.Instances.percent_boundaries,
  ``Tauten.Instances.port_try_new_shape,
  ``Tauten.Instances.port_try_new_ok_iff,
  ``Tauten.Instances.port_try_new_sound,
  ``Tauten.Instances.port_try_new_err_iff,
  ``Tauten.Instances.port_try_from_spec,
  ``Tauten.Instances.port_derived_spec,
  ``Tauten.Instances.port_boundaries,
  ``Tauten.Instances.offset_try_new_shape,
  ``Tauten.Instances.offset_try_new_ok_iff,
  ``Tauten.Instances.offset_try_new_sound,
  ``Tauten.Instances.offset_try_new_err_iff,
  ``Tauten.Instances.offset_try_from_spec,
  ``Tauten.Instances.offset_derived_spec,
  ``Tauten.Instances.offset_boundaries,
  ``Tauten.Instances.count_try_new_shape,
  ``Tauten.Instances.count_try_new_ok_iff,
  ``Tauten.Instances.count_try_new_sound,
  ``Tauten.Instances.count_try_new_err_iff,
  ``Tauten.Instances.count_try_from_spec,
  ``Tauten.Instances.count_derived_spec,
  ``Tauten.Instances.count_boundaries,
  ``Tauten.Instances.tag_control_spec,
  ``Tauten.Instances.deposit_shape,
  ``Tauten.Instances.deposit_requires_iff,
  ``Tauten.Instances.deposit_never_ensures,
  ``Tauten.Instances.deposit_ok,
  ``Tauten.Instances.deposit_overflow,
  ``Tauten.Instances.withdraw_shape,
  ``Tauten.Instances.withdraw_requires_iff,
  ``Tauten.Instances.withdraw_requires_keeps_balance,
  ``Tauten.Instances.withdraw_never_ensures,
  ``Tauten.Instances.withdraw_ok,
  ``Tauten.Instances.withdraw_insufficient,
  ``Tauten.Instances.scale_shape,
  ``Tauten.Instances.scale_requires_iff,
  ``Tauten.Instances.scale_ensures_iff,
  ``Tauten.Instances.scale_ok,
  ``Tauten.Instances.scale_overflow,
  ``Tauten.Instances.share_shape,
  ``Tauten.Instances.share_requires_skips_body,
  ``Tauten.Instances.share_requires_iff,
  ``Tauten.Instances.share_ok,
  ``Tauten.Instances.contract_examples,
  -- schematic: what every expansion reduces to (P3); no extracted function
  ``Tauten.Macros.refined_ok_iff,
  ``Tauten.Macros.refined_ok_value,
  ``Tauten.Macros.refined_err_iff,
  ``Tauten.Macros.refined_total,
  ``Tauten.Macros.contract_requires,
  ``Tauten.Macros.contract_ok,
  ``Tauten.Macros.contract_err,
  ``Tauten.Macros.contract_mut_requires,
  ``Tauten.Macros.contract_mut_ok,
  ``Tauten.Macros.contract_mut_err,
]

/-- The functions translated from each crate's `[package.metadata.charon] start_from`. -/
def reviewedFunctions : List Name := [
  ``tauten_check_kernel.tiers.rank,
  ``tauten_check_kernel.tiers.may_depend_on,
  ``tauten_check_kernel.severity.is_blocking,
  ``tauten_check_kernel.severity.severity,
  ``tauten_check_kernel.exception.permitted,
  ``tauten_check_kernel.exception.excuse,
  ``tauten_check_kernel.ratchet.count_at,
  ``tauten_check_kernel.ratchet.take,
  ``tauten_check_kernel.ratchet.take_at,
  ``tauten_check_kernel.ratchet.apply,
  ``tauten_check_kernel.ratchet.apply_loop,
  ``tauten_check_kernel.ratchet.apply_loop.body,
  ``tauten_check_kernel.ratchet.stale,
  ``tauten_check_kernel.ratchet.stale_loop,
  ``tauten_check_kernel.ratchet.stale_loop.body,
  ``tauten_check_kernel.ratchet.zeros,
  ``tauten_check_kernel.ratchet.zeros_loop,
  ``tauten_check_kernel.ratchet.zeros_loop.body,
  ``tauten_check_kernel.ratchet.add_error,
  ``tauten_check_kernel.ratchet.add_error_at,
  ``tauten_check_kernel.ratchet.count_errors,
  ``tauten_check_kernel.ratchet.count_errors_loop,
  ``tauten_check_kernel.ratchet.count_errors_loop.body,
  ``tauten_check_kernel.ratchet.grows_at,
  ``tauten_check_kernel.ratchet.increases,
  ``tauten_check_kernel.ratchet.increases_loop,
  ``tauten_check_kernel.ratchet.increases_loop.body,
  ``tauten_check_kernel.table.is_at,
  ``tauten_check_kernel.table.find_before,
  ``tauten_check_kernel.table.find_before_loop,
  ``tauten_check_kernel.table.find_before_loop.body,
  ``tauten_check_kernel.table.contains,
  ``tauten_check_kernel.table.repeat_at,
  ``tauten_check_kernel.table.first_duplicate,
  ``tauten_check_kernel.table.first_duplicate_loop,
  ``tauten_check_kernel.table.first_duplicate_loop.body,
  ``tauten_check_kernel.table.absent,
  ``tauten_check_kernel.table.missing_at,
  ``tauten_check_kernel.table.first_missing,
  ``tauten_check_kernel.table.first_missing_loop,
  ``tauten_check_kernel.table.first_missing_loop.body,
  ``tauten_check_kernel.table.owner_at,
  ``tauten_check_kernel.table.duplicate,
  ``tauten_check_kernel.table.validate,
  -- tauten_proof_instances (P3): the macros' expansion, the closures it
  -- generates, the derives, and the runtime helpers made transparent by
  -- `include = ["tauten::contract", "tauten::error"]`
  ``tauten_proof_instances.contracts.OpError.Insts.CoreConvertFromContractViolation.from,
  ``tauten_proof_instances.contracts.deposit,
  ``tauten_proof_instances.contracts.deposit.closure.Insts.CoreOpsFunctionFnTupleResultU64OpError.call,
  ``tauten_proof_instances.contracts.deposit.closure_1.Insts.CoreOpsFunctionFnOnceTupleShared1U64Bool.call_once,
  ``tauten_proof_instances.contracts.scale,
  ``tauten_proof_instances.contracts.scale.closure.Insts.CoreOpsFunctionFnTupleResultU32OpError.call,
  ``tauten_proof_instances.contracts.scale.closure_1.Insts.CoreOpsFunctionFnOnceTupleShared1U32Bool.call_once,
  ``tauten_proof_instances.contracts.share,
  ``tauten_proof_instances.contracts.share.closure.Insts.CoreOpsFunctionFnTupleResultU32OpError.call,
  ``tauten_proof_instances.contracts.withdraw,
  ``tauten_proof_instances.contracts.withdraw.closure.Insts.CoreOpsFunctionFnMutTupleResultU64OpError.call_mut,
  ``tauten_proof_instances.contracts.withdraw.closure_1.Insts.CoreOpsFunctionFnOnceTupleShared1U64Bool.call_once,
  ``tauten_proof_instances.refined.__tauten_count.Count.Insts.CoreCloneClone.clone,
  ``tauten_proof_instances.refined.__tauten_count.Count.Insts.CoreCmpEq.assert_fields_are_eq,
  ``tauten_proof_instances.refined.__tauten_count.Count.Insts.CoreCmpPartialEqCount.eq,
  ``tauten_proof_instances.refined.__tauten_count.Count.Insts.CoreConvertAsRefU64.as_ref,
  ``tauten_proof_instances.refined.__tauten_count.Count.Insts.CoreConvertTryFromU64RefineError.try_from,
  ``tauten_proof_instances.refined.__tauten_count.Count.get,
  ``tauten_proof_instances.refined.__tauten_count.Count.into_inner,
  ``tauten_proof_instances.refined.__tauten_count.Count.try_new,
  ``tauten_proof_instances.refined.__tauten_count.Count.try_new.closure.Insts.CoreOpsFunctionFnOnceTupleShared0U64Bool.call_once,
  ``tauten_proof_instances.refined.__tauten_offset.Offset.Insts.CoreCloneClone.clone,
  ``tauten_proof_instances.refined.__tauten_offset.Offset.Insts.CoreCmpEq.assert_fields_are_eq,
  ``tauten_proof_instances.refined.__tauten_offset.Offset.Insts.CoreCmpPartialEqOffset.eq,
  ``tauten_proof_instances.refined.__tauten_offset.Offset.Insts.CoreConvertAsRefI32.as_ref,
  ``tauten_proof_instances.refined.__tauten_offset.Offset.Insts.CoreConvertTryFromI32RefineError.try_from,
  ``tauten_proof_instances.refined.__tauten_offset.Offset.get,
  ``tauten_proof_instances.refined.__tauten_offset.Offset.into_inner,
  ``tauten_proof_instances.refined.__tauten_offset.Offset.try_new,
  ``tauten_proof_instances.refined.__tauten_offset.Offset.try_new.closure.Insts.CoreOpsFunctionFnOnceTupleShared0I32Bool.call_once,
  ``tauten_proof_instances.refined.__tauten_percent.Percent.Insts.CoreCloneClone.clone,
  ``tauten_proof_instances.refined.__tauten_percent.Percent.Insts.CoreCmpEq.assert_fields_are_eq,
  ``tauten_proof_instances.refined.__tauten_percent.Percent.Insts.CoreCmpPartialEqPercent.eq,
  ``tauten_proof_instances.refined.__tauten_percent.Percent.Insts.CoreConvertAsRefU8.as_ref,
  ``tauten_proof_instances.refined.__tauten_percent.Percent.Insts.CoreConvertTryFromU8RefineError.try_from,
  ``tauten_proof_instances.refined.__tauten_percent.Percent.get,
  ``tauten_proof_instances.refined.__tauten_percent.Percent.into_inner,
  ``tauten_proof_instances.refined.__tauten_percent.Percent.try_new,
  ``tauten_proof_instances.refined.__tauten_percent.Percent.try_new.closure.Insts.CoreOpsFunctionFnOnceTupleShared0U8Bool.call_once,
  ``tauten_proof_instances.refined.__tauten_port.Port.Insts.CoreCloneClone.clone,
  ``tauten_proof_instances.refined.__tauten_port.Port.Insts.CoreCmpEq.assert_fields_are_eq,
  ``tauten_proof_instances.refined.__tauten_port.Port.Insts.CoreCmpPartialEqPort.eq,
  ``tauten_proof_instances.refined.__tauten_port.Port.Insts.CoreConvertAsRefU16.as_ref,
  ``tauten_proof_instances.refined.__tauten_port.Port.Insts.CoreConvertTryFromU16RefineError.try_from,
  ``tauten_proof_instances.refined.__tauten_port.Port.get,
  ``tauten_proof_instances.refined.__tauten_port.Port.into_inner,
  ``tauten_proof_instances.refined.__tauten_port.Port.try_new,
  ``tauten_proof_instances.refined.__tauten_port.Port.try_new.closure.Insts.CoreOpsFunctionFnOnceTupleShared0U16Bool.call_once,
  ``tauten_proof_instances.refined.__tauten_tag.Tag.Insts.CoreCloneClone.clone,
  ``tauten_proof_instances.refined.__tauten_tag.Tag.Insts.CoreConvertAsRefU32.as_ref,
  ``tauten_proof_instances.refined.__tauten_tag.Tag.Insts.CoreDefaultDefault.default,
  ``tauten_proof_instances.refined.__tauten_tag.Tag.get,
  ``tauten_proof_instances.refined.__tauten_tag.Tag.into_inner,
  ``tauten_proof_instances.refined.__tauten_tag.Tag.new,
  ``tauten_proof_instances.tauten.contract.holds,
  ``tauten_proof_instances.tauten.contract.snapshot,
  ``tauten_proof_instances.tauten.error.ContractViolation.ensures,
  ``tauten_proof_instances.tauten.error.ContractViolation.requires,
  ``tauten_proof_instances.tauten.error.RefineError.new,
]

def auditedTheorems (env : Environment) : Array Name :=
  let audited : Name := `Tauten
  (env.constants.fold (init := (#[] : Array Name)) fun acc decl info =>
    if audited.isPrefixOf decl && decl.isInternal = false && info.isThm then acc.push decl
    else acc).qsort (fun a b => a.toString < b.toString)

def auditAxioms : CommandElabM Unit := do
  let env ← getEnv
  let sorted := auditedTheorems env
  if sorted.isEmpty then
    throwError "axiom audit: no theorems under Tauten"
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

/-- The crates whose translation is audited. -/
def extractedCrates : List Name := [`tauten_check_kernel, `tauten_proof_instances]

/-- Extracted functions (of the audited crates) whose result is `Result T`
with `T` mentioning `ty`, read from the unreduced signature (Aeneas emits a
refined type as a reducible `def` of its raw type, so a reduced signature
would hide it). -/
def constructorsOf (env : Environment) (ty : Name) : Array Name :=
  (env.constants.fold (init := (#[] : Array Name)) fun acc decl info =>
    if extractedCrates.any (·.isPrefixOf decl) && info.isDefinition then
      let rec target : Expr → Expr
        | .forallE _ _ b _ => target b
        | e => e
      let result := target info.type
      if result.getAppFn.isConstOf ``Aeneas.Std.Result
          && (result.getUsedConstants.contains ty) then acc.push decl else acc
    else acc).qsort (fun a b => a.toString < b.toString)

/-- Refined types of `tauten_proof_instances`, with the only functions
allowed to return them: the checked constructor, `TryFrom`, which a claim
equates with it, and the derived `Clone`, which a claim shows is the
identity. -/
def refinedTypes : List (Name × List Name) := [
  (``tauten_proof_instances.refined.__tauten_percent.Percent, [
    ``tauten_proof_instances.refined.__tauten_percent.Percent.try_new,
    ``tauten_proof_instances.refined.__tauten_percent.Percent.Insts.CoreConvertTryFromU8RefineError.try_from,
    ``tauten_proof_instances.refined.__tauten_percent.Percent.Insts.CoreCloneClone.clone]),
  (``tauten_proof_instances.refined.__tauten_port.Port, [
    ``tauten_proof_instances.refined.__tauten_port.Port.try_new,
    ``tauten_proof_instances.refined.__tauten_port.Port.Insts.CoreConvertTryFromU16RefineError.try_from,
    ``tauten_proof_instances.refined.__tauten_port.Port.Insts.CoreCloneClone.clone]),
  (``tauten_proof_instances.refined.__tauten_offset.Offset, [
    ``tauten_proof_instances.refined.__tauten_offset.Offset.try_new,
    ``tauten_proof_instances.refined.__tauten_offset.Offset.Insts.CoreConvertTryFromI32RefineError.try_from,
    ``tauten_proof_instances.refined.__tauten_offset.Offset.Insts.CoreCloneClone.clone]),
  (``tauten_proof_instances.refined.__tauten_count.Count, [
    ``tauten_proof_instances.refined.__tauten_count.Count.try_new,
    ``tauten_proof_instances.refined.__tauten_count.Count.Insts.CoreConvertTryFromU64RefineError.try_from,
    ``tauten_proof_instances.refined.__tauten_count.Count.Insts.CoreCloneClone.clone]),
]

/-- The control: a plain newtype with `derive(Default)`, and the
constructors the enumeration must find for it. -/
def controlType : Name × List Name :=
  (``tauten_proof_instances.refined.__tauten_tag.Tag, [
    ``tauten_proof_instances.refined.__tauten_tag.Tag.new,
    ``tauten_proof_instances.refined.__tauten_tag.Tag.Insts.CoreDefaultDefault.default])

def auditConstructors : CommandElabM Unit := do
  let env ← getEnv
  for (ty, allowed) in refinedTypes do
    let found := constructorsOf env ty
    logInfo m!"CONSTRUCTORS {ty}: {found.toList}"
    for decl in found do
      unless allowed.contains decl do
        throwError "constructor audit: {decl} returns a {ty} and is not try_new, try_from or clone"
    for decl in allowed do
      unless found.contains decl do
        throwError "constructor audit: {decl} was not found; the enumeration is not seeing {ty}"
  let (control, expected) := controlType
  let found := constructorsOf env control
  logInfo m!"CONSTRUCTORS {control} (control): {found.toList}"
  for decl in expected do
    unless found.contains decl do
      throwError "constructor audit is vacuous: {decl} of the control was not found"
  logInfo m!"CONSTRUCTOR_AUDIT ok: {refinedTypes.length} refined types, control found {expected.length} constructors"

run_cmd auditConstructors

/-- Definitions Aeneas wrote for functions of the audited crates: the
extracted functions return `Result _`; inductive types, closure state
types, trait-instance records and generated helpers do not. -/
def extractedFunctions (env : Environment) : Array Name :=
  (env.constants.fold (init := (#[] : Array Name)) fun acc decl info =>
    if extractedCrates.any (·.isPrefixOf decl) && info.isDefinition then
      let rec target : Expr → Expr
        | .forallE _ _ b _ => target b
        | e => e
      if (target info.type).getAppFn.isConstOf ``Aeneas.Std.Result then acc.push decl else acc
    else acc).qsort (fun a b => a.toString < b.toString)

/-- The constants of the audited crates a definition's body uses: extracted
functions, and also the trait-instance records through which a function
calls a closure (`holds Inst value closure` reaches the closure's
`call_once` only through `Inst`). -/
def calls (env : Environment) (decl : Name) : Array Name :=
  match env.find? decl with
  | some info => match info.value? with
    | some value => value.getUsedConstants.filter fun c => extractedCrates.any (·.isPrefixOf c)
    | none => #[]
  | none => #[]

/-- The extracted functions stated in a claim, then everything they call. -/
def coveredByClaims (env : Environment) (found : Array Name) : Array Name := Id.run do
  let mut visited : Array Name := #[]
  let mut todo : Array Name := #[]
  for claim in claims do
    if let some info := env.find? claim then
      todo := todo ++ info.type.getUsedConstants.filter found.contains
  while h : todo.size > 0 do
    let decl := todo[todo.size - 1]
    todo := todo.pop
    unless visited.contains decl do
      visited := visited.push decl
      todo := todo ++ calls env decl
  return visited.filter found.contains

def auditCoverage : CommandElabM Unit := do
  let env ← getEnv
  let found := extractedFunctions env
  logInfo m!"EXTRACTED {found.toList}"
  let covered := coveredByClaims env found
  for decl in found do
    unless reviewedFunctions.contains decl do
      throwError "coverage audit: extracted {decl} is not in reviewedFunctions"
    unless covered.contains decl do
      throwError "coverage audit: no required claim states anything about {decl} or a caller of it"
  for decl in reviewedFunctions do
    unless found.contains decl do
      throwError "coverage audit: reviewed {decl} was not extracted"
  logInfo m!"COVERAGE ok: {found.size} extracted functions, each stated in a claim or called by one"

run_cmd auditCoverage
