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

stamp "extract: charon $(charon version) / aeneas $(aeneas -version)"
sh proofs/extract-all.sh /tmp/gate-extract

stamp "drift check against proofs/Extracted"
if ! diff -ru proofs/Extracted /tmp/gate-extract; then
    echo "FAIL: proofs/Extracted differs from a fresh translation of the proved crates;" >&2
    echo "      run 'cargo fmt', then 'sh proofs/refresh.sh', and commit the result" >&2
    exit 1
fi

stamp "lake build (theorems, axiom and coverage audit)"
cd proofs
lake build
cd ..

stamp "mutant calibration"
python3 proofs/mutants.py

stamp "proofs gate: OK"
