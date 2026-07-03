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
