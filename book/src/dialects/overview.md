# The Middle Layer

MLIR is excellent below the waterline — arith, scf, cf, func, memref, ptr,
llvm — and empty above it for general-purpose languages. There is no upstream
closure dialect, no ADT dialect, no GC dialect, no exception dialect. Every
serious frontend (Flang/FIR, Mojo/KGEN, cairo-native, ClangIR) reinvents this
middle layer privately. frankish's product is that middle layer, built once,
curated, verified, and composable (SPEC §1).

A **kernel dialect** is one sealed unit of that layer: the ops, types, and
semantics for a single PL idiom — algebraic data, first-class functions,
allocation, strings, dynamic values, control effects. Frontends emit kernel
ops; the kernel lowering takes them to LLVM-dialect form. Users of v1 compose
framework-owned dialects; they do not define dialects (D-006).

## The K contract

No kernel dialect is "done" until it ships all of K1–K7 (SPEC §3, D-007).
Partial dialects live on branches, not main.

| Clause | Obligation |
|---|---|
| K1 | Definition: ops, types, attributes, documented invariants, a verifier enforcing them. IRDL definitions embedded in the framework, loaded at context startup; invariants beyond IRDL's constraint language are enforced by a frankish verification pass that runs before any execution or lowering. |
| K2 | Eval: every op implements the Eval interface. The derived interpreter over K2 is the dialect's reference semantics (D-008). |
| K3 | Lowerings: at least one lowering to strictly lower dialects, with named strategy variants where the design calls for them (mem: arena \| rc). Lowerings preserve locations (§6.5). |
| K4 | Runtime component: whatever the lowering needs at run time ships in `frk-rt` behind a documented C ABI, freestanding-first. |
| K5 | Goldens: a corpus exercising every op and every lowering strategy, green under the differential law (L3). |
| K6 | Docs: semantics, lowering contracts, interaction-matrix rows, portability tier impact. |
| K7 | Ledger: every design fork encountered gets a D-entry. |

Verifier and goldens land first — law L1. The verifier is the spec; the
implementation is fungible.

## IRDL runtime loading, and nothing else

Registration is governed by D-031, an entry the human struck into the ledger
by superseding D-030: kernel dialects register via **IRDL runtime loading
only**. There is no C++ ODS shim anywhere in v1; the build stays pure
Rust/melior, and the design bends instead. No kernel op may require traits —
no custom terminators, no successors, no trait-relaxed regions, because
LLVM-22 IRDL cannot declare them.

The visible consequence: `frk_adt` has no region-based `match` op. The
dialect is pure value ops; multiway dispatch rides upstream `cf.switch`, and
surface `match` is compiled by the Maranget decision-tree pass straight to
dispatch IR ([frk_adt](adt.md)).

Registration itself is one function in `crates/frk-dialects/src/lib.rs`. All
seven IRDL sources load as a single module, because `frk_closure`'s IRDL
references `@frk_adt::@product` and IRDL symbol refs resolve only within the
module being loaded:

```rust
pub fn register(context: &Context) -> Result<(), RegisterError> {
    // One combined module: frk_closure's IRDL references
    // @frk_adt::@product, and IRDL symbol refs resolve only within the
    // module being loaded.
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        adt::IRDL, closure::IRDL, mem::IRDL, str_dialect::IRDL,
        dyn_dialect::IRDL, bstr::IRDL, ctl::IRDL
    );
    register_one(context, &combined, "frk kernel dialects")
}
```

The interpreter side is symmetric: `register_eval` plugs each dialect's K2
evaluators into a fresh `Interp` — the hook every harness runner calls right
after `Interp::new`.

## The verification pass (K1, second half)

