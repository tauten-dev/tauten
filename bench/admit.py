#!/usr/bin/env python3
"""Check task admission criteria on validation results from run.py.

    python3 -I bench/admit.py results.jsonl

reference: oracle 100% and 0 profile errors in every arm it ran in
null:      an escaped defect
sloppy:    an escaped defect and at least one profile error
Exits 1 listing every violation; 0 if every task in the file is admitted.
"""

import json
import sys
from collections import defaultdict


def main():
    rows = [json.loads(l) for path in sys.argv[1:] for l in open(path) if l.strip()]
    seen = defaultdict(set)
    problems = []
    for r in rows:
        where = f"{r['task']} arm {r['arm']} ({r['agent']})"
        seen[r["task"]].add(r["agent"])
        if r["agent"] == "reference":
            if r["escaped_defect"] or r["oracle_failed"] or not r["oracle_ran"]:
                problems.append(f"{where}: reference fails its oracle")
            if r["tauten"]["errors"]:
                problems.append(f"{where}: reference has {r['tauten']['errors']} profile error(s)")
            if not r["visible_ok"]:
                problems.append(f"{where}: reference fails the visible tests")
        elif r["agent"] == "null":
            if not r["escaped_defect"]:
                problems.append(f"{where}: the oracle passes without any change")
        elif r["agent"] == "sloppy":
            if not r["escaped_defect"]:
                problems.append(f"{where}: the oracle misses the sloppy solution's defect")
            if not r["tauten"]["errors"]:
                problems.append(f"{where}: the profile misses the sloppy solution's mistake")
    for task, agents in sorted(seen.items()):
        missing = {"reference", "null", "sloppy"} - agents
        if missing:
            problems.append(f"{task}: not validated with {', '.join(sorted(missing))}")
    for p in problems:
        print(f"not admitted: {p}")
    print(f"{len(seen)} task(s) checked, {len(problems)} problem(s)")
    sys.exit(1 if problems else 0)


if __name__ == "__main__":
    main()
