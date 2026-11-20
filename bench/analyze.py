#!/usr/bin/env python3
"""Summarise benchmark results (JSON lines from run.py) per arm, with paired
bootstrap confidence intervals clustered by task.

    python3 -I bench/analyze.py results/*.jsonl [--agent claude] [--label X] [--split heldout]

Primary metric (preregistered): escaped-defect rate, the share of trials
whose submission fails the hidden oracle; primary comparison C - A over all
tasks in the selection. Everything else is secondary: task success (all
`base__` tests pass), other arm differences (B - A, C - B), per-stratum
differences (exploratory), failures by oracle class tag, arm C compliance,
timeouts, cost and tokens. A sign-flip permutation test on per-task
differences is reported beside the bootstrap as a sensitivity check.

Each model is analysed separately (never pooled). Rows with an `infra_error`
are not scored; they are counted and listed. The escaped-defect rate is
also given per build profile (dev, release), with the relative reduction
(C − A) / A, per-arm medians of cost and wall time, and an exploratory
escaped-defect rate restricted to trials whose every agent session ended
with subtype `success`. A balance and duplicate check runs first.
"""

import argparse
import itertools
import json
import random
import statistics
from collections import defaultdict

SEED = 20261118
RESAMPLES = 10000


def agent(r):
    return r.get("agent_result") or {}


def profile_defect(profile):
    """Escaped defect in one build profile; rows from before the release run count as dev only."""
    def fn(r):
        value = r.get(f"escaped_defect_{profile}")
        if value is None and profile == "dev":
            value = r["escaped_defect"]
        return None if value is None else float(value)
    return fn


def all_sessions_succeeded(r):
    subtypes = agent(r).get("subtypes") or [s.get("subtype") for s in agent(r).get("steps", [])]
    return bool(subtypes) and all(s == "success" for s in subtypes)


METRICS = [
    ("escaped-defect rate", lambda r: float(r["escaped_defect"])),
    ("escaped-defect rate, dev profile", profile_defect("dev")),
    ("escaped-defect rate, release profile", profile_defect("release")),
    ("task success (all base__ pass)", lambda r: float(bool(r.get("task_success", not r["escaped_defect"])))),
    ("trials with a reachable panic", lambda r: float(r["oracle_panics"] > 0)),
    ("trials with a hang", lambda r: float(r.get("oracle_hangs", 0) > 0)),
    ("visible tests pass", lambda r: float(r["visible_ok"])),
    ("test tampering (removed tests or added #[ignore])", lambda r: float(r["tests_removed"] > 0 or r["ignores_added"] > 0)),
    ("stubs added", lambda r: float(r["stubs_added"] > 0)),
    ("trials ending with profile errors", lambda r: float(r["tauten"]["errors"] > 0)),
    ("profile errors (mean per trial)", lambda r: float(r["tauten"]["errors"])),
    ("agent timeouts", lambda r: float(bool(agent(r).get("timed_out")))),
    ("agent wall time (s, mean)", lambda r: float(agent(r).get("seconds") or 0.0)),
    ("model cost (USD, mean)", lambda r: float(agent(r).get("total_cost_usd") or 0.0)),
    ("turns (mean)", lambda r: float(agent(r).get("num_turns") or 0.0)),
    ("output tokens (mean)", lambda r: float(agent(r).get("output_tokens") or 0.0)),
]
PRIMARY = METRICS[0][1]


def mean_of(values):
    values = [v for v in values if v is not None]
    return statistics.fmean(values) if values else None


def per_task_means(rows, metric):
    per_task = defaultdict(lambda: defaultdict(list))
    for r in rows:
        value = metric(r)
        if value is not None:
            per_task[r["task"]][r["arm"]].append(value)
    return per_task


def per_task_diffs(rows, arm, base, metric):
    per_task = per_task_means(rows, metric)
    tasks = sorted(t for t, arms in per_task.items() if arm in arms and base in arms)
    return {t: statistics.fmean(per_task[t][arm]) - statistics.fmean(per_task[t][base]) for t in tasks}


