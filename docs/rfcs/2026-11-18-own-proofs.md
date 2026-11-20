---
rfc: "2026-11-18-own-proofs"
title: "Lean proofs of tauten's own enforcement code"
status: accepted
implementation: complete
authors:
  - tknawara
created: 2026-11-18
updated: 2026-11-20
issue: null
milestone: null
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Lean proofs of tauten's own enforcement code

Body: [docs/PROOF-PLAN.md](../PROOF-PLAN.md).

Decision: tauten's own decisions (tiers, severities, the baseline ratchet, subsumption) live in `crates/tauten-check-kernel`, are extracted with Charon and Aeneas and proved in Lean, with a drift check, an axiom audit and calibrated mutants in the gate. P0-P5 landed; later kernel work continues under [2026-11-19-toolkit-proofs](2026-11-19-toolkit-proofs.md).

## Decision log

- 2026-11-20: record backfilled when the RFC process was adopted. The plan body is unchanged and stays at its original path because the site and other documents link to it; its own status lines are historical, this frontmatter is current.
