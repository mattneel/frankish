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

Status: M7 done — the memory axis and the world. Memory strategy is
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