def bootstrap(diffs):
    """Point estimate and 95% CI of the mean per-task difference, resampling tasks."""
    tasks = sorted(diffs)
    point = statistics.fmean(diffs.values())
    rng = random.Random(SEED)
    samples = sorted(statistics.fmean(diffs[rng.choice(tasks)] for _ in tasks) for _ in range(RESAMPLES))
    return point, samples[int(0.025 * RESAMPLES)], samples[int(0.975 * RESAMPLES) - 1]


def sign_flip_p(diffs):
    """Two-sided sign-flip permutation p-value for mean per-task difference = 0."""
    values = list(diffs.values())
    observed = abs(statistics.fmean(values))
    if len(values) <= 16:
        flips = itertools.product((1, -1), repeat=len(values))
        stats = [abs(statistics.fmean(s * v for s, v in zip(f, values))) for f in flips]
    else:
        rng = random.Random(SEED)
        stats = [abs(statistics.fmean(v * rng.choice((1, -1)) for v in values)) for _ in range(RESAMPLES)]
    return sum(s >= observed - 1e-12 for s in stats) / len(stats)


def relative_change(rows, arm, base, metric):
    """(mean arm − mean base) / mean base over tasks having both arms, with a 95% bootstrap CI over tasks."""
    per_task = per_task_means(rows, metric)
    tasks = sorted(t for t, arms in per_task.items() if arm in arms and base in arms)
    if not tasks:
        return None
    means = {t: (statistics.fmean(per_task[t][arm]), statistics.fmean(per_task[t][base])) for t in tasks}

    def ratio(sample):
        b = statistics.fmean(means[t][1] for t in sample)
        return None if b == 0 else statistics.fmean(means[t][0] for t in sample) / b - 1.0

    point = ratio(tasks)
    if point is None:
        return None
    rng = random.Random(SEED)
    samples = sorted(x for x in (ratio([rng.choice(tasks) for _ in tasks]) for _ in range(RESAMPLES)) if x is not None)
    return point, samples[int(0.025 * len(samples))], samples[int(0.975 * len(samples)) - 1], len(tasks)


def check_balance(rows):
    """Duplicate trials and unequal trial counts per (task, arm)."""
    problems = []
    keys = defaultdict(int)
    for r in rows:
        keys[(r["task"], r["arm"], r["trial"], r.get("label", ""))] += 1
    problems += [f"duplicate trial {k} ({n} rows)" for k, n in sorted(keys.items()) if n > 1]
    cells = defaultdict(int)
    for r in rows:
        cells[(r["task"], r["arm"])] += 1
    tasks = sorted({r["task"] for r in rows})
    arms = sorted({r["arm"] for r in rows})
    counts = {cells.get((t, a), 0) for t in tasks for a in arms}
    if len(counts) > 1:
        uneven = ", ".join(f"{t}/{a}={cells.get((t, a), 0)}" for t in tasks for a in arms)
        problems.append(f"unbalanced trials per (task, arm): {uneven}")
    return problems


def verdict(low, high):
    return "no measured difference" if low <= 0.0 <= high else ("lower" if high < 0 else "higher")


