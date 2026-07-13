# The GC Ladder

frankish does not have "a GC". It has a ladder — a sequence of rungs
climbed in order, each one landing with its verifier, on top of the rc
strategy the previous chapter described. The ladder was chosen at the
M10 gate, in writing, before any collector code existed: `docs/gc-spike.md`
compares rc + cycle collection (Bacon–Rajan trial deletion) against MMTk
and rules for rc + cycles (D-053), because it is the only candidate that
keeps the two properties this project treats as identity, not features:

1. **The two-twin runtime** (D-042). Anything the collector needs at
   runtime must be expressible twice — in the Rust crate the JIT links
   and in a few hundred lines of portable C the grid compiles per triple
   with `zig cc`. MMTk has no C-mirror story.
2. **The five-triple grid.** x86_64, aarch64, riscv64, wasm32-wasi, and
   the s390x big-endian canary, every golden, both strategies,
   byte-exact. rc + cycles needs nothing from the host but malloc/free;
   MMTk breaks two of five legs (wasm32-wasi impractical, s390x
   untested).

MMTk keeps its Tier-2 slot with named revisit conditions (measured
GC-bound throughput, MMTk-on-wasm maturing, or a deliberate reach
reduction). The ladder's rungs, with execution state per the spike
report:

1. Liveness/release pass — DONE (M11; the leak canary).
2. Sized releases — DONE (M12): three-word headers, real frees.
3. The layout-descriptor rung — DONE (M12): per-site layout words from
   the lowering's slot kinds, lockstep-tested against the runtime.
4. Candidate buffer + trial deletion — DONE (M12): Bacon–Rajan three
   phases, both twins, explicit deterministic `collect()`.
5. Threshold tuning — deferred until a corpus program measurably needs
   it (the counters are the evidence hooks).

## The header

Every rc allocation carries a three-word header before the payload
pointer the program sees (D-057, amending D-041's original one-word
header):

```text
   base                                              payload (what IR holds)
   │                                                 │
   ▼                                                 ▼
   ┌───────────────┬───────────────┬─────────────────┬──────────────────────
   │ layout : u64  │ size : u64    │ rcword          │ payload words …
   └───────────────┴───────────────┴─────────────────┴──────────────────────
   payload-24      payload-16      payload-8         payload
```

The size word serves dealloc *and* bounds the tracer's scan. The rcword
packs the count and all Bacon–Rajan bookkeeping:

```text
   bit  63 62   61        60 ……………………………………………… 0
        └─┬─┘   │         └── count (bits 0..60)
          │     └──────────── buffered (bit 61)
          └────────────────── color (bits 62..63):
                              0 black, 1 gray, 2 white, 3 purple
```

Both twins spell it identically:

```rust
const COLOR_SHIFT: u32 = 62;
const COLOR_MASK: i64 = 0b11 << COLOR_SHIFT;
const BUFFERED_BIT: i64 = 1 << 61;
const COUNT_MASK: i64 = (1 << 61) - 1;
const BLACK: i64 = 0;
const GRAY: i64 = 1;
const WHITE: i64 = 2;
const PURPLE: i64 = 3;
```

```c
#define FRK_COLOR_SHIFT 62
#define FRK_COLOR_MASK (3ULL << FRK_COLOR_SHIFT)
#define FRK_BUFFERED (1ULL << 61)
#define FRK_COUNT_MASK ((1ULL << 61) - 1)
```

The color occupies the sign bits of a 64-bit word, so **all rcword shift
arithmetic is unsigned/logical** — an arithmetic shift smears the color
to −1. The C twin uses `uint64_t` throughout for exactly this reason;
the Rust twin's decoder routes through `u64`:

```rust
fn color_of(rcword: i64) -> i64 {
    // LOGICAL shift: the color occupies the sign bits, and an
    // arithmetic i64 shift smears them to -1 (found by the header
    // probe: purple read back as -1 and never matched).
    ((rcword as u64 & COLOR_MASK as u64) >> COLOR_SHIFT) as i64
}
```

`frk_rt_rc_alloc(payload_bytes, layout)` writes the header and returns
the payload pointer; a fresh object is count 1, black, unbuffered:

