#!/bin/sh
# Regenerate proofs/Extracted/ from crates/tauten-check-kernel inside the
# pinned proofs image and copy the result out as host-owned files. Run
# `cargo fmt` first (the translation records source line numbers), then
# commit the result; the gate refuses any drift.
set -eu
cd "$(dirname "$0")/.."
fresh="$(mktemp)"
trap 'rm -f "$fresh"' EXIT
docker compose -f compose.proofs.yaml run --rm -T proofs \
    sh /repo/proofs/container.sh sh -c \
    'sh proofs/extract.sh crates/tauten-check-kernel /tmp/fresh -- -abort-on-error >&2 && tar -C /tmp/fresh -cf - .' \
    > "$fresh"
rm -rf proofs/Extracted
mkdir -p proofs/Extracted
tar -C proofs/Extracted -xf "$fresh"
echo "refreshed: proofs/Extracted ($(ls proofs/Extracted | tr '\n' ' '))"
