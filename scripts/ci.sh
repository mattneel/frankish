#!/bin/sh
# frankish CI — plain shell, provider-agnostic (M0 exit criterion; law L6).
# Any CI vendor config (or a human at a fresh checkout) invokes this script
# and nothing else. Prerequisites are exactly what scripts/setup.sh checks;
# it names anything missing.
set -eu
cd "$(dirname "$0")/.."
make setup
make build CARGOFLAGS=--locked
make test CARGOFLAGS=--locked

# The AOT native slice (D-042): continuous L3 coverage of the compiled
# path without the full cross grid (that is `make grid`, and the
# s390x canary is `make canary`, scheduled nightly).
make grid-native CARGOFLAGS=--locked
