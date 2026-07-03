# frk.adt — algebraic data types (K6 page)

Dialect namespace `frk_adt`. Contract status: K1–K7 complete at M3
(this page is K6). Rulings: D-025 (decision trees), D-031 (pure-IRDL,
de-regioned match), D-032 (lowering), D-034 (dtree pass scope).

## Types

    !frk_adt.sum<[[t...], [t...], ...]>   variants, each a field list
    !frk_adt.product<[t...]>              tuples ARE products

Parameters are nested type-array attributes. IRDL constrains kinds and
bases; deep shape invariants live in the frk verification pass.

## Ops (pure value ops — no regions, no terminators, no variadics;
## D-031 + D-036)

    %e = "frk_adt.product_new"()                       : () -> product<[]>
    %p = "frk_adt.product_snoc"(%e, %v)                : (product, T) -> product+T
    %s = "frk_adt.make_sum"(%p) {variant = V : i64}    : (payload product) -> sum
    %t = "frk_adt.tag_of"(%s)                          : (sum) -> i64
    %v = "frk_adt.extract"(%s) {variant, field}        : (sum) -> <field type>
    %v = "frk_adt.get"(%p) {field}                     : (product) -> <field type>

Heterogeneous payloads flow through explicit product chains — IRDL-22
unifies every element of a variadic group to one type, so variadic op
surfaces cannot carry mixed-type fields at all (D-036; proven by
make_sum(i64, i1) being parse-rejected under the original design).

There is deliberately no `match` op: dispatch is `tag_of` +
`cf.switch` + per-arm tag-guarded `extract`, produced from a pattern
matrix by the decision-tree pass (`frk_dialects::adt_dtree`, Maranget
per D-025; matrix→tree goldens in its test suite; IR emission arrives
with ml_core at M5, D-034).

## Semantics (K2 — reference: the derived interpreter)

Runtime value: `Value::Adt { tag, fields }`; products are tag-0 adts.
`extract` against a value holding a different variant **traps**
deterministically (D-029 family). Static verification is two-layered:
IRDL-generated verifiers (arity, operand/result bases, attribute
kinds) plus the frk semantic pass (index ranges, extract/get result
type = the named field's type, make arity/types vs declared shape) —
run by every harness runner before execution or lowering.

## Lowering contract (K3, D-032)

One strategy in v1, `lower-frk-adt` — an external MLIR pass, first in
the shared pipeline (stage 01 in every `emit --stages` dump):

    sum      → !llvm.struct<(i64 tag, i64 × max-field-count)>
    product  → !llvm.struct<(i64 × field-count)>

Uniform i64 slots; narrow integer fields `arith.extui` in and
`arith.trunci` out. Deliberately wasteful and obviously correct —
niche/tag-packing is a later, separately-goldened pass (D-025).

Fences: field types must be builtin integers ≤ 64 bits (nested adts
and non-integer payloads wait for the memory axis, frk.mem/M7).
Wrong-variant `extract` is *unspecified* in lowered code while the
interpreter traps — therefore extracts must be tag-guarded (the
decision-tree output shape) and an unguarded extract is inadmissible
as a golden.

## Runtime component (K4)

None in v1 — satisfied vacuously. Values are flat, by-value LLVM
structs; construction and projection allocate nothing and call
nothing, so no frk-rt symbol exists for this dialect. Revisit with
frk.mem (M7): heap payloads, recursive types, and boxed
representations will bring the first real K4 surface.

## Interaction matrix rows (SPEC §5)

- **adt × dyn** (pre-solved, future): boxed sums share the dyn tag
  plan; niche optimization is disabled on boxed representations.
  Activates when frk.dyn exists (M10).
- **adt × mem** (costed, future): D-032's flat layout is the no-heap
  representation; recursive/boxed layouts are frk.mem work, not an
  adt-side change (the ops don't move, the type mapping does).

## Portability tier impact (SPEC §10)

Tier 0 clean: no runtime, no libc, no allocation — every LLVM triple
that can pass integers in structs can run lowered adt code. Nothing in
this dialect narrows any tier.

## Corpus

`goldens/adt/*` — construction, projection, tag dispatch through
`cf.switch`, multi-variant/multi-field cases; green under every
registered runner (interp + jit) per L3 on every `make test`.
