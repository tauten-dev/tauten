import Tauten.Ratchet

/-!
# What the proof gate established for one claim (`tauten_check_kernel::outcome`)

`cargo tauten proofs` reads the gate's result lines into `outcome.Facts` and
calls the extracted `outcome`, which decides the claim's outcome and so
whether it is reported, and badged, as proved.

Proved here, over the extracted function:
* `outcome_proved_iff`: `Proved` exactly when the gate ran, all five stages
  reported `ok`, every theorem passed the axiom audit, the `total` theorem
  has the no-failure shape, every mutant was killed and the sources did not
  change during the run. No other combination of facts is reported proved.
* `outcome_tool_missing_iff`, `outcome_mutant_survived_iff`: the outcome
  names the first failure in gate order; a later failure never masks an
  earlier one.

Trusted, not proved: the shell's reading of the gate's output into facts
(`crates/tauten-check/src/proofs.rs`, tested against recorded gate output).
-/

open Aeneas Aeneas.Std Result
open tauten_check_kernel

namespace Tauten.Kernel

def passedN : outcome.Stage → Bool
  | .Ok => true | .Failed => false | .Absent => false

theorem passed_spec (s : outcome.Stage) : outcome.passed s = ok (passedN s) := by
  cases s <;> rfl

/-- All the facts that must hold for a claim to be proved. -/
def allGood (f : outcome.Facts) : Prop :=
  f.ran = true ∧ f.tools = .Ok ∧ f.extract = .Ok ∧ f.drift = .Ok ∧ f.build = .Ok ∧
    f.audit = .Ok ∧ f.theorems = true ∧ f.total = true ∧ f.mutants = true ∧ f.changed = false

theorem passedN_iff (s : outcome.Stage) : passedN s = true ↔ s = .Ok := by
  cases s <;> simp [passedN]

/-- Proved exactly when every fact is good. -/
theorem outcome_proved_iff (f : outcome.Facts) :
    outcome.outcome f = ok .Proved ↔ allGood f := by
  rcases f with ⟨ran, tools, extract, drift, build, audit, theorems, total, mutants, changed⟩
  simp only [outcome.outcome, allGood, passed_spec, bind_ok]
  cases ran <;> cases tools <;> simp [passedN] <;> cases extract <;> simp [passedN] <;>
    cases drift <;> simp [passedN] <;> cases build <;> simp [passedN] <;>
    cases audit <;> simp [passedN] <;> cases theorems <;> cases total <;> cases mutants <;>
    cases changed <;> simp

/-- The toolchain is reported missing exactly when the gate did not run or
its `tools` stage did not pass, whatever else was reported. -/
theorem outcome_tool_missing_iff (f : outcome.Facts) :
    outcome.outcome f = ok .ToolMissing ↔ (f.ran = false ∨ f.tools ≠ .Ok) := by
  rcases f with ⟨ran, tools, extract, drift, build, audit, theorems, total, mutants, changed⟩
  simp only [outcome.outcome, passed_spec, bind_ok]
  cases ran <;> cases tools <;> simp [passedN] <;> cases extract <;> simp [passedN] <;>
    cases drift <;> simp [passedN] <;> cases build <;> simp [passedN] <;>
    cases audit <;> simp [passedN] <;> cases theorems <;> cases total <;> cases mutants <;>
    cases changed <;> simp

/-- A surviving mutant is reported exactly when everything before the
mutant stage passed and some mutant survived. -/
theorem outcome_mutant_survived_iff (f : outcome.Facts) :
    outcome.outcome f = ok .MutantSurvived ↔
      (f.ran = true ∧ f.tools = .Ok ∧ f.extract = .Ok ∧ f.drift = .Ok ∧ f.build = .Ok ∧
        f.audit = .Ok ∧ f.theorems = true ∧ f.total = true ∧ f.mutants = false) := by
  rcases f with ⟨ran, tools, extract, drift, build, audit, theorems, total, mutants, changed⟩
  simp only [outcome.outcome, passed_spec, bind_ok]
  cases ran <;> cases tools <;> simp [passedN] <;> cases extract <;> simp [passedN] <;>
    cases drift <;> simp [passedN] <;> cases build <;> simp [passedN] <;>
    cases audit <;> simp [passedN] <;> cases theorems <;> cases total <;> cases mutants <;>
    cases changed <;> simp

/-- Non-vacuity: the all-good facts are proved, and a change during the run
or a drifted translation is not. -/
theorem outcome_examples :
    outcome.outcome ⟨true, .Ok, .Ok, .Ok, .Ok, .Ok, true, true, true, false⟩ = ok .Proved ∧
    outcome.outcome ⟨true, .Ok, .Ok, .Ok, .Ok, .Ok, true, true, true, true⟩ = ok .Drift ∧
    outcome.outcome ⟨true, .Ok, .Ok, .Failed, .Ok, .Ok, true, true, false, false⟩ = ok .Drift := by
  simp [outcome.outcome, passed_spec, passedN]

end Tauten.Kernel
