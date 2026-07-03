#!/bin/sh
# Pin-coherence verifier (M0 exit criterion: versions.env is the single pin
# point). Any value mirrored outside versions.env must match it or the suite
# is red. Extend this script in the same commit that adds a new mirror (L1).
set -eu
cd "$(dirname "$0")/.."
. ./versions.env

fail=0
err() { printf 'check-pins: %s\n' "$*" >&2; fail=1; }

# rust-toolchain.toml mirrors RUST_TOOLCHAIN.
channel=$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)
[ "$channel" = "$RUST_TOOLCHAIN" ] || \
	err "rust-toolchain.toml channel '$channel' != RUST_TOOLCHAIN '$RUST_TOOLCHAIN'"

# Cargo.toml exact-pins melior to MELIOR_VERSION...
melior_req=$(sed -n 's/^melior = "=\(.*\)"$/\1/p' Cargo.toml)
[ "$melior_req" = "$MELIOR_VERSION" ] || \
	err "Cargo.toml melior pin '=$melior_req' != MELIOR_VERSION '$MELIOR_VERSION'"

# ...and Cargo.lock actually resolved that version.
melior_locked=$(awk '/^name = "melior"$/ {getline; sub(/^version = "/,""); sub(/"$/,""); print; exit}' Cargo.lock)
[ "$melior_locked" = "$MELIOR_VERSION" ] || \
	err "Cargo.lock resolved melior '$melior_locked' != MELIOR_VERSION '$MELIOR_VERSION'"

# The Makefile's mlir-sys/tblgen env var names must derive from LLVM_MAJOR.
grep -q "MLIR_SYS_${LLVM_MAJOR}0_PREFIX" Makefile || \
	err "Makefile does not export MLIR_SYS_${LLVM_MAJOR}0_PREFIX (LLVM_MAJOR=$LLVM_MAJOR)"
grep -q "TABLEGEN_${LLVM_MAJOR}0_PREFIX" Makefile || \
	err "Makefile does not export TABLEGEN_${LLVM_MAJOR}0_PREFIX (LLVM_MAJOR=$LLVM_MAJOR)"

if [ "$fail" -eq 0 ]; then
	printf 'check-pins: ok\n'
else
	exit 1
fi
