# The frankish Book

**frankish** is a language-construction workbench built on MLIR. Its product
is a curated library of **kernel dialects** — the PL-idiom middle layer
(algebraic data, closures, memory strategies, dynamic values, control
effects) that sits between language frontends and MLIR's upstream compute
dialects — plus a frontend kit, a verification harness, and a driver,
`frnksh`.

Real languages are implemented as **specimens** to force the dialects into
existence, then re-based onto what they forced. Four exist today: an OCaml
slice, a TypeScript subset, Lua 5.1, and R7RS Scheme — each held
byte-identical to its upstream implementation.

## The numbers, as of `m15-done`

| Measure | Value |
|---|---|
| Execution paths held in agreement | 8 — reference interpreter, JIT ×{arena, rc}, AOT grid, and four upstream oracles (`ocaml`, `node`, `lua5.1`, `chibi-scheme`) |
| Differential matrix | `diff[interp,jit,jit-rc,ocaml,node,lua,scheme,repl]: 77 case(s), 0 divergent` |
| Architectures, every golden, both memory strategies | x86_64, aarch64, riscv64, wasm32-wasi — plus an s390x **big-endian canary** |
| Kernel dialects | 7 — `frk_adt`, `frk_closure`, `frk_mem`, `frk_str`, `frk_bstr`, `frk_dyn`, `frk_ctl` |
| Milestones shipped, tagged, clean-clone verified | 16 (`m0-done` … `m15-done`) |
| Decision-ledger rulings | 61 (`D-001` … `D-061`, append-only) |
| Garbage collector | reference-counting + Bacon–Rajan cycle collection, implemented **twice** (Rust and C), held equal by the differential law |

## How to read this book

- **The Method** explains the laws the project runs under — verifier-first
  development, the differential law, golden discipline, and the decision
  ledger. Everything else is a consequence.
- **Architecture** and **The Kernel Dialects** describe the machine: the
  pipeline, the reference interpreter, the two-twin runtime, the grid, and
  each dialect's contract.
- **Memory** and **Control Effects** are the two deep dives: a real
  cycle-collecting GC in two runtimes, and κ_frk — the handler calculus
  behind `frk_ctl`, with escape continuations running on every
  architecture without an unwinder.
- **The Specimens** shows the forcing loop in action, one language at a
  time. **Provenance** names the prior art, including the in-house
  languages this project exists to stop rewriting.

Source: [github.com/mattneel/frankish](https://github.com/mattneel/frankish).
Everything in this book is checked against that repository; where the book
states a number, a law, or a bit position, the repo is the authority.
