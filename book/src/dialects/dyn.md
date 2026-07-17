# frk_dyn — Dynamic Values and Tables

`frk_dyn` is the uniform-value dialect (SPEC §4.5): one type,
`!frk_dyn.dyn`, that can hold any of a closed set of runtime kinds, plus the
table ops femto_lua forced. The contract landed at M10 (D-051), the lowering
at M11 with the femto_lua implementation (D-054) — dyn goldens rode
`runners=interp` fences in between, the staged-bring-up mechanism built for
exactly this (D-033).

## Fat values

D-051 settled the tagging fork: v0 is **fat values** — a two-slot
`{tag: i64, payload: i64}` pair, riding the exact machinery closures proved
(two-slot kinds, word-verbatim copies, `ptrtoint`/`bitcast` payload
adaptation per tag). NaN-boxing and pointer tagging are representation
optimizations behind the same dialect surface; the K contract makes
representation a lowering detail, so the swap is a later profile knob,
decided on measurement, not aesthetics.

Fat values won v0 on four grounds, quoted from the entry: no bit games on
the big-endian canary, no 48-bit pointer assumptions (wasm32 is 32-bit;
riscv64 sv48+ looms), trivially correct interp semantics
(`Value::Dyn(tag, Box<Value>)`), and honest-first debuggability.

The tag space is a closed enum per profile — femto_lua's six, checked by the
verifier (`tag {tag} outside the closed v0 space 0..6 (D-051)`):

| Tag | Constant | Kind |
|---|---|---|
| 0 | `TAG_NIL` | nil |
| 1 | `TAG_BOOL` | boolean |
| 2 | `TAG_NUM` | number (f64 bits in the payload word) |
| 3 | `TAG_STR` | interned byte string (canonical pointer) |
| 4 | `TAG_TABLE` | table (shell pointer) |
| 5 | `TAG_FUN` | function (boxed closure pointer) |

D-055 records the standing warning: TS-1 unions are coming for those tags —
the widening is a named revisit, not a surprise.

## Op surface

| Op | Operands | Results | Semantics |
|---|---|---|---|
| `wrap` {tag} | value | dyn | Injects a typed value under the tag. |
| `unwrap` {tag} | dyn | value | Projects; **traps on tag mismatch** (total semantics, D-029). |
| `tag_of` | dyn | i64 | Reads the tag word. |
| `payload_word` | dyn | i64 | The raw payload word, for identity comparison only. |
| `table_new` | — | dyn | Fresh empty table (tag 4). |
| `raw_get` | table, key | dyn | Raw read; absent key yields nil. No metamethods. |
| `raw_set` | table, key, value | — | Raw write; assigning nil deletes; nil key traps. |
| `table_len` | table | i64 | The `#` border: largest n with t[1..n] all present. |
| `set_meta` | table, meta | — | Stores the metatable (nil clears). |
| `get_meta` | table | dyn | Reads the metatable, nil if unset. |
| `table_next` | table, key | next_key, next_value | Iteration step for `pairs`/`next`. |

Everything is dyn-in, dyn-out — the ops are the raw substrate. The metatable
*protocol* is deliberately not kernel: `__index` (table and function forms)
is a synthesized IR helper the frontend emits once per module
(`@__lua_index` walks the chain, dispatching function forms through
`frk_closure.apply`) — ordinary IR that runs identically on interp, JIT, and
all five AOT triples, zero rt-callback machinery (D-056.2). Same pattern for
`@__lua_print`, `@__lua_truthy`, `@__lua_eq`, `@__lua_concat`.

`table_next` is the **first two-result kernel op**. It stays inside the
D-036 no-variadics ceiling because every operand and result is the
parameter-free `!frk_dyn.dyn`: a reused IRDL constraint variable unifies
values, and here there is exactly one value to unify.

## wrap, unwrap, and the native trap

