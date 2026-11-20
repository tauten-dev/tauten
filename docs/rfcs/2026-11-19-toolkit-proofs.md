---
rfc: "2026-11-19-toolkit-proofs"
title: "Every toolkit decision in the proved kernel"
status: accepted
implementation: in-progress
authors:
  - tknawara
created: 2026-11-19
updated: 2026-11-20
issue: null
milestone: 0.1.0
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Every toolkit decision in the proved kernel

Body: [docs/TOOLKIT-PROOF-PLAN.md](../TOOLKIT-PROOF-PLAN.md). Owner requirement (19 November 2026): the whole toolkit is proved by Lean or an equivalent, so that it raises no false positives and never claims a guarantee it does not have.

Decision: every verdict the checker gives (effects, outcome, subsumption, freshness, badges, exceptions, the exit verdict) is a kernel function proved before it is wired in. What cannot be proved (syn scanning, file IO) is listed in [proofs/TRUST.md](../../proofs/TRUST.md) with its tests. Remaining items are tracked as T4-T7 (#31-#34).

## Decision log

- 2026-11-20: record backfilled when the RFC process was adopted. The plan body is unchanged and stays at its original path because the site and other documents link to it; its own status lines are historical, this frontmatter is current.
