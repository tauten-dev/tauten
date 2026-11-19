import Extracted.TautenCheckKernel

/-!
# Proof subsumption (`tauten_check_kernel::subsume`, P5)

`cargo tauten check` calls the extracted `subsumes` for every TAU-N001
(arithmetic) and TAU-P003 (indexing) finding that lies inside a function
with a claim in a project's `proofs/claims.toml`. A subsumed finding is
reported as "subsumed by proof <theorem>" instead of as an error.

Trusted, not proved (`docs/proofs/README.md`, `proofs/TRUST.md`): that the
shell finds the enclosing function correctly, computes `fresh` from the
fingerprint recorded by `cargo tauten proofs`, and sets `proved` only for
a recorded `proved` outcome whose claim has a total no-failure theorem.
-/

open Aeneas Aeneas.Std Result
open tauten_check_kernel

namespace Tauten.Kernel

/-- The kernel tier. -/
def isKernelTier : tiers.Tier → Bool
  | .Kernel => true
  | .Core => false
  | .Edge => false

/-- The findings a proof can subsume: arithmetic (TAU-N001) and indexing
(TAU-P003). -/
def isPanicFinding : subsume.Finding → Bool
  | .Arithmetic => true
  | .Indexing => true
  | .Other => false

/-- A finding is subsumed iff the crate is a kernel crate, the finding is
about arithmetic or indexing, the claim lists its rule, the claim is proved
and the proof is for the current source. Never fails. -/
theorem subsumes_spec (t : tiers.Tier) (f : subsume.Finding) (listed proved fresh : Bool) :
    subsume.subsumes t f listed proved fresh =
      ok (isKernelTier t && isPanicFinding f && listed && proved && fresh) := by
  cases t <;> cases f <;> cases listed <;> cases proved <;> cases fresh <;>
    simp [subsume.subsumes, isKernelTier, isPanicFinding]

/-- A proof recorded for another source never subsumes a finding. -/
theorem stale_proof_never_subsumes (t : tiers.Tier) (f : subsume.Finding) (listed proved : Bool) :
    subsume.subsumes t f listed proved false = ok false := by
  cases t <;> cases f <;> cases listed <;> cases proved <;> simp [subsume.subsumes]

/-- Outside kernel crates nothing is subsumed. -/
theorem subsumes_only_in_kernel (f : subsume.Finding) (listed proved fresh : Bool) :
    subsume.subsumes .Core f listed proved fresh = ok false ∧
    subsume.subsumes .Edge f listed proved fresh = ok false := by
  cases f <;> cases listed <;> cases proved <;> cases fresh <;> simp [subsume.subsumes]

/-- Only arithmetic and indexing findings are subsumed; and they are, when
everything else holds (non-vacuity). -/
theorem subsumes_only_arithmetic_and_indexing (t : tiers.Tier) (listed proved fresh : Bool) :
    subsume.subsumes t .Other listed proved fresh = ok false ∧
    subsume.subsumes .Kernel .Arithmetic true true true = ok true ∧
    subsume.subsumes .Kernel .Indexing true true true = ok true := by
  cases t <;> cases listed <;> cases proved <;> cases fresh <;> simp [subsume.subsumes]

end Tauten.Kernel
