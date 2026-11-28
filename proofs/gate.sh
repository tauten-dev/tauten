#!/bin/sh
# The proofs gate (docs/proofs/README.md), adapted from writfold's
# proofs/gate.sh: regenerate the translation of crates/tauten-check-kernel
# and crates/tauten-proof-instances (the macros' expansion) and refuse drift
# against the committed proofs/Extracted/, build the
# theorems with the axiom and coverage audit, then calibrate with mutants
# that must fail. Run from the repository root inside the proofs image, with
# proofs/.lake holding the prebuilt dependencies (proofs/container.sh sets
# that up).
set -eu
cd "$(dirname "$0")/.."
started="$(date +%s)"
stamp() { echo "[$(( $(date +%s) - started ))s] $*"; }
# Result lines for `cargo tauten proofs` (docs/proofs/README.md, P5): one per
# stage; a failing step reports its stage as failed on exit.
report() { printf 'TAUTEN-PROOFS {"stage":"%s","status":"%s"}\n' "$1" "$2"; }
stage=tools
trap 'code=$?; if [ "$code" -ne 0 ] && [ -n "$stage" ]; then report "$stage" fail; fi' EXIT
for tool in charon aeneas lake python3; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        printf 'TAUTEN-PROOFS {"stage":"tools","status":"missing","detail":"%s is not on PATH"}\n' "$tool"
        stage=""
        exit 3
    fi
done
report tools ok

# The CPU budget (#82). `nproc` is this process's affinity; under a CPU
# quota (`docker run --cpus`, compose `cpus`) it still counts every host
# CPU, so the cgroup's quota cuts it down. Mutant jobs x Lean threads stay
# within the budget: TAUTEN_MUTANT_JOBS and LEAN_NUM_THREADS are honoured
# up to it, and a shortage lowers the parallelism, never the mutant set.
# One build gains little beyond four threads (docs/proofs/README.md), so
# the rest of the budget runs mutants side by side.
cpus="$(nproc)"
if [ -r /sys/fs/cgroup/cpu.max ]; then
    read -r quota period < /sys/fs/cgroup/cpu.max
    if [ "$quota" != max ]; then
        limit=$(( (quota + period - 1) / period ))
        [ "$limit" -ge "$cpus" ] || cpus="$limit"
    fi
fi
[ "$cpus" -ge 1 ] || cpus=1
whole() { case "$1" in '' | *[!0-9]* | 0*) return 1 ;; esac; }
jobs="${TAUTEN_MUTANT_JOBS:-}"
threads="${LEAN_NUM_THREADS:-}"
whole "$jobs" || jobs=""
whole "$threads" || threads=""
# A build that runs alone (the theorems, the extract-only check) takes the
# whole budget, or LEAN_NUM_THREADS when that is lower.
alone="${threads:-$cpus}"
[ "$alone" -le "$cpus" ] || alone="$cpus"
if [ -z "$threads" ] && [ -z "$jobs" ]; then
    threads=$(( cpus < 4 ? cpus : 4 ))
elif [ -z "$threads" ]; then
    [ "$jobs" -le "$cpus" ] || jobs="$cpus"
    threads=$(( cpus / jobs ))
fi
[ "$threads" -le "$cpus" ] || threads="$cpus"
most=$(( cpus / threads ))
if [ -z "$jobs" ] || [ "$jobs" -gt "$most" ]; then jobs="$most"; fi
stamp "budget: $cpus CPU(s); mutants: $jobs job(s) x $threads Lean thread(s)"
export LEAN_NUM_THREADS="$alone"

stage=extract
stamp "extract: charon $(charon version) / aeneas $(aeneas -version)"
sh proofs/extract-all.sh /tmp/gate-extract
report extract ok
if [ "${1:-}" = "--extract-only" ]; then
    # `cargo tauten proofs extract`: one result per proved crate.
    # Build the fresh translation in Lean (without the proofs): Lean decides
    # whether the generated names compile (#46).
    rm -rf proofs/Extracted && cp -r /tmp/gate-extract proofs/Extracted
    for pair in tauten-check-kernel:TautenCheckKernel tauten-proof-instances:TautenProofInstances; do
        crate="${pair%%:*}"
        if (cd proofs && lake build "Extracted.${pair#*:}") > /tmp/gate-extract-build.log 2>&1; then
            printf 'TAUTEN-PROOFS {"stage":"extract","status":"ok","crate":"crates/%s"}\n' "$crate"
        else
            cat /tmp/gate-extract-build.log >&2
            printf 'TAUTEN-PROOFS {"stage":"extract","status":"fail","crate":"crates/%s","detail":"the translation does not compile in Lean; see the log"}\n' "$crate"
        fi
    done
    stage=""
    exit 0
fi

stage=drift
stamp "drift check against proofs/Extracted"
if ! diff -ru proofs/Extracted /tmp/gate-extract; then
    echo "FAIL: proofs/Extracted differs from a fresh translation of the proved crates;" >&2
    echo "      run 'cargo fmt', then 'sh proofs/refresh.sh', and commit the result" >&2
    exit 1
fi
report drift ok

stage=build
stamp "lake build (theorems, axiom and coverage audit)"
status=0
(cd proofs && lake build) > /tmp/gate-lake.log 2>&1 || status=$?
cat /tmp/gate-lake.log
stage=""
python3 proofs/report.py lake /tmp/gate-lake.log "$status"

stage=mutants
if [ "${1:-}" = "--only-mutants" ] && [ -z "${2:-}" ]; then
    # Ship level with no claim to re-calibrate: no mutant runs; the caller
    # reuses the recorded kills.
    stamp "mutant calibration: none selected"
elif [ "${1:-}" = "--only-mutants" ]; then
    # `cargo tauten proofs try` and the ship level: only the named mutants.
    stamp "mutant calibration (partial: ${2:-})"
    TAUTEN_MUTANT_JOBS="$jobs" LEAN_NUM_THREADS="$threads" \
        python3 proofs/mutants.py $(printf '%s' "${2:-}" | tr ',' ' ')
else
    stamp "mutant calibration"
    TAUTEN_MUTANT_JOBS="$jobs" LEAN_NUM_THREADS="$threads" python3 proofs/mutants.py
fi
report mutants ok

# What the run used, from the container's cgroup (absent outside one).
if [ -r /sys/fs/cgroup/cpu.stat ] && [ -r /sys/fs/cgroup/memory.peak ] && [ -r /sys/fs/cgroup/pids.peak ]; then
    used="$(awk '$1 == "usage_usec" { print int($2 / 1000000) }' /sys/fs/cgroup/cpu.stat)"
    stamp "used: ${used}s CPU, $(( $(cat /sys/fs/cgroup/memory.peak) / 1048576 )) MiB memory peak, $(cat /sys/fs/cgroup/pids.peak) tasks peak"
fi
stamp "proofs gate: OK"
