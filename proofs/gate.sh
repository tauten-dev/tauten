#!/bin/sh
# The P0 probe gate, adapted from writfold's proofs/gate.sh: regenerate the
# translation of proofs/probe and refuse drift against the committed
# proofs/Extracted/, build the theorems, audit axioms and constructors.
# No mutant calibration yet (P1). Run from the repository root inside the
# proofs image, with proofs/.lake holding the prebuilt dependencies.
set -eu
cd "$(dirname "$0")/.."
started="$(date +%s)"
stamp() { echo "[$(( $(date +%s) - started ))s] $*"; }

stamp "extract: charon $(charon version) / aeneas $(aeneas -version)"
rm -rf /tmp/gate-extract
sh proofs/extract.sh proofs/probe /tmp/gate-extract -- -abort-on-error

stamp "drift check against proofs/Extracted"
if ! diff -ru proofs/Extracted /tmp/gate-extract; then
    echo "FAIL: proofs/Extracted differs from a fresh translation of proofs/probe" >&2
    exit 1
fi

stamp "lake build (theorems, axiom and constructor audit)"
cd proofs
lake build
cd ..
stamp "probe gate: OK"
