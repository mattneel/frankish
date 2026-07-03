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
