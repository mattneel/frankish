# frankish

A language-construction workbench on MLIR. frankish supplies the missing
middle layer — kernel dialects for ADTs, closures, memory strategies,
control effects, dynamic dispatch, contracts, and staging — plus a frontend
kit, a differential-testing harness, and `frnksh`, a REPL-first driver over
MLIR/LLVM JIT and AOT. Real languages are implemented as pinned-subset
*specimens* to force the dialects into existence, verified against their
upstream implementations per commit.

Named for the language that survives only as loanwords inside other
languages. `frnksh` is frankish written the way the other end of the trade
route would have spelled it.

Start here: `AGENTS.md` (law) → `docs/SPEC.md` (design) →
`docs/DECISIONS.md` (ledger) → `STATE.md` (now).

Status: M12 done — the rc strategy COLLECTS: sized releases, layout
descriptors the compiler writes and the runtime walks, and
Bacon–Rajan cycle collection, in both runtime twins, byte-agreeing —
proven by hand-built cycle drills, a cross-twin zigcc rig, and the
entire corpus running with real frees on five architectures. Three
languages ride the kernel, each held
byte-identical to its own upstream: core ML against ocaml,
TypeScript against node/V8, and now Lua 5.1 against PUC-Rio —
closures, metatables, interned strings, %.14g and all — across the
interpreter, two memory strategies, and five architectures
(64 cases, 7 runners, 0 divergent; grid 59/59 × 5 × 2). The
scheduled program (M0–M10) closed one milestone earlier: The
runtime dragon's cage is built and the door is open: frk.dyn's
contract is live (fat values, located traps), femto_lua's manifest
is ratified against the installed 5.1.5 oracle, and the GC gate is
decided in writing (rc+cycles; docs/gc-spike.md). Before that, a
second language rode the kernel: TypeScript
(TS-0: functions, number/boolean/string, arrays, control flow)
compiles through the frozen loanword interchange into the same
dialects ml_core forced, and runs byte-identical to node/V8 on every
golden — across the interpreter, both memory strategies, and five
AOT architectures including the big-endian canary. The demo claim is
the Static Hermes one, deliberately: predictable performance and
instant startup — fib(30) end-to-end in 3.0 ms where node spends
53.7 ms mostly booting V8 (a boot-dominated microbenchmark; V8
closes on steady-state hot loops). Before that, frankish grew a
face: bare `frnksh` is the frankish shell (ml_core on the reference interpreter, typed value rendering, :type/:load/:emit/:profile), goldened by scripted transcripts like everything else. Before that, the memory axis and the world: Memory strategy is
a compiler knob, not a language feature: the same IR runs under arena
and rc lowerings, byte-identically. And the grid proves it
everywhere: every golden × both strategies × five architectures
(x86_64, aarch64, riscv64, wasm32-wasi, s390x big-endian) — 37/37 on
all of them, interpreter = JIT = AOT = upstream OCaml. The first
specimen compiles: ml_core (a
MinCaml-shaped core ML: HM inference, let-polymorphism, ADTs, nested
match, closures, currying, mutual recursion) parses, type-checks, and
compiles through the kernel dialects; its 18-program corpus runs
byte-identically under the frankish interpreter, the frankish JIT,
and upstream OCaml — diff[interp,jit,ocaml]: 33 cases, 0 divergent.
`make dashboard` renders conformance per suite per runner. Green from
a clean clone: `make setup && make build && make test`.