def table(rows, arms, by_arm):
    print("| metric | " + " | ".join(f"arm {a} (n={len(by_arm[a])})" for a in arms) + " |")
    print("|---|" + "---|" * len(arms))
    for title, fn in METRICS:
        cells = []
        for a in arms:
            present = [v for v in (fn(r) for r in by_arm[a]) if v is not None]
            value = mean_of(present)
            note = "" if len(present) == len(by_arm[a]) else f" (n={len(present)})"
            cells.append("-" if value is None else f"{value:.3f}{note}")
        print(f"| {title} | " + " | ".join(cells) + " |")
    for title, fn in (("model cost (USD, median)", lambda r: agent(r).get("total_cost_usd")),
                      ("agent wall time (s, median)", lambda r: agent(r).get("seconds"))):
        cells = []
        for a in arms:
            values = [float(v) for v in (fn(r) for r in by_arm[a]) if v is not None]
            cells.append(f"{statistics.median(values):.3f}" if values else "-")
        print(f"| {title} | " + " | ".join(cells) + " |")


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("files", nargs="+")
    p.add_argument("--agent", default=None, help="only rows from this agent (null, reference, sloppy, evasive, claude, cmd)")
    p.add_argument("--label", default=None, help="only rows with this label")
    p.add_argument("--split", default=None, help="dev | calibration | heldout")
    args = p.parse_args()

    rows = []
    for path in args.files:
        with open(path) as f:
            rows += [json.loads(line) for line in f if line.strip()]
    rows = [
        r for r in rows
        if (args.agent is None or r["agent"] == args.agent)
        and (args.label is None or r.get("label") == args.label)
        and (args.split is None or r["split"] == args.split)
    ]
    if not rows:
        raise SystemExit("no rows match")
    groups = defaultdict(list)
    for r in rows:
        groups[r.get("model") or f"agent {r['agent']}"].append(r)
    for model in sorted(groups):
        print(f"\n# Model: {model}\n")
        report(groups[model])


