import Lean
import Tauten

/-!
# Axiom audit and coverage of the extracted kernel

Adapted from writfold's `proofs/Audit.lean` and the P0 probe.

1. Every theorem under `Tauten` depends only on Lean's standard three axioms
   (`propext`, `Classical.choice`, `Quot.sound`). This reads the transitive
   axiom closure (`collectAxioms`); an Aeneas `axiom` for an opaque function,
   a `sorry` (`sorryAx`) or a native-evaluation axiom would fail it.
2. Every required claim exists and is a theorem. `proofs/mutants.py` also
   requires every theorem a mutant names to be in this list.
3. Every function extracted from `tauten_check_kernel` is reviewed (listed
   below) and is covered by a claim: it is mentioned in the statement of a
   required claim, or it is called (transitively, through the extracted
   definitions) by a function that is. Loop bodies and the ratchet's
   per-finding helpers are covered this way by the theorems about `apply`,
   `stale`, `count_errors` and `increases`. A function added to
   `start_from` without a theorem fails the build.

Constructor enumeration (docs/PROOF-PLAN.md section 2) is not relevant to
the kernel: it defines no refined type. It returns with the macro instances
in P3; the P0 prototype is in git history (docs/proofs/probe.md).
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
]

/-- The functions translated from `[package.metadata.charon] start_from`. -/
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

/-- Definitions Aeneas wrote for `tauten_check_kernel` functions: the
extracted functions return `Result _`; inductive types and their generated
helpers do not. -/
def extractedFunctions (env : Environment) : Array Name :=
  (env.constants.fold (init := (#[] : Array Name)) fun acc decl info =>
    if (`tauten_check_kernel).isPrefixOf decl && info.isDefinition && !decl.isInternal then
      let rec target : Expr → Expr
        | .forallE _ _ b _ => target b
        | e => e
      if (target info.type).getAppFn.isConstOf ``Aeneas.Std.Result then acc.push decl else acc
    else acc).qsort (fun a b => a.toString < b.toString)

/-- The extracted functions a definition's body uses. -/
def calls (env : Environment) (found : Array Name) (decl : Name) : Array Name :=
  match env.find? decl with
  | some info => match info.value? with
    | some value => value.getUsedConstants.filter found.contains
    | none => #[]
  | none => #[]

/-- The extracted functions stated in a claim, then everything they call. -/
def coveredByClaims (env : Environment) (found : Array Name) : Array Name := Id.run do
  let mut covered : Array Name := #[]
  let mut todo : Array Name := #[]
  for claim in claims do
    if let some info := env.find? claim then
      todo := todo ++ info.type.getUsedConstants.filter found.contains
  while h : todo.size > 0 do
    let decl := todo[todo.size - 1]
    todo := todo.pop
    unless covered.contains decl do
      covered := covered.push decl
      todo := todo ++ calls env found decl
  return covered

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
