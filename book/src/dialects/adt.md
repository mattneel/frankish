# frk_adt — Algebraic Data

`frk_adt` is sums, products, and tuples as parametric types with pure value
ops (SPEC §4.1 as amended by D-031). The dialect namespace is `frk_adt`;
SPEC prose writes "frk.adt" for the same thing. It was the first kernel
dialect (M3) and the first customer of every registration ruling: it lost its
region-based `match` op to D-031 and its variadic constructors to D-036.

## Type encoding

A sum's single type parameter is an array of variants, each an array of field
types; a product's parameter is its field-type array. Tuples ARE products.

```mlir
!frk_adt.sum<[[], [i64]]>        // Option<i64>: None | Some(i64)
!frk_adt.product<[i64, i64]>     // a pair
!frk_adt.product<[]>             // unit — also the empty payload
```

## Op surface

Packed per D-036 — no variadics. Heterogeneous payloads flow through explicit
product chains.

| Op | Operands | Results | Semantics |
|---|---|---|---|
| `product_new` | — | `!frk_adt.product<[]>` | The empty product. |
| `product_snoc` | product, value | product with one more field | Append one field; the result type carries the grown field list. |
| `make_sum` {variant} | payload product | `!frk_adt.sum<...>` | Injects the payload as the named variant. |
| `tag_of` | sum | `i64` | The variant index. |
| `extract` {variant, field} | sum | the field's type | Reads one field of one variant. Wrong variant: interp traps; lowered code is unspecified — see the guard law below. |
| `get` {field} | product | the field's type | Reads one product field. |

The IRDL definition enforces shape — base types, attribute kinds, arity:

```mlir
irdl.operation @extract {
  %sum = irdl.base @frk_adt::@sum
  %any = irdl.any
  %vidx = irdl.base "#builtin.integer"
  %fidx = irdl.base "#builtin.integer"
  irdl.operands(sum: %sum)
  irdl.results(value: %any)
  irdl.attributes { "variant" = %vidx, "field" = %fidx }
}
```

Two IRDL landmines learned against mlir-opt 22.1.8 are recorded in the module
doc: a reused constraint variable unifies *values* (one `%idx` shared by
`variant` and `field` would demand variant == field, so every
independently-valued attribute gets its own variable), and `irdl.is i64`
means "equals the type i64" while `irdl.base "#builtin.integer"` means "any
integer attribute".

What IRDL cannot say, the frankish verification pass enforces before any
execution or lowering (K1 second half): indices in range, `product_snoc`'s
result = operand fields + the appended type, `make_sum`'s payload shape =
the variant's shape, `extract`/`get` result types = the named field's type.

## Reference semantics (K2)

The interpreter represents both shapes as one value: `Value::Adt { tag,
fields }`; products are tag-0 adts. Wrong-variant extraction is a
deterministic trap:

```rust
let (tag, fields) = value.as_adt()?;
if tag != variant {
    return Err(EvalError::Trap(format!(
        "frk_adt.extract: value holds variant {tag}, extract names variant {variant}"
    )));
}
```

The trap is a compiler-bug detector, not a program outcome: the decision-tree
pass only emits extracts guarded by tag dispatch, so reaching this trap means
something upstream of the interpreter emitted an unguarded extract.

## The decision-tree compiler

There is deliberately no `match` op (D-031). Surface `match` compiles via
Maranget's algorithm (*Compiling Pattern Matching to Good Decision Trees*,
D-025) in `adt_dtree.rs`: a frontend hands over a pattern matrix, the pass
hands back a tree whose nodes are exactly the ops the dialect has.

The v0 pattern language (D-034): variant constructors, product destructuring,
integer literals, wildcards, bindings. Columns are typed — each carries an
*occurrence* (a path of `SumField`/`ProductField` accesses from the
scrutinee) and a `ValueType`. The column-choice heuristic is Maranget's
baseline: leftmost column where the first row holds a constructor. Products
specialize in place without emitting a node — they are single-constructor
types.

Tree nodes:

