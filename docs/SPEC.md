# frankish — Design Specification

Status: v0.1, ratified 2026-07-02. Amendments require a DECISIONS.md entry.

## §0 Reading guide

Milestone → required sections: M0–M1 → §7, §12; M2 → §7.1–7.3; M3–M4 → §3, §4;
M5–M6 → §6, §8, specimens/ml_core; M7 → §4.3, §10; M8 → §9; M9 → §6.3, §8,
specimens/typescript; M10 → §4.5, specimens/femto_lua. Everyone reads §1–§2 once.

## §1 Thesis

MLIR is excellent below the waterline (arith, scf, cf, func, memref, ptr,
llvm, gpu, vector) and empty above it for general-purpose languages: there is
no upstream closure dialect, no ADT dialect, no GC dialect, no exception
dialect. Every serious frontend (Flang/FIR, Mojo/KGEN, cairo-native, ClangIR)
reinvents this middle layer privately. frankish's product is that middle layer,
built once, curated, verified, and composable:

- **Kernel dialects** (§4): the PL-idiom layer, each shipped as a sealed unit
  under the contract in §3.
- **Frontend kit** (§6): readers, a uniform green tree, name resolution, a
  type kit with an honest ceiling, diagnostics that survive lowering.
- **Harness** (§7): a derived reference interpreter, differential testing as
  a built-in primitive, byte-exact goldens, per-stage IR dumps.
- **Driver** (§9): `frnksh` — REPL-first CLI over JIT (ORC) and AOT (LLVM).

A **language profile** is a set of enabled kernel dialects plus a lowering
strategy per axis (memory, control, dispatch) plus a reader. Users of v1
*compose* framework-owned dialects; they do not define dialects (D-006).
Real languages are hosted as **specimens** (§8) to force the dialects into
existence; the framework is correctly factored iff each specimen re-bases
onto promoted dialects as a thin profile.

Design north star: verification is centralized and cheap; implementation is
marginal and largely agent-executed. The scarce inputs are specs, verifiers,
and ledger rulings.

## §2 Architecture layers

- **L0 Substrate.** MLIR context/pass-manager/ExecutionEngine via melior
  (LLVM/MLIR 22.x, D-005), LLVM AOT, target machinery. Location threading and
  diagnostic bridging (§6.5) live here.
- **L1 Kernel dialects.** §4. The product.
- **L2 Frontend kit.** §6. Readers → green tree → binder → type kit →
  emission into L1.
- **L3 Driver + harness.** §7, §9. frnksh, golden/differential runners,
  stage dumps, conformance dashboard, CI grid.

## §3 Kernel dialect contract

Every kernel dialect ships all of K1–K7 before it is "done". Partial dialects
live on branches, not main.

- **K1 Definition.** Ops, types, attributes; documented invariants; verifier
  enforcing them. Registered dialects, loaded at context startup from IRDL
  definitions embedded in the framework — no C++/ODS anywhere in v1
  (amended per D-031, which also bars trait-requiring op designs);
  invariants beyond IRDL's constraint language are enforced by a frankish
  verification pass that runs before any execution or lowering. IRDL
  runtime loading doubles as the v2 user-dialect hatch (D-006).
- **K2 Eval.** Every op implements the Eval interface (§7.1). The derived
  interpreter over K2 is the dialect's reference semantics.
- **K3 Lowerings.** At least one lowering pass to strictly lower dialects
  (upstream or kernel), with named strategy variants where the design calls
  for them (e.g. mem: arena|rc|gc). Lowerings preserve locations (§6.5).
- **K4 Runtime component.** Whatever the lowering requires at run time ships
  in `frk-rt` behind a documented C ABI, freestanding-first (§10).
- **K5 Goldens.** A golden corpus exercising every op and every lowering
  strategy, green under the differential law (L3).
- **K6 Docs.** One page: semantics, lowering contracts, interaction-matrix
  rows it participates in (§5), portability tier impact (§10).
- **K7 Ledger.** Every design fork encountered gets a D-entry.

