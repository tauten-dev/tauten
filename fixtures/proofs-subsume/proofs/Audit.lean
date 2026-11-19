import Lean
import Proofs

/-!
# Axiom audit and total-theorem check (tauten proofs template)

Read by `proofs/gate.py`; `cargo tauten proofs` turns the result into one
outcome per claim of `proofs/claims.toml`.

1. Every theorem declared in the `Proofs` library depends only on Lean's
   standard three axioms (`propext`, `Classical.choice`, `Quot.sound`). This
   reads the transitive axiom closure (`collectAxioms`), so a `sorry`
   (`sorryAx`), an Aeneas `axiom` for an opaque function or a
   native-evaluation axiom fails the build. Each theorem is printed as
   `AXIOMS <name>: [...]`.
2. A theorem of the shape `∀ (x₁ : α₁) … (xₙ : αₙ), f x₁ … xₙ ⦃ _ ⦄`, where
   no `αᵢ` is a proposition (no hypotheses) and `f` is applied to exactly
   those variables, states that `f` never fails (no panic, no overflow, no
   out-of-bounds access, terminates) for every input. Each is printed as
   `TOTAL <theorem> <f>`. A claim's `total` theorem must be one of these
   before `cargo tauten check` lets it subsume arithmetic and indexing
   findings.
-/

open Lean Elab Command Meta

def allowedAxioms : List Name := [``propext, ``Classical.choice, ``Quot.sound]

/-- Theorems declared in a module of the `Proofs` library. -/
def proofTheorems (env : Environment) : Array Name :=
  (env.constants.fold (init := (#[] : Array Name)) fun acc decl info =>
    match env.getModuleIdxFor? decl with
    | some idx =>
      let module := env.header.moduleNames[idx.toNat]!
      if (`Proofs).isPrefixOf module && info.isThm && !decl.isInternal then acc.push decl
      else acc
    | none => acc).qsort (fun a b => a.toString < b.toString)

/-- The function a theorem states never fails for every input, if it has
that shape. -/
def totalOver (type : Expr) : MetaM (Option Name) :=
  forallTelescope type fun xs body => do
    for x in xs do
      if ← isProp (← inferType x) then return none
    unless body.isAppOfArity ``Aeneas.Std.WP.spec 3 do return none
    let program := body.getAppArgs[1]!
    let fn := program.getAppFn
    unless fn.isConst do return none
    let args := program.getAppArgs
    unless args.size == xs.size do return none
    for (arg, x) in args.zip xs do
      unless arg == x do return none
    return some fn.constName!

def audit : CommandElabM Unit := do
  let env ← getEnv
  let theorems := proofTheorems env
  if theorems.isEmpty then
    throwError "axiom audit: the Proofs library declares no theorem"
  for decl in theorems do
    let axioms := (← collectAxioms decl).qsort (fun a b => a.toString < b.toString)
    logInfo m!"AXIOMS {decl}: {axioms.toList}"
    for ax in axioms do
      unless allowedAxioms.contains ax do
        throwError "axiom audit: {decl} depends on {ax}, which is not a standard Lean axiom"
    if let some info := env.find? decl then
      if let some fn ← liftTermElabM (totalOver info.type) then
        logInfo m!"TOTAL {decl} {fn}"
  logInfo m!"AXIOM_AUDIT ok: {theorems.size} theorems, axioms ⊆ {allowedAxioms}"

run_cmd audit
