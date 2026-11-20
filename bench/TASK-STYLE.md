# Task style (calibration record)

Round 1 (2026-11-19, original oracles): arm A escaped defects 0/15 Sonnet 5.5, 0/15 Haiku 5.5.
Round 2 (2026-11-19, reviewed oracles, dev+release): 1/15 Sonnet 5.5, 0/15 Haiku 5.5.

Round 3 style knob S1: prompts no longer end with "The ground rules in SPEC.md apply to
everything you add." Each host README links SPEC.md; SPEC's ground rules state that they
apply to every feature. Every oracle expectation is still fixed by SPEC.md + the prompt.
Unchanged knobs: requirements per task, host size, --max-turns 60, 900 s, 2 USD.

Deviation D1 (not a style knob): H02-statement, H07-include, H15-ratelimit and
H19-quoted-names were moved from held-out to calibration before any freeze, chosen by
random.Random(20261118) over sorted ids (T x2, A x1, N x1, in that order). Reason: the
listed knobs were exhausted or not binding (turns 11-34 of 60; SPEC read in 30/30 trials);
the demoted tasks estimate the held-out arm A rate directly. They never return to held-out.
Held-out is 16 tasks (T 9, A 3, N 4). Round 3 runs 10 trials per task (protocol: 5).

Go/no-go after round 3, per model over the 4 probe tasks (40 trials):
- GO: 10-26 of 40 escaped; at least 2 probe tasks with 2+ escapes; at least one escape on
  H07, H19 or H15 in the class its expected rule targets; no determinacy problem found on
  transcript review. Then freeze and run held-out (16 x 3 x 10).
- GREY (5-9 of 40): do not run held-out as designed; run the finding-validity audit (E1).
- NO-GO (4 or fewer in both models, or escapes only in non-targeted classes): stop the
  held-out; publish the calibration rounds; run E1; decide on maintenance chains (E2) separately.
- Above 26 of 40: check for a broken oracle or prompt first.
No fourth calibration round (protocol cap: 3).

## Round 3 result (2026-11-19) and decision

Arm A escaped defects, 10 trials per task:

| task | Sonnet 5.5 | Haiku 5.5 |
|---|---|---|
| H02-statement (probe) | 0/10 | 0/10 |
| H07-include (probe) | 0/10 | 0/10 |
| H15-ratelimit (probe) | 1/10 | 0/10 |
| H19-quoted-names (probe) | 0/10 | 0/10 |
| K01-discount | 0/10 | 0/10 |
| K02-lists | 0/10 | 0/10 |
| K03-depends | 0/10 | 0/10 |

Probe total: Sonnet 1/40, Haiku 0/40. No infrastructure errors. Decision: **NO-GO**
(4 or fewer in both models). The held-out run is stopped and its budget is not spent.
Across all three rounds arm A escaped 2 of 200 trials.

## E1: finding-validity audit (registered before any finding is classified)

Population: every deny-level profile finding (`cargo tauten check --format json
--no-baseline`, cargo-tauten 0.0.2 as used in the runs) in the final repos of the 200
calibration trials (rounds 1-3, arm A): 420 findings in 98 trials.

Sample: 120 findings drawn by `random.Random(20261119).sample` over the findings sorted
by (trial label, task, trial, file, line, rule). Results are reported for the sample;
the 420 are listed.

Primary outcome: the share of sampled findings that are confirmed defects. A finding is
*confirmed* if a legal input (per SPEC.md and the task prompt, expected output derivable
from them) makes the binary deviate (wrong output, panic, hang) in the dev or release
profile, and repairing only the flagged expression removes the deviation. It is
*refuted* if a written bound argument shows no legal input reaches a failing value.
Otherwise *undetermined*, counted and listed. Two independent classifiers, each blind
to the other; disagreements are listed and count as undetermined in the primary share.
Reported per rule with n and an exact 95% interval. If under 5% of the audited findings
are confirmed, the report states that the profile's findings on agent-written code in
these hosts are predominantly not defects, and no defect-reduction claim is made.

Secondary: for each escaped trial (2), whether a deny-level finding lies on the failing
path.
