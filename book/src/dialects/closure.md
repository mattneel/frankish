# frk_closure — Functions as Values

`frk_closure` is first-class functions: `make` packages a lifted function
symbol with captured values, `apply` calls the package (SPEC §4.2). Dialect
namespace `frk_closure`; trait-free per D-031; strategy and fences ruled in
D-035, ahead of code. It landed at M4, forced by the church-encoding
milestone gate.

## The type

```mlir
!frk_closure.fn<[i64], [i64]>    // takes i64, returns i64
```

The type is the call signature only — parameter types and result types.
Captures are existential; that is the point. Two closures with different
environments and different callees are the same type if they are callable the
same way.

## Op surface

Packed per D-036: the env and the arguments are each one `!frk_adt.product`
operand.

| Op | Operands | Results | Semantics |
|---|---|---|---|
| `make` {callee = @f} | env product | `!frk_closure.fn<[p...],[r]>` | Builds a closure over `@f` with the env's fields as captures, taken **by value** at make time (D-035). |
| `apply` | closure, args product | exactly one value | Calls the closure with the unpacked args. |

Calling convention: the lifted function takes the captures first, then the
closure's parameters — `@f : (captures..., p...) -> r`.

Two laws bind the surface:

- **One-result law (D-036).** `apply` yields exactly one result and the
  closure type declares exactly one; multi-result closures are deferred —
  every v1 specimen is single-valued. The verifier's message is literal:
  `"closures return exactly one (D-036)"`.
- **By-value capture (D-035).** Capture analysis (by-val vs by-ref) becomes
  meaningful only once `frk_mem` introduces locations; frontends that need
  mutable capture (ml refs, Lua upvalues, TS `let`) capture a
  `!frk_mem.box<T>` by value and mutate through it.

IRDL enforces the base types and the symbol-ref attribute kind. The deep
contract is the verification pass's: the callee must exist as a `func.func`
in the module, its signature must equal (capture types ++ params) → result,
and `apply`'s args product and result must match the closure type exactly,
field by field.

## Reference semantics (K2)

`Value::Closure { callee, captures }` — captures snapshot the env product's
fields by value at make time. `apply` re-enters the interpreter's function
machinery, which owns the D-029 depth guard (1024 non-tail frames, then a
trap):

```rust
let mut call_args = Vec::with_capacity(captures.len() + args.len());
call_args.extend(captures.iter().cloned());
call_args.extend(args.iter().cloned());
let results = interp.eval_function(&callee, &call_args)?;
```

The star witness in `tests/closure_eval.rs` is church encoding end to end —
a closure capturing a closure, escaping upward across a function return, and
applied through the captured value:

```mlir
func.func @two_outer(%f: !frk_closure.fn<[i64], [i64]>) -> !frk_closure.fn<[i64], [i64]> {
  %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %env = "frk_adt.product_snoc"(%e, %f)
      : (!frk_adt.product<[]>, !frk_closure.fn<[i64], [i64]>)
      -> !frk_adt.product<[!frk_closure.fn<[i64], [i64]>]>
  %two = "frk_closure.make"(%env) {callee = @two_inner}
      : (!frk_adt.product<[!frk_closure.fn<[i64], [i64]>]>) -> !frk_closure.fn<[i64], [i64]>
  return %two : !frk_closure.fn<[i64], [i64]>
}
```

`two = λf.λx. f (f x)` applied to `inc` and 40 yields 42, on every runner.

## Lowering (K3): env-struct + thunk

D-035 chose the primary strategy before a line of lowering existed: closure
value = `!llvm.struct<(ptr thunk, ptr env)>`. The rationale is structural:
church encoding requires upward escape, which kills stack envs; and
same-signature closure capture makes flat defunctionalization statically
unbounded. Heap indirection is forced, so K4 (the runtime allocator)
activated at M4 instead of waiting for M7. Defunctionalization stays a
deferred whole-program strategy for a future no-heap profile.

The kernel lowering (`kernel_lower.rs`, one pass per D-037 — adt and closure
type mappings are mutually recursive and must be solved together) does three
things per `make` site:

1. **Heap-allocates the env** through the strategy allocator
   (`frk_rt_arena_alloc` or `frk_rt_rc_alloc` — the [memory
   strategy](mem.md) is a lowering parameter) and stores the env product's
   slots. Slot model: integer ≤64 = one i64 slot; f64 bitcasts through one
   slot; a nested closure is two slots, its pointers `ptrtoint`'d in;
   managed pointers were already retained (or transfer-elided) at snoc time,
   so the product-to-heap copy adds no retains.
2. **Synthesizes a thunk**, one per make-site, named `__frk_thunk_N`:
   `func.func @__frk_thunk_N(env: ptr, params...) -> result` reloads each
   capture from the env slot by slot (`trunci`/`bitcast`/`inttoptr` per
   kind) and calls the lifted callee with captures first, then params.
3. **Takes the thunk's address** as `func.constant` plus one
   `builtin.unrealized_conversion_cast` to `!llvm.ptr` —
   `llvm.mlir.addressof` cannot reference a `func.func`; FuncToLLVM turns
   the constant into an addressof and reconcile-unrealized-casts folds the
   cast away. The pair is packed:

```rust
// {thunk, env}
let closure_type = closure_struct(context);           // !llvm.struct<(ptr, ptr)>
let undef = result_value(rewriter.insert(llvm::undef(closure_type, location)))?;
let with_fn = ... insert_value(undef, [0], thunk_ptr) ...;
let closure = ... insert_value(with_fn, [1], env_ptr) ...;
```

`apply` is the mirror image: extract field 0 (the thunk pointer) and field 1
(the env pointer), unpack the args product per slot kind, and call
indirectly — an `llvm.call` with operands `[fn_ptr, env_ptr, args...]` under
the C calling convention. The thunk's `env: ptr` first parameter is the
calling convention that makes every closure of a given signature
interchangeable at the call site, whatever it captured.

## Interaction with rc

Under the rc strategy a closure value's env pointer (word 1) is the managed
half — the lowering's retain analysis classifies closure-typed stores as
`RetainKind::ClosureEnv` and retains by extracting exactly that field. The
thunk pointer is code, never retained. This symmetry (retain coverage equals
trace coverage) is D-057 law; the tracer's layout word for an env codes a
closure field as two words: skip the fn-ptr, trace the env-ptr.

## Rulings

| Entry | Ruling |
|---|---|
| D-035 | Env-struct + function pointer lowering; heap envs (upward escape forces it); by-value captures; captures-first convention; defunctionalization deferred. |
| D-036 | Packed surface: env and args are single product operands; exactly one result. |
| D-037 | One merged kernel lowering pass; closure field = two slots in the adt slot model; `func.constant` + cast for the thunk address. |
| D-041 | Envs allocate through the strategy allocator; `frk_rt_alloc` retired by rename. |

The interpreter's closure semantics are strategy-agnostic by construction;
divergence between the interp and the lowered {thunk, env} machinery is
caught by the [differential law](../method/differential.md), not by review.
