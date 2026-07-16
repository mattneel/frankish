# Lowering Control Without an Unwinder

The interpreter can unwind an abort the honest way — it owns the call
stack. Native code cannot: frankish's Tier-0 portability floor includes
`wasm32-wasi`, where there is no `setjmp`/`longjmp` and no platform
exception machinery to borrow, and the project's default error strategy
(D-011) is **result-passing** precisely so control effects work on every
target the grid speaks. This chapter is how escape continuations run
natively — the design fork it involved, and the law that fell out.

## The pending cell

The runtime twins (Rust for JIT, C for the AOT grid — identical APIs)
carry a small control state:

- a **monotonic token source** — prompts get fresh `i64` tokens, never
  reused, so no ABA;
- a **LIFO prompt stack** of live tokens — liveness is the
  escape-past-extent trap's trigger;
- a single-slot **pending cell**: a flag, the target token, and the
  aborted value's two words (`{tag, payload}` — dyn values are 2-word
  fat values). One slot suffices because exactly one abort is ever in
  flight: no user code runs between an abort raising and its prompt
  catching.

Five functions over that state:

```c
int64_t frk_rt_ctl_prompt_enter(void);          /* push fresh token   */
void    frk_rt_ctl_prompt_exit(int64_t token);  /* LIFO pop           */
void    frk_rt_ctl_abort(int64_t, int64_t, int64_t);
                     /* dead token => trap; else park value + pending */
int64_t frk_rt_ctl_pending(void);               /* the carrier read   */
int64_t frk_rt_ctl_resolve(int64_t, int64_t *); /* my abort? clear +
                                                   write value, ret 1 */
```

## The three lowerings

`frk_ctl.prompt` lowers to a **branchless** sequence, reusing the
grid-proven out-pointer recipe the table runtime already uses: enter →
indirect-call the body closure with the token → exit → spill the body's
return into a 2-word stack slot → `resolve(token, out)` *overwrites* the
slot iff this prompt was the abort's target → reload and yield. No new
control flow, no block surgery.

`frk_ctl.abort` lowers to extracting the dyn's two words and calling the
runtime — parking the value and setting the flag. `frk_ctl.pending`
lowers to the flag read natively, and to **constant 0 in the
interpreter** — deliberately, as we'll see.

## The design panel

Everything above was uncontroversial. The genuinely hard fork was
**propagation**: after an abort sets the flag, every frame between the
abort and its prompt must return *immediately* — output emitted by a
frame that should have been unwound is a silent differential-law
violation. Who inserts those post-call checks, how does a non-terminator
`abort` divert control, and where do well-typed dummy return values come
from?

Because a wrong call here fails *silently across the whole grid*, this
decision went through a three-designer panel — independent complete
designs from three assigned angles (frontend-owned, kernel-pass-owned,
post-LLVM-pass-owned) — and an adversarial judge that scored all three
and synthesized an implementation plan. The panel converged on the op
lowerings above and surfaced the shared top risk: post-hoc block
splitting in the MLIR C-API bindings is the one genuinely fragile
primitive in every pass-based design.

The final ruling (D-061) took the judge's synthesis with one recorded
divergence: guards are emitted by the **frontend**, not inserted by a
pass. The emitter is building the CFG anyway — guard blocks are free at
that point — which sidesteps the block-splitting risk entirely. The
judge's own objection to frontend-emitted guards ("hand-written native
goldens would have to hand-author their guards") was checked against
reality and found not to bind: the hand-written `frk_ctl` goldens are
interpreter-verified, and *native* verification comes wholesale from the
Scheme differential. Design panels advise; the ledger decides; both
halves are written down.

## The guard discipline and the tail-call/guard law

After every **non-tail** call that could have aborted, the Scheme emitter
threads:

```mlir
%p = "frk_ctl.pending"() : () -> i64        // interp: 0. native: the flag.
%n = arith.cmpi ne, %p, %zero : i64
cf.cond_br %n, ^propagate, ^continue        // ^propagate: return dummy
```

Two facts make this correct rather than merely plausible:

**The interpreter never takes the branch — by design.** In the reference
semantics a real unwind happens first, so control never *returns* to the
guard with an abort in flight; `pending` answering 0 is truth, not a
stub. The guard is live only in the world that needs it. (This is also a
standing landmine in `STATE.md`: the interp/native "divergence" at this
op is the correctness argument, and must not be "fixed".)

**Tail calls are never guarded — and never need to be.** A guard is code
after a call, so guarding a tail call would destroy the tail shape that
[the previous chapter](tail-calls.md) made law. But a tail call *returns
immediately* with the callee's (dummy) result — the pending flag is still
set, so the caller's own guard, or its caller's, catches the
propagation. Guards are needed exactly where a result is *consumed* —
which is exactly where the tail shape was already absent. One predicate
partitions every call site: tail → `musttail`, unguarded; non-tail →
guarded, no `musttail`. The two subsystems cannot contend.

The abort site itself follows the same pattern: the emitter places the
runtime call, then a dummy return — dead in the interpreter (unwinding
has already left), the diversion itself natively.

## Effects-v1: the evidence stack (M24)

The v1 handlers ride the same no-unwinder discipline. `handle` is the
prompt recipe plus an **evidence push/pop** — labels are interned
byte strings, so dispatch is a pointer compare. `perform` is
**branch-free**: `perform_begin` masks the found handler and mints
the one-shot marker; the clause applies through the uniform
convention with κ built as a real native closure over that marker
(its resumer thunk marks-or-traps and returns its pack — the exact
mirror of the interpreter's special case); `perform_end` reads the
clause's returned pack head *in the runtime* and makes the
consumed-else-abort decision there, so the lowering never splits a
block. The one-shot violation trap is real state in both twins.

The license gate ran immediately: the interpreter routes every
perform through its general dispatch machinery, native through the
evidence stack — and the grid caught two real bugs before any commit
(a func.func address needing the `func.constant` recipe, and wasm32
exposing a hand-rolled κ box that read garbage under 32-bit
pointers and silently turned tail-resume into abortive). Mechanisms
disjoint, outputs byte-equal: that is the row's proof.

## The evidence

The zero-allocation license holds: aborting runs allocate no continuation
frames (the pending cell is three words of static state), checked by the
D-041 allocation counters both twins carry. And the semantics hold the
only way frankish accepts: the six-case Scheme corpus — including nested
prompts where an inner escape must *pass through* an inner prompt to an
outer one — runs byte-identical across the interpreter's real unwinding,
both JIT strategies, four AOT architectures plus the s390x canary, and
`chibi-scheme` itself. Two implementations with nothing in common but the
calculus, and the corpus cannot tell them apart. That is what κ_frk's
"licenses carry empirical gates" means in practice.
