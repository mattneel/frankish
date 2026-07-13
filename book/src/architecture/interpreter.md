# The Reference Interpreter

Law L3 names the derived interpreter as the reference semantics, and D-008
makes the assignment permanent: the JIT/AOT paths must byte-match it on
every golden; specimen upstreams join as third oracles. The consequence is
structural — goldens are *blessed* from the interpreter
(`frk_harness::runner::reference_runner()` returns `InterpRunner`), so a
compiled path can never define behavior. When compiled output disagrees,
the compiled path is wrong until a ledger entry says otherwise.

The interpreter (`crates/frk-interp/src/interp.rs`, 435 lines) is a generic
walker over MLIR IR dispatching per-op `Eval` implementations. It knows
nothing about any dialect; dialects register themselves.

## The Eval registry — K2's hook

```rust
/// K2 (SPEC §3): one op's executable semantics.
pub trait Eval {
    fn eval<'c, 'a>(
        &self,
        interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError>;
}
```

`Interp::new` arms the registry with the upstream adapters
(`upstream/{arith,cf,func,scf}.rs` via `register_all`); kernel dialects
plug in through `register_eval(op_name, evaluator)` — that call *is* the
K2 obligation every kernel dialect ships. Coverage grows with the corpus:
an op lands in the registry with its tests in the same commit or not at
all (L1), and anything unlisted fails loudly as `EvalError::UnknownOp` —
a coverage boundary, never a silent skip.

Host builtins are the third dispatch tier: a `func.call` to a bodyless
symbol routes to a registered `Builtin` closure, which receives the
argument values and the shared output buffer. That is how the print
protocols run under interpretation — the harness registers
`frk_rt_scm_display_bool`, `frk_rt_bstr_from_num`, and the rest as
builtins that append to the interpreter's output string.

## Frames and the Step machine

A `Frame` is the SSA environment for one activation: a `HashMap` keyed by
MLIR value identity (the C-API pointer), meaningful only against the one
module the interpreter walks. Executing an op yields a `Step`:

```rust
pub enum Step<'c, 'a> {
    Continue,                          // results bound, fall through
    Return(Vec<Value>),                // func.return
    Branch(BlockRef<'c, 'a>, Vec<Value>), // cf.br / cf.cond_br
    Yield(Vec<Value>),                 // scf.yield
    TailCall(String, Vec<Value>),      // frame REPLACEMENT (D-059)
}
```

`exec_block` runs ops until a non-`Continue` step; `run_cfg` loops blocks
for function bodies; `run_structured_block` executes single-block scf
bodies to their `Yield` (multi-block structured regions are out of v0
scope, refused loudly). A `Yield` escaping its region, a `Return` inside
one, or a block ending without a terminator are all `Malformed` — the
interpreter treats IR-shape violations as bugs, never as outcomes.

## The tail-call trampoline

D-059 made proper tail calls reference semantics before native caught up.
`exec_block` intercepts the tail shape — a `func.call` whose results are
*exactly* (by value identity) the operands of the immediately following
`func.return` — and returns `Step::TailCall` instead of recursing. The
loop in `eval_function` is the trampoline:

```rust
let mut counted = false;
let result = loop {
    // ...resolve builtins / callee...
    if !counted {
        if self.depth.get() >= MAX_CALL_DEPTH {
            return Err(EvalError::Trap(format!(
                "call depth exceeded {MAX_CALL_DEPTH} frames (D-029)")));
        }
        self.depth.set(self.depth.get() + 1);
        counted = true;
    }
    match self.run_body(function, &args) {
        Ok(CfgOutcome::Return(values)) => break Ok(values),
        Ok(CfgOutcome::TailCall(next, next_args)) => {
            name = next;
            args = next_args;      // frame REPLACED, not stacked
        }
        Err(error) => break Err(error),
    }
};
```

Successive tail callees run at one depth unit — the `counted` flag charges
the ceiling once per non-tail entry, exactly as D-029's exemption clause
promised ("proper TCO is a lowering obligation, not deeper recursion").
The `tailcall` goldens hold this: 10^6-deep self and mutual recursion pass
under a 1024-frame cap only because frames are replaced.

Two constants govern the host stack:

| Constant | Value | Why |
|---|---|---|
| `MAX_CALL_DEPTH` | 1024 | deep recursion traps deterministically, not runner-dependently (D-029) |
| `STACK_SIZE` | 64 MiB | an interpreted frame costs a few KiB of host stack; 1024 frames need roughly 8 MiB — 2 MiB default test threads are not enough |

`InterpRunner` therefore spawns interpretation on a `STACK_SIZE` thread
around the whole parse+interpret unit (melior IR handles are not `Send`,
so the thread cannot be introduced mid-flight).

## Totality: traps, not UB

D-029: the reference semantics is total and deterministic. What native
codegen leaves undefined, the interpreter defines as a trap — and the
corollary cuts the other way: the golden corpus must be UB-free, because
native paths do whatever LLVM does with UB, so UB can never be compared.
The failure taxonomy (`error.rs`):

| `EvalError` | Meaning |
|---|---|
| `UnknownOp` | no Eval registered — a coverage boundary, not an input error |
| `Unsupported` | recognized, outside what v0 chooses to support |
| `Malformed` | IR shape the MLIR verifier should have rejected, or an interpreter bug |
| `TypeMismatch` | operand/result type violation |
| `Trap` | deterministic runtime trap: division by zero, signed-div overflow, call-depth exhaustion, non-positive `scf.for` step |
| `CalleeNotFound` | call to an absent symbol |
| `Abort { token }` | not a failure — the ctl unwinding channel (below) |

## The ctl unwinding channel

Since M15 the interpreter *really* unwinds for `frk_ctl` (κ_frk, D-060),
while native uses result-passing (D-011/D-061) — and L3 holds the two
observably equal. The machinery is three cells on `Interp`:

- `ctl_prompts` — live prompt tokens, innermost last, strictly LIFO.
  `ctl_push_prompt()` mints from a monotonic counter (tokens are never
  reused within a run, so a stale escape can never alias a fresh prompt —
  no ABA); `ctl_pop_prompt(token)` truncates from the token's position
  (defensive — a well-typed run pops the exact top).
- `ctl_prompt_live(token)` — the "escape past extent" trap's trigger: an
  abort toward a dead token is a trap, not an unwind.
- `ctl_aborted` — the parked value cell. `EvalError::Abort` carries *only*
  the token; the aborted `Value` rides this cell because `Value` is not
  `Eq` and only one abort ever unwinds at a time (an abort unwinds
  atomically — no user code runs between raise and catch).

An in-flight `frk_ctl.abort` parks its value (`ctl_set_aborted`) and
returns `Err(EvalError::Abort { token })`, which threads up through the
ordinary `Result` plumbing — every `?` in the walker is the unwinder. The
matching `frk_ctl.prompt` catches it and collects the value
(`ctl_take_aborted`); an `Abort` reaching the top of `eval_function` is by
construction an interpreter bug, and prints as
`uncaught control abort to prompt N` if one ever escapes.

This is the channel the scheme escape goldens exercise: the interpreter
unwinds for real, the native lowering threads a pending flag through
returns, and `diff` holds the two byte-identical against chibi-scheme —
including a 1000-deep tail-chain abort among the six K2 verifiers landed
with the dialect (STATE.md, m15-done).