The lowering (`kernel_lower.rs`) builds a dyn as an `!llvm.struct<(i64,
i64)>`: insert the constant tag at field 0, the payload word at field 1. The
payload word is produced by kind: narrow ints `extui`, f64 `arith.bitcast`,
pointers `llvm.ptrtoint` — and multi-word payloads (closures, adt structs)
heap-box through the strategy allocator first, storing the aggregate and
using the box pointer as the word.

`unwrap` extracts the tag and makes a straight-line call:

```rust
// Straight-line native tag check (D-054): the rt aborts on
// mismatch — no CFG surgery mid-rewrite. In-process JIT
// corpus law keeps mismatches out of jit goldens; the trap
// contract is verified at interp (semantics) and AOT
// (subprocess) levels.
rewriter.insert(direct_call_void(
    context, "frk_rt_dyn_check", &[actual, expected], location,
)?);
```

`frk_rt_dyn_check(actual, expected)` prints
`frk: dyn tag mismatch: expected {expected}, got {actual} (D-051)` and
aborts the process. The interpreter's version is a located trap
(`dyn tag mismatch: ... at <loc>`) carrying the op's threaded source
location — the §6.5 discipline applied from the dialect's birth. The
division of labor is explicit: semantics verified at interp, the native
abort path verified where the runner is a subprocess (AOT), and in-process
JIT goldens keep mismatches out by corpus law.

`payload_word` exists for `__lua_eq`'s table/function arm: identity
comparison. The numeric value is meaningless outside equality — reference
types yield a stable per-object address, everything else the address of the
payload cell.

## Tables

Tables are the runtime's largest component, present in both twins (Rust
`frk-rt/src/lib.rs`, C `frk-rt/c/frk_rt.c`; the grid holds them
behaviorally equal).

**The shell.** A table object is a 4-word shell `[cap, count, slots, meta]`,
allocated through the *strategy* (32 bytes, `LAYOUT_TABLE_SHELL`), then
initialized by `frk_rt_table_init` — so under rc, dying tables release like
any other object, and the tracer knows the shell shape. The slot array is a
separate allocation the shell points to, size-prefixed so the collector can
free it with the shell (the D-056 internals debt, paid at the
layout-descriptor rung).

**The hash.** Pure hash, no array part: slots are
`{state, ktag, kpay, vtag, vpay}` — five i64s. state ∈ {0 empty, 1 full,
2 tombstone}. Probing is linear over a power-of-two capacity; the hash is a
splitmix-style scramble over both key words:

```rust
fn table_hash(ktag: i64, kpay: i64) -> u64 {
    let mut h = (ktag as u64).wrapping_mul(0x9E3779B97F4A7C15) ^ (kpay as u64);
    h ^= h >> 30;
    h = h.wrapping_mul(0xBF58476D1CE4E5B9);
    h ^= h >> 27;
    h
}
```

Growth doubles capacity (from 8) when count reaches 70% of capacity
(`fields[1] * 10 >= fields[0] * 7`), reinserting live slots and skipping
tombstones. Assigning nil tombstones the slot (Lua: nil deletes); deleting
an absent key is a no-op. Number keys hash by f64 bits — 1.0 and 1 are the
same f64, and Lua agrees; NaN keys are fenced (Lua errors; the corpus stays
clear).

**The out-pointer ABI (D-056.3).** All table entry points take and return
plain i64 words; dyn *results* return through a caller-provided out pointer:
`frk_rt_table_raw_get(shell, ktag, kpay, out)` writes `out[0]=tag,
out[1]=payload`. The lowering allocas two i64s, calls, and loads the pair
back into a dyn struct. Struct-return conventions across five triples are
exactly the ABI risk the wasm `signature_mismatch` incident taught the
project to refuse — an out-parameter is boring on every target. The same
recipe was later judged "Tier-0 strongest" and reused for the ctl prompt's
result slot (D-061).

`set_meta`/`get_meta` do not call the runtime at all: meta is shell word 3,
and both lower to inline loads/stores.

