#!/bin/sh
# The cross C driver (D-018/D-042): zig cc, version-pinned. Handles
# both a plain zig install and an anyzig-style version-manager shim
# (which requires the version as the first argument). POSIX sh.
set -eu

script_dir=$(dirname "$0")
# shellcheck disable=SC1091
. "$script_dir/../versions.env"

if zig version >/dev/null 2>&1; then
    exec zig cc "$@"
else
    exec zig "$ZIG_VERSION" cc "$@"
fi
