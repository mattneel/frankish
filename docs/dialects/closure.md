# frk.closure — first-class functions (K6 page)

Dialect namespace `frk_closure`. Contract status: K1–K7 complete at M4.
Rulings: D-031 (trait-free), D-035 (strategy + fences), D-036 (packed
surfaces), D-037 (kernel lowering + slot model).

## Type

    !frk_closure.fn<[param types], [result type]>

The call signature only — captures are existential, which is the point.
Exactly one result in v1 (D-036; every v1 specimen is single-valued).

## Ops (packed, D-036)

    %c = "frk_closure.make"(%env) {callee = @f} : (product) -> fn
    %r = "frk_closure.apply"(%c, %args)         : (fn, product) -> <result>

`env` and `args` are `!frk_adt.product` packs. Calling convention: the
lifted callee takes the captures first, then the parameters —
`@f : (captures..., params...) -> result`. Captures are taken BY VALUE
at make time (D-035); by-ref capture becomes meaningful when frk.mem
introduces locations (M7). By-value capture cannot tie a recursive
knot — self-reference arrives with mutable cells, also M7.

## Semantics (K2 — reference: the derived interpreter)

Runtime value: `Value::Closure { callee, captures }`; apply re-enters
the interpreter's function machinery (which owns the D-029 call-depth
guard) with `captures ++ unpacked args`. Static verification is
two-layered as always: IRDL shape (closure/product bases, symbol-ref
attribute kind) plus the frk semantic pass, which resolves the callee
in the module symbol table and checks the full convention.

## Lowering contract (K3, D-035/D-037)

One strategy in v1, inside the single `lower-frk-kernel` pass:

    fn value    → !llvm.struct<(ptr thunk, ptr env)>
    make        → frk_rt_alloc(env slots × 8) + slot stores +
                  {&thunk, env} — thunk address via func.constant +
                  one unrealized_conversion_cast (folded post-FuncToLLVM)
    apply       → extractvalue ×2 + arg-pack unpack + indirect llvm.call
    thunk       → synthesized per make-site: reloads captures, calls the
                  lifted callee

Inside adt slots a closure occupies two i64 words (ptrtoint in,
inttoptr out — D-037). Fences: capture/param/result types are integers
≤64 and closure types; adt values at closure boundaries wait for the
shared layout oracle (M7). Defunctionalization — the no-heap
whole-program strategy — is deferred until a Tier-0 profile demands it
(D-035).

## Runtime component (K4 — the first real one)

`frk_rt_alloc(bytes) -> ptr` in frk-rt: extern "C", 8-aligned,
**leaks by design** in v0 (D-035) — closure environments live for the
process. The frk.mem discipline (arena/rc, M7) replaces the
implementation behind the same symbol; callers never change. The JIT
runner registers the symbol from the linked frk-rt rlib; AOT (M7)
links the staticlib.

## Interaction matrix rows (SPEC §5)

- **closure × mem/arena** (pre-solved, SPEC seed row): escape analysis
  promotes or rejects escapees at verify time — activates at M7; today
  every env is heap(-leak) allocated, so escape is always safe.
- **closure × mem/gc**: captures as roots via shadow-stack maps —
  M7+/Tier 2.
- **closure × adt** (costed, deferred): closures inside adt fields work
  (two-slot encoding); adt values as captures/params wait for the
  shared layout oracle (D-035 fence).

## Portability tier impact (SPEC §10)

Tier 0 with an asterisk: the lowering needs frk_rt_alloc, which is
freestanding-implementable (bump over a static region), so no libc
dependency is implied — but unbounded make-sites leak. Honest reading:
closures are Tier 0 for arena-shaped workloads once frk.mem lands; the
v0 leak is a development-tier stance, not a shipping one.

## Corpus

`goldens/closure/*` — church (λf.λx. f (f x) applied to inc and 40:
closure-capturing-closure, upward escape) and counter_fold (a +3
closure folded through scf.for; the stateful counter waits for
frk.mem). Green under interp AND jit per L3 on every `make test`.
