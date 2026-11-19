#!/bin/sh
# The proofs gate (docs/proofs/README.md), adapted from writfold's
# proofs/gate.sh: regenerate the translation of crates/tauten-check-kernel
# and crates/tauten-proof-instances (the macros' expansion) and refuse drift
# against the committed proofs/Extracted/, build the
# theorems with the axiom and coverage audit, then calibrate with mutants
# that must fail. Run from the repository root inside the proofs image, with
# proofs/.lake holding the prebuilt dependencies (proofs/container.sh sets
# that up).
set -eu
cd "$(dirname "$0")/.."
started="$(date +%s)"
stamp() { echo "[$(( $(date +%s) - started ))s] $*"; }
# Result lines for `cargo tauten proofs` (docs/proofs/README.md, P5): one per
# stage; a failing step reports its stage as failed on exit.
report() { printf 'TAUTEN-PROOFS {"stage":"%s","status":"%s"}\n' "$1" "$2"; }
stage=tools
trap 'code=$?; if [ "$code" -ne 0 ] && [ -n "$stage" ]; then report "$stage" fail; fi' EXIT
for tool in charon aeneas lake python3; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        printf 'TAUTEN-PROOFS {"stage":"tools","status":"missing","detail":"%s is not on PATH"}\n' "$tool"
        stage=""
        exit 3
    fi
done
report tools ok

stage=extract
stamp "extract: charon $(charon version) / aeneas $(aeneas -version)"
sh proofs/extract-all.sh /tmp/gate-extract
report extract ok

stage=drift
stamp "drift check against proofs/Extracted"
if ! diff -ru proofs/Extracted /tmp/gate-extract; then
    echo "FAIL: proofs/Extracted differs from a fresh translation of the proved crates;" >&2
    echo "      run 'cargo fmt', then 'sh proofs/refresh.sh', and commit the result" >&2
    exit 1
fi
report drift ok

stage=build
stamp "lake build (theorems, axiom and coverage audit)"
status=0
(cd proofs && lake build) > /tmp/gate-lake.log 2>&1 || status=$?
cat /tmp/gate-lake.log
stage=""
python3 proofs/report.py lake /tmp/gate-lake.log "$status"

stage=mutants
stamp "mutant calibration"
python3 proofs/mutants.py
report mutants ok

stamp "proofs gate: OK"
