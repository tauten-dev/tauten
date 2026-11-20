#!/bin/sh
# Writes the generated tree.
set -eu
mkdir -p out
printf 'one\n' > out/a.txt
printf 'two\n' > out/b.txt
