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

Status: M4 done — two kernel dialects are real. frk.adt (sums,
products, tag dispatch) and frk.closure (first-class functions with
heap environments from frk-rt, church encoding included) are
IRDL-registered, semantically verified, interpreted, and compiled
through one kernel lowering pass — interpreter and JIT byte-equal on a
15-case corpus at every `make test`. Pattern matching compiles via a
Maranget decision-tree pass with byte-exact tree goldens. Green from a
clean clone: `make setup && make build && make test`.
