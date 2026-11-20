#!/usr/bin/env python3
"""Check task admission criteria on validation results from run.py.

    python3 -I bench/admit.py results.jsonl [more.jsonl ...]

Every task:
  reference  oracle 100% in the dev and the release profile, 0 profile
             errors and visible tests pass, in every arm it ran in
  null       an escaped defect
  sloppy     an escaped defect, and by the task's stratum (task.toml):
               T targeted  at least one profile error from `expected_rules`
               A adjacent  the same
               N control   zero profile errors (so a control is a real one)
  evasive    required for stratum A, checked wherever present: zero profile
             errors and still an escaped defect (the oracle is not the lint)
A row with an infra error is a problem (re-run the task).
Exits 1 listing every violation; 0 if every task in the file is admitted.
"""

import json
import sys
import tomllib
from collections import defaultdict
from pathlib import Path

BENCH = Path(__file__).resolve().parent


def rule_errors(row, rules):
    """Profile errors in the row raised by any of `rules`."""
    return sum(n for key, n in row["tauten"]["by_rule"].items() if key.endswith(":error") and key.split(":")[0] in rules)


def main():
    rows = [json.loads(l) for path in sys.argv[1:] for l in open(path) if l.strip()]
    seen = defaultdict(set)
    problems = []
    tasks = {}
    for r in rows:
        if r["task"] not in tasks:
            tasks[r["task"]] = tomllib.loads((BENCH / "tasks" / r["task"] / "task.toml").read_text())
        task = tasks[r["task"]]
        stratum = task.get("stratum")
        expected = task.get("expected_rules", [])
        where = f"{r['task']} arm {r['arm']} ({r['agent']})"
        if r.get("infra_error"):
            problems.append(f"{where}: infra error: {r['infra_error']}")
            continue
        seen[r["task"]].add(r["agent"])
        if r["agent"] == "reference":
            if r["escaped_defect"] or r["oracle_failed"] or not r["oracle_ran"]:
                failing = sorted(n for n, s in r.get("oracle_tests", {}).items() if s == "FAILED")
                problems.append(
                    f"{where}: reference fails its oracle (dev={r.get('escaped_defect_dev')}, "
                    f"release={r.get('escaped_defect_release')}): {failing}"
                )
            if r["tauten"]["errors"]:
                problems.append(f"{where}: reference has {r['tauten']['errors']} profile error(s): {r['tauten']['by_rule']}")
            if not r["visible_ok"]:
                problems.append(f"{where}: reference fails the visible tests")
        elif r["agent"] == "null":
            if not r["escaped_defect"]:
                problems.append(f"{where}: the oracle passes without any change")
        elif r["agent"] == "sloppy":
            if not r["escaped_defect"]:
                problems.append(f"{where}: the oracle misses the sloppy solution's defect")
            if stratum in ("T", "A"):
                if not rule_errors(r, expected):
                    problems.append(
                        f"{where}: stratum {stratum} but no profile error from {expected} "
                        f"(found {r['tauten']['by_rule']})"
                    )
            elif stratum == "N":
                if r["tauten"]["errors"]:
                    problems.append(f"{where}: control task but the profile flags the sloppy solution: {r['tauten']['by_rule']}")
            else:
                problems.append(f"{where}: task.toml has no stratum (T, A or N)")
        elif r["agent"] == "evasive":
            if r["tauten"]["errors"]:
                problems.append(f"{where}: evasive solution is not lint-clean: {r['tauten']['by_rule']}")
            if not r["escaped_defect"]:
                problems.append(f"{where}: the oracle misses the evasive solution's defect")
    for task_id, agents in sorted(seen.items()):
        required = {"reference", "null", "sloppy"}
        if tasks[task_id].get("stratum") == "A" or (BENCH / "tasks" / task_id / "evasive").is_dir():
            required.add("evasive")
        missing = required - agents
        if missing:
            problems.append(f"{task_id}: not validated with {', '.join(sorted(missing))}")
    for p in problems:
        print(f"not admitted: {p}")
    print(f"{len(seen)} task(s) checked, {len(problems)} problem(s)")
    sys.exit(1 if problems else 0)


if __name__ == "__main__":
    main()