## §4 Kernel dialect inventory (v1 target set)

Reuse upstream, never wrap gratuitously: arith, scf, cf, func, memref, ptr,
index, llvm; gpu/vector reserved for the height axis (§10).

### §4.1 frk.adt
Sums, products, tuples as parametric `!frk_adt` types; pure value ops
`make`, `tag_of`, `extract`. There is no region-based `match` op (amended
per D-031): multiway dispatch rides upstream `cf.switch`, and surface
`match` is compiled directly to dispatch IR by the decision-tree pass.
Passes: Maranget-style decision-tree compilation from the frontend's
pattern matrix to dispatch IR (its own goldens over the matrix→IR
mapping, D-025); exhaustiveness/usefulness via the rustc_pattern_analysis
crate behind a trait boundary; niche/tag-packing optimization as a later,
separately-goldened pass. Invariants beyond IRDL's constraint language
are enforced by the frk verification pass (K1, D-031). Lowering: LLVM
structs + integer tag + switch.

### §4.2 frk.closure
`closure.make` (fn ref + capture list) and `closure.apply`. Capture analysis
pass (by-value vs by-ref per the memory axis); lowering to env-struct +
function pointer; defunctionalization as an alternate whole-program strategy
for the no-heap profile. Interacts with §4.3 (captures are roots).

### §4.3 frk.mem
One allocation/ownership surface, swappable lowerings: **arena**, **rc**
(with elision pass), **gc** (MMTk binding; Tier-2 only), **manual**
(explicit-allocator, Zig-flavored). Escape analysis feeds arena and rc.
The strategy is a profile knob, not a language feature.

### §4.4 frk.ctl
Errors: **result-passing** is the default lowering (Tier-0 friendly, D-011);
**unwinding** (Itanium/SEH) is a Tier-2 opt-in strategy of the same ops.
Effects & handlers: evidence-passing lowering à la Koka when this lands
(D-012); the human's Rocq handler calculus is the semantic anchor — its
typing/charge discipline becomes this dialect's verifier obligations.
Coroutines/async: LLVM coro lowering; TS async arrives via the ported tsc
state-machine transform (specimens/typescript).

