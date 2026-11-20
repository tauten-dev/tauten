---
rfc: "2026-11-19-roadmap-five-primitives"
title: "Five general primitives; roadmap issues #1-#38"
status: accepted
implementation: in-progress
authors:
  - tknawara
created: 2026-11-19
updated: 2026-11-20
issue: https://github.com/tauten-dev/tauten/issues/30
milestone: null
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Five general primitives; roadmap issues #1-#38

Body: [docs/ROADMAP.md](../ROADMAP.md) and the issue bodies of #1-#38, each of which states its problem, kernel primitive, theorems, mutants and acceptance. Synthesised by a planning agent from a durability plan for the gripsack crash and filesystem gaps, a survey of seven of the owner's Rust projects, and a survey of safety mechanisms in other languages and the C++ safety-profile proposals. Accepted by the owner on 19 November 2026.

Decision: about sixty proposals reduce to five general primitives (controls and badges, bindings, step machines, registries, effect rows). An item is accepted only if it is general across projects, decided by the proved kernel or the compiler, and makes the flow lighter. Rejected and parked ideas, with what would reopen each, are in #30. K004 (a proof-shape lint) was dropped for an extract-only stage (#8).

## Decision log

- 2026-11-20: record backfilled when the RFC process was adopted. The plan body is unchanged and stays at its original path because the site and other documents link to it; its own status lines are historical, this frontmatter is current.