```rust
let payload = base.add(24);
let (l, s, r) = header(payload);
l.write(layout);
s.write(payload_bytes.max(1));
r.write(1); // count 1, black, unbuffered
```

## Layout descriptors: the tracer's map

Trial deletion traverses the object graph, so the runtime must know
which payload words hold managed pointers — knowledge that, before M12,
lived only in the compiler (the D-049 managed/unmanaged slot split).
D-055.1 named this as an explicit rung: runtime-visible layout
descriptors, in both twins, designed rather than discovered mid-scan.

The descriptor is the u64 `layout` word the **lowering** computes per
allocation site from the slot kinds it already knows, and the **runtime**
decodes when tracing. The encoding (D-057):

```text
bits 0..1   kind: 0 WORDMAP, 1 TABLE_SHELL, 2 ARRAY
bits 2..3   ARRAY only — element code: 0 skip, 1 managed ptr, 2 dyn pair
bits 4..63  WORDMAP only — a 2-bit code per payload word i at bit 4+2i,
            words 0..29: 0 skip, 1 managed pointer, 2 dyn-tag (this word
            is a tag; the NEXT word is its payload, traced when the tag
            is table or fun)
LEAF = 0    (the all-zero wordmap: nothing to trace)
```

```rust
pub const LAYOUT_LEAF: u64 = 0;
pub const LAYOUT_TABLE_SHELL: u64 = 1;
pub const LAYOUT_ARRAY_LEAF: u64 = 2;
pub const LAYOUT_ARRAY_PTR: u64 = 2 | (1 << 2);
pub const LAYOUT_ARRAY_DYN: u64 = 2 | (2 << 2);

pub fn layout_wordmap(codes: &[u8]) -> u64 {
    let mut layout = 0u64;
    for (index, &code) in codes.iter().enumerate().take(30) {
        layout |= (code as u64 & 0b11) << (4 + 2 * index);
    }
    layout
}
```

On the compiler side, `kinds_layout` maps slot kinds to codes — note the
closure case (thunk pointer is unmanaged code, env pointer is managed
heap) and the dyn case (a two-word tag/payload pair, traced by tag):

```rust
fn kinds_layout(kinds: &[SlotKind<'_>], types: &[Type<'_>]) -> u64 {
    let mut codes = Vec::new();
    for (kind, ty) in kinds.iter().zip(types) {
        match kind {
            SlotKind::Int(_) | SlotKind::F64 => codes.push(0),
            SlotKind::Ptr { managed } => codes.push(if *managed { 1 } else { 0 }),
            SlotKind::Closure => {
                codes.push(0); // thunk fn-ptr
                codes.push(1); // env ptr (managed)
            }
            SlotKind::Words { slots, .. } => {
                if ty.to_string() == "!frk_dyn.dyn" {
                    codes.push(2); // dyn tag; pair traced by tag
                    codes.push(0);
                } else {
                    codes.extend(std::iter::repeat_n(0u8, *slots));
                }
            }
        }
    }
    layout_wordmap(&codes)
}
```

The frontier is deliberately conservative: payloads past 30 words and
embedded aggregates (`Words` slots) code as skip. Untraced edges act as
external references — **leak-biased, never corrupt** — and it is the
same frontier the retain side has, which is not a coincidence but a law
(below). `box<!frk_dyn.dyn>` — every Lua local — codes as a dyn pair, so
dynamic-language garbage is reachable by the tracer; that was the
milestone's point.

The runtime's `for_each_child` walks by layout: WORDMAP words by code
(a dyn pair's payload is visited only when its tag is table = 4 or
fun = 5); ARRAY reads the runtime length from payload word 0 and applies
the element code; TABLE_SHELL knows the 4-word shell
`[cap, count, slots*, meta]` — the meta word is traced, the malloc'd
slot array is walked tag-directed over keys *and* values, and it is
freed with the shell (the D-056 table-internals debt, paid).

The encoding is duplicated in lowering and runtime **by design** — no
dependency between the crates — and
`crates/frk-dialects/tests/layout_parity.rs` is the lockstep: it asserts
the constants and wordmap bit placements against `frk_rt`'s canonical
values. "A drift here means the tracer walks the wrong words."

