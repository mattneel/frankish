# frankish — Landscape & Pinned Facts

Verified as of 2026-07-02 in the founding conversation. Agents: trust these
over training data; re-verify anything load-bearing that is older than ~a
quarter before depending on it, and update this file when you do.

## Substrate

- **melior / mlir-sys / tblgen-rs** (mlir-rs org): Rust MLIR bindings,
  active (commits June 2026), targets LLVM/MLIR 22. Alpha; C API unstable;
  touching unloaded dialects can segfault — load dialects eagerly.
  https://github.com/mlir-rs/melior
- **cairo-native** (LambdaClass): production language on melior; their blog
  is the honest field guide to off-path MLIR C API pain.
  https://blog.lambdaclass.com/cairo-and-mlir/
- **IRDL** (upstream MLIR dialect): dialect definitions as IR programs;
  runtime registration; generated verifiers; DynamicOpDefinition supports
  verifier + optional parser/printer/fold hook. Our v2 user-dialect hatch.
  https://mlir.llvm.org/docs/Dialects/IRDL/
- **xDSL**: Python-native MLIR-compatible framework; release Apr 2026;
  validated against MLIR 22.1.2; shares IRDL; xdsl-gui/notebooks for
  interactive transform exploration. Our prototyping sidecar.
  https://github.com/xdslproject/xdsl
- **IRDL expressiveness ceiling** (verified 2026-07-02 on LLVM 22.1.8):
  irdl.operation covers operands/results/attributes/regions with real
  constraint solving — type variables unify across positions;
  `irdl.base "#builtin.integer"` constrains attribute kind (`irdl.is`
  means attribute-equals, beware) — but there are NO trait
  declarations: dynamic ops cannot be terminators ("block with no
  terminator"), cannot carry successors ("successors in
  non-terminator"), cannot relax block-terminator rules on their
  regions. Region-bearing kernel ops therefore need the C++ shim
  (D-030 Tier B). Proof lives in crates/frk-dialects/tests/
  registration.rs; apt's llvm-22-dev + libmlir-22-dev ship the full
  C++ headers and MLIRConfig.cmake needed for Tier B.

## Watch items (time-sensitive)

- **Mojo compiler open-sourcing — fall 2026 committed.** When it lands, KGEN
  becomes the largest readable corpus of exactly our kernel-dialect layer.
  Schedule a study milestone when it drops. https://docs.modular.com/mojo/faq/
- **TypeScript 7.0** (Corsa, Go-native): RC 2026-06-18, GA ~a month later;
  **no stable programmatic API until 7.1** — tools/loanword-ts builds on the
  TS 6 API (`@typescript/typescript6` side-by-side package) until then.
  Migration to the Corsa API is a planned M9+ follow-up.
- **MLIR/LLVM major bumps**: melior tracks them with lag; versions.env is
  the single pin point; bump deliberately, never implicitly.
- **Upstream IRDL trait support**: the moment IRDL can declare traits
  (terminator et al.), D-030's Tier B (C++ shim) collapses into Tier A
  (pure IRDL). Check at every LLVM major bump.

## Peers & oracles (AOT JS/TS lane)

- **Porffor**: from-scratch AOT JS/TS → Wasm → C → native; tracks test262
  per commit; deliberately avoids WasmGC for reach. Peer + harness pattern.
- **Static Hermes** (Meta): typed-JS AOT via C; the documented `number`-has-
  no-integers wall informed D-013; framing: predictable perf, not JIT-beating.
- **AssemblyScript**: the sound-subset precedent.
- **typescript-go/tsc**: the checker we import as oracle, never reimplement.

## Bill of materials (Rust core)

chumsky + logos (native readers) · ariadne or miette (diagnostics) ·
ena (unification) · rustc_pattern_analysis (exhaustiveness, real crate) ·
scopegraphs (name resolution) · insta or custom golden runner · MMTk
(Tier-2 GC) · tree-sitter grammars for borrowed specimens · zig (cross
`cc`/linker driver) · qemu-user + wasmtime (grid execution).

## Specimen oracles

ml_core: ocaml (executable oracle) + min-caml sources (readable spec).
femto_lua: PUC-Rio Lua 5.1.5 (pin) + official tests; LuaJIT as perf yardstick.
r7rs_core: chibi-scheme (readable) + chez (ceiling).
c_oracle: clang/gcc + csmith + creduce; clang for ABI/layout diffing.
typescript: node/V8 (ground truth), tsc baselines, curated test262 slice;
license-check every vendored corpus (Lua MIT; test262 BSD; others verify).

## Paper crib list

Maranget, *Compiling Pattern Matching to Good Decision Trees* (decision-tree
pass). Xie & Leijen, evidence-passing effect handlers (Koka) (ctl lowering).
Néron/Tolmach/Visser et al., scope graphs (binder). Fehr et al., *IRDL* (PLDI
'23) and *Sidekick Compilation with xDSL* (substrate). Flatt, *Honu*/Rhombus
enforestation (enforest reader). Siek & Taha, gradual typing + blame
(dyn×contract cell). Go internals: itab dispatch (frk.dyn). Tiger Style
(frk.contract's soul).
