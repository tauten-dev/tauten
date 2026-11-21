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
3. Every theorem whose conclusion is a triple `f x₁ … ⦃ _ ⦄` over an
   extracted function, with or without hypotheses, is printed as
   `SUBJECT <theorem> <f>`, so `cargo tauten proofs draft` can join it to its
   function by name (it decides nothing).
4. Named assumptions (`@[tauten_assumption "ID"]`, see
   `Proofs/TautenAssumption.lean` and the section below): each theorem's
   `PREMISES` and, for a witness, what it `ESTABLISHES`.
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

/-- The extracted function a theorem's conclusion is a triple about (`… ⦃ _ ⦄`
over an application of a constant), whatever its hypotheses. Read only by
`cargo tauten proofs draft` to join a theorem to its function by its fully
qualified name; it decides nothing. -/
def subjectOf (type : Expr) : MetaM (Option Name) :=
  forallTelescope type fun _ body => do
    unless body.isAppOfArity ``Aeneas.Std.WP.spec 3 do return none
    let fn := body.getAppArgs[1]!.getAppFn
    unless fn.isConst do return none
    return some fn.constName!

/-! ## Named assumptions (A8)

3. For every theorem, the named assumptions (`@[tauten_assumption "ID"]`)
   its hypotheses mention, through definitions of the `Proofs` library, are
   printed as `PREMISES <theorem>: [IDs]`. A theorem with no tagged
   hypothesis whose conclusion is a tagged proposition, a conjunction of
   them or an existential over them (a witness that the assumptions can
   hold together) is printed as `ESTABLISHES <theorem>: [IDs]`. A negated
   or otherwise wrapped occurrence establishes nothing.
-/

/-- Whether `decl` belongs to the `Proofs` library. -/
def inProofs (env : Environment) (decl : Name) : Bool :=
  match env.getModuleIdxFor? decl with
  | some idx => (`Proofs).isPrefixOf env.header.moduleNames[idx.toNat]!
  | none => false

/-- The assumption IDs mentioned by `e`, unfolding definitions of the
`Proofs` library (never theorems, never other libraries). -/
def assumptionsIn (env : Environment) (e : Expr) : Array String := Id.run do
  let mut seen : NameSet := {}
  let mut todo : Array Name := e.getUsedConstants
  let mut ids : Array String := #[]
  while h : todo.size > 0 do
    let c := todo[todo.size - 1]
    todo := todo.pop
    if seen.contains c then continue
    seen := seen.insert c
    if let some id := tautenAssumptionAttr.getParam? env c then
      unless ids.contains id do ids := ids.push id
    else if inProofs env c then
      if let some (.defnInfo d) := env.find? c then
        todo := todo ++ d.value.getUsedConstants
  return ids.qsort (· < ·)

/-- The IDs a conclusion establishes: a tagged proposition, a conjunction
or an existential of such. -/
partial def establishedBy (env : Environment) (e : Expr) : MetaM (Array String) := do
  let e ← whnfR e
  if e.isAppOfArity ``And 2 then
    return (← establishedBy env e.appFn!.appArg!) ++ (← establishedBy env e.appArg!)
  if e.isAppOfArity ``Exists 2 then
    let body := e.appArg!
    if let .lam n ty b bi := body then
      return ← withLocalDecl n bi ty fun x => establishedBy env (b.instantiate1 x)
    return #[]
  let fn := e.getAppFn
  if fn.isConst then
    if let some id := tautenAssumptionAttr.getParam? env fn.constName! then
      return #[id]
  return #[]

/-- (premise IDs, established IDs) of a theorem's statement. -/
def assumptionFacts (env : Environment) (type : Expr) : MetaM (Array String × Array String) :=
  forallTelescope type fun xs body => do
    let mut premises : Array String := #[]
    for x in xs do
      let t ← inferType x
      if ← isProp t then
        for id in assumptionsIn env t do
          unless premises.contains id do premises := premises.push id
    let sorted := premises.qsort (· < ·)
    if sorted.isEmpty then
      let est := (← establishedBy env body).qsort (· < ·)
      return (sorted, est.toList.eraseDups.toArray)
    return (sorted, #[])

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
      if let some fn ← liftTermElabM (subjectOf info.type) then
        logInfo m!"SUBJECT {decl} {fn}"
      let (premises, established) ← liftTermElabM (assumptionFacts env info.type)
      logInfo m!"PREMISES {decl}: {premises.toList}"
      unless established.isEmpty do
        logInfo m!"ESTABLISHES {decl}: {established.toList}"
  logInfo m!"AXIOM_AUDIT ok: {theorems.size} theorems, axioms ⊆ {allowedAxioms}"

run_cmd audit