## The symmetry law

The retain side classifies what an owning store must retain:

```rust
pub(crate) enum RetainKind {
    None,
    /// A managed pointer (box/array).
    Ptr,
    /// A closure value: its env pointer (word 1) is managed.
    ClosureEnv,
    /// A dyn pair: retained iff the tag is table/fun — emitted
    /// branch-free (select to null; retain(null) is a no-op).
    DynPair,
}
```

Compare with `kinds_layout` above: `Ptr` ↔ code 1, `ClosureEnv` ↔ the
closure's `[0, 1]` pair, `DynPair` ↔ code 2, `None` ↔ skip. This is
D-057's frontier symmetry: **retain coverage equals trace coverage**.
The tracer's trial decrement assumes every edge it walks was counted;
an edge the tracer sees but retains never counted is a count deficit and
a premature free. The frontier widens symmetrically or not at all — a
law bought with a core dump ([war stories](war-stories.md), bug two).

## Trial deletion

Releases drive the collector. `release_inner`: count-to-zero cascades
through children by layout and frees (unless the candidate buffer holds
a reference — then `collect` frees it at markRoots); count-to-nonzero on
a **non-leaf** buffers a purple candidate:

```rust
let count = count_of(rcword) - 1;
if count == 0 {
    *r = with_color(count | (rcword & BUFFERED_BIT), BLACK);
    if rcword & BUFFERED_BIT == 0 {
        for_each_child(payload, |child| release_inner(child));
        free_object(payload);
    }
} else {
    // Possibly a cycle root: buffer non-leaf objects purple.
    let leaf = *l == LAYOUT_LEAF;
    /* leaf: back to black; non-leaf: purple, and if not already
       buffered, set BUFFERED_BIT and push to the candidate buffer */
}
```

`frk_rt_rc_collect()` — an explicit ABI entry, called by tests and
harness; the trigger is deterministic by ruling (D-057), automatic
thresholds being the ladder's deferred last rung — runs the classic
three phases over the drained candidate buffer:

1. **markRoots** — purple roots with count > 0 are `mark_gray`ed
   (subtree trial-decremented); everything else is unbuffered, and only
   a *black* count-0 candidate is a deferred free. The guard matters: a
   gray count-0 here is a sibling root's trial zero, not ours to free.
2. **scanRoots** — `scan`: gray with count > 0 means externally
   reachable, so `scan_black` restores the subtree's counts; gray with
   count 0 turns white and the scan descends.
3. **collectRoots** — `collect_white` frees white, unbuffered objects,
   recoloring black as it descends so shared structure frees once.

The C twin is the same algorithm with function-pointer visitors instead
of closures:

```c
static void frk_mark_gray_visit(unsigned char *child) {
    *frk_hdr_rc(child) -= 1;   /* trial decrement */
    frk_mark_gray(child);
}
static void frk_mark_gray(unsigned char *payload) {
    uint64_t *r = frk_hdr_rc(payload);
    if (frk_color(*r) != FRK_GRAY) {
        *r = frk_with_color(*r, FRK_GRAY);
        frk_for_each_child(payload, frk_mark_gray_visit);
    }
}
```

## Held equal by count, not by faith

The Rust twin's tests build cycles through the raw ABI: a release
cascade frees transitively (2 frees); a dead two-object cycle survives
pure rc and dies at `collect()` (2 frees); a live cycle survives collect
with counts *restored* by `scan_black`, then dies when its last external
reference goes (2 more). The C twin, compiled and driven through the
zigcc rig, reports the byte-identical cascade/dead-cycle/live-cycle
free-count story — 2/2/4/4/6 cumulative — on the other side of the
language boundary.

And above the unit rigs sits the standing detector: after M12, every rc
golden on every runner and every grid leg executes with real frees live.
The M12 exit was 59/59 cases × 5 architectures × 2 strategies with the
collector on — every one of those runs is a use-after-free detector that
has been green since.

What still leaks is counted and deliberate: deleted table keys,
overwritten box payloads, payloads past 30 words, non-dyn aggregate
interiors — the conservative frontier, biased to leak, forbidden to
corrupt, waiting for the rung that widens both sides at once.
