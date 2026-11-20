import Tauten.Control
import Tauten.Severity

/-!
# What a rule may claim (`tauten_check_kernel::rule`)

Proved here, over the extracted functions:
* `incomplete_never_error`: a finding of an incomplete rule is a warning,
  whatever its level.
* `finding_severity_spec`: a complete rule's finding has the severity its
  level gives (`deny` an error, `warn` a warning, the fallback without a
  level).
* `unparsed_not_clean`: an input the rule could not read is never clean;
  `clean_spec`: a read input is clean exactly when nothing was violated.
* `fixtures_ok_iff`: the table check passes exactly when every complete
  rule that blocks in some tier names a reject fixture.

Trusted, not proved: the shell's assignment of `complete`, `sees` and
`dominion` per rule in `rules.toml` (reviewed text, summarised in
`proofs/TRUST.md`), and that a listed fixture exercises its rule (the
fixture suite checks that each one reports it).
-/

open Aeneas Aeneas.Std Result
open tauten_check_kernel

namespace Tauten.Kernel

/-- An incomplete rule never blocks. -/
theorem incomplete_never_error (level : Option severity.Level) (fallback : severity.Severity) :
    rule.finding_severity level fallback false = ok severity.Severity.Warning := by
  simp [rule.finding_severity]

/-- A complete rule's finding has the severity its level gives. -/
theorem finding_severity_spec (fallback : severity.Severity) :
    rule.finding_severity none fallback true = ok fallback ∧
    rule.finding_severity (some .Deny) fallback true = ok .Error ∧
    rule.finding_severity (some .Warn) fallback true = ok .Warning := by
  simp [rule.finding_severity, severity.severity, severity.is_blocking]

/-- An input the rule could not read is never clean. -/
theorem unparsed_not_clean (violated : Bool) :
    rule.clean rule.Input.Unparsed violated = ok false := by
  simp [rule.clean]

/-- A read input is clean exactly when nothing was violated. -/
theorem clean_spec (input : rule.Input) (violated : Bool) :
    rule.clean input violated ⦃ r => (r = true ↔ input = .Read ∧ violated = false) ⦄ := by
  cases input <;> cases violated <;> simp [rule.clean]

def fixturesOkN (l : List rule.RuleFacts) : Bool :=
  l.all (fun r => !r.complete || !r.blocks || r.fixture)

theorem fixtures_ok_spec (rules : Slice rule.RuleFacts) :
    rule.fixtures_ok rules ⦃ r => r = fixturesOkN rules.val ⦄ := by
  unfold rule.fixtures_ok rule.fixtures_ok_loop
  apply WP.spec_mono (fold_loop rules
    (fun a r => a && (!r.complete || !r.blocks || r.fixture)) _ ?_ ?_ true 0#usize (by simp))
  · intro r hr; rw [hr, foldl_and]; simp [fixturesOkN]
  · intro a i h
    have hlt : i < Slice.len rules := by scalar_tac
    simp only [rule.fixtures_ok_loop.body, hlt, ite_true, get_ok rules i h, bind_ok, lift]
    cases a
    · simp
    · cases hc : (rules.val[i.val]).complete <;> cases hb : (rules.val[i.val]).blocks <;> simp [hc, hb]
  · intro a i h
    have hlt : ¬ i < Slice.len rules := by scalar_tac
    simp only [rule.fixtures_ok_loop.body, hlt, ite_false, WP.spec_ok]

/-- The table check passes exactly when every complete rule that blocks
somewhere names a reject fixture. -/
theorem fixtures_ok_iff (rules : Slice rule.RuleFacts) :
    rule.fixtures_ok rules ⦃ r => (r = true ↔
      ∀ x ∈ rules.val, x.complete = true → x.blocks = true → x.fixture = true) ⦄ := by
  apply WP.spec_mono (fixtures_ok_spec rules)
  intro r hr
  subst hr
  simp only [fixturesOkN, List.all_eq_true, Bool.or_eq_true, Bool.not_eq_true']
  constructor
  · intro h x hx hc hb
    rcases h x hx with (h1 | h1) | h1
    · rw [hc] at h1; cases h1
    · rw [hb] at h1; cases h1
    · exact h1
  · intro h x hx
    cases hc : x.complete <;> cases hb : x.blocks <;> simp_all

end Tauten.Kernel
