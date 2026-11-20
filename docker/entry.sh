#!/bin/sh
# Entry point of the tools services (compose.yaml): run the command as the
# owner of the mounted checkout, not as root (#42). A gate that runs as root
# passes tests the developer's host fails (root writes over a 0444 file).
# Running as root is an explicit, recorded opt-in: TAUTEN_GATE_ROOT=1.
set -eu
if [ "$(id -u)" = 0 ] && [ "${TAUTEN_GATE_ROOT:-0}" != 1 ]; then
    uid=$(stat -c %u /work/tauten)
    gid=$(stat -c %g /work/tauten)
    if [ "$uid" != 0 ]; then
        # The cache volumes were created by root; hand them over once.
        for dir in /cache/target /usr/local/cargo/registry /usr/local/cargo/git; do
            mkdir -p "$dir"
            [ "$(stat -c %u "$dir")" = "$uid" ] || chown -R "$uid:$gid" "$dir"
        done
        HOME="/tmp/home-$uid"
        mkdir -p "$HOME"
        chown "$uid:$gid" "$HOME"
        export HOME
        exec setpriv --reuid="$uid" --regid="$gid" --clear-groups "$@"
    fi
fi
exec "$@"
