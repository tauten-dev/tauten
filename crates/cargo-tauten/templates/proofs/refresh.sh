#!/bin/sh
# Regenerate proofs/Extracted/ inside the pinned image and copy it out as
# host-owned files. Run `cargo fmt` first (the translation records source
# line numbers), then commit the result: the gate refuses any drift.
set -eu
cd "$(dirname "$0")/.."
fresh="$(mktemp)"
trap 'rm -f "$fresh"' EXIT
sh proofs/run.sh python3 proofs/gate.py --extract-tar > "$fresh"
rm -rf proofs/Extracted
mkdir -p proofs/Extracted
tar -C proofs/Extracted -xf "$fresh"
echo "refreshed: proofs/Extracted ($(ls proofs/Extracted | tr '\n' ' '))"
