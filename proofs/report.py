#!/usr/bin/env python3
"""Result lines of the proofs gate for `cargo tauten proofs` (P5).

    python3 proofs/report.py lake <lake-log> <lake-exit-status>

Reads the output of `lake build` (theorems plus Audit.lean) and prints
`TAUTEN-PROOFS <json>` lines: the build stage, one line per theorem the
axiom audit listed (ok iff its axioms are within Lean's standard three),
and the audit stage. Errors only in Audit.lean fail the audit, any other
error fails the build. Exit status 0 iff both passed.
"""
import json
import re
import sys

ALLOWED = {"propext", "Classical.choice", "Quot.sound"}
ERROR = re.compile(r"^error:\s*(?P<file>[^\s:]+\.lean):\d+:\d+:", re.M)
AXIOMS = re.compile(r"AXIOMS (?P<name>\S+): \[(?P<axioms>[^\]]*)\]", re.S)


def emit(**fields) -> None:
    print("TAUTEN-PROOFS " + json.dumps(fields, sort_keys=True), flush=True)


def lake(log: str, status: int) -> int:
    files = {m.group("file") for m in ERROR.finditer(log)}
    if status != 0 and (not files or files != {"Audit.lean"}):
        emit(stage="build", status="fail")
        return 1
    emit(stage="build", status="ok")
    for match in AXIOMS.finditer(log):
        axioms = {a.strip() for a in match.group("axioms").split(",") if a.strip()}
        emit(theorem=match.group("name"), status="ok" if axioms <= ALLOWED else "fail")
    emit(stage="audit", status="ok" if status == 0 else "fail")
    return 0 if status == 0 else 1


if __name__ == "__main__":
    if len(sys.argv) != 4 or sys.argv[1] != "lake":
        raise SystemExit(__doc__)
    with open(sys.argv[2], encoding="utf-8") as handle:
        sys.exit(lake(handle.read(), int(sys.argv[3])))
