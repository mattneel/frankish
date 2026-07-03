# frankish — Decision Ledger

Append-only. Format: `D-NNN [scope] ruling — rationale. Revisit: condition.`
Agents: consult before designing (law L4); append with rationale when ruling
an unadjudicated blocking fork; never relitigate an entry silently. Humans:
strike by appending a superseding entry, never by editing history.

Entries D-001..D-026 were ratified in the founding design conversation
(2026-07-02). Entries marked ⚑ are calls made *for* the human under the
veto-ledger pattern and most deserve their review.

- D-001 [name] Project is **frankish** — lingua franca nod; survives-as-
  loanwords etymology is the thesis. Revisit: never.
- D-002 [cli] Binary is **frnksh**; bare invocation = the REPL ("the
  frankish shell"); `frankish` ships as alias symlink. Revisit: never.
- D-003 [format] Typed-AST interchange is named **loanword**. Revisit: never.
- D-004 [docs] Thin constitution (AGENTS.md) + per-specimen MANIFESTs, not a
  monolith. Revisit: if manifests drift from SPEC, consolidate.
- D-005 ⚑ [stack] Core in **Rust** on melior/mlir-sys @ LLVM/MLIR 22.x; xDSL
  as non-gating prototyping sidecar; TS frontend in TypeScript as a separate
  process. Beaver/Elixir+Zig and pure-Zig-on-C-API logged as roads not taken
  with revisit conditions (SPEC §11). Rationale: crate ecosystem covers the
  frontend half; cairo-native proves melior at production scale. Revisit: if
  melior/C-API gaps dominate two consecutive milestones.
- D-006 [dialects] v1 users compose framework-owned dialects only; user-
  defined dialects deferred to v2 via IRDL runtime loading. Revisit: v2.
- D-007 [contract] Every kernel dialect ships K1–K7 (SPEC §3); verifier and
  goldens land first (law L1). Revisit: never.
- D-008 [semantics] The derived interpreter is reference semantics; JIT/AOT
  must byte-match on goldens; specimen upstreams are third oracles (law L3).
  Revisit: never.
- D-009 [specimens] Order: ml_core → TS-0 (demo/loanword forcing) →
  femto_lua → r7rs_core; c_oracle rig early and parallel, as oracle not
  frontend. Rationale: abstraction risk before runtime dragon. Revisit: after
  M6 retrospective.
- D-010 [specimens] Subsets are named, versioned, frozen against a pinned
  upstream; admission rule = a feature enters only carrying a new idiom;
  fence lists are law (L5). Revisit: never.
- D-011 [ctl] Default error lowering is result-passing; unwinding is a
  Tier-2 opt-in strategy of the same ops. Revisit: if a specimen's oracle
  semantics are unrepresentable without unwinding below Tier 2.
- D-012 [ctl] Effects/handlers lower via evidence passing (Koka-style); the
  Rocq handler calculus is the semantic anchor and source of verifier
  obligations. Revisit: when ctl effects milestone is scheduled.
- D-013 [ts] `number` is f64, specimen-faithful; i32/i64 annotations are a
  named profile extension (a frankish dialect of TS), not the specimen.
  Revisit: never for the specimen; profile evolves freely.
- D-014 [profiles] Sealed-world (closed unions, final classes → devirt) is a
  profile switch, default off. Revisit: never.
- D-015 [dyn×contract] Gradual boundary casts are contract ops with blame
  payloads — gradual typing = dyn × contract, no fourth mechanism. Revisit:
  if blame tracking needs its own dialect state.
- D-016 [wasm] wasm32-wasi via the normal LLVM path is the supported wasm
  target (Tier 1, linear-memory rt); WasmGC deferred. Revisit: when WasmGC
  support is commonly implemented in deploy targets we care about.
- D-017 [portability] Portability is a CI grid (specimen × triple), executed
  via qemu-user + wasmtime; s390x is the big-endian canary. Revisit: grid
  composition at M7.
- D-018 [toolchain] Cross linking via bundled `zig cc` driver; clang+sysroots
  documented as fallback. Revisit: if zig driver churn bites twice.
- D-019 [frontends] Borrowed specimens ride tree-sitter/upstream parsers as
  scaffolding; native readers (pratt/sexp/enforest/phrase) are reserved for
  original languages. Revisit: never.
- D-020 [types] Trait/typeclass solving is dictionary-passing only in v1;
  declarative type-system genericity is out of scope. Revisit: v2.
- D-021 [scope] Lazy evaluation is a v1 non-goal. Revisit: only with a
  specimen that forces it.
- D-022 [scope] LSP/editor tooling is a v1 non-goal; pipeline stays pure and
  coarse-grained so incrementality is addable. Revisit: after first external
  user.
- D-023 [agents] Agent-portability laws (L6–L7): AGENTS.md canonical with
  CLAUDE.md symlink; all workflows via make; STATE.md handoff mandatory; no
  vendor feature is load-bearing. Revisit: never.
- D-024 [loanword] Canonical encoding v0 = sorted-key canonical JSON, UTF-8,
  SHA-256 content id; CBOR revisited at freeze (M9) with measurements.
  Revisit: M9.
- D-025 [adt] Pattern-match compilation is Maranget decision trees; niche/
  tag-packing is a separate, separately-goldened pass. Revisit: never for
  the algorithm; heuristics free to evolve.
- D-026 [dyn] Structural interface dispatch uses Go-style itabs (cached
  interface/type pairs); inline caches deferred. Revisit: at femto_lua
  metatable design if itabs misfit.
- D-027 [harness] Golden runner is custom, not insta: corpus at
  goldens/<suite>/<case>/ (case.mlir + expected.out + gitignored
  *.actual), directives as `// frk-case: k=v` comments, v0 entry protocol
  = nullary entry returning i64 with llvm.emit_c_interface, JIT lowering
  = scf-to-cf → convert-to-llvm → reconcile-unrealized-casts (one shared
  pipeline table). Rationale: bless/diff/multi-runner flow doesn't fit
  cargo-snapshot tools and this costs zero new deps. Revisit: if
  directive creep demands a real manifest format.
- D-028 [harness] Stage dumps v0 = one single-pass PassManager per
  pipeline entry, snapshots in MLIR default textual form, out dir
  recreated whole, dumps never goldened (pedagogy artifact tracking
  MLIR's printer). Rationale: exact after-pass-N snapshots without
  C-API IR-printing instrumentation. Revisit: if melior grows pass
  printing hooks or a dump ever needs to gate.
- D-029 [interp] The derived interpreter is total and deterministic:
  MLIR-level UB (div by zero, signed-div overflow, non-positive scf.for
  step) traps; call depth caps at 1024 frames and traps. Corollary: the
  golden corpus must be UB-free — native paths do whatever LLVM does
  with UB, so UB can never be compared. Rationale: reference semantics
  (D-008) cannot have undefined outcomes. Revisit: depth cap if a
  specimen legitimately exceeds it (scheme tail calls are exempt by
  design — proper TCO is a lowering obligation, not deeper recursion).
- D-030 [dialects] Kernel dialect registration is two-tier. Tier A:
  IRDL runtime loading (melior load_irdl_dialects) for ops/types fully
  expressible as operand/result/attribute/region count+type
  constraints — verifiers are generated (type variables solve across
  positions), types are parametric, parse/print/builder all round-trip.
  Tier B: a small C++ ODS shim compiled once into the framework and
  registered through the C API — for any op needing traits:
  terminators, successors, NoTerminator regions, custom semantic
  verifiers. Evidence: crates/frk-dialects/tests/registration.rs proves
  Tier A end to end; LLVM 22 IRDL cannot declare traits — dynamic ops
  are rejected as block terminators ("block with no terminator") and
  cannot carry successors ("successors in non-terminator") — exactly
  what frk.adt.match's region arms require, so frk.adt is Tier B's
  first customer (match + its arm-yield terminator). Rationale: this is
  the C++/ODS-adjacent cost K1 already prices in, paid once inside the
  framework, never by users; D-005 and D-006 stand unchanged. Revisit:
  fold Tier B into Tier A when upstream IRDL grows trait declarations
  (LANDSCAPE watch item).
- D-031 [dialects] **Supersedes D-030 (struck by the human,
  2026-07-02).** Kernel dialects register via IRDL runtime loading
  ONLY; there is no C++ ODS shim anywhere in v1 — the build stays pure
  Rust/melior (D-005's rationale intact) and the design bends instead:
  no kernel dialect op may require traits (terminators, successors,
  trait-relaxed regions). Consequences, effective now: frk.adt drops
  the region-based `match` op — the dialect is pure value ops (`make`,
  `tag_of`, `extract` over parametric `!frk_adt` types), multiway
  dispatch rides upstream `cf.switch`, and surface `match` is compiled
  by the Maranget decision-tree pass (D-025) from the frontend's
  pattern matrix straight to dispatch IR, goldened over the matrix→IR
  mapping. Invariants beyond IRDL's constraint language (e.g.
  extract's result type = the named field's type) are enforced by a
  frankish verification pass run before any execution or lowering —
  K1's "verifier enforcing invariants" hosted in a pass, not in C++.
  SPEC §3 K1 and §4.1 amended citing this entry. Revisit: only if a
  future dialect design demonstrably suffers from de-regioning (bring
  the suffering as evidence), or upstream IRDL grows trait support
  (LANDSCAPE watch item), and then only by a new entry.
- D-032 [adt] K3 v0 lowering is an external MLIR pass (melior
  create_external) in the shared pipeline table — "lower-frk-adt",
  stage 01 in every dump. Representation: sum →
  !llvm.struct<(i64 tag, i64 × max-field-count)>, product →
  !llvm.struct<(i64 × fields)>; narrow integer fields extui/trunci
  through uniform i64 slots. Mechanics: plan-then-apply (layouts
  decoded from original frk types; set_type sweep over block args and
  op results; function_type attribute rewrite; IrRewriter op
  replacement in program order). Fences: field types must be builtin
  integers ≤64 — nested adts wait for the memory axis (M7); and
  wrong-variant extract is unspecified in lowered code while the
  interpreter traps (D-029), so extracts must be tag-guarded (exactly
  the decision-tree output shape) and an unguarded extract is
  inadmissible as a golden. Rationale: obviously-correct wasteful
  layout first — niche/tag-packing is its own later, separately-
  goldened pass (D-025). Revisit: representation when frk.mem lands
  (heap/recursive types); pass packaging if melior grows
  DialectConversion bindings.
- D-034 [adt] Decision-tree pass v0 (D-025 executed): pure matrix→tree
  compilation in frk-dialects (adt_dtree) — pattern language =
  variant / product / int-literal / wildcard / binding; typed columns
  (occurrence + nested ValueType); Maranget baseline heuristic
  (leftmost first-row constructor); products specialize without a
  switch node; SwitchTag omits its default iff tag coverage is
  complete. Tree goldens are literal renderings inside the pass's test
  suite until a textual matrix format exists (M5) — byte-exact under
  the same L2 duties. Exhaustiveness/usefulness derive from the tree
  (reachable Fail → witness; leaf-absent arm → redundant) behind the
  PatternAnalysis trait — complete for this pattern language; SPEC
  §4.1's rustc_pattern_analysis clause is deferred to M5 behind that
  same boundary (adopt when ml_core patterns need or-patterns, ranges,
  guards). IR emission from trees lands with its first consumer
  (ml_core, M5), never speculatively. Revisit: both deferrals at the
  M5 manifest freeze.
- D-036 [dialects] **No variadic operand/result groups in kernel
  dialects** — hardening D-031 with a newly proven ceiling: LLVM-22
  IRDL constraint variables bind once per op instance, so every
  element of a variadic group unifies to ONE type; heterogeneous
  variadics are inexpressible (proof: make_sum(i64, i1) is rejected at
  parse — "expected 'i64' but got 'i1'" — meaning M3's variadic op
  surface never supported mixed-type fields; the corpus passed only
  because it was uniformly typed. Filed as a first-rank finding).
  Response: explicit packing. frk_adt: make_product is replaced by
  product_new() + product_snoc(product, value) chains; make_sum takes
  ONE payload operand of the variant's product type. frk_closure:
  make(env product) {callee}; apply(closure, args product) yields
  exactly one result (multi-result closures deferred — every v1
  specimen is single-valued). Deep typing stays in the frk semantic
  pass (unchanged in strength). Rationale: ≤2 operands and ≤1 result
  per op means every IRDL variable sits in one position — sound by
  construction; packing chains are honest ANF-style kernel IR that
  frontends/emission produce mechanically. Revisit: if upstream IRDL
  gains per-element fresh variables, variadic surfaces may return
  (goldens re-blessed under L2).
- D-059 [m14/ctl] Tail calls as law, first rung ("Keep going" ⇒
  queue order; r7rs is queue-top but its OWN stub gates ratification
  on the ctl effects design, and SPEC §4.4 anchors that design to
  the human's Rocq handler calculus — an artifact only the human can
  supply. ESCALATED per the constitution; see For-the-human. The
  stub's hardest obligation is payable independently NOW, and is
  ALREADY OWED: Lua 5.1 mandates proper tail calls and our
  `return f(x)` still stacks.)
  M14 scope: (1) REFERENCE SEMANTICS — the interpreter trampolines
  every tail-shaped call (a func.call whose single result feeds the
  immediately following func.return): Step::TailCall threads to
  eval_function's loop, replacing recursion; the D-029 depth cap
  counts non-tail entries only, exactly as its exemption clause
  promised. Full generality: direct, indirect-through-thunks, any
  frontend. (2) NATIVE — a post-conversion pass (frk-tail-calls)
  rewrites llvm.call TailCallKind to musttail where the tail SHAPE
  holds AND the callee is a DIRECT call whose LLVM function type is
  IDENTICAL to the caller's (self-recursion always qualifies; equal-
  signature mutual recursion qualifies; ccc both). Indirect and
  cross-signature tails are the LEDGERED GAP: guaranteeing them
  needs the uniform-signature convention (every function one LLVM
  type — the pack convention's logical completion, designed as the
  r7rs prerequisite, implemented when the Rocq anchor unblocks the
  track). wasm32 needs -mtail-call at compile; wasmtime 46 has the
  tail-call proposal on. (3) THE LAW'S VERIFIER: fixed-stack deep
  recursion goldens — 10^6 self-tail and 10^6 equal-signature
  mutual — which FAIL without each rung (the interp depth cap trips;
  the native stack overflows ~48MB into an 8MB limit). Corpus law:
  lua/scheme deep tail recursion beyond the native gap stays
  interp-fenced until the uniform convention lands. Revisit: the
  uniform convention at r7rs open; s390x musttail behavior is the
  canary's to report.
- D-058 [m13/lua] femto_lua v0.2 ("Continue" ⇒ queue order, L4).
  THE CONVENTION CHANGE: every Lua function adopts the uniform PACK
  convention — fn<[arr<dyn>], [arr<dyn>]> — one argument pack in,
  one values pack out. Consequences, all bought with FRONTEND-ONLY
  changes (the kernel is untouched; M12's collector already traces
  arr<dyn> via LAYOUT_ARRAY_DYN): (1) the D-054 exact-arity fence
  DISSOLVES — missing params nil-fill and extras drop at the callee
  prologue, Lua's real semantics; (2) multiple return values are the
  pack itself — `return e1, e2`, destructuring `local a, b = f()`
  and `a, b = f()`, expression context truncates to element 0; (3)
  dyn fun unwraps use ONE fn type everywhere (no per-arity types).
  Still fenced (v0.3+): varargs `...`, spread-in-the-middle
  (f(g()) expanding g's pack as trailing arguments), and multis from
  anything but calls. v0.2 also lifts: repeat/until, break, generic
  for with pairs/ipairs/next, and a two-function string module
  (string.sub with Lua's 1-based negative-tolerant indexing,
  string.rep). ITERATION ORDER canon rule: pairs order is
  implementation-defined in Lua AND here (our slot order ≠ PUC's) —
  corpus law: pairs loops print only order-independent aggregates;
  ordered output uses ipairs. next is an rt entry
  (frk_rt_table_next, both twins, slot-order deterministic). Exit
  bars: the eight v0.1 cases stay green under the new convention
  (regression is the first bar); the v0.2 corpus (multis, nil-fill,
  repeat/break, pairs/ipairs, string) ≥90% vs lua5.1; seven-runner
  diff and the five-triple grid green. Revisit: varargs at v0.3;
  pack-allocation cost if a benchmark ever cares (the counters are
  live).
- D-057 [m12/gc] The human picked the GC ladder ("Do it", second
  time). M12 = the remaining rungs of D-053/D-055's sequencing,
  climbed in order, both twins, exit bars:
  (1) SIZED RELEASES — release-to-zero FREES. The rc header grows to
  THREE words (amending D-041's one-word header):
  [layout: u64 @ ptr-24][size: u64 @ ptr-16][refcount word @ ptr-8].
  The size word serves dealloc AND the tracer's scan extent; the
  refcount word carries Bacon–Rajan bookkeeping in its high bits
  (bits 62..63 color: 0 black, 1 gray, 2 white, 3 purple; bit 61
  buffered; bits 0..60 count). frk_rt_rc_alloc gains a LAYOUT
  parameter: (payload_bytes, layout) -> ptr; arena_alloc is
  unchanged (arena has no headers and never traces).
  (2) THE LAYOUT-DESCRIPTOR RUNG (D-055.1, designed here): layout is
  a u64 the LOWERING computes per allocation site from the slot
  kinds it already knows. Encoding: bits 0..1 = kind — 0 WORDMAP,
  1 TABLE_SHELL, 2 ARRAY. WORDMAP: bits 4..63 hold 2-bit codes for
  payload words 0..29 — 0 skip, 1 managed pointer, 2 dyn-tag (this
  word is a tag; the NEXT word is its payload, traced when tag ∈
  {table, fun}). LEAF is the all-zero wordmap. ARRAY: bits 2..3 =
  element code (same 0/1/2); the tracer reads the runtime length
  from payload word 0. TABLE_SHELL: the tracer knows the 4-word
  shell — meta word traced, the malloc'd slot array walked
  tag-directed over keys AND values, and FREED with the shell (the
  D-056 internals debt paid). CONSERVATIVE FRONTIER: payloads past
  30 words, and aggregate-embedded pointers (Words slots), code as
  skip — untraced edges act as external references: leak-biased,
  never corrupt (the same frontier the retain side already has).
  box<!frk_dyn.dyn> — every Lua local — codes as a dyn-pair, so Lua
  garbage is REACHABLE by the tracer, which is the milestone's
  point.
  (3) RELEASE CASCADE + TRIAL DELETION: release-to-zero walks
  children by layout (recursive release) then frees; release-to-
  nonzero on a NON-LEAF buffers a purple candidate.
  frk_rt_rc_collect() runs the classic three phases (mark-gray trial
  decrement, scan restore, collect white) over the candidate buffer.
  THE TRIGGER IS EXPLICIT in v1: collect is an ABI entry called by
  tests/harness — deterministic and diffable; automatic thresholds
  are the ladder's LAST rung, deferred until a corpus program needs
  them (frk_rt_alloc/free counters are the evidence hooks).
  (4) VERIFIERS: hand-built cycles through the raw ABI in BOTH twins
  (the C side through the zigcc rig, %.14g-style); a release-cascade
  test (box-in-box frees transitively); the full corpus under jit-rc
  and the rc grid leg become the use-after-free detector the moment
  frees are real — that is the corpus earning its keep, not a risk.
  Revisit: thresholds when measured; the Words frontier at the same
  time as the retain side's (one design, both directions).
- D-056 [bstr/dyn] The femto_lua kernel prerequisites (M11 bar 3
  design; executes D-052's deferred representation choice).
  (1) BYTE STRINGS get their own micro-dialect frk_bstr — NOT an
  overload of frk_str: UTF-16 code-unit semantics is TS's law, byte
  semantics is Lua's, and one dialect faking both would divert both
  oracles. Ops: lit {text} / concat / eq / len over !frk_bstr.str.
  INTERNING IS THE REPRESENTATION (D-052): the rt owns a global
  intern table; lit and concat both return canonical pointers.
  Payoff: eq lowers INLINE to pointer comparison (identity ≡ content
  by construction — no rt call), len lowers inline to a header load;
  only intern, concat, and from_num are rt calls. The interpreter
  uses Value::Bytes with CONTENT equality — observably identical to
  interned identity, no intern table needed in reference semantics
  (noted, deliberate). String keys hash by canonical pointer.
  v0.1 literal fence: printable ASCII + standard escapes (the IR
  attribute carries decoded bytes; arbitrary 8-bit literals need an
  encoding ruling — Lua VALUES are 8-bit clean via concat/from_num
  regardless). frk_rt_bstr_from_num formats %.14g into an interned
  string — tostring and ..-coercion ride the SAME formatter the
  D-055 parity rig proved.
  (2) TABLES are frk_dyn ops, RAW only: table_new / raw_get /
  raw_set / table_len / set_meta / get_meta — all operands/results
  dyn. THE METATABLE PROTOCOL IS NOT A KERNEL OP: __index (table AND
  function forms) is a SYNTHESIZED IR HELPER the frontend emits once
  per module (@__lua_index walks the chain; function-form dispatches
  through frk_closure.apply) — ordinary IR that runs identically on
  interp, JIT, and all five AOT triples, zero rt-callback machinery.
  Same pattern for the other Lua protocols: @__lua_print (tag
  dispatch to the per-type rt prints), @__lua_truthy (false/nil
  falsy, everything else truthy — 0 is TRUE), @__lua_eq (tag-pair
  equality; str eq is pointer eq; table/fun identity), @__lua_concat
  (.. with number coercion via bstr_from_num).
  (3) Table rt ABI (both twins): the table OBJECT allocates through
  the STRATEGY (rc headers work; dying tables release); internal
  array/hash parts are malloc-domain until the sized-release +
  layout-descriptor rungs add destructors (the leak is counted
  honestly by the existing counters). dyn results cross the C ABI by
  OUT-PARAMETER ({tag,payload} into a caller alloca) — struct-return
  conventions across five triples are exactly the ABI risk the wasm
  signature_mismatch taught us to refuse. Number keys hash by f64
  bits (1.0 and 1 are the same f64 — Lua agrees); NaN keys are
  fenced (Lua errors; corpus stays clear).
  Revisit: literal encoding when a non-ASCII corpus case arrives;
  table internals' domain at the layout-descriptor rung.
- D-055 [gc/canon] Third review integration (2026-07-03): the M10
  rulings endorsed; two directives executed.
  (1) THE LAYOUT-DESCRIPTOR RUNG IS NAMED (gc-spike sequencing
  amended): Bacon–Rajan trial deletion traverses the object graph,
  so the collector must know which slots of a table/closure/box hold
  managed pointers AT RUNTIME — knowledge that today lives only in
  the compiler (D-049's managed/unmanaged SlotKind split). Between
  "sized releases" and "candidate buffer" the ladder now has an
  explicit bar: runtime-visible layout descriptors in BOTH twins
  (type maps in headers or side tables, inside the
  few-hundred-lines-of-portable-C budget) — designed, not discovered
  mid-scan.
  (2) THE %.14g ROUNDING CONTRACT (bar 4 steer): %.14g ROUNDS — 14
  significant digits, lossy, unlike the TS-0 printers' shortest
  round-trip — so the Rust twin must reproduce C's rounding at the
  14th digit INCLUDING half-even ties, plus %g's positional/exponent
  switchover (exponent form when exp < −4 or ≥ 14 — note: the Lua
  fence upper bound is therefore 1e14, one decade TIGHTER than
  TS-0's 1e15). Both printers are correctly-rounding with
  round-half-to-even, so byte parity is achievable and is VERIFIED
  by a cross-twin test compiling the C twin via zigcc and diffing
  against the Rust emulation on deliberate tie values (15th
  significant digit exactly 5, binary-exact). A corpus tie case
  joins the first frontend goldens. Integral-prints-bare rides the
  TS-0 fence precedent. Also recorded: the tag-space widening for
  TS-1 unions remains the named D-051 revisit — "unions are coming
  for those tags."
- D-054 [m11] The human picked the recommended track (2026-07-03,
  "Do it"): femto_lua implementation INTERLEAVED with the GC ladder,
  named M11. Exit bars (L4-defined, this entry): (1) dyn K3 — the
  fat-value lowering lands and the interp fence lifts from dyn
  goldens; (2) GC ladder step 1 — block-local liveness releases in
  the rc strategy (D-041's debt), a release counter in both twins,
  and a leak assertion proving allocations die; (3) the femto_lua
  v0.1 frontend over the D-052 manifest with a hand corpus ≥90%
  conformant against lua5.1 across interp+jit×2 and the grid;
  (4) the Lua number-print canon fence (%.14g) with three-way
  byte agreement (Rust twin emulation, C twin native %.14g, the
  lua oracle) inside the TS-0-precedent value fence.
  Slice fences for v0.1 (documented divergences, D-038
  stricter-is-deterministic precedent): call arity is EXACT (Lua's
  nil-fill/drop adjustment is v0.2 — corpus law: arities match);
  runtime type errors are traps, corpus stays in-fence; print takes
  nil/bool/num/str only (table/function printing embeds addresses —
  not canonizable). Parser: hand-rolled recursive descent again
  (D-019 scaffolding stance, ml_core precedent; tree-sitter revisit
  if grammar maintenance bites). Sequencing within M11: dyn K3 + GC
  step 1 first (kernel-side, self-verifying), frontend second,
  tables/metatables third, conformance close last.
- D-053 [gc] The M10 GC gate is decided: rc + cycle collection
  (Bacon–Rajan trial deletion) over the shipped rc strategy; MMTk
  stays the Tier-2 slot. Full comparison in docs/gc-spike.md (the
  written spike report SPEC §13 demands). The deciding constraints:
  the two-twin runtime (D-042 — no C-mirror story exists for MMTk)
  and the five-triple grid (no practical MMTk on wasm32-wasi; s390x
  untested) — the two properties this project treats as identity.
  Sequencing: D-041's liveness/release pass first (its ratification
  rider, frk_rt_alloc_count, is the waiting metric), then sized
  releases, then the candidate buffer + trial deletion. Revisit:
  a specimen with MEASURED GC-bound throughput, or MMTk-on-wasm
  maturing, or a deliberate reach reduction.
- D-052 [femto_lua] MANIFEST ratified (M10 exit item) + the Lua
  string ruling. Strings: Lua 5.1 strings are 8-BIT-CLEAN BYTE
  STRINGS, INTERNED at creation, equality = pointer identity after
  intern (the PUC-Rio model, observable through table keys and ==
  cost). They are NOT frk_str values — UTF-16 code-unit semantics is
  TS's law, byte semantics is Lua's; one dialect faking both would
  divert both oracles. Representation: a byte-string sibling surface
  (or a unit-width parameter on frk_str) lands with the femto_lua
  implementation milestone; THIS ruling fixes semantics only:
  bytes, interned, identity-equal. v0.1 subset (ratified into the
  MANIFEST): nil/boolean/number(f64)/string; locals; functions +
  closures (upvalues by reference — frk_mem boxes); tables (array +
  hash parts unified, the Lua way); if/while/numeric-for; metatable
  __index ONLY; print + tostring for the oracle protocol. FENCED at
  v0.1: coroutines (frk.ctl milestone), goto, varargs,
  multiple-return-values beyond call-in-tail-position,
  load/loadstring, weak tables, string library beyond ..
  concatenation and #length, string-format exotica, __newindex and
  the rest of the metamethod family. Oracle: lua5.1 (5.1.5 pinned,
  versions.env) under LC_ALL=C through canon; number printing needs
  its own canon fence (Lua %.14g vs our printers — rule at
  implementation, the TS precedent applies). Revisit: fences per
  admission rule as the corpus grows.
- D-051 [dyn] The tagging fork (due at M10 per SPEC §4.5): frk.dyn
  v0 uses FAT VALUES — a two-slot {tag: i64, payload: i64} pair,
  riding the exact machinery closures proved (two-slot kinds,
  word-verbatim copies, ptrtoint/bitcast payload adaptation per
  tag). NaN-boxing and pointer tagging are REPRESENTATION
  OPTIMIZATIONS behind the same dialect surface — the K-contract
  makes representation a lowering detail, so the swap is a profile
  knob later, decided on measurement (the fib-native rig pattern),
  not aesthetics now. Fat values win v0 on: no bit games on the
  big-endian canary, no 48-bit pointer assumptions (wasm32 is
  32-bit; riscv64 sv48+ looms), trivially correct interp semantics
  (Value::Dyn(tag, Box<Value>)), and honest-first debuggability.
  v0 surface (K1/K2 land at M10 — "contract underway"; K3+ with the
  implementation milestone): !frk_dyn.dyn; wrap(T){tag}→dyn;
  unwrap(dyn){tag}→T traps on tag mismatch (total semantics,
  D-029); tag_of(dyn)→i64. Tag space v0 = femto_lua's six: nil=0,
  bool=1, num=2, str=3, table=4, fun=5; tags are a CLOSED enum per
  profile (sealed-world D-014 interplay noted). Dispatch (itabs,
  D-026) is deliberately NOT in v0's surface — metatable dispatch
  design belongs to the implementation milestone with the table
  design in hand. Revisit: representation at first measured
  dyn-bound benchmark; tag space at TS-1 (unions want tags too).
- D-050 [human-review] Second review integration (2026-07-03, arrived
  mid-implementation of the strings/arrays session):
  (1) noImplicitReturns: true joins the producer options — the
  checker-as-oracle COROLLARY is now law: when the oracle offers a
  flag that eliminates a divergence class, set the flag. The reader's
  fall-off zero-synthesis demotes to defensive dead code, commented
  as such.
  (2) The freeze contract's MUSTs are refused as well as obeyed: a
  bit-flipped artifact fixture is rejected naming BOTH hashes
  (tests/loanword_contract.rs) — the M8 error-path treatment applied
  to D-046.
  (3) §6.5 has its witness: the source-mapped OOB trap
  ("...out of bounds... at oob.ts:4:13") — TS-0's first genuine
  runtime trap, threaded producer-span → line table → FileLineColLoc
  → interpreter message. OOB-as-trap is stricter-than-JS by ruling
  (D-049; D-038 stricter-is-deterministic precedent) — JS undefined
  has no representation in a pure-f64 world.
  (4) The strings steer and the implementation CROSSED IN FLIGHT: the
  review recommended deferring the UTF-16 ruling until .length makes
  representation observable — but the shipped slice already included
  .length and the surrogate-counting golden, i.e. the named trigger
  fired at implementation time. The ruling stands as what the
  trigger would have produced: true UTF-16 in both runtime twins,
  decided on the evidence the review asked for ("😀".length === 2
  diffed against V8 across every runner). Recorded rather than
  unwound — ripping out .length to restore deferability would be
  theater.
  (5) Startup-number framing corrected to the Static Hermes claim
  already in LANDSCAPE: predictable performance and INSTANT STARTUP
  on a boot-dominated microbenchmark — not "beats V8 at compute";
  V8 closes on steady-state hot loops. The grid proves the former.
  (6) Noted for the ts0 fuzzing arrival: the canon §6 print fence
  must be baked into VALUE GENERATION or false print divergences
  bury real signal (recorded in canon.md).
  (7) Acknowledged as future refinements, not current debt:
  assignment-driven let-boxing (one line, using facts tsc computes)
  when the box count ever matters; it does not today.
- D-049 [ts0/mem/str] Strings + arrays for TS-0 manifest completion
  (M9 second half). ARRAYS are an allocation shape, so they live in
  frk.mem: !frk_mem.arr<T> + array_new(len)/array_get/array_set/
  array_len (packed, trait-free; literals lower to new + set chain —
  no variadics, D-036). Elements are one-slot kinds only until a case
  demands more. Representation: {len: i64, data: word × len} behind
  the STRATEGY allocator (arrays are user heap). Out-of-bounds is
  OUTSIDE the v0 contract: the interpreter traps deterministically
  (D-029), native is unchecked (UB), JS neither — corpus law:
  in-bounds only; a checked profile is frk.contract territory later.
  JS reference semantics: interp arrays are shared mutable
  (Value::Array, identity equality), aliases observe writes.
  STRINGS are immutable rt values with UTF-16 code-unit semantics
  (JS .length, surrogate pairs count 2 — interp stores Vec<u16>, not
  Rust String). New kernel dialect frk_str: type @str; ops lit
  (UTF-8 attr, lowered to a UTF-16 global) / concat / eq / len. All
  lower to rt calls: frk_rt_str_from_units / _concat / _eq / _len /
  frk_rt_print_str (UTF-16→UTF-8 out), layout {len: u64, units:
  u16×len}, one allocation. v0 strings allocate via plain malloc
  INSIDE the rt, uniform across strategies — strings are rt-owned
  values, not user allocations; revisit at the M10 GC gate when
  tracing wants them. CORRECTNESS corollary: SlotKind::Ptr splits
  into managed (boxes/arrays — rc header at ptr-8, retain legal) and
  unmanaged (strings — NO header; a retain would corrupt ptr-8), so
  the rc lowering retains only managed pointers.
  .length returns i64 at kernel level; TS emission converts sitofp
  (JS lengths are numbers). Loanword vocabulary: ADDITIVE within v1
  per D-046's extension rule (str/arr type rows; str literal, arr
  literal, index, iset, len nodes); the reader's own type synthesis
  disambiguates + (addf vs concat) and === — no producer type
  annotations needed. console.log(string) prints raw (JS). Fences:
  no push/methods, no string relationals, no templates, no holes,
  integer indices only. Revisit: push at the first corpus case that
  needs it; string tracing at M10.
- D-048 [front] D-039's hard M9 trigger fires and resolves: the green
  tree is NOT adopted. Evidence: loanword v1 shipped without lossless
  trees — self-contained artifacts (embedded source + byte spans)
  cover diagnostics and location threading, and no reprinter exists
  or is scheduled (SPEC §9 lists fmt as "later"). rowan-vs-custom
  dissolves for lack of a consumer; ml_core's plain AST and the tsc
  AST both stand. Revisit: if fmt is ever scheduled, or a frontend
  needs incremental reparse — whichever brings an actual consumer.
- D-047 [ts0] TS-0 slice conventions (M9). number = f64 (D-013
  faithful; the FIRST float in the kernel — it enters through the
  admission rule as the idiom ml_core's fence excluded); boolean =
  i1. Monomorphic fully-annotated functions lower to plain func.func
  + func.call — closure-lite arrives only when a corpus case demands
  it (the admission rule cuts both ways). `let` locals are frk_mem
  boxes (assignment is the idiom TS carries that ml_core lacked; the
  mem surface's first frontend consumer); parameters immutable
  (assignment to them fenced, loud). console.log lowers to calls to
  bodyless @frk_rt_print_f64/_bool declarations, resolved three ways
  and DIFFED: interpreter builtins (append to the interp's output
  buffer), in-process capturing JIT symbols (thread-local — the JIT
  shares harness stdout), the real C runtime for AOT. Entry protocol:
  ts cases emit @main() -> () and their output IS the captured
  prints; the AOT shim is the void variant. JS semantics mappings:
  === → cmpf oeq, !== → cmpf une (NaN !== NaN true), <,<=,>,>= →
  ordered predicates (false on NaN), % → arith.remf (fmod, dividend
  sign), &&/|| → strict select (pure subset). Number printing is JS
  ToString within the CANON FENCE: printed values are 0 or |v| ∈
  [1e-4, 1e15) and finite — inside it, Rust Display, the C
  round-trip-precision search, and V8 agree byte-exactly (proven by
  the float_precision golden four ways); outside it JS switches to
  exponent spellings we do not reproduce yet. Dead code after
  `return` drops (tsc-legal); fall-off-the-end of a value function
  returns zero (tsc default lacks noImplicitReturns — fence note).
  Revisit: the print fence when TS-1 needs full ToString; parameter
  boxing when a case demands it.
- D-046 [loanword] loanword v1 FROZEN (M9; SPEC §6.3, D-024
  executed). Canonical encoding: JSON with recursively sorted keys,
  no whitespace, UTF-8; content id = SHA-256 over the canonical bytes
  WITHOUT the sha256 field, then the field is inserted and the whole
  re-canonicalized for output; consumers MUST verify. Mandatory
  fields: loanword (version, =1), producer, file, source (full text —
  artifacts are self-contained; spans index into it), types (interned
  table), decls, stmts. Every node carries "span": [start, end) byte
  offsets; consumers thread them into FileLineColLoc via a line table
  over the embedded source — §6.5 span threading LANDS with this
  entry (loanword programs trap with file:line:col; ml_core's own
  reader still owes its spans, scheduled with its v0.2). v1 node
  vocabulary is the TS-0 slice (fn/log/let/assign/if/while/ret/expr;
  num/bool/var/bin/un/cond/call); vocabulary EXTENSIONS are
  version-gated, encoding changes are a v2. CBOR: measured DEFERRED —
  the fib artifact is ~2KB canonical JSON; content-addressing and
  debuggability beat the bytes at this scale (D-024's revisit
  condition answered with the measurement it asked for). Rust
  consumer deps: serde_json + sha2 (boring-standard; first
  non-melior runtime deps in the workspace, noted deliberately).
  Producer: tools/loanword-ts on the tsc 6.0.3 API, checker-as-oracle
  (strict; we never reimplement the checker), node ≥ 20 runs it
  directly via native type stripping — no build step. Revisit:
  encoding at 100× artifact scale; vocabulary at TS-1.
- D-045 [repl] Amendment to D-043's revisit clause (human directive,
  2026-07-03): revisit ADDITIONALLY when the shell can observe effects
  (IO or cross-line identity) — re-elaboration's replay becomes
  semantics at that point, not implementation. Rationale: D-043 is
  legal today only because D-029's interpreter is total,
  deterministic, and effect-free, so re-running the prefix is
  unobservable; the moment IO or box identity crosses lines (femto_lua
  is precisely the specimen that makes it reachable), replay is
  visible semantics — effects replay N times, a box is a different box
  each line. Named now so M10 cannot shift semantics silently.
  Revisit: fires at first observable effect in the shell.
- D-044 [human-review] First ⚑-queue adjudication (2026-07-03), with
  dispositions from the human, recorded verbatim in effect:
  (1) D-041 RATIFIED — v0 rc without liveness releases is correct
  staging ("releases without a liveness pass are either wrong or
  theater"); rider executed: frk_rt_alloc_count() lands in BOTH
  runtime twins now, so the M10 release pass has a measurable target
  and a leak-canary golden becomes writable the day releases land.
  (2) D-038 RATIFIED, all three flags — float-out is the admission
  rule applied against the manifest's own text (the manifest scope
  line is amended by this entry: ", float" struck, per its own
  amendment rule); redundancy-as-error is stricter-than-oracle in the
  deterministic direction; the min-caml deferral is what the license
  law demands, the 18-case hand corpus is a legitimate v0 wall.
  (3) D-005 RATIFIED WITH PREJUDICE — the stack question is closed by
  evidence (three IRDL dialects, an interpreter, a merged
  type-changing lowering, external passes, JIT symbol registration,
  five-target AOT, against one shimmed library bug); its "if gaps
  dominate two milestones" revisit clause is retired.
  (4) M8 EXIT AMENDED — shell errors must at minimum echo the
  offending source line (a REPL whose trap messages point at nothing
  ships §6.5's bug-by-law); full Location threading stays at M9 per
  D-039. Implemented as the `  at: <line>` echo, proven by the
  division-trap transcript golden. Revisit: none — this entry is a
  record of rulings, not a fork.
- D-043 [repl] The shell's semantics (M8; SPEC §9 as amended by
  D-044.4). Session = an accumulated ml_core declaration prefix,
  RE-ELABORATED WHOLE each line (no incremental typing state to
  corrupt; trivially correct at shell scale); evaluation is the
  reference interpreter, always (D-008) — :profile switches only the
  :emit strategy. Decl lines: typecheck prefix+line, commit on
  success, print `val name : τ` (types only — values would force
  evaluating possibly-dead bindings). Expression lines: compile
  prefix + `let main () = ( line )` under MainPolicy::OptionalAny
  (main optional; if present unit → any concrete τ; lenient zonk
  leaves scheme vars in), interpret, print `- : τ = value` with typed
  rendering (bools/tuples/constructors by name; functions as <fun>).
  Polymorphic expressions (necessarily functions, value restriction)
  print `- : σ = <fun>` WITHOUT emission — there is no concrete type
  to emit at; σ shows normalized 'a/'b vars. Failing lines leave the
  session unchanged; every error echoes `  at: <line>` (D-044.4).
  Classification is by the real parser (decl-parse first, expr-wrap
  second), never by token sniffing. Transcript goldens: transcript.in
  scripts the session; output echoes `PROMPT line` then responses;
  the repl runner drives the EXACT library engine the interactive
  binary runs. :load errors name the requested file only (resolved
  paths are cwd-dependent). ORC per-cell redefinition remains the
  stretch it was scoped as — the re-elaboration model makes it a
  pure performance upgrade later, not a semantic change. Revisit:
  re-elaboration cost if sessions grow pathological; ORC at M10+.
- D-042 [grid] The AOT/cross protocol (M7 second half). (1) AOT flow:
  pre-lowering, the entry func.func is RENAMED to @frk_entry (corpus
  protocol: entry functions are externally-invoked-only, so the rename
  is reference-free and the C shim's main() never collides); then the
  normal strategy pipeline, mlir-translate --mlir-to-llvmir, and
  scripts/zigcc.sh (zig cc, ZIG_VERSION-pinned, plain-zig and anyzig
  shims both handled) links {case.ll, generated shim.c printing
  %lld of frk_entry(), crates/frk-rt/c/frk_rt.c} per triple. (2) The C
  runtime mirror: the grid compiles frk_rt.c per triple instead of
  cross-building the Rust crate — zero rustup-target setup; the Rust
  crate stays canonical for the JIT; the two implementations are held
  behaviorally equal BY the grid (aot must byte-match jit on every
  golden, law L3). ABI corollary, found by the first wasm grid run:
  allocator SIZES are u64 on every target — the kernel lowering
  passes i64 unconditionally and 32-bit-word targets (wasm) enforce
  exact import signatures, so a size_t runtime signature traps at
  link (signature_mismatch); both runtime twins take u64 and cast
  down. (3) Triples are musl-static (zig bundles libc), so
  qemu-user runs them sysroot-free: x86_64-linux-musl (native exec),
  aarch64/riscv64 via qemu-user, wasm32-wasi via wasmtime; s390x is
  the big-endian nightly canary (D-017) — the slot model is
  same-width load/store symmetric, which is exactly what the canary
  proves. (4) Runner placement: the dev loop (make test/diff) keeps
  the 4 fast runners; AOT lives in `make grid` (native + cross ×
  both strategies) and `make ci` runs the native slice — continuous
  L3 coverage without a 6-runner dev loop. Revisit: fold aot into
  default_runners if compile latency stops mattering.
- D-041 [mem] ⚑ frk.mem v0 surface + strategy knob, designed to retire
  four ledgered debts as one design (D-032 boxed reps, D-035 arena
  discipline and by-ref captures, D-038 recursive ADTs — the last two
  UNLOCKED by this surface, scheduled separately at v0.2). Surface
  (packed/trait-free per D-031/D-036): !frk_mem.box<T> with
  box_new(value) / box_get / box_set — the cell primitive. Strategy is
  a LOWERING PARAMETER, never IR: the kernel lowering takes
  Strategy ∈ {Arena, Rc}; both lower box<T> AND closure envs to
  !llvm.ptr with payloads stored as their lowered forms; boxes occupy
  one slot inside adts/envs (SlotKind::Ptr, ptrtoint/inttoptr).
  Runtime ABI per strategy: arena → frk_rt_arena_alloc (the M4 bump
  formalized — the v0 arena is process-lifetime; region reset entry
  points come with real region inference); rc → frk_rt_rc_alloc (i64
  refcount header at ptr-8, payload pointer returned, count starts 1)
  + frk_rt_rc_retain / frk_rt_rc_release (frees at zero).
  frk_rt_alloc is retired — D-035's same-symbol clause executed as a
  rename; the JIT registers all strategy symbols.
  ⚑ rc v0 policy: a retain accompanies every new owning store of a
  managed pointer (into envs or boxes), ELIDED when the stored value's
  only use is that store — ownership transfer, the minimal elision
  pass, real and SSA-checkable. NO automatic releases yet: release
  insertion needs liveness and lands with the M10 GC-gate work. v0 rc
  therefore proves the strategy plumbing end to end (distinct ABI,
  headers, retain+elision, corpus-identical results enforced by a
  second JIT runner in the diff matrix) but collects nothing. Strike
  this clause if liveness-based releases should gate M7 instead.
  Interp semantics: Value::Box — a shared mutable cell with identity
  equality; reference semantics is strategy-agnostic by construction.
  Revisit: releases + escape analysis at the M10 GC gate; box layout
  again when recursive ADTs land (ml_core v0.2).
- D-040 [specimens] M6 retrospective fires D-009's revisit: the order
  is CONFIRMED. Evidence: ml_core-first retired the abstraction risk
  exactly as intended — the M3/M4 dialects carried a full ML subset
  with zero private ops and zero ad-hoc lowerings; the only forcings
  were a slot-model widening and one component built a layer too high
  (promoted at M6). The runtime dragon (femto_lua) still sleeps.
  Next: TS-0 at M9 per D-009. Revisit: at M10 entry (femto_lua gate).
- D-039 [front] Green-tree decision (SPEC §15, due M5, decided at M6
  with evidence): DEFERRED with a named trigger. Plain AST + byte
  offsets sufficed for ml_core — no reprinting, no incrementality, no
  lossless-tree consumer existed. rowan-vs-custom gets decided when
  loanword (M9) needs lossless trees or a reprinter appears,
  whichever first; §6.5 span threading is scheduled with the same
  milestone (docs/type-kit.md records the debt). Rationale: deciding
  representation on one specimen's evidence would be coin-flipping;
  M9 brings a second consumer. Revisit: M9, hard.
- D-038 [ml_core] ⚑ M5 frontend rulings; items (1),(2),(6) touch the
  ratified manifest's surface and deserve human review. (1) FLOAT is
  fenced out of v0.1 by the manifest's own admission rule: it carries
  no idiom the kernel library lacks (it is upstream arith), and its
  canon-divergence work (print_float rendering) is not idiom-bearing;
  the corpus is float-free; revisit at v0.2 alongside the canon rule.
  (2) RECURSIVE ADTs are rejected at declaration: the structural type
  encoding cannot spell them; gated on the memory axis + a nominal-
  type story (M7). (3) Polymorphism: inference has real let-poly
  (value restriction: fun rhs only); emission is monomorphic — zero
  instantiations drops a binding, one concretizes it, several is an
  error; the monomorphization pass is v0.2. (4) match redundancy is a
  compile ERROR (OCaml merely warns) and non-exhaustiveness errors
  with a witness — stricter is deterministic. (5) The parser is
  hand-rolled recursive descent: D-019's scaffolding stance at zero
  research cost, replaceable wholesale. (6) min-caml test vendoring
  is DEFERRED pending license verification; the 18-program hand
  corpus (100%% three-way) is the v0 conformance corpus. (7) Oracle
  protocol: corpus files define `let main () = <int expr>`; the
  oracle runner appends `print_int (main ())` and runs ocaml under
  LC_ALL=C; values stay under 2^62 (the 63-bit rule). Revisit:
  (1)(2)(3)(6) at the v0.2 manifest freeze.
- D-037 [dialects] The kernel lowering is ONE pass ("lower-frk-kernel",
  superseding D-032's per-dialect packaging; representation and fences
  unchanged): adt products carry closure-typed fields and closure
  envs/args are adt products, so the type mapping must be solved
  together. Slot model: integer field ≤64 = one i64 slot (extui/
  trunci); closure field = TWO slots, its {thunk, env} pointers
  ptrtoint'd in and inttoptr'd out; nested adt fields stay fenced
  (M7). Closure mechanics: fn type → !llvm.struct<(ptr, ptr)>; make
  heap-allocates the env via frk_rt_alloc (llvm.func declared once per
  module; JIT registers the symbol, AOT links frk-rt), stores the env
  product's slots, and takes the per-make-site synthesized thunk's
  address as func.constant + one unrealized_conversion_cast to ptr
  (llvm.mlir.addressof cannot reference func.func; FuncToLLVM +
  reconcile fold the cast away — verified end to end); apply extracts
  {thunk, env}, unpacks the arg product per slot kinds, and calls
  indirectly. Revisit: with D-032's clauses at frk.mem (M7).
- D-035 [closure] v0 strategy rulings, made ahead of code. (1) The
  lowering is SPEC §4.2's primary env-struct + function pointer:
  closure value = !llvm.struct<(ptr thunk, ptr env)>; envs come from
  frk-rt's first real component, frk_rt_alloc (documented C ABI; v0
  implementation = leaking bump allocator — the arena/rc discipline
  replaces the implementation behind the same symbol at M7); one
  synthesized thunk per make-site loads captures and calls the lifted
  callee. Rationale: church encoding requires upward escape, which
  kills stack envs; and same-signature closure capture makes flat
  defunctionalization statically unbounded — heap indirection is
  forced, so K4 activates now instead of M7. (2) Defunctionalization
  (the no-heap whole-program strategy) is deferred until a Tier-0
  profile demands it. (3) Captures are by-value; capture *analysis*
  (by-val vs by-ref) becomes meaningful when frk.mem introduces
  locations. (4) Boundary fence: closure signature types and capture
  types are builtin integers ≤64 and closure types; adt captures wait
  for a shared layout oracle (the closure×adt matrix cell is costed
  and deferred). Calling convention: the lifted callee takes captures
  first, then params. Revisit: every clause at M7 (frk.mem).
- D-033 [harness] Golden cases may declare runner applicability
  (`// frk-case: runners=a,b`; default all) per SPEC §7.2 "all
  applicable runners" — for op sets ahead of some execution path.
  Guard rails are law: skips print per case, a corpus whose every case
  skips a runner is an error, and a case no registered runner executes
  is red in the diff matrix. Rationale: staged dialect bring-up needs
  interp-first goldens without weakening L3 where both runners apply.
  Revisit: if directive lists rot after paths catch up (a skip that
  never flips back is a smell — grep for runners= at milestone exits).
