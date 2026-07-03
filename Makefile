# frankish — build entry points.
# Law L6 (AGENTS.md): every workflow runs through make + POSIX-portable
# scripts; no vendor-specific machinery is load-bearing. Pins: versions.env.

include versions.env
# Optional, gitignored, machine-local overrides (e.g. MLIR_PREFIX on hosts
# with unusual LLVM locations).
-include versions.local.env

# Where LLVM/MLIR lives. Priority: environment / versions.local.env, then
# llvm-config-$(LLVM_MAJOR) on PATH, then the apt.llvm.org default prefix.
MLIR_PREFIX ?= $(shell llvm-config-$(LLVM_MAJOR) --prefix 2>/dev/null || echo /usr/lib/llvm-$(LLVM_MAJOR))

# The mlir-sys and tblgen build scripts key off these. Their names derive
# from LLVM_MAJOR; scripts/check-pins.sh asserts they agree.
export MLIR_SYS_220_PREFIX ?= $(MLIR_PREFIX)
export TABLEGEN_220_PREFIX ?= $(MLIR_PREFIX)

CARGO ?= cargo
CARGOFLAGS ?=

.PHONY: setup build test bless diff dashboard grid grid-native canary ci clean

# Verify the pinned toolchain is present; names anything missing. Never
# mutates the system.
setup:
	sh scripts/setup.sh

build:
	$(CARGO) build --workspace $(CARGOFLAGS)

test:
	sh scripts/check-pins.sh
	$(CARGO) test --workspace $(CARGOFLAGS)

# Rewrite golden expectations from the reference runner. Law L2: the
# commit blessing new bytes must say WHY the output changed; never bless
# a diff you don't understand.
bless:
	$(CARGO) run -q -p frnksh $(CARGOFLAGS) -- bless

# Runner-agreement matrix over the golden corpus (SPEC §7.2; law L3).
diff:
	$(CARGO) run -q -p frnksh $(CARGOFLAGS) -- diff

# Conformance % per suite per runner (SPEC §8: a number, not a vibe).
dashboard:
	$(CARGO) run -q -p frnksh $(CARGOFLAGS) -- dashboard

# The Tier-0 AOT cross grid (SPEC §10; D-042): every golden × every
# grid triple × both memory strategies, executed via qemu/wasmtime.
grid:
	$(CARGO) run -q -p frnksh $(CARGOFLAGS) -- grid

# The host-triple slice of the grid — what CI runs on every push.
grid-native:
	$(CARGO) run -q -p frnksh $(CARGOFLAGS) -- grid --native

# The big-endian nightly canary (D-017): the same grid on s390x.
canary:
	$(CARGO) run -q -p frnksh $(CARGOFLAGS) -- grid --canary

# Exactly what CI runs; plain shell all the way down.
ci:
	sh scripts/ci.sh

clean:
	$(CARGO) clean
