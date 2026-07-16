# Tail Calls as Law

Proper tail calls are not an optimization in frankish; they are a
semantic guarantee — *the law* — established at M14 (D-059) and verified
by goldens that fail without them. This chapter covers both halves of the
implementation, and the gap that M18 closed.

## Why a law, not a flag

Two specimens *require* proper tail calls for correctness, not speed.
Lua 5.1 mandates them: `return f(x)` must not grow the stack. Scheme has
no loop syntax at all — iteration *is* tail recursion, and a Scheme
implementation without the guarantee is simply wrong. The project's
depth-capped interpreter (D-029: runaway recursion traps deterministically
at a fixed frame ceiling) had carried an explicit exemption clause since
M2 — "tail calls are a lowering obligation, not deeper recursion" — and
M14 is where the clause was finally cashed.

## The shape

Both halves key on the same syntactic property, the **tail shape**: a
call whose results are exactly the operands of the immediately following
return.

```mlir
%r = func.call @spin(%n2, %acc2) : (i64, i64) -> i64
return %r : i64
```

## Half one: the interpreter trampoline

The reference interpreter's block executor intercepts a tail-shaped
`func.call` *before* dispatching it. Instead of recursing into
`eval_function` (one host frame + one counted depth unit per call), it
returns a `Step::TailCall(callee, args)` control signal. `eval_function`
runs a loop: a tail call **replaces** the current frame's role — new
callee, new arguments, same loop iteration — so a million-step tail chain
occupies one depth unit and constant host stack.

Two details make this exact rather than approximate:

- The depth cap now counts **non-tail entries only**, which is precisely
  what D-029's exemption promised. Runaway *non-tail* recursion still
  traps at the ceiling.
- The trampoline changed the meaning of one old test. The M2-era
  "runaway recursion traps" fixture was written as `return f()` — which
  is *tail-shaped*, and under the law is a legitimate infinite loop
  (exactly like `while true`), not a stack trap. The fixture now consumes
  the call's result to stay non-tail. That a test had to change is the
  measure of the change: this was semantics, not tuning.

## Half two: native `musttail`

Natively, a fifth pipeline pass (`frk-tail-calls`) runs over final
LLVM-dialect form and rewrites qualifying calls' `TailCallKind` to
`musttail`, which LLVM guarantees to lower as a frame-replacing jump.
"Qualifying" is deliberately conservative in v1:

- tail shape (call feeds the adjacent return), **and**
- a *direct* call whose callee's LLVM function type is **identical** to
  the caller's.

Self-recursion always qualifies; equal-signature mutual recursion
qualifies. `wasm32` needs the tail-call feature at compile time
(`-mtail-call`; wasmtime speaks the proposal), and the s390x canary
answers for big-endian.

## The verifiers

Two kernel goldens make the law executable, sized so each half *fails*
without its implementation:

- `tailcall/countdown` — 500,000 self-tail iterations accumulating
  `sum 1..500000 = 125000250000`.
- `tailcall/mutual` — even/odd ping-pong, 500,000 deep, identical
  signatures.

Without the trampoline, the interpreter's depth cap trips at 1,024 —
500× short. Without `musttail`, half a million frames want roughly 24 MB
of stack; the JIT executes these goldens on a 2 MB test thread, which raw
recursion would overflow twelve times over. The goldens pass on all five
grid architectures under both memory strategies — half a million
`musttail` frames on big-endian s390x and through wasm's tail-call
instruction included.

## The gap, closed: the uniform-signature convention (M18)

Indirect and cross-signature tails were v1's ledgered gap — and D-063
closed it. The **uniform-signature convention**: a closure callee may
take `(!frk_closure.envref, params…)` — one opaque env pointer, its
captures read via `closure.env_load` — instead of unpacked capture
parameters. Uniform callees get no synthesized thunk (the closure holds
their address directly), so every function of a pack-convention
language shares ONE native signature. The tail-call pass gained the
indirect case: a tail-shaped indirect call whose callsite prototype
equals the caller's function type is `musttail` — which, under the
convention, holds by construction.

femto_lua adopted it wholesale: every lua function is natively
`(ptr, ptr) -> ptr`, zero thunks, and `return f(x)` finally does what
the Lua 5.1 manual mandates. The witness is `lua/tail_recursion` —
100,000 tail frames at fixed stack, byte-checked against PUC `lua5.1`
itself, green on every grid architecture including wasm (whose
`return_call_indirect` carries it) and big-endian s390x. The
interpreter side needed no new machinery at all: closure-apply
evaluators return the same `Step::TailCall` the M14 trampoline already
speaks.

One fence remains, recorded in D-063: native TCO under the **rc
strategy** — block-exit releases sit between a tail call and its
return, breaking the tail shape. Release scheduling is its own future
rung; the deep goldens fence rc-native runners meanwhile, and the M14
depth-cap lesson replayed on schedule (the runaway-closure test's
tail-shaped fixture became a legitimate infinite loop and now consumes
its result to stay non-tail).

The payoff for the next chapter: because a guarded call cannot be a tail
call (a pending-check is code *after* the call), the tail-call law and
the control-effects lowering partition every call site cleanly between
them — the same tail-shape predicate decides both.
