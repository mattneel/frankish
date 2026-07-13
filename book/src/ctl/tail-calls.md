# Tail Calls as Law

Proper tail calls are not an optimization in frankish; they are a
semantic guarantee — *the law* — established at M14 (D-059) and verified
by goldens that fail without them. This chapter covers both halves of the
implementation and the one honest gap.

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

## The honest gap

Indirect calls and cross-signature tails are **not** rewritten in v1 —
the pass cannot verify an indirect callee's type, and cross-signature
`musttail` is exactly where stack-argument ABIs break promises. The
ledger records the gap and its resolution path: the *uniform-signature
convention* (every function in a language one LLVM type — the logical
completion of femto_lua's pack convention), which would make `musttail`
legal by construction for closure-carried calls too.

Until then the division of labor is explicit: the interpreter trampoline
covers **all** tail shapes (it resolves closure targets dynamically), the
native pass covers the identical-signature subset, and specimen corpora
are sized so the difference is unobservable — deep-loop cases run at
depths native can absorb, with the law's full-depth witnesses living in
the kernel goldens above. Reference semantics leads; native follows; the
gap is measured, ledgered, and fenced rather than papered over.

The payoff for the next chapter: because a guarded call cannot be a tail
call (a pending-check is code *after* the call), the tail-call law and
the control-effects lowering partition every call site cleanly between
them — the same tail-shape predicate decides both.
