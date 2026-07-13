# The Repository

frankish is one Cargo workspace, one non-cargo TypeScript package, one golden
corpus, and a documentation set with legal force. Law L6 requires that any
agent or human can take over from `make test` plus `STATE.md` alone; this
chapter is the floor plan they would reconstruct.

```text
AGENTS.md            law (CLAUDE.md is a symlink; the two must never diverge)
STATE.md             live handoff state: phase, in-flight work, next action
docs/                SPEC, the decision ledger, contracts, design docs
specimens/           per-specimen MANIFESTs (frozen subsets, oracles)
goldens/             the execution corpus — 14 suites, 77 cases
crates/              the Rust workspace — 8 crates
tools/loanword-ts/   the TypeScript loanword producer (M9)
scripts/             POSIX-portable workflow scripts (law L6)
versions.env         THE single pin point
book/                this book (`make book`; mdbook 0.5.2 per versions.env)
```

## The workspace

```rust
// Cargo.toml
members = [
    "crates/frnksh",
    "crates/frk-core",
    "crates/frk-dialects",
    "crates/frk-interp",
    "crates/frk-front",
    "crates/frk-harness",
    "crates/frk-repl",
    "crates/frk-rt",
]
```

Edition 2024, Rust 1.96, and a deliberately short dependency list: melior
exact-pinned at `=0.27.2` (alpha — bump only deliberately, mirrored as
`MELIOR_VERSION` in `versions.env` and asserted by `scripts/check-pins.sh`),
`mlir-sys` 220 used directly only to work around melior 0.27.2's miswired
`ArrayAttribute::try_from`, `ena` for unification, and `serde_json` + `sha2`
for the loanword format (D-024/D-046 — the first non-melior runtime deps,
noted deliberately).

| Crate | Role |
|---|---|
| `frnksh` | the driver (SPEC §9); bare invocation is the REPL (D-002) |
| `frk-core` | MLIR context plumbing and the diagnostics bridge |
| `frk-dialects` | the kernel dialect library and its lowerings |
| `frk-interp` | the derived interpreter — reference semantics (D-008) |
| `frk-front` | the frontend kit and every specimen frontend |
| `frk-harness` | golden runner, diff runner, stage dumps, dashboard |
| `frk-repl` | the shell engine, library-first (D-043) |
| `frk-rt` | runtime components behind a documented C ABI (K4) |

**frnksh** carries the harness-facing subcommands: `test`, `bless`, `diff`,
`dashboard`, `emit --stages FILE [--out DIR]`, and
`grid [--canary|--native]`. Every one is reachable through a `make` target;
no vendor agent feature is load-bearing (L6).

**frk-core** constructs contexts with dialects registered *and loaded*
eagerly: melior is alpha and touching an unloaded dialect can segfault
(docs/LANDSCAPE.md), so a little startup time buys an absent failure mode.

**frk-dialects** holds one module per kernel dialect, each shipped whole
under the K1–K7 contract (D-007). Registration is IRDL runtime loading and
nothing else (D-031) — designs stay trait-free. The crate is also home to
the semantic verification pass, the Maranget decision-tree compiler
(`adt_dtree`, `dtree_emit`), the single kernel lowering pass
(`kernel_lower.rs`), and the `frk-tail-calls` pass.

**frk-interp** is the project's semantics. From M2 on, every other
execution path must byte-match it on every golden (law L3); its trap
discipline is ruled in D-029. It gets its own chapter.

**frk-front** is the frontend kit — readers, binder, the HM type kit over
`ena` — plus the resident specimen frontends: ml_core (`compile_ml`), the
loanword consumer (`loanword::compile_loanword`), femto_lua
(`lua::compile_lua`), and r7rs_core (`scheme::compile_scheme`).

**frk-harness** owns the golden runner (custom, not insta — D-027), the
differential runner (a disagreement is a first-rank finding, L3), the stage
dumper (D-028), the shared lowering pipeline table, and the only
implementation of the canonicalization contract (`canon`, docs/canon.md).

**frk-repl** is library-first so the transcript-golden runner drives the
*exact* engine the interactive binary runs (D-043): a session is an
accumulated ml_core declaration prefix, re-elaborated whole each line.

**frk-rt** is the Rust half of the two-twin runtime; its C mirror lives at
`crates/frk-rt/c/frk_rt.c`. The next chapters cover both.

## The golden corpus

`goldens/` is the execution corpus, governed by L2 (byte-exact after
canonicalization; blessing requires a commit-message justification) and L3
(every applicable runner must agree on every case). Layout per D-027:

```text
goldens/<suite>/<case>/
  case.mlir       the program (or case.ml / case.ts / case.lua / case.scm /
                  transcript.in, per suite)
  expected.out    blessed canonical output (committed)
  output.actual   written on mismatch for diffing (gitignored)
```

