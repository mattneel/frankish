# The Pipeline

Every consumer that lowers IR — the two JIT runners, the AOT grid runners,
the stage dumper — must see the same passes in the same order. So the
pipeline is defined exactly once, in `crates/frk-harness/src/pipeline.rs`,
and everything else consumes that table:

```rust
/// Stage names + fresh Pass objects for one strategy, in order.
pub fn stages(strategy: Strategy) -> Vec<(&'static str, Pass)> {
    vec![
        ("lower-frk-kernel", frk_dialects::lower_kernel_pass(strategy)),
        ("convert-scf-to-cf", pass::conversion::create_scf_to_control_flow()),
        ("convert-to-llvm", pass::conversion::create_to_llvm()),
        ("reconcile-unrealized-casts",
            pass::conversion::create_reconcile_unrealized_casts()),
        // The tail-call law's native rung (M14, D-059): runs last,
        // over final LLVM form.
        ("frk-tail-calls", frk_dialects::tail_calls_pass()),
    ]
}
```

The memory strategy (D-041) parameterizes only the kernel stage; the
upstream conversions are strategy-blind. `Strategy::Arena` and
`Strategy::Rc` therefore share four of five stages verbatim, which is what
lets the diff matrix hold `jit` and `jit-rc` byte-equal over one corpus.

## Before any pass runs

