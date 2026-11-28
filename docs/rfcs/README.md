# tauten RFCs

An RFC is the durable record of a decision that is costly to reverse or that
touches more than one part of tauten, together with its evidence. Work is
tracked in [GitHub issues](https://github.com/tauten-dev/tauten/issues) and
[milestones](https://github.com/tauten-dev/tauten/milestones), ordered in the
[roadmap](../ROADMAP.md). An RFC is the design behind an issue; it never
replaces the issue.

This process is adopted from writfold's (20 November 2026) so that the
owner's projects share one. The standing authorities sit above every RFC:
[AGENTS.md](../../AGENTS.md) and the acceptance gate there. Every accepted
proposal is general across projects, is decided by the proved kernel or the
compiler (anything weaker is labelled honestly), and fits the vision. An RFC
cannot waive any of those rules.

## When an RFC is required

Write an RFC before implementing:

- a new rule, lint, verb, flag, exit code, output line or JSON key, or a
  change to one that an existing project can notice (anything that would go
  in [CHANGELOG.md](../../CHANGELOG.md));
- a new or changed file format (`claims.toml`, `claims.json`,
  `assumptions.toml`, baselines, manifests);
- new or changed decision logic: what counts as proved, fresh, admitted,
  blocking, allowed or refused. Its proof obligations come first (below);
- a new kernel primitive, dependency or runtime component that limits future
  design;
- an exception to a rule in [AGENTS.md](../../AGENTS.md).

Bug fixes, refactors that keep behaviour, dependency upkeep, documentation
corrections and gate or release plumbing do not need one. Use an issue and a
commit that names it.

Research and planning agents write proposals as `draft` RFCs. A roadmap
issue whose design is fully stated in its body and in an accepted RFC (for
example the [roadmap RFC](2026-11-19-roadmap-five-primitives.md)) needs no
RFC of its own unless implementation changes that design.

## File and heading format

- Filename: `YYYY-MM-DD-kebab-title.md`. The date is the day drafting started
  and equals `created`. No number is allocated.
- Heading: `# RFC: Title`. The RFC id is the filename without `.md`.
- `0000-template.md` is reserved and is not an RFC.
- A retitled RFC keeps its filename. Revise the one file and record material
  changes in its decision log. Do not make `v2` or `final` copies.

## Frontmatter

```yaml
---
rfc: "2026-11-20-short-descriptive-title"
title: "Short descriptive title"
status: draft
implementation: not-started
authors:
  - handle
created: 2026-11-20
updated: 2026-11-20
issue: null
milestone: null
supersedes: []
superseded_by: []
blocked_on: []
---
```

- `status` is `draft`, `accepted`, `rejected` or `superseded`.
- `implementation` is `not-started`, `in-progress`, `partial`, `complete`,
  `removed` or `n/a`.
- `issue` is the tracking issue URL; `milestone` the milestone title or
  `null`.
- `blocked_on` names concrete dependencies: RFC ids, issue URLs or an owner
  question.
- `status` and `implementation` are recorded only in the frontmatter.

## Lifecycle

```text
draft ──owner decision──▶ accepted ──accepted replacement──▶ superseded
  └───owner decision──▶ rejected
```

- **Draft:** under design. A draft may be merged to `main`; nothing may be
  built on its authority.
- **Accepted:** the owner approved the decision and its boundaries.
  `implementation` advances as work lands, each commit naming the issue.
- **Rejected:** keep the file with a short rationale.
- **Superseded:** a later accepted RFC owns the decision. Link both ways.

Agents may draft RFCs and propose changes. Only the owner accepts or rejects
one.

## Proof obligations come first

An RFC that changes decision logic lists, before its design:

1. The kernel function in `crates/tauten-check-kernel`, with its inputs and
   outputs.
2. The theorems to be proved in Lean (`proofs/Tauten/`) and the mutants
   (`proofs/mutants.py`) that must break them.
3. What stays tested and unproved (syn scanning, file IO, rendering), and
   how each part is tested.

The proofs gate (drift, `lake build`, axiom audit, mutants) must pass before
the kernel is wired into the checker.

## Plans written before this process

Before 20 November 2026, plans were written as `docs/*.md` files. They stay
where they are, because the site and other documents link to them. Each has
a backfilled RFC record below whose frontmatter is current; the plan bodies
are unchanged, including their own status lines, which are historical.

## Registry

Rows are in creation order.

| RFC | Decision | Status | Implementation |
|---|---|---|---|
| [2026-11-18-plan-a](2026-11-18-plan-a.md) | Strict Rust profile instead of a new language ([PLAN.md](../PLAN.md)) | accepted | partial |
| [2026-11-18-own-proofs](2026-11-18-own-proofs.md) | Lean proofs of tauten's own enforcement code ([PROOF-PLAN.md](../PROOF-PLAN.md)) | accepted | complete |
| [2026-11-19-discipline](2026-11-19-discipline.md) | Code discipline + proofs ([DISCIPLINE-PROPOSAL.md](../DISCIPLINE-PROPOSAL.md), [D0-SPEC.md](../D0-SPEC.md)) | accepted | in-progress |
| [2026-11-19-toolkit-proofs](2026-11-19-toolkit-proofs.md) | Every toolkit decision in the proved kernel ([TOOLKIT-PROOF-PLAN.md](../TOOLKIT-PROOF-PLAN.md)) | accepted | in-progress |
| [2026-11-19-roadmap-five-primitives](2026-11-19-roadmap-five-primitives.md) | Five general primitives; roadmap issues #1-#38 ([ROADMAP.md](../ROADMAP.md)) | accepted | in-progress |
| [2026-11-21-draft-claim-entries](2026-11-21-draft-claim-entries.md) | Reviewable claim-entry drafts from existing declarations and mutants ([#49](https://github.com/tauten-dev/tauten/issues/49)) | accepted | partial |
| [2026-11-21-recover-proof-records](2026-11-21-recover-proof-records.md) | Retry unavailable tools and recover generated proof artifacts on merge ([#50](https://github.com/tauten-dev/tauten/issues/50), [#52](https://github.com/tauten-dev/tauten/issues/52)) | accepted | complete |
| [2026-11-21-shared-test-module-allow](2026-11-21-shared-test-module-allow.md) | Accept a reasoned file-level `allow(dead_code)` in a non-root test module ([#59](https://github.com/tauten-dev/tauten/issues/59)) | accepted | complete |
| [2026-11-21-publish-absent-target](2026-11-21-publish-absent-target.md) | Companion theorems for atomic publish to an absent target ([#58](https://github.com/tauten-dev/tauten/issues/58)) | accepted | complete |
| [2026-11-21-bounded-capability-reads](2026-11-21-bounded-capability-reads.md) | Require an explicit byte limit on `ReadDir` and attenuate an `Env` snapshot ([#64](https://github.com/tauten-dev/tauten/issues/64)) | accepted | not-started |
| [2026-11-21-result-world-induction](2026-11-21-result-world-induction.md) | One invariant lemma for result-returning machines and worlds, `rsys_inv_of_step` ([#60](https://github.com/tauten-dev/tauten/issues/60)) | accepted | not-started |
| [2026-11-28-bounded-build-resources](2026-11-28-bounded-build-resources.md) | Limit the proof gate's container and keep mutant jobs × Lean threads within the CPUs allowed ([#82](https://github.com/tauten-dev/tauten/issues/82)) | accepted | in-progress |