### §4.5 frk.dyn
Uniform values: tagging schemes (NaN-boxing vs pointer tagging is a D-entry
at M10); boxes; dynamic calls; **itab-style structural interface dispatch**
(Go's model is the crib) covering both Lua metatable-ish dispatch and TS
structural interfaces. Inline caches are a later pass, not v1.

### §4.6 frk.contract
`require` / `ensure` / `invariant` ops that lower to trapping asserts or
erase per profile. Tiger Style as a dialect. Doubles as the gradual-typing
boundary: dyn↔typed casts are contract ops with blame payloads (D-015).

### §4.7 frk.stage
Comptime/partial evaluation over kernel IR: the M2 interpreter running at
compile time over `stage.quote` regions, splicing values (v1: scalars and
adt values). Comptime types/generics beyond monomorphization are fenced
(§14) — this dialect is the research frontier, sequenced last.

## §5 Interaction matrix

The matrix is first-class IP: every cell is *pre-solved* (names the passes
and runtime it requires), *costed*, or *refused with reasons*. Seed rows —
extend in place, one D-entry per new ruling:

| Cell | Ruling |
|---|---|
| closure × mem/arena | requires escape analysis; escapees promote or reject at verify time |
| closure × mem/gc | captures registered as roots via shadow-stack maps in frk-rt |
| ctl/effects × FFI | handlers do not cross foreign frames; verifier rejects; document |
| ctl/unwind × Tier-0 | refused; result-passing only below Tier 2 |
| dyn × contract | gradual typing: boundary casts are contract ops w/ blame (D-015) |
| adt × dyn | boxed sums share the dyn tag plan; niche opt disabled on boxed reps |
| stage × macros | three stances (cosmetic / split / unified) are a profile enum, not resolved globally |
| sealed-world switch | profile flag enabling devirt + closed unions (D-014); default off |

## §6 Frontend kit

### §6.1 Readers
Pluggable, all producing the same green tree: **pratt** (default C-family),
**sexp**, **enforest** (Honu/Rhombus-style macro-aware C-family),
**phrase** (Inscription-style templates). Borrowed specimens use tree-sitter
grammars or upstream parsers as scaffolding (D-019); the native readers are
reserved for original languages where syntax is the point.

### §6.2 Green tree
Uniform `(head, span, children)` nodes, lossless enough to reprint. Concrete
representation (rowan vs compact custom) is decided at M5 (§15). Spans are
mandatory on every node — no synthetic-span nodes without a `derived-from`
back-pointer.

### §6.3 loanword (interchange)
The typed-AST artifact specimen frontends emit and frnksh consumes.
Invariants (frozen at M9, sketch now): canonical byte encoding (sorted-key
canonical JSON v0; CBOR revisit per D-024); interned type table; every node
carries span + optional type-ref; SHA-256 of canonical bytes is the artifact
id; version field mandatory. The TS frontend (tools/loanword-ts) is the
first producer; any future out-of-process frontend speaks it.

### §6.4 Binding & types
Name resolution via scope graphs (scopegraphs crate or faithful port).
Type kit: unification (ena), a bidirectional skeleton with holes,
exhaustiveness via §4.1's boundary, HM inference for ml_core. Honest
ceiling: trait/typeclass solving is dictionary-passing only in v1 (D-020);
declarative type-system genericity is a non-goal.

### §6.5 Diagnostics & location law
Every green-tree span threads into MLIR Location attributes at emission.
Every MLIR verifier/pass diagnostic must surface as a source-mapped report
(ariadne/miette) against original source. A diagnostic that points at IR
instead of source is a bug with a regression test.

## §7 Harness

### §7.1 Eval interface & derived interpreter
Per-op Eval trait (K2). The generic interpreter walks any mix of kernel +
supported upstream ops. It is the reference semantics and the REPL's fast
path. Built in M2 over upstream dialects first to de-risk the inversion.

### §7.2 Differential law (mechanics)
Runners: `interp`, `jit` (ORC), `aot` (per-target, §10), plus per-specimen
`oracle` (upstream implementation). `make diff` executes the golden corpus
across all applicable runners and byte-compares canonicalized output.

### §7.3 Stage dumps & reduction
`frnksh emit --stages` writes numbered per-pass IR snapshots to a directory
(diffable; the pedagogy artifact). Bug minimization via mlir-reduce
integration; pass-pipeline bisection helper in the harness.

### §7.4 Canonicalization contract
docs/canon.md (written in M1) governs all cross-runner comparison: float
printing (shortest round-trip), map/iteration ordering, error-text
normalization, newline/locale policy. Oracle outputs are normalized through
the same filter. No diff is judged outside the contract.

### §7.5 Fuzzing
Typed-program generators per specimen subset (arbitrary-based), differential
across runners; c_oracle brings csmith/creduce for the C slice. Nightly, not
gating, until M7.

## §8 Specimen program

Full discipline in specimens/README.md; per-language law in each MANIFEST.
Order and rationale:

1. **ml_core** (M5) — forces adt, closure, decision trees, HM, while the
   runtime is still just malloc. Abstraction risk first, runtime dragon asleep.
2. **typescript TS-0** (M9) — the motivation demo and the loanword forcing
   function; needs only arith/scf/closure-lite. Full TS staging (TS-1..4)
   rides later milestones; see its MANIFEST.
3. **femto_lua** (M10+) — wakes the runtime dragon: dyn, strings, GC gate.
4. **r7rs_core** (post-M10) — tortures ctl: proper tail calls as law,
   one-shot continuations first, hygienic macros exercising the expander.
5. **c_oracle** (early, parallel) — not a frontend: clang-bitcode import rig,
   per-target ABI/struct-layout diffing against clang, csmith/creduce.

**Extraction loop (law):** a specimen's first implementation may cheat
(private ops, ad-hoc lowerings) to go green; the deliverable is the
**promotion pass** — extract what the specimen forced into kernel dialects
under the §3 contract, then re-base the specimen and show conformance
unchanged. A specimen still fat after promotion is evidence the abstraction
is wrong; file it, don't paper it.

**Oracle triangulation (law):** upstream reference ↔ derived interpreter ↔
JIT/AOT, pairwise, over the vendored conformance corpus, per commit.
Dashboard: conformance % per specimen per runner per target — a number, not
a vibe.

## §9 frnksh CLI

Bare `frnksh` = the frankish shell: REPL backed by the derived interpreter,
profile shown in prompt, `:load`, `:emit`, `:profile`, `:type`. Subcommands:
`build` (AOT via profile+triple), `run` (JIT), `test` (goldens+diff),
`emit --stages`, `bless`, `reduce`, `grid` (portability matrix), `fmt`
(later). Binary name `frnksh`; `frankish` ships as alias symlink (ripgrep
precedent). Machine-readable output (`--json`) on test/grid for dashboards.

## §10 Portability

Two axes: **breadth** (LLVM triples) and **height** (heterogeneity via MLIR
gpu → NVVM/ROCDL/SPIR-V; deferred until a specimen demands it, ledger'd).

Tiers are profile properties, not slogans:
- **Tier 0 freestanding** — arena/rc + result errors + no threads/libc →
  every LLVM triple. frk-rt is freestanding-first to keep this tier big.
- **Tier 1 hosted** — + libc (musl / wasi-libc), strings, IO.
- **Tier 2 managed** — MMTk GC, unwinding, threads → documented short list,
  widened deliberately.

CI grid (M7): cross-compile golden corpus per tier's triple set; execute
under qemu-user and wasmtime; grid = specimen × triple, green/red, per
commit. s390x row as the big-endian canary (nightly acceptable). Cross
linking via bundled `zig cc` driver (D-018); wasm32-wasi through the normal
LLVM path; WasmGC deferred (D-016).

## §11 Host stack (ruling D-005 — flagged for human review)

**Core in Rust** on melior/mlir-sys pinned to LLVM/MLIR 22.x. Bill of
materials: chumsky+logos (native readers), ariadne or miette (diagnostics),
ena (unification), rustc_pattern_analysis (exhaustiveness), scopegraphs,
insta (snapshots) or custom golden runner, MMTk (Tier 2). Known sharp edges
(docs/LANDSCAPE.md): melior is alpha; unloaded-dialect access can segfault;
off-path C API work is trial-and-error.

**xDSL sidecar**: kernel-dialect *designs* may be prototyped in xDSL
(Python, MLIR-22-compatible, IRDL-shared) where an experiment costs an
afternoon; keepers are committed to the Rust product. Sidecar artifacts live
in `sandbox/` and never gate CI.

**Roads not taken, with revisit conditions:** Beaver/Elixir+Zig (revisit if
a BEAM-hosted interactive service becomes a goal); pure-Zig against the C
API (revisit if melior's gaps dominate; the C API surface is the risk
either way); C++ (never, absent upstreaming ambitions).

The TypeScript frontend is TypeScript regardless (tools/loanword-ts), a
separate process speaking loanword — the checker-as-oracle architecture
(tsc 6 stable API now; migrate to the Corsa API at TS 7.1, watch item in
LANDSCAPE.md).

## §12 Toolchain & layout

Pins: rust-toolchain.toml (stable, pinned); LLVM/MLIR 22.x installed per
`make setup` docs (brew llvm@22 / apt.llvm.org; MLIR_SYS_220_PREFIX and
TABLEGEN_220_PREFIX exported); zig (cross driver) vendored version noted in
Makefile; node ≥ 20 for tools/loanword-ts (M9). All version pins live in
one place: `versions.env`, sourced by Makefile and CI.

Workspace layout (M0 creates):

    crates/frnksh        CLI + REPL (bin)
    crates/frk-core      context, locations, diagnostics bridge, green tree
    crates/frk-dialects  kernel dialects (one module per dialect)
    crates/frk-interp    eval interface + derived interpreter
    crates/frk-front     readers, binder, type kit, loanword consumer
    crates/frk-harness   golden/diff runners, stage dumps, dashboard emit
    crates/frk-rt        runtime staticlib (freestanding-first, C ABI)
    tools/loanword-ts    TypeScript frontend package (M9)
    sandbox/             xDSL prototypes, spikes (never gates CI)

## §13 Milestones

Each ends with: suite green, STATE.md milestone note, tag `mN-done`, push.

- **M0 Toolchain & smoke.** Workspace skeleton; melior pinned and building;
  smoke: construct `add(i64,i64)` module via melior, JIT through
  ExecutionEngine, assert. `make setup|build|test` work from clean clone;
  plain-shell CI script. *Exit: green clean-clone test; versions.env is the
  single pin point.*
- **M1 Harness v0.** Golden runner (byte-exact, `make bless` with
  justification law L2); `emit --stages` dump format; docs/canon.md v0;
  differential runner scaffold. *Exit: harness self-tests + ≥5 goldens over
  upstream-dialect programs.*
- **M2 Derived interpreter.** Eval trait; interpreter over func/arith/scf/cf;
  two-way diff (interp vs JIT) live on all goldens. *Exit: L3 enforced in CI.*
- **M3 frk.adt.** Full §3 contract incl. decision-tree pass + exhaustiveness.
  *Exit: K1–K7 checked; 3-way goldens green.*
- **M4 frk.closure.** Contract complete; capture analysis; env lowering.
  *Exit: church-encoding + counter goldens green under diff.*
- **M5 ml_core v0.** Parser scaffold; HM via type kit; lower to kernel;
  vendored conformance corpus; ocaml joins as oracle; dashboard row exists.
  *Exit: ≥90% manifest conformance; extraction report written.*
- **M6 Promotion pass #1.** Promote M5 cheats; re-base ml_core thin; type
  kit documented as reusable. *Exit: no private ops in ml_core; conformance
  not worse.*
- **M7 frk.mem v0 + Tier-0 grid.** arena + rc lowerings of one surface;
  minimal rc elision; CI grid {x86_64, aarch64, riscv64/qemu, wasm32-wasi/
  wasmtime}; s390x nightly canary. *Exit: grid green for ml_core corpus
  under both strategies.*
- **M8 frnksh shell v0.** Bare = REPL on the interpreter; `:emit`/`:profile`
  /`:load`; scripted-transcript goldens. ORC per-cell redefinition is the
  stretch, not the gate. *Exit: transcript goldens green.*
- **M9 loanword v1 + TS-0.** Freeze loanword; ship tools/loanword-ts on the
  tsc 6 API; frnksh consumes → native; fib.ts demo golden + startup number
  recorded (not gated); node joins as oracle. *Exit: TS-0 manifest 100%;
  diff green.*
- **M10 femto_lua opens.** frk.dyn v0 (tagging D-entry); Lua string ruling;
  GC gate: rc+cycles vs MMTk spike report before proceeding. *Exit: manifest
  ratified; dyn contract underway; GC decision logged.*

Beyond M10 (unscheduled, ordered): scheme/ctl track; effects lowering;
frk.stage; TS-1..4; height axis (gpu).

## §14 Non-goals (v1)

Lazy evaluation; user-defined dialects (IRDL is the v2 hatch); LSP/editor
tooling (keep the pipeline pure and coarse-grained so incrementality can be
added, but build none of it); WasmGC backend; full call/cc; typeclass
solving beyond dictionaries; Windows-native dev environment (WSL fine);
package manager / module registry; self-hosting; performance work beyond
what correct lowering gives for free.

## §15 Open questions (genuinely open — not blockers)

Green-tree representation (rowan vs custom) — decide with evidence at M5.
ORC redefinition strategy for REPL cells — decide at M8 (JITDylib-per-cell
vs resource trackers; clang-repl is the crib). MMTk vs shadow-stack-first
for Lua — the M10 gate. s390x in the gating grid vs nightly-only — revisit
when grid runtimes are known.
