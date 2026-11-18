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
   below) and is mentioned in the statement of at least one claim, so a
   function added to `start_from` without a theorem fails the build.

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
]

/-- The functions translated from `[package.metadata.charon] start_from`. -/
def reviewedFunctions : List Name := [
  ``tauten_check_kernel.tiers.rank,
  ``tauten_check_kernel.tiers.may_depend_on,
  ``tauten_check_kernel.severity.is_blocking,
  ``tauten_check_kernel.severity.severity,
  ``tauten_check_kernel.exception.permitted,
  ``tauten_check_kernel.exception.excuse,
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

def auditCoverage : CommandElabM Unit := do
  let env ← getEnv
  let found := extractedFunctions env
  logInfo m!"EXTRACTED {found.toList}"
  for decl in found do
    unless reviewedFunctions.contains decl do
      throwError "coverage audit: extracted {decl} is not in reviewedFunctions"
    let covered := claims.any fun claim =>
      match env.find? claim with
      | some info => (info.type.find? fun e => e.isConstOf decl).isSome
      | none => false
    unless covered do
      throwError "coverage audit: no required claim states anything about {decl}"
  for decl in reviewedFunctions do
    unless found.contains decl do
      throwError "coverage audit: reviewed {decl} was not extracted"
  logInfo m!"COVERAGE ok: {found.size} extracted functions, each stated in a claim"

run_cmd auditCoverage
