#!/bin/sh
# The proof gate, from the host: checks that the toolchain is there, then
# runs proofs/gate.py in the pinned image (proofs/compose.yaml). This is
# the `[gate] command` of proofs/claims.toml, which `cargo tauten proofs`
# runs. A missing tool is reported as a `tools: missing` result line (and
# exit 3), never as a pass.
#
#   sh proofs/run.sh                         # the gate
#   sh proofs/run.sh python3 proofs/gate.py --refresh   # print a fresh extraction (refresh.sh)
set -u
cd "$(dirname "$0")/.."
image="${PROOFS_IMAGE:-ghcr.io/tauten-dev/proofs-toolchain:lean-4.35.0-aeneas-c5eb4a2-charon-8cae8fd}"
missing() {
    printf 'TAUTEN-PROOFS {"stage":"tools","status":"missing","detail":"%s"}\n' "$1"
    echo "proofs: $1" >&2
    exit 3
}
command -v docker >/dev/null 2>&1 || missing "docker is not installed"
docker info >/dev/null 2>&1 || missing "the docker daemon is not reachable"
docker compose version >/dev/null 2>&1 || missing "docker compose is not available"
docker image inspect "$image" >/dev/null 2>&1 \
    || missing "the proofs image $image is not present; pull it first (about 20 GB)"
PROOFS_IMAGE="$image" exec docker compose -f proofs/compose.yaml run --rm -T proofs \
    sh /repo/proofs/container.sh "$@"
