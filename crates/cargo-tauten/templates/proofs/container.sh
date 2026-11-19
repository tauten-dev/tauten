#!/bin/sh
# Inside the proofs image: copy the read-only checkout (/repo) to /work,
# keeping the image's prebuilt Lean dependencies (/work/proofs/.lake), then
# run the gate, or the command given.
set -eu
repo="${PROOFS_REPO:-/repo}"
cd /work
find . -mindepth 1 -maxdepth 1 ! -name proofs -exec rm -rf {} +
find proofs -mindepth 1 -maxdepth 1 ! -name .lake -exec rm -rf {} +
tar -C "$repo" --exclude=./target --exclude='*/target' --exclude=./.git \
    --exclude=./proofs/.lake -cf - . | tar -xf -
if [ "$#" -eq 0 ]; then
    set -- python3 proofs/gate.py
fi
exec "$@"
