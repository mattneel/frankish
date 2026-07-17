# frk_mem — Boxes, Arrays, Strategies

`frk_mem` is the allocation/ownership surface (SPEC §4.3, D-041): one set of
ops, swappable lowerings. The memory strategy — arena or rc today — is a
**lowering parameter, never IR**. The same kernel module runs under every
strategy; nothing in the program text says how it is allocated. Boxes landed
at M7 with the Tier-0 grid; arrays joined at M9 when TS-0's manifest needed
them (D-049: arrays are an allocation shape, so they live in `frk_mem`).

## Op surface

Packed and trait-free (D-031/D-036).

| Op | Operands | Results | Semantics |
|---|---|---|---|
| `box_new` | value | `!frk_mem.box<T>` | Allocate + initialize the cell. |
| `box_get` | box | `T` | Read the cell. |
| `box_set` | box, value | — | Write the cell (zero results — the mutable-cell primitive). |
| `array_new` | len : i64 | `!frk_mem.arr<T>` | Allocate; interp zero-fills. Literals lower to new + set chains — no variadics. |
| `array_get` | arr, index : i64 | `T` | Read one element. |
| `array_set` | arr, index : i64, value | — | Write one element. |
| `array_len` | arr | i64 | The stored length. |

IRDL enforces the base types; the verification pass enforces the element-type
equations (`box_new`'s T = operand type, get/set against the parameter).
Array elements are one-slot kinds or dyn (D-049/D-058); the lowering rejects
anything wider.

## Reference semantics: cells and bounds

The interpreter's box is `Value::Box` — a shared mutable cell with identity
equality; aliases observe writes. Arrays are `Value::Array`, shared and
mutable the same way (JS reference semantics, D-049). Reference semantics is
strategy-agnostic by construction: the interpreter does not know arenas or
refcounts exist.

Bounds are the D-049 discipline, and the split is deliberate:

```rust
/// Bounds discipline (D-049): OOB traps deterministically here; the
/// native path is unchecked — the corpus stays in-bounds by law.
fn index_of(value: &Value, len: usize) -> Result<usize, EvalError> {
    let index = value.as_signed()?;
    if index < 0 || index as usize >= len {
        return Err(EvalError::Trap(format!(
            "array index {index} out of bounds for length {len} (D-049)"
        )));
    }
    Ok(index as usize)
}
```

The trap carries the op's threaded source location (§6.5) — the eval wraps
it with `at {op.location()}`, and TS-0's witness golden prints
`...out of bounds... at oob.ts:4:13` (D-050.3). Out-of-bounds is *outside*
the v0 contract: the interpreter traps deterministically (D-029), native is
unchecked (UB), JS returns `undefined` which has no representation in a
pure-f64 world. Corpus law: in-bounds only. A checked profile is
`frk.contract` territory later.

## The strategy knob

```rust
pub enum Strategy { Arena, Rc }

impl Strategy {
    fn alloc_symbol(self) -> &'static str {
        match self {
            Self::Arena => "frk_rt_arena_alloc",
            Self::Rc => "frk_rt_rc_alloc",
        }
    }
}
```

`lower_kernel_pass(strategy)` builds the one kernel pass
("lower-frk-kernel") for a given strategy; the harness pipelines construct
it fresh per run. Both strategies lower `box<T>`, `arr<T>`, and closure envs
to `!llvm.ptr`; the difference is confined to the runtime ABI:

- **Arena**: `frk_rt_arena_alloc(bytes: u64) -> ptr` — bump allocation,
  process lifetime in v0, no headers, never traces. Region reset entry
  points arrive with real region inference, not before.
- **Rc**: `frk_rt_rc_alloc(bytes: u64, layout: u64) -> ptr` — a three-word
  header at `[layout @ ptr-24][size @ ptr-16][rcword @ ptr-8]` (D-041 as
  amended by D-057), plus `frk_rt_rc_retain`/`frk_rt_rc_release`. The
  `layout` word is the D-057 descriptor the lowering computes per allocation
  site from the slot kinds it already knows — the tracer's map of which
  payload words are managed pointers.

Sizes are u64 on every target: wasm32 enforces exact import signatures, and
a `size_t` runtime signature trapped at link on the first wasm grid run
(D-042). Both runtime twins take u64 and cast down.

One IR, two runtimes, held equal by force: the dev loop diffs a second JIT
runner (`jit-rc`) against the interpreter and the arena JIT, and the grid
runs the full corpus × 4 triples × both strategies. `diff[interp,jit,jit-rc,
ocaml,node,lua,scheme,repl]: 77 case(s), 0 divergent` is the current state
of that contract.

## Representation

An array is `{len: i64, data: word × len}` behind the strategy allocator.
Element addresses compute as `base + 8 + index * stride` via `ptrtoint`
arithmetic — portable across every grid triple. Box payloads store as their
lowered forms in 8-byte slots; a box occupies one slot inside adts and envs
(`SlotKind::Ptr { managed: true }`).

`managed` matters: D-049 split pointer slots into managed (boxes, arrays —
rc header present, retain legal) and unmanaged (strings — no header; a
retain would corrupt the word at ptr-8). The rc lowering retains only
managed pointers.

## Retains, transfers, and dying at the terminator

The rc policy is D-041's: a retain accompanies every new owning store of a
managed pointer, **elided when the stored value's only use is that store** —
ownership transfer, checked on SSA use counts at plan time.

Releases are the GC ladder's first rung (D-053/D-054), and they are
deliberately narrow: an allocation whose every use sits in its own block —
none escaping through a branch, call, or return — dies at that block's end.
The lowering records the block terminator as the release anchor at plan time
and, after all rewriting, inserts `frk_rt_rc_release(ptr)` immediately
before it. Terminators are cf/func ops the kernel pass never touches, so the
anchors stay valid across the rewrite. Cross-block lifetimes leak — the
documented conservative frontier the cycle collector's ladder continues
from.

One exclusion earned its comment the hard way (D-057, found by a corpus
use-after-free):

```rust
// TRANSFER-vs-RELEASE exclusion (D-057, found by the corpus UAF):
// a value whose ONLY use is an owning store TRANSFERRED its one
// reference there (the retain was elided) — a block-exit release
// would spend that reference twice and free an object its new
// owner still holds. Such values get no die_at.
```

Elision and block-exit release are each sound alone; composed naively they
double-spend a reference. The corpus under `jit-rc` and the rc grid leg are
the use-after-free detector that caught it — the moment frees became real,
every golden became a memory-safety test.

The rest of the ladder — sized releases, the release cascade, Bacon–Rajan
trial deletion over the purple candidate buffer, the explicit
`frk_rt_rc_collect()` trigger — lives in the runtime and is the subject of
[The GC Ladder](../memory/gc.md). The strategy-as-parameter design itself is
examined in [Strategy as a Lowering Parameter](../memory/strategies.md).

## Records: field-granular boxes (TS-2)

TS-2's classes forced the record idiom (D-073): a class instance is a
**managed box of a product** — identity from the box, shape from the
product, tracing from the allocation's layout word — plus the two ops
no earlier specimen needed, `field_get(box){field}` and
`field_set(box, value){field}`. Reads gep to the word slot and adapt;
writes retain-new under rc and store, mirroring `box_set`'s
leak-biased no-release-old frontier. The layout recursion for boxed
products went slot-kind-driven at the same time, so records holding
strings, arrays, or other records trace — and both collector twins
drill the record shape (pointer at word 1) to identical free counts.

Recursive classes hit a real wall: `Node.next: Node` makes
`box<product<…>>` infinite as a structural parametric type. D-074
unties it with **type erasure**: class-reference fields store as the
opaque `!frk_mem.recref`, with identity ops `rec_ref`/`rec_cast` at
store/read — the target product's own ref fields are recref, so the
type closes. Both lower to nothing; object identity survives because
the value *is* the box either way. The `this.next = this` bootstrap
gets `recref_null`: a construction-only placeholder seeded into the
product and back-patched with `rec_ref(box)` immediately after
`box_new` — reading one is a frontend bug, never a program outcome.

## Rulings

| Entry | Ruling |
|---|---|
| D-041 | The box surface; strategy as a lowering parameter; rc v0 = headers + retain + transfer elision, no automatic releases yet. |
| D-049 | Arrays join `frk_mem`; `{len, data}` representation; interp-trap/native-unchecked bounds; managed/unmanaged pointer split. |
| D-053 | rc + cycle collection wins the GC gate over MMTk (two-twin runtime and the five-triple grid decide it). |
| D-054 | GC ladder step 1: block-local liveness releases, release counter in both twins, leak assertion. |
| D-057 | Three-word rc header; layout descriptors on every allocation; release cascade + trial deletion; the transfer-vs-release exclusion. |
| D-073 | Records: class instances as boxes of products; field_get/field_set; slot-kind-driven product layouts (managed fields trace). |
| D-074 | Recursive records: type-erased recref + rec_ref/rec_cast close the μ-type knot; recref_null seeds the `this.next = this` bootstrap. |