| Node | Meaning | Emission |
|---|---|---|
| `SwitchTag` | Dispatch on a sum tag; `default` is present exactly when the cases don't cover every variant | `frk_adt.tag_of` + `cf.switch` |
| `SwitchInt` | Dispatch on integer literals, always with a default | `cf.switch` on the value |
| `Leaf` | Arm selected; bindings map each bound name to its occurrence | `extract`/`get` chains + branch to merge |
| `Fail` | No row matches — the match was inexhaustive and this path is its counterexample | never emitted; see below |

Exhaustiveness and usefulness fall out of the tree: a reachable `Fail` yields
a `Witness` (the branch constraints on the path to it), and an arm appearing
in no leaf is redundant. This analysis is complete for the v0 pattern
language and sits behind the `PatternAnalysis` trait, so
rustc_pattern_analysis can slot in when the language outgrows it
(or-patterns, ranges, guards — D-034's named deferral).

Trees golden as literal renderings (byte-exact under the same L2 duties).
For `Option<i64>` matched by `Some(x) => …, _ => …`, `compile` produces:

```text
switch-tag $
  case v1:
    leaf arm=0 x=$.v1f0
  default:
    leaf arm=1
```

`$` is the scrutinee; `$.v1f0` reads variant 1, field 0.

## Emission: matrix → dispatch IR

`dtree_emit.rs` turns trees into IR. It was built inside the ml_core frontend
and promoted into `frk-dialects` at M6 — the extraction loop working as
designed: the specimen built it, the promotion pass moved it down to where
every match-bearing frontend can reach it.

The component is frontend-agnostic because the kernel types carry everything:
occurrence typing walks the scrutinee's `!frk_adt` type through
`decode_sum`/`decode_product`, so the only thing a frontend supplies is
arm-body emission — a callback receiving the arm index and the pattern
bindings as SSA values. Dispatch shapes, all D-031-honest (no region ops):

- `SwitchTag` on a sum: `frk_adt.tag_of` + `cf.switch`. On an `i1`
  occurrence (frontends encode bool as a two-variant sum in the matrix):
  `arith.select` to i64 + `cf.switch`.
- `SwitchInt`: `cf.switch` on the value directly.
- `cf.switch` always needs a default successor; with complete tag coverage
  the last case doubles as it, and a zero-explicit-case switch
  (single-variant dispatch) emits the sole subtree inline — `cf.switch`
  cannot carry an empty case list.
- Leaves recompute occurrence values per branch (`extract`/`get` are pure
  ops; no sharing needed) and branch to the merge block with the arm's value.

`Fail` reaching emission is a caller bug, and the layer says so:

```rust
DecisionTree::Fail => Err(
    "FAIL reached dispatch emission — the caller must reject \
     inexhaustive matches before emitting".to_string(),
),
```

The ml_core frontend obeys: it rejects inexhaustive matches with the witness
and rejects redundant arms as errors — stricter than OCaml's warning, because
stricter is deterministic (D-038.4).

## Lowering (K3)

The kernel lowering maps sums to `!llvm.struct<(i64 tag, i64 × K)>` where K
is the max variant slot count, products to `!llvm.struct<(i64 × S)>`; narrow
integer fields pass through uniform i64 slots via `extui`/`trunci` (D-032,
slot model widened by D-037). It is obviously-correct wasteful layout by
ruling — niche/tag-packing is a separate, separately-goldened later pass
(D-025).

D-032 also fixes the guard law: wrong-variant `extract` is unspecified in
lowered code while the interpreter traps, so extracts must be tag-guarded —
exactly the decision-tree output shape — and an unguarded extract is
inadmissible as a golden.

## Rulings

| Entry | Ruling |
|---|---|
| D-025 | Maranget decision trees; niche/tag-packing is a separate goldened pass. |
| D-031 | IRDL-only registration; `match` de-regioned to `tag_of` + `cf.switch` + guarded `extract`. |
| D-032 | Struct + tag representation; tag-guard law; wasteful-first layout. |
| D-034 | v0 pattern language; tree-derived exhaustiveness behind `PatternAnalysis`; literal tree goldens. |
| D-036 | Packed surface: `product_new`/`product_snoc` chains, single-payload `make_sum`. |
| D-038 | Recursive ADTs rejected at declaration (structural encoding cannot spell them); match redundancy is a compile error. |

See [the ledger](../method/ledger.md) for full texts and revisit conditions.
