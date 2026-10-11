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

**Read the Book:** deep documentation lives at
[mattneel.github.io/frankish](https://mattneel.github.io/frankish/) —
the method, the architecture, every kernel dialect, the GC, the κ_frk
control calculus, and every specimen. Built from `book/` by
`make book`; deployed by CI on every push.

Start here: `AGENTS.md` (law) → `docs/SPEC.md` (design) →
`docs/DECISIONS.md` (ledger) → `STATE.md` (now).

Status: M35 done — femto_lua v0.4 brought coroutines to the Lua specimen
as the **resumable-frame pattern**: stackful behavior, zero stacks
switched (D-084) — plain helpers, 800-frame non-tail chains, nested
coroutines, 100k yield/resume cycles, all byte-equal to lua5.1, with the
kernel re-entrant rung still fenced and now named. Before that, M33/M34
shipped r7rs_core v0.4 (top-level value defines, parameterize, guard,
plain raise — plus the post-diff adversarial review of the whole
change) and TS emitter hygiene (block-scoped let, throw-at-function-end,
reserved synthesized names). Before that: TS-3 async/await with the tick
model panel-certified against node; TS-2 classes, interfaces, object
closures; r7rs v0.3 pairs-mutation, strings, vectors; TS-1 union
narrowing over imported checker facts; r7rs v0.1 forcing `frk.ctl`;
femto_lua v0.2/v0.3 (the pack convention, metatables, explists); TS-0
and the frozen loanword interchange — and underneath them all, the
M0–M10 spine: the kernel dialects, the derived interpreter that IS the
reference semantics, the golden/differential harness, the memory axis
with a real cycle collector, the Tier-0 grid, and the REPL. Today's
numbers: diff[interp,jit,jit-rc,ocaml,node,lua,scheme,repl] — 162 cases,
0 divergent; grid 157/157 × both strategies × 4 triples + the s390x
big-endian canary.

The current workstream is **Calque** (M36+, user-selected): the first
ORIGINAL specimen — a strict, Zig-like functional language with a
self-hosted compiler, `comptime` running on the same interpreter,
generics, and developer tools. Its acceptance contract is
`docs/calque/GOAL.md`; the charter (stage C0) freezes the bootstrap
subset, the auditable seed, and the bootstrap/fixed-point rules under
`docs/calque/`; the live, command-backed evidence matrix is
`docs/calque/ACCEPTANCE.md`, checked by `make calque-test`. Nothing in
this paragraph runs until its evidence is in that matrix.

Green from a clean clone: `make setup && make build && make test`.