Directives are `// frk-case: key=value` comments (`runners=` per D-033).
Current counts, 77 cases across 14 suites:

| Suite | Cases | Exercises |
|---|---:|---|
| `upstream` | 8 | func/arith/scf/cf baseline (add, fib, loops, branches) |
| `ml_core` | 18 | OCaml specimen programs, `ocaml` as oracle (D-038) |
| `lua` | 12 | femto_lua corpus, `lua5.1` as oracle (D-052/D-058) |
| `ts0` | 8 | TS-0 corpus via the loanword producer, `node` as oracle |
| `scheme` | 6 | r7rs_core corpus incl. call/cc escapes, `chibi-scheme` oracle |
| `adt` | 5 | frk_adt value ops: sums, products, mixed fields |
| `mem` | 5 | box/array surface under every strategy runner |
| `repl` | 5 | scripted shell transcripts (`transcript.in`, D-043) |
| `dyn` | 3 | fat dyn values and tables (D-051/D-056) |
| `closure` | 2 | church encoding, counter/fold captures |
| `tailcall` | 2 | 10^6-deep self and mutual tail recursion (D-059's verifier) |
| `bstr` | 1 | interned byte strings — identity is content (D-056) |
| `ctl` | 1 | `escape_direct` — the D-061 lowering's verifier, yields 42 |
| `str` | 1 | UTF-16 strings: concat/eq/len (D-049) |

The AOT entry protocol rides on this corpus: grid runners rename the entry
function to `frk_entry` before lowering, so entry functions must be
externally-invoked-only (D-042; goldens/README.md).

Suite health as of m15-done: `make test` runs 38 test blocks; the
differential matrix runs 77 cases through 8 runners
(interp, jit, jit-rc, ocaml, node, lua, scheme, repl) with 0 divergent.

## docs/

| File | Standing |
|---|---|
| `SPEC.md` | design spec v0.1, ratified 2026-07-02; §0 maps milestones to sections; amendments require a ledger entry |
| `DECISIONS.md` | the veto ledger, append-only, D-001..D-061; consulted before any design choice (L4) |
| `LANDSCAPE.md` | verified prior art and pinned facts — "trust these over training data" |
| `canon.md` | canonicalization contract v0 (SPEC §7.4); amendments require a D-entry |
| `stages.md` | stage-dump format (D-028); dumps are pedagogy, never goldened |
| `gc-spike.md` | the M10 GC gate in writing: rc+cycles vs MMTk, decided as D-053 |
| `ctl-calculus.md` | κ_frk, the handler calculus — law as of D-060, promoted from atli |
| `type-kit.md` | the type kit documented as reusable (M6, SPEC §6.4) |
| `dialects/` | per-dialect docs: `adt`, `bstr`, `closure`, `dyn`, `mem` |

## Specimens and the producer

Each specimen directory holds exactly one file — its MANIFEST, which *is*
its scope (law L5: fence lists are law, not TODO lists):

| Specimen | Oracle (pin in `versions.env`) |
|---|---|
| `ml_core` | `ocaml` (4.14.1 tested) |
| `typescript` | `node`/V8 (node ≥ 20), tsc as checker-oracle |
| `femto_lua` | PUC-Rio `lua5.1` — "5.1.5 is the spec" |
| `r7rs_core` | `chibi-scheme` (0.9.1 tested) |
| `c_oracle` | clang/gcc rig — an oracle, never a frontend (D-009) |

`tools/loanword-ts/` is the TypeScript producer: built on the tsc 6.0.3 API
(pinned in its `package.json`), checker-as-oracle — the checker is imported,
never reimplemented (D-046). node ≥ 20 runs it directly via native type
stripping; there is no build step. CI installs its locked deps with
`npm ci` before the setup doctor runs.

## Pins and scripts

`versions.env` is the single pin point (M0 exit criterion): plain
`KEY=VALUE` lines, `include`d by GNU make and `.`-sourced by POSIX sh.
It pins Rust 1.96.0, LLVM major 22 (22.1.8 tested), melior 0.27.2,
zig 0.16.0, wasmtime 46.0.1 tested, lua 5.1.5, chibi 0.9.1, node ≥ 20,
mdbook 0.5.2. Every mirror of a pin elsewhere is asserted by
`scripts/check-pins.sh` in the same commit that adds the mirror (L1).

`scripts/ci.sh` is the whole CI story — plain shell, provider-agnostic; any
vendor config invokes it and nothing else. `scripts/setup.sh` is a doctor:
it checks prerequisites and never mutates the system. `scripts/zigcc.sh` is
the cross C driver (D-018); the grid chapter covers it.
