#!/bin/sh
# Regenerate proofs/Extracted/ from crates/tauten-check-kernel and
# crates/tauten-proof-instances inside the
# pinned proofs image and copy the result out as host-owned files. Run
# `cargo fmt` first (the translation records source line numbers), then
# commit the result; the gate refuses any drift.
set -eu
cd "$(dirname "$0")/.."
fresh="$(mktemp)"
trap 'rm -f "$fresh"' EXIT
sh proofs/run.sh \
    sh /repo/proofs/container.sh sh -c \
    'sh proofs/extract-all.sh /tmp/fresh >&2 && tar -C /tmp/fresh -cf - .' \
    > "$fresh"
rm -rf proofs/Extracted
mkdir -p proofs/Extracted
tar -C proofs/Extracted -xf "$fresh"
echo "refreshed: proofs/Extracted ($(ls proofs/Extracted | tr '\n' ' '))"