## Iteration and the order question

`table_next` powers `pairs`: a nil key asks for the first entry; otherwise
the entry after the given key; a nil key result signals the end. The rt
scans slots from the found position + 1 and writes four words
`{ktag, kpay, vtag, vpay}` through the out pointer.

Interp and native disagree on order, legally:

```rust
/// Iteration for pairs/next (D-058): nil key → first entry; else the
/// entry AFTER the given key. Order here is INSERTION order; the
/// native path iterates slot order — both are legal Lua (pairs order
/// is implementation-defined), and the canon rule (D-058) keeps
/// corpus output order-independent.
```

The interpreter's `TableData` keeps an entries vector in insertion order;
the native table iterates hash-slot order. Lua 5.1 defines `pairs` order as
implementation-defined, so both are conforming — but the differential law
compares bytes, so D-058 adds a corpus canon rule: `pairs` loops may print
only order-independent aggregates; ordered output uses `ipairs`. The
legality lives in the semantics; the comparability lives in the canon. Both
are written down, which is the point.

## rc interplay

A dyn is two words, but only tags 4 and 5 carry a managed pointer. Stores of
dyn values under rc use `RetainKind::DynPair`: the lowering emits a
branch-free mask — `tag ∈ {4, 5} ? payload-as-ptr : null` via two `cmpi`,
an `ori`, and a `select` — and calls `frk_rt_rc_retain` on the result
(retain of null is a no-op). Table stores own both key and value (D-057),
retaining both masked pairs before the set and releasing the overwritten
value after. In the D-057 layout language a dyn field codes as `2` — a
tag word whose successor word is traced when the tag is table or fun — which
is what makes every Lua local (a `box<!frk_dyn.dyn>`) reachable by the
tracer.

## Rulings

## Interfaces: the itab pair (TS-2)

D-026 reserved Go-style itabs for structural interface dispatch; TS-2
cashed the reservation (D-075). An interface value is `!frk_dyn.iface`
— an opaque two-slot `{obj, itab}` pair, the third two-word citizen
after closures and fat dyns. Two ops:
`iface_make(box){methods = [@C__m…]}` converts at a statically known
site (sealed world: the symbol list is an attribute), and
`iface_call(iface, args){method = k}` dispatches.

The pair is the cleanest demonstration yet of the K-contract's
representation freedom: the **interpreter** evaluates `iface_make` as
a *dictionary* — a product of bound closures, one per method, each
capturing the object — while **native** builds a real itab (a table
of method addresses; entries point at the class methods directly,
because the call site knows the signature from the interface
definition, exactly as Go does) and dispatches with one load and an
indirect call. Same ops, two representations, and the differential
matrix arbitrates every run. v0 tables materialize on the stack at
the conversion site (hoisted to the function entry, loop-safe); the
static-global cache is a later lowering upgrade behind the same
surface, sound because v0 interface values are *borrows* —
parameter-passing only, by fence.

| Entry | Ruling |
|---|---|
| D-051 | Fat values; the six-tag closed space; wrap/unwrap/tag_of surface; trap-on-mismatch totality. |
| D-054 | dyn K3 as an M11 exit bar; the straight-line native check discipline. |
| D-056 | Tables as raw kernel ops; metatable protocol as synthesized IR; out-pointer ABI; f64-bits key hashing. |
| D-057 | Table stores own keys and values; dyn-pair retain masking; shell + slot-array tracing and freeing. |
| D-058 | `table_next` + the iteration-order canon rule. |
| D-026 | Itab-style structural dispatch reserved at founding; executed by D-075 at TS-2. |
| D-075 | The iface pair; dictionary-vs-itab dual representation; borrows-only v0; arrows onto frk_closure. |

For how femto_lua drives this surface — the pack convention, `__index`, the
`%.14g` print canon — see
[femto_lua and the Pack Convention](../specimens/femto-lua.md).
