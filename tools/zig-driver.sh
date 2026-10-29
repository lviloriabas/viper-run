#!/bin/sh
# rustc sends GNU ld's optimisation switch to Zig's linker. Zig 0.15 no
# longer implements that switch; remove the unsupported argument at the
# driver boundary, keeping every other option and all linker diagnostics.
set -eu
count=$#
case "${1:-}" in
    cc|c++)
        while [ "$count" -gt 0 ]; do
            arg=$1
            shift
            case "$arg" in
                -Wl,-O1) ;;
                *) set -- "$@" "$arg" ;;
            esac
            count=$((count - 1))
        done
        ;;
esac
exec "${VIPER_REAL_ZIG:?build.sh must supply the real Zig executable}" "$@"
