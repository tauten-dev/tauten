#!/bin/sh
# Translate a crate to Lean: Charon (Rust -> LLBC) then Aeneas (LLBC -> Lean).
# Adapted from writfold's proofs/extract.sh (same pins, same --preset=aeneas).
#
#   sh proofs/extract.sh <crate-dir> <dest-dir> [charon args...] [-- aeneas args...]
#
# The crate is copied into a throwaway shadow workspace at its own
# repository-relative path, together with every crate it reaches through
# `path` dependencies (for the probe: crates/tauten and crates/tauten-macros),
# so relative paths in Aeneas's output are stable. The shadow root carries
# the real root's resolver and the `[workspace.package]` keys those crates
# inherit, nothing else; Cargo.lock is copied so registry versions match the
# real workspace.
#
# EXTRACT_NO_METADATA=1 ignores the crate's [package.metadata.charon] (Charon
# merges it with command-line --start-from/--include; proofs/probe.sh needs
# each item alone).
#
# Without `-abort-on-error` among the Aeneas args, Aeneas reports errors and
# still writes what it could; the probe wants that, the gate will not.
set -eu
crate="$(cd "$1" && pwd)"
dest="$2"
shift 2
charon_args=""
aeneas_args=""
side=charon
for arg in "$@"; do
    if [ "$arg" = "--" ]; then side=aeneas; continue; fi
    if [ "$side" = charon ]; then charon_args="$charon_args $arg"; else aeneas_args="$aeneas_args $arg"; fi
done
root="$(cd "$(dirname "$0")/.." && pwd)"
work="${EXTRACT_WORK:-$(mktemp -d)}"
mkdir -p "$work"
[ -n "${EXTRACT_WORK:-}" ] || trap 'rm -rf "$work"' EXIT

rel="$(python3 - "$root" "$crate" "$work" <<'PY'
import os, shutil, sys, tomllib
root, crate, work = sys.argv[1], sys.argv[2], sys.argv[3]
with open(os.path.join(root, "Cargo.toml"), "rb") as handle:
    ws = tomllib.load(handle)["workspace"]
package = ws.get("package", {})
seen, todo, inherited = [], [crate], set()
while todo:
    here = os.path.realpath(todo.pop())
    if here in seen:
        continue
    seen.append(here)
    with open(os.path.join(here, "Cargo.toml"), "rb") as handle:
        manifest = tomllib.load(handle)
    inherited |= {k for k, v in manifest["package"].items()
                  if isinstance(v, dict) and v.get("workspace")}
    for table in ("dependencies", "build-dependencies"):
        for dep in manifest.get(table, {}).values():
            if isinstance(dep, dict) and "path" in dep:
                todo.append(os.path.join(here, dep["path"]))
for here in seen:
    rel = os.path.relpath(here, root)
    assert not rel.startswith(".."), rel
    shutil.copytree(here, os.path.join(work, rel),
                    ignore=shutil.ignore_patterns("target"))
main = os.path.relpath(seen[0], root)
if os.environ.get("EXTRACT_NO_METADATA"):
    # The probe driver's isolated runs: Charon merges the manifest's
    # start_from/include with the command line, so drop the manifest's.
    path = os.path.join(work, main, "Cargo.toml")
    kept, skip = [], False
    for line in open(path).read().splitlines():
        if line.strip().startswith("["):
            skip = line.strip() == "[package.metadata.charon]"
        if not skip:
            kept.append(line)
    open(path, "w").write("\n".join(kept) + "\n")
lines = ["[workspace]", f'members = ["{main}"]', f'resolver = "{ws.get("resolver", "2")}"',
         "", "[workspace.package]"]
lines += [f'{k} = "{package[k]}"' for k in sorted(inherited)]
open(os.path.join(work, "Cargo.toml"), "w").write("\n".join(lines) + "\n")
if os.path.exists(os.path.join(root, "Cargo.lock")):
    shutil.copy(os.path.join(root, "Cargo.lock"), os.path.join(work, "Cargo.lock"))
print(main)
PY
)"
name="$(python3 -c 'import sys, tomllib; print(tomllib.load(open(sys.argv[1], "rb"))["package"]["name"].replace("-", "_"))' "$crate/Cargo.toml")"

cd "$work/$rel"
# --preset=aeneas is the option set Aeneas's own test-suite drives Charon
# with; [package.metadata.charon] or the extra args choose start_from.
# shellcheck disable=SC2086
charon cargo --preset=aeneas $charon_args --dest-file "$work/$name.llbc" -- --lib

rm -rf "$dest"
mkdir -p "$dest"
# shellcheck disable=SC2086
aeneas -backend lean -no-progress-bar $aeneas_args -dest "$dest" "$work/$name.llbc"
echo "extracted: $(ls "$dest" | tr '\n' ' ')"
