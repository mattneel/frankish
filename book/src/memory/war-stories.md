# The Collector War Stories

M12 is the milestone where frees became real. Until then the rc strategy
retained honestly and released nothing — correct staging, ratified as
such (D-044.1) — which means no reference-count mistake could hurt you.
The moment `release`-to-zero started calling `free`, every latent error
in the bookkeeping became memory corruption, and D-057.4 wrote down the
prediction in advance: the full corpus under `jit-rc` and the rc grid
legs "become the use-after-free detector the moment frees are real —
that is the corpus earning its keep, not a risk."

Three bugs surfaced, in order, within minutes of frees going live. Every
one was found by a verifier — a header probe test, then the corpus,
twice. None was found by inspection. That is not an embarrassing
admission; it is the project's thesis measured on its own collector.

## Bug one: the arithmetic shift smear

The rcword packs the Bacon–Rajan color into bits 62–63 — the sign bits
of a 64-bit word. The first decoder shifted an `i64` right to read the
color back. In Rust (and C, on `int64_t`), that shift is *arithmetic*:
it drags the sign bit down. Purple is 3, so a purple rcword is negative,
and `rcword >> 62` came back as −1. No candidate ever matched `PURPLE`;
the collector's markRoots phase saw nothing it recognized.

A header probe test caught it — write a color, read it back — and the
fix went to both twins in their own idiom. The Rust decoder now routes
the load through `u64` so the shift is logical, with the scar documented
at the site:

```rust
fn color_of(rcword: i64) -> i64 {
    // LOGICAL shift: the color occupies the sign bits, and an
    // arithmetic i64 shift smears them to -1 (found by the header
    // probe: purple read back as -1 and never matched).
    ((rcword as u64 & COLOR_MASK as u64) >> COLOR_SHIFT) as i64
}
```

The C twin refuses the trap wholesale — every rcword access is
`uint64_t`, and its header comment says why:

```c
/* ... ALL rcword arithmetic
 * is UNSIGNED — the color lives in the sign bits, and an arithmetic
 * shift smears it (the Rust twin found this the hard way; D-057). */
```

The STATE.md landmine list carries the rule forward for the next agent:
rcword arithmetic is unsigned-only, both twins.

## Bug two: the retain/trace frontier asymmetry

The layout-descriptor rung taught the tracer to see edges: a
`box<!frk_dyn.dyn>` codes its payload as a dyn pair, and `for_each_child`
follows the pair's pointer whenever the tag is table or fun. But the
retain side predated that rung. The M7-era policy retained managed
`Ptr` slots and closure env pointers — it had never heard of dyn pairs,
because before M12 nothing walked them.

So the two sides of the reference count disagreed about what an edge
was. Trial deletion's mark phase decrements the count of every child
*the tracer can see*; retains had only ever incremented for the edges
*the compiler counted*. Every dyn-held table or closure sat with a count
lower than its true in-degree. Result: premature free, then a core dump
the first time the corpus touched the freed object.

The fix is the `RetainKind` classification — the retain side now speaks
exactly the tracer's vocabulary:

```rust
pub(crate) enum RetainKind {
    None,
    Ptr,
    ClosureEnv,
    /// A dyn pair: retained iff the tag is table/fun — emitted
    /// branch-free (select to null; retain(null) is a no-op).
    DynPair,
}
```

The dyn arm is compiled branch-free: `tag ∈ {table, fun} ? payload : null`
via `arith.select`, then one retain call — `retain(null)` is a no-op, so
no control flow is spent on the tag test. The same discipline went into
the table runtime path: `table_raw_set` and `table_set_meta` retain
incoming pair-payloads (masked, transfer-elided) before the store and
release the overwritten value after it, because a table owns its keys
and values.

The M12 extraction promoted the incident to law: **retain coverage must
equal trace coverage**. Widening one side of the frontier without the
other corrupts (tracer sees more than was counted) or leaks (counts
edges the tracer never reclaims). The frontier moves symmetrically or
not at all.

## Bug three: the transfer-vs-release double-spend

Two individually correct optimizations, composed into a use-after-free.

First: transfer elision (D-041). A value whose *only* use is an owning
store doesn't need a retain — its allocation-time count of 1 transfers
to the new owner. Correct on its own.

Second: block-exit releases (ladder step 1). An allocation whose every
use is block-local dies at the block's end; release it before the
terminator. Correct on its own.

Compose them on a box whose single use is being stored into another box:
the store transfers the one reference, *and* the block-exit release
spends that same reference again. Count hits zero while the owner still
holds the pointer; the object frees; the owner reads garbage. The
corpus surfaced it as tag confusion in the closure-heavy Lua case —
dyn values whose tag word had been reused after the free.

The fix is an exclusion, not a heuristic: the planner takes a census of
owning consumption sites (`owned_operands`), and a sole-use value
consumed by one of them is marked transferred and gets no `die_at`
anchor — no release is ever planned for it. The comment at the decision
site records the composition explicitly:

```rust
// TRANSFER-vs-RELEASE exclusion (D-057, found by the corpus UAF):
// a value whose ONLY use is an owning store TRANSFERRED its one
// reference there (the retain was elided) — a block-exit release
// would spend that reference twice and free an object its new
// owner still holds. Such values get no die_at.
let transferred = users.len() == 1
    && owned_operands.get(&key).copied().unwrap_or(0) >= 1;
```

The extraction's phrasing of the law: lifetime analyses must respect
ownership *transfer* — a moved reference cannot also be a dying one.

## What the stories are for

Notice the pattern in how each bug announced itself: not one was
deduced. The shift smear was caught by a probe test written alongside
the header format; the frontier asymmetry and the double-spend were
caught by ordinary corpus programs the day their preconditions became
reachable — D-057.4's prediction ("found by tests/corpus within
minutes") held to the letter, and the M12 milestone note records all
three finds in order.

This family has lineage, too. Back at M7, the very first rc verifiers
caught sharing being decided mid-rewrite — operand replacement had
already rewritten the SSA values that use counts were keyed on, so every
store read as a transfer and every retain was silently elided. Same
organ, same failure genus: reference-count discipline is exactly the
kind of code where a plausible reading and the truth diverge by one
in-place mutation.

The honest conclusion is not "we are careful". It is: nobody's
inspection catches these. What catches them is a corpus in which every
golden runs under two strategies on eight runners and five
architectures with frees live, so that the window between "bug lands"
and "bug detected" is minutes, and the blast radius is one red diff
instead of a shipped collector. The war stories are the differential
law's invoice — three memory bugs, found, fixed, and each one promoted
into a law (unsigned rcword arithmetic; retain coverage equals trace
coverage; transfer excludes release) that the next rung of the ladder
inherits for free.
