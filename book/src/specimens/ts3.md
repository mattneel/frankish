# TS-3 — Exceptions and Async on the Effects Lane

TS-3 put the TypeScript specimen onto frankish's control lane, in two
halves — and both halves shipped with **zero new control machinery**:
the κ_frk calculus built for Scheme carried JS exceptions verbatim,
and async/await needed nothing from the kernel but one genuinely new
rung: module-level mutable state.

## Exceptions (M30)

`throw` is `frk_ctl.perform`; `finally` is `frk_ctl.wind`; try/catch
is a handler whose clause is a static *marker* that aborts without
applying κ — the catch statements run inline at the handle site,
dispatched on the outcome tag after the prompt returns. That design
was forced by a real finding: κ_frk runs clauses at the perform site,
but JS runs `finally` **before** the catch code, and node caught the
reference interpreter producing the wrong order. Moving user code out
of the clause fixed the order on both twins *by construction* — and
deleted the catch-lifting machinery. The TS emitter adopted the D-061
guard discipline wholesale, with a typed poison table (a class-typed
function's unwind poison is `rec_cast(recref_null)` — the D-074
placeholder's second life).

## Async/await (M32)

The riskiest artifact in the milestone was a *model*: five rules for
when continuations enter the microtask queue, claimed byte-equivalent
to node's ordering within the slice's fences. Before implementation,
three adversarial agents attacked it — walking the spec algorithms,
running ~38,000 randomized differential programs against node, and
mutation-testing their own harnesses to prove a green result means
something. The model survived; every hazard they found lands *outside*
the fences and certifies a fence as load-bearing (returning a promise
from an async function costs two extra ticks; a user method literally
named `then` makes a thenable; static await dispatch presumes the
type system is sound).

The implementation is direct emission, not a runtime port: async
bodies split at their top-level awaits into continuation pack
closures (the arrow/try capture rule — parameters by value, lets by
box — carries state across awaits for free), promises are records,
and the queue is a global-cells FIFO of (continuation, value) pairs
drained after main's synchronous body. Await dispatch is static: the
producer knows whether the operand is a promise. The global-cells
rung itself (D-078) reduced to almost nothing — an LLVM global slot
*is* a box, so `global_get` lowers to one `addressof` — and it is
exactly the rung Scheme's `parameterize` has been queued on.

TS-3 is SHIPPED and frozen: four ordering-heavy async goldens and
four exception goldens run byte-identical to node across all eight
runners and the five-architecture grid.
