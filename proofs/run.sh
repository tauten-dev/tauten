#!/bin/sh
# The proofs gate from the host (the `[gate] command` of proofs/claims.toml,
# which `cargo tauten proofs` runs): checks for Docker and the pinned image,
# then runs the Compose service (compose.proofs.yaml). A missing tool is a
# `tools: missing` result line and exit 3, never a pass.
set -u
cd "$(dirname "$0")/.."
image="ghcr.io/tauten-dev/proofs-toolchain:lean-4.35.0-aeneas-c5eb4a2-charon-8cae8fd"
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
exec docker compose -f compose.proofs.yaml run --rm -T proofs "$@"
