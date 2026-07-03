# The M10 GC gate: rc+cycles vs MMTk — spike report

SPEC §13 requires this comparison *in writing, before proceeding*.
Decision at the bottom; ledgered as D-053.

## What the collector must fit into

Five constraints, all already shipped and all load-bearing:

1. **The strategy knob (D-041).** Memory strategy is a lowering
   parameter over one IR. The rc strategy already exists: headers at
   `ptr-8`, retain-at-owning-store with transfer elision, a fourth
   runner holding it corpus-identical, and `frk_rt_alloc_count()` in
   both twins as the waiting measurement target. Releases are the
   ledgered gap (D-041's ⚑ clause, ratified as staged).
2. **The two-twin runtime (D-042).** The grid compiles
   `crates/frk-rt/c/frk_rt.c` per triple with zig cc; the Rust crate
   serves the JIT. Anything the collector needs at runtime must be
   expressible in BOTH — in practice, in a few hundred lines of
   portable C.
3. **The five-triple grid.** x86_64, aarch64, riscv64, wasm32-wasi,
   s390x — every golden, both strategies, byte-exact. The grid is the
   project's portability proof; a collector that can't ride it
   forfeits the thesis.
4. **Reference semantics stay collector-free (D-008/D-029).** The
   interpreter hosts values in Rust `Rc`; observable behavior can
   never depend on collection timing. Goldens compare output bytes,
   so ANY correct collector diffs clean — but trap/debug texts must
   stay deterministic.
5. **femto_lua's actual pressure.** The v0.1 corpus (tables, strings,
   closures at test scale) is correctness-bound, not throughput-bound.
   The MANIFEST names LuaJIT a yardstick, *informational only*.

## Candidate A: rc + cycle collection (Bacon–Rajan trial deletion)

Extend the shipped rc strategy: (1) the D-041 liveness pass inserts
releases (last-use analysis over the CFG — the deferred debt, now
due); (2) release-to-zero frees through a sized-release ABI; (3)
possibly-cyclic objects (tables, closures — anything that can point
at its own type) go to a candidate buffer on refcount decrement;
(4) a deterministic trial-deletion pass over candidates collects
cycles, triggered by allocation-count thresholds
(`frk_rt_alloc_count` is already the metric).

- **Two twins:** yes — the whole algorithm is refcount fields, a
  candidate vector, and three recursive walks; a few hundred lines of
  portable C, same again in Rust.
- **Grid:** rides as-is, all five triples, no new toolchain demands.
  wasm32-wasi works because rc+cycles needs nothing from the host but
  malloc/free.
- **Determinism:** collection points are functions of the allocation
  and release sequence — reproducible across runs, diffable when it
  matters.
- **Object model fit:** headers exist; the managed/unmanaged pointer
  split (D-049) already marks exactly the class of pointers the
  cycle walker must trace. The type map the tracer needs (which slots
  of which allocation are managed pointers) falls out of the same
  slot-kind machinery the lowering already computes.
- **Costs:** mutator overhead on every retain/release (partially
  paid already); cycle passes are O(candidates); throughput ceiling
  well below a real tracing nursery. Accepted: see constraint 5.

## Candidate B: MMTk

The framework path: bind MMTk-core, implement the VM-side traits
(object model, root scanning, write barriers), pick a plan (Immix).

- **Two twins:** no. MMTk is a large Rust framework; there is no
  C-mirror story. The grid would bifurcate into "strategies the grid
  proves" and "the strategy it can't".
- **Grid:** wasm32-wasi is not a supported MMTk target in any
  practical sense; s390x untested. Two of five legs break.
- **Roots:** MMTk needs stack maps or conservative scanning. LLVM
  statepoints through our AOT pipeline is a real project (months,
  not days); conservative scanning across five ABIs is its own swamp.
- **Determinism:** plan-dependent pause/collection points;
  correctness unaffected, debuggability worse.
- **Payoff:** real throughput, real plans, free future (generational,
  concurrent). None of it needed by constraint 5 yet.

## Decision (D-053)

**rc + cycles wins the gate.** It is the only candidate that keeps
the two-twin runtime and the five-triple grid — the two properties
this project treats as identity, not features. MMTk keeps its Tier-2
slot in SPEC §4.3 with named revisit conditions: a specimen with
MEASURED GC-bound throughput (the counter hooks exist to measure), or
MMTk-on-wasm maturing, or the grid deliberately dropping reach.

**Sequencing** (implementation milestones, not M10; amended per
D-055.1): first the D-041 liveness/release pass against
`frk_rt_alloc_count` (DONE — M11 step 1, the leak canary passes);
then sized releases; then **the layout-descriptor rung** — trial
deletion traverses the object graph, so the managed/unmanaged slot
knowledge (D-049) that today lives only in the compiler becomes a
runtime-visible layout descriptor in BOTH twins (type maps in
headers or side tables, within the portable-C budget) — a named bar
so it is designed, not discovered mid-scan; then the candidate
buffer + trial deletion; then threshold tuning against the femto_lua
corpus. Strings stay outside (rt-owned, D-049) until the tracer
exists, then join as leaf objects.