Every runner shares the same front half: parse, MLIR verification, then
frankish semantic verification — SPEC §3 K1 as amended by D-031, which
requires semantic verification *before any execution or lowering*. This is
a consequence of how the dialects are registered: IRDL runtime loading only,
no C++ ODS shim anywhere in v1 (D-031, struck down from D-030 by the human).
IRDL generates structural verifiers, but invariants beyond its constraint
language (for example, that an `extract`'s result type equals the named
field's type) live in a frankish verification pass — K1's "verifier
enforcing invariants" hosted in a pass, not in C++.

The same ruling shapes the op surface the pipeline consumes. LLVM-22 IRDL
cannot declare traits, so no kernel op is a terminator, carries successors,
or owns trait-relaxed regions; multiway dispatch rides upstream `cf.switch`
(D-031). And IRDL constraint variables bind once per op instance, so
heterogeneous variadics are inexpressible — kernel ops take at most two
operands and one result, with explicit packing chains
(`product_new()` + `product_snoc(...)`) instead of variadic groups (D-036).

## Stage 1: lower-frk-kernel — one pass, not five

D-037 superseded D-032's per-dialect packaging: the kernel lowering is ONE
external MLIR pass. The reason is value nesting — adt products carry
closure-typed fields (church's env is `product<[fn<...>]>`) and closure
envs/args are adt products — so the type mapping must be solved together.
A per-dialect pass ordering would need each pass to understand the other
dialects' types anyway; one pass makes the shared slot model explicit.

Representations (from the module doc of `kernel_lower.rs`):

| frk type | LLVM form |
|---|---|
| sum | `!llvm.struct<(i64 tag, i64 × K)>`, K = max variant slots |
| product | `!llvm.struct<(i64 × S)>` |
| fn | `!llvm.struct<(ptr thunk, ptr env)>` |

Slot model (D-037): an integer field ≤ 64 bits occupies one i64 slot
(`extui` in, `trunci` out); a closure field occupies two slots, its two
pointers `ptrtoint`'d in and `inttoptr`'d back out.

Mechanically the pass is plan-then-apply, in three sweeps over the module:

1. **Collect.** One walk gathers a `Vec<Planned>` — one plan per kernel op
   (adt, closure, mem, str, dyn, bstr, ctl) — plus a `retypes` list of
   `(Value, mapped Type)` pairs for every block argument and op result
   whose type must change, and function-signature rewrites. Under `Rc`,
   the same walk feeds the block-local liveness analysis that plans
   release points (GC ladder step 1, D-053/D-054).
2. **Retype.** `value.set_type(mapped)` runs over the collected values, and
   function `function_type` attributes are rewritten. Only then are the
   per-make-site closure thunks and runtime declarations synthesized —
   against retyped values, so they see final types.
3. **Apply.** An `IrRewriter` (melior's `RewriterBase`) replaces the planned
   ops in program order, then appends the planned block-end releases before
   terminators.

The pass is packaged with `melior::pass::create_external` (D-032's
packaging clause), which is why it can sit in the same `PassManager` table
as the upstream conversions.

## Stages 2–4: the upstream conversions

`convert-scf-to-cf`, `convert-to-llvm`, and `reconcile-unrealized-casts`
are stock MLIR conversions (the JIT lowering shape was ruled with the
harness itself, D-027). The reconcile stage is load-bearing, not
ceremonial: closure `make` takes its thunk's address as `func.constant`
plus one `builtin.unrealized_conversion_cast` to `!llvm.ptr`, because
`llvm.mlir.addressof` cannot reference a `func.func`. FuncToLLVM turns the
constant into `llvm.mlir.addressof`; reconcile folds the cast away
(D-035/D-037, verified end to end).

## Stage 5: frk-tail-calls

The native rung of the tail-call law (M14, D-059) runs *last*, over final
LLVM-dialect form: any direct `llvm.call` in tail shape — its results are
exactly the operands of the immediately following `llvm.return` — whose
callee has an LLVM function type identical to its caller's gets

```rust
let musttail = Attribute::parse(context, "#llvm.tailcallkind<musttail>")
```

which LLVM guarantees to lower as a frame-replacing jump. The
identical-signature gate is D-059's deliberate v1 frontier: self-recursion
always qualifies, equal-signature mutual recursion qualifies; indirect and
cross-signature tails are the ledgered gap awaiting the uniform-signature
convention. The interpreter's trampoline covers all shapes meanwhile —
reference semantics leads, native follows. On wasm32 the downstream
compile adds `-mtail-call` (the wasm tail-call feature; wasmtime 46 has
the proposal on by default).

The verifier for this stage is the `tailcall` suite: 10^6-deep self and
mutual recursion goldens that fail without each rung — the interpreter's
depth cap trips, and the native stack overflows without `musttail`.

## Stage dumps

`frnksh emit --stages FILE [--out DIR]` writes numbered per-pass snapshots
(default `out/stages/<FILE stem>/`):

```text
00-parsed.mlir
01-lower-frk-kernel.mlir
02-convert-scf-to-cf.mlir
03-convert-to-llvm.mlir
04-reconcile-unrealized-casts.mlir
05-frk-tail-calls.mlir
```

Mechanics ruled in D-028: v0 runs one single-pass `PassManager` per
pipeline entry, so each snapshot is exactly "the module after pass N" —
exact snapshots without C-API IR-printing instrumentation, at the cost of
not exercising multi-pass scheduling here (the JIT runner covers that path
over the same table). The out directory is removed and recreated whole on
every dump; stale snapshots cannot linger.

Two non-guarantees, both deliberate (docs/stages.md): snapshots are MLIR's
default textual form, and dumps are never goldened — their bytes track
MLIR's printer, not frankish semantics. Diff adjacent stages of one dump;
do not diff dumps across MLIR versions. Because pass names come from the
shared pipeline table, a dump sequence *is* the pipeline definition — there
is no parallel truth to drift.

## The same table, three consumers

| Consumer | Entry | What follows the pipeline |
|---|---|---|
| JIT runners (`jit`, `jit-rc`) | `pipeline::lower_to_llvm` | ORC `ExecutionEngine`, runtime symbols registered in-process |
| AOT grid runners | `pipeline::lower_to_llvm` after the `frk_entry` rename | `mlir-translate --mlir-to-llvmir`, pinned clang, zig cc link |
| Stage dumper | `pipeline::stages` one pass at a time | snapshot per stage |

That single-definition discipline is the pipeline's actual design: the
passes are ordinary; the refusal to let two consumers own two pass lists is
the part that keeps L3 checkable.
