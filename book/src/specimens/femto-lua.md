# femto_lua and the Pack Convention

`femto_lua` is the Lua 5.1 specimen, held against `lua5.1` (PUC-Rio, pinned
at 5.1.5 — *5.1.5 is the spec*, D-052). It brought the dynamic runtime to
the kernel: fat values, tables and metatables, garbage collection, and — at
v0.2 — the **pack calling convention** that dissolved Lua's arity fence
with a single kernel widening. Its 12-case corpus lives under
`goldens/lua/`.

## All-dyn emission

A Lua value's type is a runtime property, so every femto_lua value is a
`!frk_dyn.dyn` — the [two-word fat value](../dialects/dyn.md) `{tag,
payload}` (D-051). Locals are `box<dyn>` cells (Lua upvalues are mutable),
`_G` is a `dyn` table threaded everywhere, and numbers print via the
`%.14g` emulation with a proven half-even tie-rounding contract (D-055) so
they match PUC Lua to the digit.

## Protocols are synthesized IR, not kernel ops

A defining choice (D-056.2): Lua's *protocols* — truthiness, `tostring`,
`print`, equality, concatenation, length, indexing, `setmetatable`,
`pairs`/`ipairs`/`next` — are **synthesized IR helper functions the
frontend emits**, not kernel operations. `__lua_truthy(v)` is a `func.func`
that switches on the value's tag; `__lua_print` dispatches to the runtime's
typed printers. This keeps the kernel dialects language-agnostic: Lua's
semantics live in Lua's frontend, expressed in kernel ops, rather than
leaking Lua-specific ops into the shared library. The kernel provides the
*substrate* (dyn, tables, dispatch); the specimen composes the *policy*.

The corpus exercises the real thing — tables, metatables, iteration:

```lua
-- pairs order is implementation-defined (canon rule): aggregate only.
local t = { alpha = 3, beta = 5, gamma = 7 }
local sum, names = 0, 0
for k, v in pairs(t) do sum = sum + v; names = names + #k end
print(sum)
print(names)
```

The comment is load-bearing. Lua leaves `pairs` order unspecified, and so
does frankish's canon: the interpreter iterates insertion order, the native
path iterates hash-slot order, and the corpus is written to *aggregate*
(sum the values, count the key lengths) so both orders produce identical
bytes. This is the differential law's "close the freedom by construction"
move made concrete — `ipairs`, being ordered, is safe to print in sequence.

## The pack convention (D-058)

femto_lua's headline kernel contribution is a calling convention. Lua
functions have flexible arity — extra arguments are dropped, missing ones
become `nil`, and functions return multiple values — none of which a
fixed-arity signature can express. At v0.2 every Lua function adopted the
**uniform pack convention**:

```
fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>
```

One argument pack in, one values pack out. Parameters are read through a
bounds-checked nil-filling helper (`__lua_arg`), so missing arguments
become `nil` and extras drop unread — Lua's real arity semantics, for
free. Multiple return values become surface syntax rather than plumbing.
The generic `for` loop runs the real `(f, s, ctrl)` iterator protocol over
the pack.

What makes this a *kernel* story rather than a frontend one is the price
the kernel paid: **one widening**. The pack's elements are two-slot
`arr<dyn>` entries (stride-addressed), which is exactly the memory layout
the M12 collector's `ARRAY_DYN` tracer already walked. So the garbage
collector absorbed argument packs with **zero new GC code** — the
convention change touched neither closures, adts, mem, the dyn core, nor
the collector. That a calling-convention change this deep cost one memory
layout widening and nothing else is the extraction thesis working: the
kernel was already general enough.

## Byte strings and the second string dialect

Lua strings are 8-bit-clean byte strings, interned at creation, with
pointer-equality comparison (D-052). This forced `frk_bstr` — a micro
byte-string dialect (D-056) with intern tables in *both* runtime twins,
distinct from TS-0's UTF-16 `frk_str`. Two languages, two genuinely
different string semantics, two dialects — see [the string
chapter](../dialects/str-bstr.md).

At v0.3 the arity story completed: the **explist adjustment engine**
implements Lua's rule (non-final expressions truncate to one value,
the final call or `...` expands) once, and returns, destructuring,
call arguments, constructor tails, and the generic-for explist all
consume it — which makes varargs, multi-expression RHS, and explicit
`(f, s, ctrl)` iterator triples one feature, not three. `__newindex`
joined `__index` as an IR intrinsic (existing keys raw-assign; the
table form re-enters settable as a tail call, so metatable chains
ride the trampoline). The corpus grew to 18 cases — and forced two
kernel ownership findings (a created-pack borrow-locality gap and an
unsound sole-use retain elision), both caught as jit-rc segfaults by
the differential law before any commit.

femto_lua shipped v0.3. Its legacy: the `frk_dyn` fat-value core, `frk_bstr`,
the table runtime, the real garbage collector (its debugging is [its own
chapter](../memory/war-stories.md)), and the pack convention — which the
tail-call law would later name as the seed of the uniform-signature
convention that generalizes native `musttail`.
