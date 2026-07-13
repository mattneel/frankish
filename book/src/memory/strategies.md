# Strategy as a Lowering Parameter

The memory strategy is a lowering parameter, never IR. That is the whole
ruling (D-041), and everything in this chapter is its consequence: a
frontend emits `frk_mem.box_new`, `frk_closure.make`, `frk_mem.array_new`
once, with no memory opinion, and the kernel lowering — the single
`lower-frk-kernel` pass — is constructed *at* a strategy:

```rust
/// The memory strategy (D-041): a lowering parameter, never IR. Arena
/// bump-allocates (process-lifetime v0); Rc adds refcount headers and
/// retain calls at owning stores (elided on ownership transfer);
/// releases arrive with the M10 GC-gate liveness work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Strategy {
    Arena,
    Rc,
}
```

One IR, two lowerings. There is no `frk_gc` dialect, no strategy
attribute on ops, no per-strategy frontend path. The strategy decides
which runtime symbol an allocation site calls and whether reference
bookkeeping gets woven around it:

```rust
impl Strategy {
    fn alloc_symbol(self) -> &'static str {
        match self {
            Self::Arena => "frk_rt_arena_alloc",
            Self::Rc => "frk_rt_rc_alloc",
        }
    }
}

/// Strategy allocation call: arena takes bytes; rc takes
/// (bytes, layout) — the D-057 descriptor rides every allocation.
fn strategy_alloc<'c>(/* context, rewriter, strategy, size, layout, location */)
    -> Result<Value<'c, 'c>, String>
{
    match strategy {
        Strategy::Arena => result_value(rewriter.insert(direct_call(
            context, strategy.alloc_symbol(), &[size], ptr, location,
        )?)),
        Strategy::Rc => {
            let layout_value = /* arith.constant(layout as i64) */;
            result_value(rewriter.insert(direct_call(
                context, strategy.alloc_symbol(), &[size, layout_value], ptr, location,
            )?))
        }
    }
}
```

(`crates/frk-dialects/src/kernel_lower.rs`; trimmed to the shape.)

## The two strategies

**Arena** is the honest floor: `frk_rt_arena_alloc(bytes)` is a bump
allocation with process lifetime — no headers, no frees, no tracing,
ever. The v0 arena is never reset; region reset entry points arrive with
real region inference, not before. Arena is what makes "the collector is
wrong" a falsifiable claim: every golden also runs without any collector
at all.

**Rc** calls `frk_rt_rc_alloc(payload_bytes, layout)`, which prepends the
three-word header described in [The GC Ladder](gc.md), and the lowering
weaves a retain/release discipline around the program:

- **Retains at owning stores.** A retain accompanies every new owning
  store of a managed value. The owning-store sites, exhaustively, are:
  `product_snoc` (operand 1), `box_new` (operand 0), `box_set`
  (operand 1), `array_set` (operand 2), `dyn.wrap` of a boxed payload
  (operand 0), `table_raw_set` (operands 1 *and* 2 — a table owns stored
  keys and values), and `table_set_meta` (operand 1). What gets retained
  is decided per slot kind (`RetainKind`): a managed pointer directly, a
  closure's env pointer (word 1 of its `{thunk, env}` pair), or a dyn
  pair's payload masked by tag — see the symmetry law in
  [The GC Ladder](gc.md).

- **Transfer elision.** The retain is *elided* when the stored value's
  only SSA use is that store: ownership transfers, the allocation's
  initial count of 1 moves to the new owner. This is the minimal elision
  pass, and it is computed against **pre-lowering** use counts:

  ```rust
  // Sharing must be resolved BEFORE any rewriting: use counts key on
  // pre-lowering SSA values, and op replacement rewrites operands in
  // place (a mid-rewrite lookup would miss and misread transfer).
  let mut retain_shared: HashMap<(usize, usize), bool> = HashMap::new();
  ```

  The comment is a scar, not a flourish: the M7 verifiers caught sharing
  being decided mid-rewrite, which made every store read as a transfer
  and silently elided every retain. Resolved-pre-rewrite is now law in
  the pass.

  ```rust
  fn maybe_retain<'c>(/* … */) -> Result<(), String> {
      if strategy != Strategy::Rc || !shared || kind == RetainKind::None {
          return Ok(());
      }
      let managed = managed_ptr_of(context, rewriter, kind, value, location)?;
      let Some(managed) = managed else {
          return Ok(());
      };
      rewriter.insert(direct_call_void(
          context, "frk_rt_rc_retain", &[managed], location,
      )?);
      Ok(())
  }
  ```