IRDL enforces shape: operand and result base types, attribute kinds, arity.
What it cannot say — variant indices in range, `extract`'s result equals the
named field's type, a closure callee's signature equals captures ++ params —
is enforced by `frk_dialects::verify`, a semantic pass that walks every op
recursively and dispatches on the `frk_*.` name prefix. Runners call it right
after MLIR's own verifier and before any execution or lowering. Failures are
`Finding`s carrying the offending op printed in generic form. This is K1's
"verifier enforcing invariants" hosted in a pass, not in C++ (D-031).

## Packed surfaces: no variadics

D-036 hardened D-031 with a proven ceiling: LLVM-22 IRDL constraint variables
bind once per op instance, so every element of a variadic group unifies to
one type. Heterogeneous variadics are inexpressible — the proof was
`make_sum(i64, i1)` rejected at parse with "expected 'i64' but got 'i1'",
which meant the earlier variadic op surface had never supported mixed-type
fields. Filed as a first-rank finding; the response is explicit packing:

- `frk_adt.make_product` was replaced by `product_new()` +
  `product_snoc(product, value)` chains; `make_sum` takes one payload operand
  of the variant's product type.
- `frk_closure.make` takes its captures as one env product;
  `frk_closure.apply` takes one args product and yields exactly one result.

The entry's rationale for those surfaces: at ≤2 operands and ≤1 result per
op, every IRDL variable sits in one position — sound by construction. The
durable law for every later dialect is the general form: no variadic
operand/result groups, fixed arities only, and one constraint variable per
independently-typed position. Packing chains are honest ANF-style kernel IR
that frontends produce mechanically. The one op with two results,
`frk_dyn.table_next`, stays inside the ceiling because every operand and
result is the parameter-free `!frk_dyn.dyn` — a reused IRDL variable unifies
values, and there is only one value to unify.

## The roster

Seven dialects are registered today. Each entered the library because a
specimen forced it — the admission rule (L5) works at the dialect level too.

| Dialect | Types | Role | Forced at | Anchor rulings |
|---|---|---|---|---|
| `frk_adt` | `sum<...>`, `product<[...]>` | Sums, products, tuples; pure value ops; Maranget decision trees for `match` | M3 (ml_core queued) | D-025, D-031, D-032, D-034, D-036 |
| `frk_closure` | `fn<[p...],[r]>` | First-class functions: `make`/`apply`, env-struct + thunk lowering | M4 (church encoding) | D-035, D-036, D-037 |
| `frk_mem` | `box<T>`, `arr<T>` | Allocation surface; the memory strategy is a lowering parameter | M7 (boxes; arrays at M9) | D-041, D-049, D-057 |
| `frk_str` | `str` | Immutable UTF-16 strings, JS code-unit semantics | M9 (TS-0) | D-049, D-050 |
| `frk_bstr` | `str` | Interned 8-bit byte strings, Lua semantics | M11 (femto_lua) | D-052, D-056, D-058 |
| `frk_dyn` | `dyn` | Fat tagged values, dynamic tables | M10 contract, M11 lowering | D-051, D-054, D-056 |
| `frk_ctl` | — (ops only) | Escape continuations: prompt/abort/pending, result-passing native lowering | M15 (r7rs_core) | D-011, D-060, D-061 |

`frk.contract` and `frk.stage` remain in the SPEC §4 target set, unforced;
they arrive when a specimen carries their idiom, not before (L9).

## What holds it together

Every dialect's three semantics — interpreter, JIT, AOT — are held equal by
the harness, not by review. The suite is 38 test blocks; `make diff` reports
`diff[interp,jit,jit-rc,ocaml,node,lua,scheme,repl]: 77 case(s), 0 divergent`;
the grid runs the full corpus across
{x86_64, aarch64, riscv64, wasm32-wasi}-musl × {arena, rc} with an s390x
big-endian canary, green. A dialect whose lowering drifts from its K2
semantics turns a runner red the same day.

Design forks are settled in [the ledger](../method/ledger.md); the
cross-runner comparison rules live in
[the differential law](../method/differential.md). The chapters that follow
take the dialects one at a time.
