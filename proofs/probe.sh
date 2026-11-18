#!/bin/sh
# P0 probe driver (docs/proofs/probe.md). Translates each probe item on its
# own (Charon --start-from, Aeneas without -abort-on-error so it reports
# every error and still writes what it can), then type-checks whatever Aeneas
# wrote against the pinned Aeneas Lean library. Run inside the proofs image
# from the repository root, with the Lean project in $LAKE_DIR:
#
#   sh proofs/probe.sh <out-dir> [item...]
#
# Per item, <out-dir>/<item>/ holds charon.log, aeneas.log, lean.log and the
# generated Lean; <out-dir>/summary.tsv has one line per item.
set -u
out="$1"
shift
root="$(cd "$(dirname "$0")/.." && pwd)"
lake_dir="${LAKE_DIR:-/work/proofs}"
mkdir -p "$out"

# Charon arguments per item. `*_incl` items also make the tauten runtime
# crate's helpers and error types transparent (translated with bodies)
# instead of opaque external declarations.
incl="--include tauten::contract --include tauten::error"
charon_args() {
    p="--start-from tauten_probe"
    case "$1" in
        tier) echo "$p::tier::rank $p::tier::may_depend_on" ;;
        tier_derived) echo "$p::tier::may_depend_on_derived" ;;
        ratchet) echo "$p::ratchet" ;;
        percent) echo "$p::percent" ;;
        percent_incl) echo "$p::percent $incl" ;;
        percent_try_new) echo "$p::percent::__tauten_percent::Percent::try_new" ;;
        percent_try_new_incl) echo "$p::percent::__tauten_percent::Percent::try_new $incl" ;;
        percent_hand) echo "$p::percent_hand" ;;
        percent_hand_incl) echo "$p::percent_hand $incl" ;;
        contract) echo "$p::contract::deposit" ;;
        contract_incl) echo "$p::contract::deposit $incl" ;;
        contract_hand) echo "$p::contract_hand::deposit_hand" ;;
        contract_hand_incl) echo "$p::contract_hand::deposit_hand $incl" ;;
        ledger) echo "$p::ledger" ;;
        control) echo "$p::control" ;;
        control_fresh) echo "$p::control::fresh" ;;
        *) echo "unknown item $1" >&2; exit 2 ;;
    esac
}

items="$*"
[ -n "$items" ] || items="tier tier_derived ratchet percent percent_incl percent_try_new percent_try_new_incl percent_hand percent_hand_incl contract contract_incl contract_hand contract_hand_incl ledger control control_fresh"
printf 'item\tcharon\taeneas\tlean\tseconds\n' > "$out/summary.tsv"
for item in $items; do
    dir="$out/$item"
    rm -rf "$dir"
    mkdir -p "$dir/lean"
    args="$(charon_args "$item")"
    started="$(date +%s)"
    work="$(mktemp -d)"
    # Charon alone first, so its status is separate from Aeneas's.
    # shellcheck disable=SC2086
    if EXTRACT_NO_METADATA=1 EXTRACT_WORK="$work" sh "$root/proofs/extract.sh" "$root/proofs/probe" "$dir/lean" $args \
        > "$dir/extract.log" 2>&1; then
        status=ok
    else
        status=fail
    fi
    if [ -f "$work/tauten_probe.llbc" ]; then charon=ok; else charon=fail; fi
    if [ "$charon" = ok ] && grep -q '\[Error\]\|Error\b' "$dir/extract.log"; then
        if grep -q '^\[.*Error.*\]' "$dir/extract.log" && ! grep -q 'Generated' "$dir/extract.log"; then
            aeneas=fail
        else
            aeneas=errors
        fi
    elif [ "$charon" = ok ] && [ "$status" = ok ]; then
        aeneas=ok
    else
        aeneas=-
    fi
    lean=-
    if [ -f "$dir/lean/TautenProbe.lean" ]; then
        if (cd "$lake_dir" && lake env lean "$dir/lean/TautenProbe.lean") > "$dir/lean.log" 2>&1; then
            lean=ok
        else
            lean=fail
        fi
    fi
    rm -rf "$work"
    printf '%s\t%s\t%s\t%s\t%s\n' "$item" "$charon" "$aeneas" "$lean" "$(( $(date +%s) - started ))" >> "$out/summary.tsv"
done
cat "$out/summary.tsv"
