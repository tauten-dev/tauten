import Extracted.TautenCheckKernel

/-!
# Severity (`tauten_check_kernel::severity`)

`tauten_check::Severity::decide` calls the extracted `severity` for every
profile finding: compiler messages (fallback: the compiler's own severity)
and tauten's structural findings (fallback: `Error`).
-/

open Aeneas Aeneas.Std Result
open tauten_check_kernel

namespace Tauten.Kernel

/-- Only `deny` blocks; `is_blocking` never fails. -/
theorem is_blocking_spec (l : severity.Level) :
    ∃ b, severity.is_blocking l = ok b ∧ (b = true ↔ l = .Deny) := by
  cases l <;> simp [severity.is_blocking]

/-- A level, when the rule has one in the tier, decides: `deny` is an error,
`warn` a warning, whatever the fallback. Without a level the fallback
stands. Never fails. -/
theorem severity_spec (fallback : severity.Severity) :
    severity.severity none fallback = ok fallback ∧
    severity.severity (some .Deny) fallback = ok .Error ∧
    severity.severity (some .Warn) fallback = ok .Warning := by
  simp [severity.severity, severity.is_blocking]

end Tauten.Kernel
