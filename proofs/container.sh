#!/bin/sh
# Entry point of the `proofs` Compose service and the CI workflow. The
# checkout is mounted read-only at /repo; the image keeps the prebuilt Lean
# dependencies in /work/proofs/.lake. Copy in only what the gate reads (no
# target/, no host .lake), then run the gate, or the command given:
#
#   sh proofs/container.sh                       # the gate
#   sh proofs/container.sh python3 proofs/mutants.py warn-blocks
set -eu
repo="${PROOFS_REPO:-/repo}"
cd /work
rm -rf crates proofs/Extracted proofs/Tauten
mkdir -p crates
cp "$repo/Cargo.toml" "$repo/Cargo.lock" .
for crate in tauten-check-kernel tauten tauten-macros tauten-proof-instances; do
    cp -r "$repo/crates/$crate" crates/
    rm -rf "crates/$crate/target"
done
for entry in "$repo"/proofs/*; do
    cp -r "$entry" proofs/
done
if [ "$#" -eq 0 ]; then
    set -- sh proofs/gate.sh
fi
exec "$@"
