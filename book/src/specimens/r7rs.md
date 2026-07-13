# r7rs_core — Scheme Forces frk.ctl

`r7rs_core` is the Scheme specimen (M15), held against `chibi-scheme -q`
(pinned at 0.9.1). It exists for one reason: to **torture `frk_ctl`**.
Scheme makes two things load-bearing that no earlier specimen did — proper
tail calls (there is no loop syntax; iteration *is* tail recursion) and
first-class escape via `call/cc` — and admitting them forced the
control-effects dialect into existence. Its 6-case corpus lives under
`goldens/scheme/`.

## The ratification gate

r7rs_core is also the cleanest instance of a specimen manifest gating the
kernel. The stub manifest forbade its own ratification "before the ctl
effects design lands," and the spec anchored that design to a handler
calculus only the author could supply. This produced a real block,
escalated through the proper channel (`STATE.md`, "For the human"). It was
resolved by *delegation* — the calculus was already written, as
[atli](../provenance/toylangs.md) — and D-060 records the resolution:
[κ_frk](../ctl/calculus.md) promotes atli's handler core, the stub's gate
opens, the manifest ratifies, all in the same commit. The
[control-effects part](../ctl/calculus.md) covers the calculus; this
chapter is the frontend that consumes it.

## Lambda-lifting for real tail calls

The Scheme emitter (`crates/frk-front/src/scheme/`) makes a calling-
convention choice **opposite** to femto_lua's, and the contrast is the
point. femto_lua chose the pack convention (uniform first-class functions,
one heap-allocated argument array per call) because Lua leads with *arity*.
Scheme leads with *tail recursion*, so r7rs_core **lambda-lifts**
procedures to direct `func.func` calls: free variables (locals and escape
tokens) thread through as leading parameters, and a call is a plain
`func.call`. That makes a Scheme tail call a *real* [M14 tail
call](../ctl/tail-calls.md) — the trampoline in the interpreter,
`musttail` natively — with no per-call allocation. Each language pulls the
convention that fits its idiom; the kernel supports both.

## call/cc as escape, escape as prompt

`call/cc` in v0 is escape-only (the continuation appears only in operator
position — `k` is applied, never stored or returned; multi-shot use stays
fenced by κ_frk's keystone). The lowering is direct:

- `(call/cc (lambda (k) body))` becomes a `frk_ctl.prompt` over a *receiver
  closure* — an `fn<[i64],[dyn]>` capturing the receiver's free variables,
  with `k` bound to the prompt's token.
- `(k v)` becomes `frk_ctl.abort(token, v)`.

The corpus's sharpest case is `escape_nested`, which pins down that an
inner escape reaching an outer prompt must *pass through* an inner one:

```scheme
(define (inner-escapes-inner)
  (+ 100 (call/cc (lambda (k) (+ 1 (k 5))))))
(define (inner-escapes-outer)
  (call/cc (lambda (outer)
    (+ 100 (call/cc (lambda (inner) (outer 8)))))))
(display (inner-escapes-inner)) (newline)   ; 105
(display (inner-escapes-outer)) (newline)    ; 8
```

`105`: the inner escape lands at its own prompt, so the `+ 100` outside
still runs. `8`: the inner escape targets the *outer* prompt, unwinding
past the `+ 100`. Both numbers come out identical across the interpreter's
real unwinding, both JIT strategies, four AOT architectures plus the s390x
canary, and `chibi-scheme` — the two-implementations-one-calculus proof
that [the lowering chapter](../ctl/lowering.md) is about.

## The guards, and why the interpreter ignores them

Because native code has no unwinder, the emitter threads a `frk_ctl.pending`
check plus a conditional early-return after every **non-tail** call
(D-061). This is frontend-explicit guarding: the emitter builds the guard
blocks as it builds the CFG. The interpreter evaluates the same guards
harmlessly — `frk_ctl.pending` answers 0 there, because a real unwind has
already happened before any guard is reached, so the propagate branch is
interpreter-dead. This apparent interp/native divergence at one op *is* the
correctness argument, and is a recorded landmine: it must not be "fixed."

## The chibi protocol

The oracle runs `chibi-scheme -q case.scm`, stdout only, `LC_ALL=C`. Two
practical notes the frontend handles: chibi under `-q` needs `(import
(scheme base) (scheme write))` to have `call/cc` and `display` in scope, so
corpus files carry it and the reader treats `import` as a top-level no-op;
and uncaught `error` goes to chibi's stderr with a nonzero exit, so it is
*not* stdout-matchable and stays fenced from the v0 differential.

r7rs_core is v0 SHIPPED. It forced `frk_ctl` into existence, made tail
calls load-bearing corpus-wide, and demonstrated the calling-convention
fork — the fourth language on the kernel, and the one that proved the
control lane.
