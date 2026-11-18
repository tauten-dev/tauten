#!/bin/sh
# Translate every proved crate into one directory, as committed under
# proofs/Extracted/: crates/tauten-check-kernel (P1, P2) and
# crates/tauten-proof-instances (P3, the macros' expansion of its
# instances). Each crate is translated on its own (proofs/extract.sh), with
# `-abort-on-error`.
#
#   sh proofs/extract-all.sh <dest-dir>
set -eu
cd "$(dirname "$0")/.."
dest="$1"
parts="$(mktemp -d)"
trap 'rm -rf "$parts"' EXIT
rm -rf "$dest"
mkdir -p "$dest"
for crate in tauten-check-kernel tauten-proof-instances; do
    sh proofs/extract.sh "crates/$crate" "$parts/$crate" -- -abort-on-error
    cp "$parts/$crate"/*.lean "$dest/"
done
echo "extracted all: $(ls "$dest" | tr '\n' ' ')"
