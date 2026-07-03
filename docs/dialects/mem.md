# frk.mem — the allocation/ownership surface

One surface, swappable lowerings (SPEC §4.3; D-041). The memory
strategy is a **lowering parameter** — a profile knob, never a
language feature: identical IR runs under every strategy, and the
diff matrix holds `jit` (arena) and `jit-rc` byte-equal corpus-wide.

## Ops (K1: IRDL bases + semantic type equations)

| op | signature | semantics |
|----|-----------|-----------|
| `box_new` | `(T) -> !frk_mem.box<T>` | allocate, initialize |
| `box_get` | `(box<T>) -> T` | load |
| `box_set` | `(box<T>, T) -> ()` | store (the mutable cell) |

## Reference semantics (K2)

A box is a shared mutable cell with identity equality. Aliases (from
products, envs, anywhere) observe each other's writes. The reference
semantics is strategy-agnostic by construction — that is the point.

## Lowering (K3, both strategies)

`box<T>` → `!llvm.ptr`; payloads stored/loaded as their lowered forms;
a box inside an adt/env occupies one slot (`SlotKind::Ptr`,
ptrtoint/inttoptr). Closure envs allocate through the same strategy.

- **arena** → `frk_rt_arena_alloc`: bump allocation, process-lifetime
  v0 (region resets arrive with region inference). No headers, no
  bookkeeping.
- **rc** → `frk_rt_rc_alloc`: i64 refcount header at `ptr - 8`, count
  starts 1. A `frk_rt_rc_retain` call accompanies every owning store
  of a directly-managed pointer (box ptr, closure env ptr) — **elided
  when the store is the value's only use** (ownership transfer; the
  minimal elision pass, decided pre-rewrite on SSA use counts).
  Releases are NOT yet inserted: that needs liveness and lands with
  the M10 GC-gate work. v0 rc proves the strategy plumbing; it does
  not collect. Aggregate-embedded pointers (word-copied sub-products)
  are retain-invisible in v0 — harmless while releases don't exist,
  owed to the same M10 pass.

## Runtime (K4)

`crates/frk-rt`: `frk_rt_arena_alloc`, `frk_rt_rc_alloc`,
`frk_rt_rc_retain`, `frk_rt_rc_release` — C ABI, 8-aligned, JIT
registers all; AOT links the staticlib.

## Verifiers (K5/K7)

`goldens/mem/*` (roundtrip, cell mutation, box-in-product exercising
the retain, struct payloads) run under interp + jit + jit-rc;
`tests/mem_smoke.rs` holds the shape negatives, the shared-cell
aliasing semantics, and the strategy-symbol + retain/elision lowering
assertions.
