# Appendix B: Versions and Pins

Every external version frankish depends on lives in **one file**,
`versions.env`, at the repository root (SPEC §12; an M0 exit criterion).
It is plain `KEY=VALUE` — `include`d by GNU `make` and `.`-sourced by
POSIX `sh` — so the single pin point is consumable by both the build and
the shell without duplication.

The law that keeps it honest: every *mirror* of a pin elsewhere
(`rust-toolchain.toml`, `Cargo.toml`, the Makefile's `mlir-sys` env-var
names, `tools/loanword-ts/package.json`) is asserted equal by
`scripts/check-pins.sh`, and that script must be extended in the same
commit that adds a new mirror (law L1). `make test` runs it first.

## The pins

| Pin | Value | Why it is pinned |
|---|---|---|
| `RUST_TOOLCHAIN` | 1.96.0 | The compiler; mirrored in `rust-toolchain.toml`. |
| `LLVM_MAJOR` | 22 | The MLIR/LLVM major. The `mlir-sys`/`tblgen` build env-var names derive from it: `MLIR_SYS_220_PREFIX`, `TABLEGEN_220_PREFIX`. |
| `LLVM_VERSION_TESTED` | 22.1.8 | Exact LLVM/MLIR the suite was last proven against (informational). |
| `MELIOR_VERSION` | 0.27.2 | The Rust MLIR bindings; exact-pinned in `Cargo.toml`. Tracks LLVM 22 with lag — bumped deliberately, never implicitly ([LANDSCAPE](../provenance/landscape.md) watch item). |
| `OCAML_VERSION_TESTED` | 4.14.1 | The ml_core oracle. |
| `ZIG_VERSION` | 0.16.0 | The cross-compilation C driver (`zig cc`) for the AOT grid; `scripts/zigcc.sh` resolves a plain `zig` or an anyzig-style shim against it. |
| `WASMTIME_VERSION_TESTED` | 46.0.1 | The wasm32-wasi grid executor (its tail-call proposal is why native `musttail` works on wasm). |
| `LUA_VERSION_TESTED` | 5.1.5 | The femto_lua oracle — *5.1.5 is the spec*, exact-pinned. |
| `NODE_MIN_MAJOR` | 20 | The TS-0 oracle floor (native type-stripping). The `typescript` package is pinned separately at **6.0.3** in `tools/loanword-ts/package.json` (checker-as-oracle). |
| `CHIBI_VERSION_TESTED` | 0.9.1 | The r7rs_core oracle. |
| `MDBOOK_VERSION` | 0.5.2 | The tool that builds this book (`make book`; the CI Pages workflow downloads exactly this version). |

## The doctor

Two scripts enforce and check the pins, both POSIX shell (L6):

- **`scripts/check-pins.sh`** asserts that every mirror agrees with
  `versions.env`. It is the first thing `make test` runs; a drifted mirror
  fails the suite loudly rather than silently compiling against the wrong
  version.
- **`scripts/setup.sh`** is the presence doctor: it verifies the pinned
  toolchain and oracles are installed and names anything missing, without
  ever mutating the system. `make setup` runs it.

The design intent is that "which version?" is never a judgment call and
never a stale comment. There is exactly one answer, in one file, and the
harness refuses to run if a copy of it has drifted.
