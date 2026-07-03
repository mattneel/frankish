#!/bin/sh
# frankish setup doctor (M0). Verifies the toolchain pinned in versions.env
# is present and names exactly what to install if not. Never mutates the
# system. POSIX sh only (law L6).
set -eu
cd "$(dirname "$0")/.."
. ./versions.env
if [ -f versions.local.env ]; then . ./versions.local.env; fi

fail=0
say() { printf '%s\n' "$*"; }
need() { # need <label> <check-command> <install-hint>
	if eval "$2" >/dev/null 2>&1; then
		say "ok:      $1"
	else
		say "MISSING: $1"
		say "  hint:  $3"
		fail=1
	fi
}

# Resolve the MLIR prefix the same way the Makefile does.
MLIR_PREFIX="${MLIR_PREFIX:-${MLIR_SYS_220_PREFIX:-$(llvm-config-"$LLVM_MAJOR" --prefix 2>/dev/null || echo /usr/lib/llvm-"$LLVM_MAJOR")}}"

need "cargo (rustup; pinned to $RUST_TOOLCHAIN via rust-toolchain.toml)" \
	"command -v cargo" \
	"install rustup: https://rustup.rs"
need "llvm-config for LLVM $LLVM_MAJOR" \
	"test -x \"$MLIR_PREFIX/bin/llvm-config\" || command -v llvm-config-$LLVM_MAJOR" \
	"apt: sudo apt-get install llvm-$LLVM_MAJOR-dev (apt.llvm.org) | brew: brew install llvm@$LLVM_MAJOR"
need "MLIR C headers ($MLIR_PREFIX/include/mlir-c)" \
	"test -d \"$MLIR_PREFIX/include/mlir-c\"" \
	"apt: sudo apt-get install libmlir-$LLVM_MAJOR-dev | brew llvm@$LLVM_MAJOR already includes MLIR"
need "mlir-tblgen ($MLIR_PREFIX/bin/mlir-tblgen)" \
	"test -x \"$MLIR_PREFIX/bin/mlir-tblgen\"" \
	"apt: sudo apt-get install mlir-$LLVM_MAJOR-tools | brew llvm@$LLVM_MAJOR already includes it"
need "ocaml (the ml_core specimen oracle, M5)" \
	"command -v ocaml" \
	"apt: sudo apt-get install ocaml | brew: brew install ocaml"
need "mlir-translate (AOT: MLIR -> LLVM IR, M7)" \
	"test -x \"$MLIR_PREFIX/bin/mlir-translate\"" \
	"apt: sudo apt-get install mlir-$LLVM_MAJOR-tools"
need "zig (the cross C driver, D-018; scripts/zigcc.sh)" \
	"sh scripts/zigcc.sh --version" \
	"https://ziglang.org/download or an anyzig shim; pin: versions.env ZIG_VERSION"
need "qemu-user (grid executors: aarch64/riscv64/s390x)" \
	"command -v qemu-aarch64 && command -v qemu-riscv64 && command -v qemu-s390x" \
	"apt: sudo apt-get install qemu-user"
need "wasmtime (grid executor: wasm32-wasi)" \
	"command -v wasmtime || test -x \"$HOME/.wasmtime/bin/wasmtime\"" \
	"curl https://wasmtime.dev/install.sh -sSf | bash"
need "node >= $NODE_MIN_MAJOR (loanword producer + TS-0 oracle, M9)" \
	"node -e 'process.exit(process.versions.node.split(\".\")[0] >= $NODE_MIN_MAJOR ? 0 : 1)'" \
	"https://nodejs.org (v$NODE_MIN_MAJOR+; v23+ runs .ts directly)"
need "loanword-ts deps (tools/loanword-ts/node_modules)" \
	"test -d tools/loanword-ts/node_modules/typescript" \
	"cd tools/loanword-ts && npm install"
need "lua5.1 (the femto_lua oracle, M10+; pin 5.1.5)" \
	"command -v lua5.1" \
	"apt: sudo apt-get install lua5.1"
need "libPolly.a (tblgen links llvm-config --libs, which names Polly)" \
	"test -f \"$MLIR_PREFIX/lib/libPolly.a\"" \
	"apt: sudo apt-get install libpolly-$LLVM_MAJOR-dev | brew llvm@$LLVM_MAJOR already bundles it"
need "libclang (for bindgen)" \
	"ls \"$MLIR_PREFIX\"/lib/libclang*.so* \"$MLIR_PREFIX\"/lib/libclang*.dylib /usr/lib/*/libclang*.so* 2>/dev/null | grep -q ." \
	"apt: sudo apt-get install libclang-$LLVM_MAJOR-dev | brew llvm@$LLVM_MAJOR already includes it"

say ""
say "MLIR prefix:        $MLIR_PREFIX"
say "exports (via make): MLIR_SYS_${LLVM_MAJOR}0_PREFIX, TABLEGEN_${LLVM_MAJOR}0_PREFIX"
say "running cargo outside make? export both of the above to the MLIR prefix."
if [ "$fail" -eq 0 ]; then
	say "setup: all present"
else
	say "setup: missing pieces above"
fi
exit "$fail"
