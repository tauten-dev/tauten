# E1: finding-validity audit (2026-11-19)

Registered in `bench/TASK-STYLE.md` (commit 26fa13b) before any finding was classified.

## Question

Calibration rounds 1-3 (200 arm-A trials, Sonnet 5.5 and Haiku 5.5) left agent-written code
with deny-level tauten findings in 98 trials, but almost no oracle-detected defects (2 of
200). Are the findings defects the oracles missed, or not defects?

## Population and sample

- `findings.json`: all 420 deny-level findings (cargo-tauten 0.0.2, `--no-baseline`, the
  host's tiers via `--assume-tier`), re-derived from the archived final repos by
  `extract.py`. Per-trial counts match the counts recorded during the runs in all 200
  trials. Oracle test files copied into the archives were excluded (they were not present
  when the runs measured).
- `sample.json`: 120 findings, `random.Random(20261119).sample` over the 420 sorted by
  (label, task, trial, file, line, rule). Rules: N001 67, P003 34, N002 14, E001 3, E003 2.

## Method

Two classifiers, run blind to each other on the same four batches of 30, with `BRIEF.md`
as their only instructions: classifier A (Claude Opus 5.5), classifier B (Claude Sonnet
5.5). Outputs: `classifier-A/`, `classifier-B/`. A confirmed verdict required a built,
executed demonstration plus a one-expression repair; a refuted verdict required a written
bound argument citing lines.

## Result

| verdict (agreed) | count |
|---|---|
| confirmed | **0** |
| refuted | 118 |
| undetermined | 2 |

Confirmed share: **0/120, exact 95% interval 0.0%-3.0%.**

Per rule (confirmed / n, exact 95% upper bound): N001 0/67 (5.4%), P003 0/34 (10.3%),
N002 0/14 (23.2%), E001 0/3, E003 0/2.

Undetermined:
- **F033** (E001, K02-lists, both classifiers undetermined): `map_err(|_| ...)` turns an
  i64-overflowing index into `value out of range`. The prompt's `index out of range` and
  SPEC G1's `value out of range` both plausibly apply; the expected output is not
  derivable, so neither confirmed nor refuted.
- **F264** (E001, K02-lists, A confirmed, B refuted): `selector()` maps an i64-overflowing
  index to `index out of range` before the key is looked up, so
  `get c.conf 'port[99999999999999999999]'` on a non-list key prints `index out of range`
  instead of ``key `port` is not of type list`` (dev and release; A's one-expression
  repair fixes it). B argued the overflow error is correct for list keys and did not test
  the non-list order. Per the registration a disagreement counts as undetermined. Even
  counted as confirmed, the share is 1/120 (0.8%, interval 0.02%-4.6%).

Typical refutations: u64 operands widened to u128 (products of two u64 values fit);
subtractions behind a guard or a clamp; indices into vectors that only grow, taken from a
name map filled with `len()` before each push; string slices at a `char_indices` offset of
an ASCII byte; `/ 100` by a constant.

Secondary (the two escaped trials, `secondary.md`): no deny-level finding lies on the
failing path in either (K02-lists r2 #0: an unflagged `-0` branch; H15-ratelimit r3 #1:
no findings at all).

## Registered conclusion

Under 5% of the audited findings are confirmed. **The profile's findings on agent-written
code in these hosts are predominantly not defects, and no defect-reduction claim is made.**

What this does not show: it says nothing about code written under the profile (arm C), or
about longer-lived code where today's bound (a guard three lines up, a vector that only
grows) is broken by a later edit. That is the question E2 (maintenance chains) would test;
it is not run and not decided.