- **Block-exit releases** (GC ladder step 1, D-053/D-054). An
  allocation whose every use sits in its own block dies at that block's
  end; the plan records the terminator as its `die_at` anchor and the
  pass inserts `frk_rt_rc_release` immediately before it. The analysis
  is deliberately conservative — a use on an op with successors, or on
  `func.return`/`func.call`, marks the value as escaping and it is never
  released (cross-block lifetimes leak toward the cycle collector, which
  is the ladder's next rung, not a bug):

  ```rust
  // GC ladder step 1 (D-053/D-054, rc only): a box/array allocation
  // whose every use sits in its own block — none escaping through a
  // branch, call, or return — dies at that block's end; mark it to
  // release before the terminator. Cross-block lifetimes leak (the
  // documented conservative frontier; the cycle collector's ladder
  // continues from here).
  ```

- **The transfer exclusion.** Transfer and block-exit release must never
  both fire. A value whose only use is an owning store already spent its
  one reference *there*; a block-exit release would spend it twice. The
  pass takes a census of owning consumption sites (`owned_operands`) and
  such values get no `die_at`:

  ```rust
  let transferred = users.len() == 1
      && owned_operands.get(&key).copied().unwrap_or(0) >= 1;
  !*escapes
      && !transferred
      && users.iter().all(|user| op_blocks.get(user).copied() == Some(def_block))
  ```

  This exclusion exists because the corpus produced a use-after-free the
  day frees became real — the full story is in
  [The Collector War Stories](war-stories.md).

## The counters are the gate

D-041 shipped ⚑-flagged: rc v0 retained but released nothing, and the
human's review (D-044.1) ratified that staging with a rider —
`frk_rt_alloc_count()` lands in **both** runtime twins immediately, so
the release pass, when it arrives, has a measurable target rather than a
vibe. The twins now carry three counters: `frk_rt_alloc_count` (both
strategies), `frk_rt_rc_release_count`, and `frk_rt_rc_free_count`.

The rider paid off as a verifier. `crates/frk-harness/tests/leak_canary.rs`
compiles three `box_new`s that all die block-locally, runs them under the
rc pipeline in the JIT, and asserts the deltas:

```rust
assert_eq!(allocs, 3, "three boxes allocated");
assert_eq!(releases, 3, "all three die block-locally and release");
```

Allocation behavior is not inferred from IR inspection; it is counted at
runtime, in the same process, through the same C ABI the grid links.

## The matrix holds them identical

The strategies are not alternatives the user picks between and hopes;
they are held byte-identical structurally. Since M7, `default_runners()`
carries **two** JIT runners — the same pipeline at each strategy:

```rust
pub struct JitRunner {
    pub strategy: frk_dialects::Strategy,
}

impl Runner for JitRunner {
    fn name(&self) -> &'static str {
        match self.strategy {
            frk_dialects::Strategy::Arena => "jit",
            frk_dialects::Strategy::Rc => "jit-rc",
        }
    }
    // parse → verify → lower_to_llvm(strategy) → ExecutionEngine
}
```

so every golden in the repository runs under both strategies forever, and
the diff matrix (law L3) reports any byte of disagreement between
`interp`, `jit`, `jit-rc`, and the specimen oracles as a first-rank
finding. The AOT grid does the same per architecture: `AotRunner` is
constructed per (triple, strategy) pair — `aot-<triple>` and
`aot-<triple>-rc` — so the standing figure ("grid green, 4 triples × 2
strategies + the s390x canary") means every golden was executed under
arena *and* rc on every architecture and produced the same bytes.

Under that regime the strategy axis is pure configuration. The lowering
carries the entire cost of the discipline; the frontends never learn
which strategy they are running under — which is the claim "strategy is
a lowering parameter" cashed as a test matrix rather than an
architecture diagram.