def report(rows):
    infra = [r for r in rows if r.get("infra_error")]
    rows = [r for r in rows if not r.get("infra_error")]
    if infra:
        print(f"Infra errors (not scored, re-run with --resume): {len(infra)}")
        for r in infra:
            print(f"- {r['task']} arm {r['arm']} #{r['trial']}: {r['infra_error']}")
        print()
    if not rows:
        print("no scored rows")
        return
    for problem in check_balance(rows):
        print(f"WARNING: {problem}")
    arms = sorted({r["arm"] for r in rows})
    by_arm = defaultdict(list)
    for r in rows:
        by_arm[r["arm"]].append(r)

    print(f"rows: {len(rows)}; tasks: {len({r['task'] for r in rows})}; splits: {sorted({r['split'] for r in rows})}; "
          f"labels: {sorted({r.get('label', '') for r in rows})}\n")
    table(rows, arms, by_arm)

    pairs = [(a, "A") for a in arms if a != "A" and "A" in arms] + ([("C", "B")] if {"B", "C"} <= set(arms) else [])
    if pairs:
        print("\nDifferences (paired by task, 95% bootstrap CI over tasks; sign-flip p for the primary metric):\n")
        print("| metric | comparison | difference | 95% CI | tasks | verdict |")
        print("|---|---|---|---|---|---|")
        for title, fn in METRICS:
            for arm, base in pairs:
                diffs = per_task_diffs(rows, arm, base, fn)
                if not diffs:
                    continue
                point, low, high = bootstrap(diffs)
                print(f"| {title} | {arm} − {base} | {point:+.3f} | [{low:+.3f}, {high:+.3f}] | {len(diffs)} | {verdict(low, high)} |")
        for arm, base in pairs:
            diffs = per_task_diffs(rows, arm, base, PRIMARY)
            if diffs:
                print(f"\nSign-flip permutation p (escaped defects, {arm} − {base}, {len(diffs)} tasks): {sign_flip_p(diffs):.4f}")
        print("\nRelative change (mean arm / mean base − 1, 95% bootstrap CI over tasks):\n")
        print("| metric | comparison | relative change | 95% CI | tasks |")
        print("|---|---|---|---|---|")
        for title, fn in (("escaped-defect rate", PRIMARY),
                          ("escaped-defect rate, dev profile", profile_defect("dev")),
                          ("escaped-defect rate, release profile", profile_defect("release")),
                          ("model cost (USD, mean)", lambda r: float(agent(r).get("total_cost_usd") or 0.0))):
            for arm, base in pairs:
                rel = relative_change(rows, arm, base, fn)
                if rel:
                    point, low, high, n = rel
                    print(f"| {title} | {arm} vs {base} | {point:+.1%} | [{low:+.1%}, {high:+.1%}] | {n} |")
        for arm, base in pairs:
            costs = {a: [float(agent(r).get("total_cost_usd") or 0.0) for r in by_arm[a]] for a in (arm, base)}
            if costs[base] and statistics.median(costs[base]) > 0:
                change = statistics.median(costs[arm]) / statistics.median(costs[base]) - 1.0
                print(f"\nMedian model cost, {arm} vs {base}: {change:+.1%}")

        strata = sorted({r.get("stratum") or "-" for r in rows})
        print("\nEscaped-defect rate by stratum (exploratory):\n")
        print("| stratum | tasks | " + " | ".join(f"arm {a}" for a in arms) + " | C − A [95% CI] |")
        print("|---|---|" + "---|" * len(arms) + "---|")
        for s in strata:
            sub = [r for r in rows if (r.get("stratum") or "-") == s]
            cells = []
            for a in arms:
                vals = [PRIMARY(r) for r in sub if r["arm"] == a]
                cells.append(f"{statistics.fmean(vals):.3f}" if vals else "-")
            diffs = per_task_diffs(sub, "C", "A", PRIMARY) if {"A", "C"} <= set(arms) else {}
            ci = "-"
            if diffs:
                point, low, high = bootstrap(diffs)
                ci = f"{point:+.3f} [{low:+.3f}, {high:+.3f}]"
            print(f"| {s} | {len({r['task'] for r in sub})} | " + " | ".join(cells) + f" | {ci} |")

    done = [r for r in rows if all_sessions_succeeded(r)]
    if done and any(agent(r).get("steps") for r in rows):
        print("\nExploratory: escaped-defect rate over trials whose every session ended with subtype `success`:\n")
        print("| | " + " | ".join(f"arm {a}" for a in arms) + " |")
        print("|---|" + "---|" * len(arms))
        cells = []
        for a in arms:
            sub = [PRIMARY(r) for r in done if r["arm"] == a]
            cells.append(f"{statistics.fmean(sub):.3f} (n={len(sub)})" if sub else "-")
        print("| escaped-defect rate | " + " | ".join(cells) + " |")
        if {"A", "C"} <= set(arms):
            diffs = per_task_diffs(done, "C", "A", PRIMARY)
            if diffs:
                point, low, high = bootstrap(diffs)
                print(f"\nC − A over those trials: {point:+.3f} [{low:+.3f}, {high:+.3f}] ({len(diffs)} tasks)")

    classes = sorted({n.split("__")[0] for r in rows for n in r.get("oracle_tests", {}) if "__" in n})
    if classes:
        print("\nTrials failing at least one test of each oracle class (exploratory; a class with fewer than "
              "3 tasks able to exhibit it, or fewer than 30 arm-A trials, is not measured):\n")
        print("| class | tasks | " + " | ".join(f"arm {a}" for a in arms) + " |")
        print("|---|---|" + "---|" * len(arms))
        for c in classes:
            having_any = [r for r in rows if any(n.startswith(c + "__") for n in r.get("oracle_tests", {}))]
            n_tasks = len({r["task"] for r in having_any})
            n_a = sum(r["arm"] == "A" for r in having_any)
            measured = n_tasks >= 3 and n_a >= 30
            cells = []
            for a in arms:
                having = [r for r in having_any if r["arm"] == a]
                failing = [r for r in having if c in r.get("failed_classes", [])]
                cell = f"{len(failing)}/{len(having)}" if having else "-"
                cells.append(cell if measured else f"not measured ({cell})")
            print(f"| {c} | {n_tasks} | " + " | ".join(cells) + " |")

    cost = sum(float(agent(r).get("total_cost_usd") or 0.0) for r in rows + infra)
    print(f"\nTotal model cost (scored and infra rows): ${cost:.2f}")


if __name__ == "__main__":
    main()
