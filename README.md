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

Status: M1 done — harness v0 is live: byte-exact goldens with `make bless`
discipline, a 6-case upstream-dialect corpus running under the MLIR JIT,
a differential-runner scaffold awaiting the M2 interpreter, and per-pass
stage dumps (`frnksh emit --stages`). Green from a clean clone:
`make setup && make build && make test`.
