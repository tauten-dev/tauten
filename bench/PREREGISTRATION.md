# Preregistration (DRAFT, not frozen)

This file is frozen by committing it, and recording its hash in the results,
**before** any held-out run. Until then it may change.

## Hypothesis

Agents working in arm C (the tauten profile enforced in their loop) leave
fewer escaped defects than in arm A (plain Rust), without a large loss in
task success or a large rise in cost. Arm B (rules stated, not enforced)
separates instruction from enforcement.

## Primary outcome

**Escaped-defect rate:** the share of trials whose final repo fails at least
one hidden-oracle test, or whose oracle fails to build. One value per trial.

## Secondary outcomes

Reported, but not used for the go/no-go:
- reachable-panic rate;
- visible tests pass;
- test tampering;
- stubs added;
- profile errors in the final repo;
- agent wall time;
- token usage, once captured.

## Design

| Item | Setting |
|---|---|
| Tasks | Dev split (currently 4): used for tuning the profile and harness. Held-out split (≥ 16, not yet written): oracles written and hashed before the profile is frozen for the run. |
| Arms | A, B, C, the same tasks for each |
| Trials | ≥ 5 per task per arm for held-out; 3 for the pilot |
| Model | One named model and version for the primary analysis. A second model as a replication, reported separately. |
| Isolation | Fresh container per trial; same turn and time limits for every arm |
| Order | Trials interleaved across arms to spread provider-side drift |

## Analysis

- Paired by task.
- Difference of arm means (C − A, B − A), averaged over tasks.
- 95% bootstrap CI resampling tasks: 10,000 resamples, seed 20261118.
- If the CI includes 0, the report says "no measured difference".
- All trials are reported, including timeouts and crashes; a trial that
  produces no change counts as a defect if its oracle fails.

## Go/no-go (from PLAN.md §9)

- **Stop** the product claims if, after the pilot and one tuning pass, C vs A
  shows no reduction in escaped defects: the CI includes 0 and the point
  estimate is under 15% relative.
- **Cut rules** if C lowers task success by more than 10 points, or raises
  median cost by more than 40%, without at least a 30% defect reduction.

## Honesty rules for publication

- Every number carries the model, date, n and CI.
- Null and negative results are published.
- Raw JSONL and transcripts are published with the harness commit.
- No result from the dev split is presented as a held-out result.
