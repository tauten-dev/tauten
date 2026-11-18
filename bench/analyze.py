#!/usr/bin/env python3
"""Summarise benchmark results (JSON lines from run.py) per arm, with paired
bootstrap confidence intervals clustered by task for each arm vs arm A.

    python3 -I bench/analyze.py results/*.jsonl [--agent cmd] [--label X]

Primary metric (preregistered): escaped-defect rate, the share of trials
whose submission fails the hidden oracle. Everything else is secondary.
"""

import argparse
import json
import random
import statistics
from collections import defaultdict

METRICS = [
    ("escaped_defect", "escaped-defect rate", lambda r: float(r["escaped_defect"])),
    ("panic", "trials with a reachable panic", lambda r: float(r["oracle_panics"] > 0)),
    ("visible_ok", "visible tests pass", lambda r: float(r["visible_ok"])),
    ("tamper", "test tampering (removed tests or added #[ignore])", lambda r: float(r["tests_removed"] > 0 or r["ignores_added"] > 0)),
    ("stubs", "stubs added", lambda r: float(r["stubs_added"] > 0)),
    ("tauten_errors", "profile errors (mean per trial)", lambda r: float(r["tauten"]["errors"])),
    ("seconds", "agent wall time (s, mean)", lambda r: float(r["agent_result"].get("seconds") or 0.0)),
]


def bootstrap_diff(rows, arm, base, metric, resamples=10000, seed=20261118):
    """95% CI for mean(metric | arm) - mean(metric | base), resampling tasks."""
    per_task = defaultdict(lambda: defaultdict(list))
    for r in rows:
        per_task[r["task"]][r["arm"]].append(metric(r))
    tasks = [t for t, arms in per_task.items() if arm in arms and base in arms]
    if not tasks:
        return None
    diffs = {t: statistics.fmean(per_task[t][arm]) - statistics.fmean(per_task[t][base]) for t in tasks}
    point = statistics.fmean(diffs.values())
    rng = random.Random(seed)
    samples = sorted(
        statistics.fmean(diffs[rng.choice(tasks)] for _ in tasks) for _ in range(resamples)
    )
    low = samples[int(0.025 * resamples)]
    high = samples[int(0.975 * resamples) - 1]
    return point, low, high, len(tasks)


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("files", nargs="+")
    p.add_argument("--agent", default=None, help="only rows from this agent (null, reference, sloppy, cmd)")
    p.add_argument("--label", default=None, help="only rows with this label")
    p.add_argument("--split", default=None, help="dev | heldout")
    args = p.parse_args()

    rows = []
    for path in args.files:
        with open(path) as f:
            rows += [json.loads(line) for line in f if line.strip()]
    rows = [
        r
        for r in rows
        if (args.agent is None or r["agent"] == args.agent)
        and (args.label is None or r.get("label") == args.label)
        and (args.split is None or r["split"] == args.split)
    ]
    if not rows:
        raise SystemExit("no rows match")
    arms = sorted({r["arm"] for r in rows})
    by_arm = defaultdict(list)
    for r in rows:
        by_arm[r["arm"]].append(r)

    print(f"rows: {len(rows)}; tasks: {len({r['task'] for r in rows})}; labels: {sorted({r.get('label', '') for r in rows})}\n")
    print("| metric | " + " | ".join(f"arm {a} (n={len(by_arm[a])})" for a in arms) + " |")
    print("|---|" + "---|" * len(arms))
    for _, title, fn in METRICS:
        cells = [f"{statistics.fmean(fn(r) for r in by_arm[a]):.3f}" for a in arms]
        print(f"| {title} | " + " | ".join(cells) + " |")

    if "A" in arms:
        print("\nDifferences vs arm A (paired by task, 95% bootstrap CI over tasks):\n")
        print("| metric | arm | difference | 95% CI | tasks | verdict |")
        print("|---|---|---|---|---|---|")
        for _, title, fn in METRICS:
            for arm in (a for a in arms if a != "A"):
                ci = bootstrap_diff(rows, arm, "A", fn)
                if ci is None:
                    continue
                point, low, high, n = ci
                verdict = "no measured difference" if low <= 0.0 <= high else ("lower" if high < 0 else "higher")
                print(f"| {title} | {arm} | {point:+.3f} | [{low:+.3f}, {high:+.3f}] | {n} | {verdict} |")


if __name__ == "__main__":
    main()
