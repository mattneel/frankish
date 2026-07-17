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
- D-075 [m29/ts2/dyn] TS-2 COMPLETES (the human picked it):
  structural interfaces on D-026's itabs + object closures — the
  stage freezes at this milestone's exit. (1) INTERFACES: method-
  only, non-void returns, fully annotated (properties in
  interfaces, void interface methods → fence loud). An interface
  VALUE is !frk_dyn.iface — an opaque TWO-SLOT {obj word, itab
  word} (the Go shape, D-026 followed as ruled). Kernel ops:
  iface_make(box){methods=[@C__m…]} -> iface (the conversion site
  is static — sealed world — so the symbol list is an attribute)
  and iface_call(iface, args product){method=k} -> result (exactly
  one; packed per D-036). (2) TWO REPRESENTATIONS, ONE SEMANTICS
  (the K-contract thesis re-demonstrated): the INTERP evaluates
  iface_make as a dictionary — a product of bound closures
  (closure(sym, [obj]) per method) — and iface_call as field-k
  apply; NATIVE lowers to a real itab — a module-level constant
  table of method addresses, DEDUPED by content, entries pointing
  at the class methods DIRECTLY (no wrapper thunks: the call site
  knows the signature from the interface def, exactly as Go does),
  iface_call = load entry k + indirect call (obj, unpacked args).
  The differential matrix arbitrates the two. (3) v0 iface values
  are BORROWS: parameter-passing only; storing an iface (lets,
  fields, arrays) is FENCED — the retain design rides the next
  consumer, not speculation. (4) OBJECT CLOSURES: arrow functions
  with annotated params and EXPRESSION bodies, lambda-lifted onto
  frk_closure.make/apply verbatim (zero new kernel ops — D-035's
  dialect absorbs its fourth frontend). Captures are BY BINDING:
  parameters by value, let-locals by their BOX — so an arrow
  capturing an object shares the alias and observes mutation, the
  JS law, witnessed against node. Function-type annotations
  ((x: T) => R rows) type closure-valued params; calls through
  closure-typed variables are frk_closure.apply. FENCED: `this` in
  arrows, block-bodied arrows, method VALUES (`c.read` unbound —
  JS this-undefined semantics we refuse to imitate; the arrow
  `() => c.read()` is the honest spelling), closures escaping into
  fields/arrays (capture-lifetime rung). Revisit: iface stores +
  retain at the first consumer; method-value bind() if a corpus
  case ever demands it.
- D-074 [m28/mem] Recursive records: THE TYPE KNOT, found at
  implementation. A self-/mutually-referential class (Node.next:
  Node) makes box<product<...>> INFINITE as a structural parametric
  type — MLIR parametric types cannot express μ-types. Ruling: class-
  reference FIELDS are stored type-erased as !frk_mem.recref, a new
  ZERO-parameter opaque type ("a managed record reference"), with
  identity ops rec_ref(box<product>)->recref (erase at store) and
  rec_cast(recref)->box<product> (un-erase at read; the target
  product's own ref fields are recref, so the type CLOSES). Both
  lower to nothing (every side is one pointer) and evaluate as
  identity. recref slots are Ptr{managed} — traced (code 1),
  retained, released; the release CASCADE is untouched because
  layout rides the ALLOCATION header (D-057), not the static type.
  rec_cast is TRUSTED: the frontend's nominal typing is the
  guarantee, verify cannot check the pointee (documented obligation,
  same class as native unchecked array bounds, D-049). Object
  identity (===, later) is box pointer equality — erase preserves
  it. Rationale for erase-over-alternatives: named module-level
  record declarations (LLVM identified-struct style) buy
  verifiability at the cost of a symbol-table coupling in a VALUE
  dialect — defer until a consumer needs the checkability; dyn
  boxing drags the tag machinery into statically-typed code.
  CONSTRUCTION KNOT: `this.next = this` (the standard TS cycle
  bootstrap) needs the record before its own reference —
  frk_mem.recref_null() -> recref exists ONLY for this: a
  placeholder slot value at construction, overwritten by
  field_set(box, rec_ref(box)) immediately after box_new; reading
  one is a frontend bug (interp: not-a-box error; native: null
  deref). retain(null)/trace(null) are no-ops by the DynPair
  precedent. Revisit: named record decls if a second frontend needs
  checked casts; niche/nullability when optional fields land (null
  recref is NOT surface nullability — it never survives a
  constructor).
- D-073 [m28/ts2/mem] TS-2 OPENS with the classes core (the human
  picked it; the stage completes over two milestones — interfaces/
  itabs (D-026) and object closures/method values are the SECOND
  half, fenced here). (1) REPRESENTATION: a class instance is a
  MANAGED BOX OF A PRODUCT — !frk_mem.box<!frk_adt.product<[...]>>,
  fields in declaration order. No new kernel TYPE: identity comes
  from the box, shape from the product, tracing from the layout
  word. What the kernel lacked is FIELD-GRANULAR MUTATION: new ops
  frk_mem.field_get(box){field}->T / field_set(box, value){field},
  the record idiom TS carries that no earlier specimen forced
  (Lua's tables are dynamic; ml_core's products immutable). (2)
  Slice: monomorphic classes, annotated fields (num/bool/str +
  class references — cycles are IN scope), a single constructor
  (param + this-assignment bodies), methods as plain functions
  taking `this` first (@Class_method; direct calls — no dispatch
  until itabs), `new` as @Class_new. Class annotations are treated
  NOMINALLY in this slice; structural typing bites at the interface
  milestone, where it belongs. (3) GC LIVE FOR TS (Tier 2):
  kinds_layout's product recursion generalizes from its dyn-only
  arm to SLOT-KIND-DRIVEN codes — managed-pointer fields code 1, so
  records holding strings/arrays/records TRACE; the retain side is
  already symmetric (product_snoc retains managed appends, D-057).
  field_set mirrors box_set: retain-new, store, NO release-old (the
  documented leak-biased frontier). field_set joins the D-057
  owned-operand exclusion. Cyclic instance graphs ride the existing
  Bacon–Rajan machinery — a cycle corpus case gates under rc.
  (4) Loanword: additive within v1 again (classdecl top row; new/
  mcall/pset nodes; prop reused for field reads). FENCED LOUD:
  interfaces, method values/object closures (next milestone),
  inheritance/extends, static members, getters/setters, optional
  and union-typed fields, field initializers at declaration,
  nested classes, `this` outside methods. Revisit: itabs next
  milestone; union-typed locals + fields when TS-2 completes
  (mutation semantics for narrowing demote by design, D-072).
- D-072 [m27/ts1/contract] TS-1 lands (the human picked it):
  discriminated unions + the imported-flow-facts verifier — and
  frk_contract is BORN (SPEC §4.6; D-015's first ops). The
  architecture is the manifest's, verbatim: the checker's narrowing
  facts are IMPORTED as cast annotations and RE-VERIFIED by our own
  dominance/dataflow pass; unverifiable casts demote to runtime
  contract checks (trust-but-verify — tsc remains untrusted input).
  (1) Loanword vocabulary extends ADDITIVELY within v1 (D-046's
  "vocabulary at TS-1" revisit fires; the D-049 additive precedent
  governs): type rows union{variants}/obj{kind, fields}; expr nodes
  obj (construction), prop (field access), narrow (an imported fact:
  "e is variant v here", with span). (2) The slice: unions of
  object-type aliases discriminated by a `kind: "<string-lit>"`
  property; payload fields num/bool/str; construction only where the
  contextual type names the union; narrowing via if on
  `s.kind ===/!==`. FENCED LOUD: switch narrowing, let-bound
  union values (box reads have no SSA identity — facts would demote
  silently; admit when a case needs them, with the demotion named),
  optional/readonly props, nested object/union payloads, structural
  interfaces (TS-2), >64-variant unions (mask width). (3)
  Representation: a union value IS !frk_adt.sum, variant order =
  union declaration order; `kind` is NOT a stored field — in test
  position `s.kind === "lit"` lowers to tag_of + arith.cmpi, in
  value position to a tag-selected frk_str literal chain. Payload
  fields keep variant declaration order, kind excluded. (4)
  frk_contract.narrow(sum){variant, blame} -> sum: a CHECKED CAST —
  identity on success, deterministic blame trap on refutation
  (blame = "cast to 'kind' at file:line:col", built from the span
  via the artifact line table). K2: the interpreter ALWAYS executes
  the check — reference semantics is maximal checking. K3: native
  lowers surviving narrows to tag extract + frk_rt_contract_check
  (actual, expected, blame ptr+len) — registry row, both twins,
  straight-line abort like frk_rt_dyn_check (D-054 pattern). (5)
  THE PROMOTION PASS (the research slice): forward must-dataflow
  over cf edges, per-function. State = possible-tag bitmask per sum
  ROOT (roots resolve through narrow results to the underlying SSA
  value); edge constraints from cf.cond_br whose condition is
  arith.cmpi eq/ne of frk_adt.tag_of(root) against a constant
  (true/false successors intersect/subtract); block entry state =
  union over predecessor exit states; fixpoint. Sums are PURE
  VALUES, so facts never invalidate — no kill set, the transfer is
  monotone. A narrow whose block-entry mask ⊆ {v} is DELETED (uses
  replaced by its operand); everything else survives to runtime.
  Runs at lower_kernel entry — NATIVE PATHS ONLY, so the
  differential law diffs proof-elided native against the
  always-checking interp: a wrong promotion IS a divergence (L3).
  (6) Exit bars: both fates witnessed — a corpus case whose narrows
  ALL promote (if/else + else-implication via mask subtraction) and
  one that DEMOTES (tsc's aliased-discriminant narrowing, which our
  pass honestly cannot see) yet stays byte-equal with node; a
  tampered artifact with a false fact trapping with blame; suite/
  diff/grid green. Revisit: switch + negative-fact vocabulary when
  a corpus case wants them; let-narrowing at TS-2 (objects force
  the mutability question anyway); promotion stats as a frnksh
  surface when the book needs the demo.
- D-071 [m26/scheme] Handler consumption (the human picked it): R7RS
  exceptions over the v1 handlers — the milestone whose THESIS BAR is
  ZERO KERNEL DELTAS. `with-exception-handler` + `raise-continuable`
  are the tail-resumptive pair made surface: the handler's return
  value IS raise-continuable's value (R7RS 6.11 verbatim — our
  clause ABI), and R7RS's "the handler runs with the OUTER handler
  current" rule is EXACTLY the D-069 masking rule, so nested
  handlers delegating outward need no new semantics. Mapping:
  (with-exception-handler h thunk) ⇒ frk_ctl.handle{label="exn"}
  with the CLAUSE lifted from h's lambda plus a synthesized
  tail-resume epilogue — body value r, then apply κ([r]) in tail
  position; the thunk lifts prompt-shaped ((captures…, token)→dyn),
  so escapes from inside handlers compose with call/cc and wind
  through the EXISTING abort machinery. (raise-continuable e) ⇒
  frk_ctl.perform{label="exn"}(e) + the D-061 guard.
  Frontend growth only: thunk-lambda lifting generalizes to n params
  read from the pack via a new __scm_arg intrinsic (nil-fill,
  frk.borrows — __lua_arg's scheme twin); Job gains the exn-clause
  epilogue and handle-body flavors. FENCED: plain `raise` (its
  handler-returned secondary-error semantics), `guard` sugar,
  `error` objects, `make-parameter`/`parameterize` (want first-class
  procedure values + top-level value defines — a named surface rung,
  not an effects gap). Exit bars: ≥4 chibi-validated corpus cases —
  basic resumption; nested delegation (a handler re-raising
  outward); a handler ESCAPING via an enclosing call/cc ACROSS a
  dynamic-wind (afters fire; the composition case); a reader-style
  dynamic-value idiom with nested rebinding. Regression 11/11;
  suite/diff/grid green; the zero-kernel-delta bar holds or the
  exception is ledgered.
- D-070 [m25/scheme] r7rs_core v0.1 (the human picked "r7rs_core").
  Pairs, quote, symbols — and DYNAMIC-WIND'S OPEN RULING CLOSES.
  (1) THE D-051 WIDENING FIRES: TAG_PAIR = 6, the first new dyn tag
  since ratification. A pair is a wrapped product<[dyn, dyn]> through
  the EXISTING wrap/unwrap ops (the manifest's "adt carriers" promise
  kept — no cons kernel op); the retain==trace symmetry law (D-057)
  names every site the widening touches: masked_dyn_ptr becomes the
  range compare 4..=6, the three tracer arms in EACH twin (table
  slots, array-dyn, wordmap dyn-pairs) accept 6, and kinds_layout's
  Words arm learns to RECURSE INTO PRODUCT FIELDS so a boxed
  [dyn,dyn] traces its two children (the previous all-zero fallback
  would have left car/cdr untraced — an rc UAF waiting for the
  first release). No pair mutation in v0.1 ⇒ pairs cannot form
  cycles ⇒ trial deletion untouched.
  (2) SYMBOLS are tag-3 byte strings: interning makes eq? a pointer
  compare for free; strings-as-strings stay fenced, so the overlap
  is unobservable. '() is the nil tag; display prints it "()".
  (3) DYNAMIC-WIND CLOSES — escape-only, as frk_ctl.wind(before,
  thunk, after) -> dyn: before(); r := thunk(); after(); yield r —
  and an abort raised inside thunk re-raises AFTER after() has run.
  The insight: natively, the D-061 pending-guard discipline IS the
  unwind-finalizer hook (after() runs unconditionally post-thunk,
  BEFORE the pending check propagates), and the interpreter mirrors
  it exactly by catching Err(Abort) around the thunk, running
  after(), and re-raising. No new runtime state; both worlds run
  after() exactly once on normal AND escape exits; before() cannot
  re-run because κ is escape-only (one-shot, outward). Re-entrant
  winds = the Tier-2 rung with re-entrant κ (κ_frk fence updated:
  "no unwind-time finalizers" becomes "wind is THE finalizer form").
  (4) scheme/intrinsics.mlir is BORN (the M17 seed-module surface,
  scheme's turn): display grows str/pair arms (proper lists spaced,
  dotted pairs " . ", '() as "()"), cons/car/cdr/null?/pair? are IR
  intrinsics over wrap/unwrap; the emitter's builder-built display
  helper dies. New rt rows (registry-first): frk_rt_scm_display_str
  (no-newline byte print) and frk_rt_ctl_pack_head (nil-filled
  pack-head read, shared by wind's lowering).
  Exit bars: corpus cases chibi-validated BEFORE implementation
  (pairs/display incl. dotted; list recursion; symbols + eq?;
  dynamic-wind normal; dynamic-wind crossed by an escape through TWO
  nested winds — afters innermost-first, exactly once); K2 wind
  verifiers (normal + escape re-raise ordering) red-first; jit-rc
  green on pair-heavy cases (the symmetry witness); suite/diff/grid
  green; manifest v0.1 SHIPPED; κ_frk OPEN ruling struck; book
  current.
- D-069 [m24/ctl] effects-v1 (the human picked "effects"). frk.ctl
  grows labeled handlers: handle/perform/resume — κ_frk's H-op-resume
  rung, scoped to the AFFINE LADDER'S TRACTABLE RUNGS. THE RULING:
  the clause runs AT THE PERFORM SITE (handler-on-top), which makes
  three clause classes exact, total, and single-threaded:
  (a) DROP — v0's prompt/abort, unchanged;
  (b) ABORTIVE — the clause returns WITHOUT consuming κ: the handle
  yields the clause's value, body-rest discarded (rides the v0
  pending-cell/abort machinery unchanged);
  (c) TAIL-RESUME — the clause consumes κ exactly once and ITS
  RETURN IS THE RESUME VALUE: perform evaluates to it and the body
  continues under the handler (deep reinstall = the dispatch mask
  lifting). FENCED to the named Tier-2 stack-switching rung: full
  re-entrant one-shot κ (non-tail resume, stored continuations,
  clause code running after body-rest) — revisit when coroutines
  land on LLVM coro or a specimen forces multi-suspension.
  SEMANTICS (κ_frk v1 rung, added to the calculus doc):
  perform ℓ v ⇒ innermost UNMASKED handler H for ℓ (handlers pushed
  during a clause dispatch outward — the handler-free-for-ℓ context
  rule); H masks for the clause call; κ = a fresh ONE-SHOT resumer,
  BORN UNIFORM (a fn<[pack],[pack]> closure over a marker whose body
  marks-or-traps and returns its pack — the identity-on-pack thunk);
  r = clause(v, κ); consumed(marker) ⇒ perform = r, else
  abort(H.token, r). κ twice ⇒ trap "one-shot violation (κ_frk)";
  no live H ⇒ trap "unhandled effect (κ_frk)".
  OPS (packed, D-036): handle(clause, body){label} -> dyn (= v0
  prompt PLUS handler push/pop — body still receives the token, so
  escapes compose); perform(v){label} -> dyn; resume(marker, v) ->
  dyn (emitted only inside resumer bodies).
  NATIVE (the license row's fast path): an evidence stack in both
  twins (label = interned bstr ptr ⇒ find is pointer-compare);
  perform lowers BRANCH-FREE — perform_begin(label, out) allocates
  the marker and masks, the clause applies through the uniform
  convention, perform_end(entry, marker, token, rtag, rpay) does the
  consumed-else-abort decision IN THE RUNTIME (abort = the v0
  pending cell; no block surgery in melior), then a select yields
  clause-return-or-dummy. The one-shot trap is REAL native state
  (resume_mark traps on a consumed marker) — interp/native parity.
  THE LICENSE GATE: the interpreter routes every perform through the
  general dispatch machinery; native uses the evidence stack +
  direct apply — mechanisms disjoint, outputs byte-equal on the
  corpus, per the κ_frk §3 forced-general-vs-fast-path row.
  Exit bars: K2 verifiers for all six behaviors (tail-resume value
  flow; abortive; label transparency through an inner handle; deep
  re-entry — the SAME label performed again after a resume; the
  one-shot trap; the unhandled trap) landing BEFORE the interp
  implementation (L1); rt rows in frk-abi with both twins compiling
  against the regenerated contract; hand-written goldens green on
  interp + jit + jit-rc + the grid (with D-061 guards written
  explicitly where aborts cross frames); suite/diff/grid green;
  κ_frk doc updated (v1 rung + the Tier-2 fence named).
- D-068 [m23/lua] femto_lua v0.3 ("Continue" ⇒ queue order): the
  four D-058 fences fall — varargs, mid-explist spreads, explicit
  iterator triples, multi-expression RHS — plus __newindex from the
  queue. THE MECHANISM IS ONE: Lua's explist ADJUSTMENT rule (every
  non-final expression truncates to one value; the final call or
  `...` expands to all its values), implemented once in the emitter
  (the explist engine) and consumed by returns, local/assign
  destructuring, call arguments, table-constructor array parts, and
  the generic-for iterator explist — which makes explicit (f, s,
  ctrl) triples just another explist. Rulings:
  (1) VARARGS are pack-native: a vararg function's `...` tail is
  pack[nparams..], COPIED at the prologue into a private arr BEFORE
  the D-067 dispose — the callee-owned-pack invariant is untouched,
  and `...` reads the private arr thereafter (Lua also materializes
  varargs; one alloc per vararg call). Copy/append are IR intrinsics
  in intrinsics.mlir (__lua_pack_tail borrows its source arr —
  frk.borrows; __lua_pack_copy_into writes through frk_mem.array_set
  so the rc retain discipline is INHERITED from the kernel lowering,
  not hand-written — the M17 authoring surface paying rent).
  (2) `...` is a parse-time capability: legal only in the body of a
  vararg function, rejected inside nested non-vararg closures (Lua
  5.1 semantics) and FENCED at top level (our chunk takes no args).
  (3) The single-tail-call return keeps its no-copy pack-forwarding
  fast path (load-bearing for the D-063 tail-call law); the engine
  handles every other shape.
  (4) __NEWINDEX mirrors __lua_index as a new IR intrinsic
  __lua_setindex implementing luaV_settable: an EXISTING key raw-
  assigns without consulting metamethods; an absent key walks
  __newindex — function form calls (t,k,v) through the uniform
  convention, table form re-enters settable on the target (a TAIL
  call, so metatable chains ride the trampoline/musttail machinery
  like __lua_index's). Constructor writes stay raw (Lua semantics).
  Emitter's AssignIndex switches from frk_dyn.raw_set to the
  intrinsic; rawset/rawget stay fenced.
  Exit bars: regression 12/12; ≥5 new corpus cases 100% vs lua5.1
  (varargs basics/forwarding, multi-RHS adjustment incl. nil-fill,
  a user-authored stateless iterator triple, __newindex both forms
  + existing-key bypass); suite/diff/grid green both strategies;
  pack_reclamation stays flat (vararg copies are collected, not
  leaked). Remaining fences named in the manifest: select(), `...`
  at top level, string.format, rawset/rawget, coroutines, goto,
  weak tables.
- D-067 [m22/gc] THE PACK TERMINAL-COUNT RULING (D-064's ledgered
  observation, picked by the human): OWNED, not accepted-leak. The
  evidence: 1000 rc calls leaked 2026 allocations — one arg pack
  (retained into the args product, whose by-value evaporation strands
  the count at 1) and one result pack (blocked from die_at by the
  func.call escapes-conservatism over its own reads) per call. THE
  PROTOCOL, three pieces:
  (1) frk_mem.dispose — end-of-ownership for a RECEIVED managed
  value. K2: semantic no-op (the interp doesn't count); K3: Rc →
  release, Arena → erased. Packs are CALLEE-OWNED: the lua emitter
  disposes the incoming pack right after the boxing prologue (long
  before any tail call — no D-064 interaction), and the intrinsics
  file's eleven pack-taking functions dispose before their returns.
  (2) frk.borrows — a unit attribute a callee declares in its own IR
  (__lua_arg carries it in intrinsics.mlir): a call to a borrowing
  callee does not mark its operands escaping. This is a fact about
  OPERANDS, not results — the distinction that bit immediately:
  (3) received packs (managed apply results) join the die_at sweep,
  gated by DERIVED-BORROW LOCALITY: the pack is releasable only if
  every borrowing-read RESULT is itself block-local and non-escaping.
  An in-block owning store (box_set/array_set) retains its own
  reference, so in-block derived uses are safe by the existing
  discipline; a cross-block borrow (generic_for's iterator triple —
  f lives across the loop) blocks the release, conservatively. THE
  HARNESS CAUGHT the missing gate as a jit-rc segfault in
  lua/generic_for before it ever reached a commit: the borrowed
  closure env was freed mid-loop. The differential law is the reason
  this ruling ships sound.
  RESULT: 1000-call leak 2026 → 24 (the process-lifetime stdlib
  seeding only); per-call leak ZERO; disposed packs surface as
  Bacon–Rajan deferred frees (buffered zeros reclaimed at collect —
  by design). Witness: tests/pack_reclamation.rs asserts no O(calls)
  term returns. Fences: TS arrays are USER-VISIBLE values, never
  callee-owned (dispose is a pack-protocol fact, emitted only by the
  lua frontend + its intrinsics); the generic_for triple still leaks
  one pack per LOOP (not per iteration) — acceptable, revisit only
  with evidence. Closes D-064's open observation.
- D-066 [m21/surfaces] D-062 IS CLOSED — every named follow-up
  executed ("finish D-062", the human, verbatim):
  (1) REGISTRY-DRIVEN REGISTRATION: the JIT symbol set and the interp
  builtin set are now DRIVEN by frk-abi rows — jit_symbol/builtin_for
  supply only what data cannot (addresses, closures); a row without
  its pointer/behavior panics at registration AND fails the coverage
  witnesses (both directions: missing binding, stale binding). The
  ~35-call hand-written registration block is gone.
  (2) DEAD EXPORTS REMOVED: print_lua_num/bool/nil deleted from both
  twins and the registry (they were linked by nothing since the
  __lua_tostring path landed; the registry exposed it at M17).
  (3) THE LAST U8 DIES: frk_rt_print_bool takes i64; loanword
  declares i64 and widens booleans at the call site (extui); the
  capture shim and interp builtin follow. AbiTy::U8 is REMOVED from
  the vocabulary — no sub-word integer crosses the ABI, anywhere,
  which retires the M15 display_bool bug class at the TYPE level
  (the verifier keeps accepting i1/i8 DECLARATIONS via the widening
  class rule for future frontends). The migration was driven by the
  machinery itself: registry edited first, then every layer refused
  to compile until fixed — twins, shims, frontend, in order, each
  named by its own enforcement point. Nothing about D-062 remains
  open; its revisit conditions (intrinsics DSL, lane filter) stay as
  written.
- D-065 [m20/surfaces] THE LUA INTRINSICS MIGRATION COMPLETES
  (D-062's named follow-up, unfenced by D-063, picked by the human).
  The _v pack wrappers, the iterator protocol (next/pairs/ipairs +
  iter), the string module wrappers (sub/rep), and __lua_index — all
  emitter-built since their births — move into lua/intrinsics.mlir
  as kernel IR (their signatures stabilized at (envref, pack) -> pack
  by the uniform convention, exactly as D-062's sequencing rule
  planned). emit_helpers is DELETED: the lua emitter builds ZERO
  helper IR — it parses the seed module and appends the program.
  Dead builder utilities (lua switch/pack_dyns, scheme switch)
  removed. The lua protocol library is now 442 lines of reviewable,
  diffable IR carrying its own runtime declarations, verified like
  any module. Revisit: never — the surface is the design.
- D-064 [m19/gc] TAIL-AWARE RELEASE SCHEDULING (D-063's fence,
  resolved; picked by the human). The problem: the rc discipline
  anchors frame releases at the block TERMINATOR, which in a
  tail-shaped block lands between the call and its return — the tail
  shape musttail needs is destroyed exactly where it matters, so
  D-063 fenced rc-native runners off the deep goldens. The evidence
  (rc-lowered lua tail loop) showed precisely ONE offending release,
  and it was HALF OF A PAIR: retain(pack) at the owning snoc into the
  args product, release(pack) at the terminator — the frame's
  reference, dropped after the call. THE RULE: in a tail-shaped block
  (func.return fed by the immediately preceding call), a frame
  release whose value carries a PAIRED in-block retain relocates to
  BEFORE the call. Soundness, two legs: (1) the pair witnesses a
  second owner (the consumer's retain), so the value still crosses
  the call at count >= 1 and nothing between the relocated release
  and the call can free it (only pure ops sit there); (2) no caller
  code runs after a tail call — the frame's references are dead the
  moment the call starts, so dropping them early is observationally
  invisible. Unpaired releases stay at the terminator (conservative:
  that block keeps its frame; correct, just not TCO'd). SSA identity
  makes the pair mechanically checkable: retain and release were
  planned against the same lowered value. Terminal refcounts are
  IDENTICAL to the old schedule (+1/-1 in a different order), so no
  leak/free accounting changes. RESULT: the deep goldens run
  UNFENCED — 100k tail frames under jit-rc and the rc grid columns
  on all five triples; grid 74/74 × BOTH strategies. The rc column
  equals arena for the first time since the fence existed. Revisit:
  if a future discipline releases callee-visible references mid-call
  (finalizers, weak refs), the crossing-count argument must be
  re-proven.
- D-063 [m18/closure] THE UNIFORM-SIGNATURE CONVENTION (D-059's
  ledgered gap, picked by the human). Problem: femto_lua's tail-call
  law is VIOLATED — every lua call is a closure-apply, which neither
  the M14 interp trampoline (func.call only) nor native musttail
  (identical-signature direct calls only) covers; deep `return f(x)`
  blows the depth cap interp-side and the stack natively (corpus
  fenced shallow since M13). Design, four rungs:
  (1) INTERP (reference semantics, fully general): closure_eval's
  Apply detects the tail shape ITSELF and returns Step::TailCall —
  the M14 trampoline machinery absorbs it with zero frk-interp
  changes (the Eval trait already speaks Step). Covers BOTH
  conventions and every frontend.
  (2) KERNEL (the convention): a closure callee MAY adopt the uniform
  form — first parameter `!frk_closure.envref` (new opaque type,
  lowers to ptr) instead of unpacked capture leading params; the body
  reads captures via the new `closure.env_load` op (operand: envref;
  attrs: index + the env product type; verifier checks index/type
  against the carried product). closure.make detects a uniform callee
  by its signature and SKIPS thunk synthesis — the closure struct
  holds the callee's address directly. closure.apply lowering is
  UNCHANGED (it already passes (fn_ptr, env_ptr, args…)). Legacy
  callees keep the thunk path; the two conventions coexist per-callee
  (ml/TS/scheme stay legacy).
  (3) NATIVE TCO: frk-tail-calls gains the INDIRECT case — an
  indirect llvm.call in tail shape whose callsite prototype
  (reconstructed from operand/result types) equals the caller's
  function type gets musttail. Under the uniform convention every
  lua function IS (ptr, ptr) -> ptr, so lua tail applies qualify by
  construction. wasm emits tail_call_indirect (proposal covers it).
  (4) LUA ADOPTS IT: lifted functions become (envref, pack) -> pack
  (env product = [_G, captures…], read via env_load); the _v wrappers
  gain the ignored envref param — EXACTLY the signature rewrite
  D-062's sequencing rule kept them emitter-built for.
  FENCE, recorded honestly: native TCO under the RC STRATEGY is NOT
  guaranteed this milestone — the block-exit releases the rc
  discipline inserts between a tail call and its return break the
  tail shape (release-before-call scheduling is its own future rung).
  The deep-recursion goldens fence rc-native runners; interp/arena/
  oracle carry the law at depth; rc stays byte-correct at corpus
  depths. Revisit: rc release scheduling when a specimen needs deep
  recursion under rc natively.
- D-062 [m17/surfaces] Intrinsics and runtimes become FIRST-CLASS
  AUTHORING SURFACES (the human's directive: "we've forgotten two very
  important parts of defining programming languages"). Until now both
  were conventions buried in code: intrinsics as D-056.2 emitter
  builder-code, the runtime ABI "documented" (K4) but enforced by
  nothing — every rt function authored 2–4× (Rust twin, C twin, interp
  builtin, JIT capture shim), the M15 display_bool bug the witnessed
  cost. TWO SURFACES, adversarially panel-reviewed before landing:
  (A) INTRINSICS MODULES (SPEC §6.6): a language's primitives are
  kernel IR in .mlir files shipped with the frontend (include_str!,
  L6-clean), parsed as the SEED MODULE the emitter appends into.
  Ordinary funcs ⇒ K2/K3 for free; verified like any module INCLUDING
  the new declaration check. Migrated at ratification: scheme fully
  (__scm_display + decls; builder code deleted); femto_lua's nine
  plain-dyn protocol helpers (truthy/tostring/print/eq/costr/len/arg/
  set+getmetatable). SEQUENCING RULE (panel): the _v pack wrappers and
  iterator protocol stay emitter-built until D-059's uniform-signature
  convention lands — their signatures ride the closure convention.
  Fences: monomorphic/dyn intrinsics only (typed instantiation waits
  for a monomorphization story); __lua_index + remaining emitter IR =
  mechanical follow-up.
  (B) THE RUNTIME ABI REGISTRY (crates/frk-abi; K4 amended): ONE
  declarative table of every frk_rt_* symbol — args/ret in an
  8-variant ABI vocabulary (incl. PtrPayload: the opaque managed-
  payload pointer, rendered void* in C / *mut u8 in Rust — the one
  deliberate asymmetric mapping), lane (per-language runtime
  extensions are first-class rows), JIT binding (Real/Capture/
  NotLinked), interp disposition. Enforcement, all L1-witnessed:
  Rust twin via build.rs-generated typed fn-pointer assertions
  (compile error on drift); C twin via generated frk_rt_abi.h
  included by frk_rt.c (every compile on every triple enforces; make
  abi regenerates; drift test asserts equality; the tamper test
  REPLAYS the display_bool bug and proves refusal); JIT capture shims
  via generated assertions (the panel's strongest finding — the one
  remaining type-erased layer); kernel_lower declarations DERIVED
  from the registry (hand tables deleted) with dedup against module-
  declared symbols; and a semantic-verifier check projecting every
  bodyless frk_rt_* declaration onto the registry (class-level;
  i1/i8↔u8 widening pinned). First compile of the header caught real
  latent drift (void*/uint8_t* on 11 fns). The registry also exposed
  print_lua_num/bool/nil as linked-by-nothing (dead twin exports) —
  cleanup follow-up. Lane ruling: Lane = owning runtime module;
  consumers filter by lane; c_header grows a lane filter when the
  first specimen twin-extension lands. Deliberately omitted (say so
  once): no varargs flag (nothing needs it), no code-pointer AbiTy
  (frk.stage will add it). NAMED FOLLOW-UPS (panel lens 3):
  registry-DRIVEN JIT/builtin registration (today the typed layers are
  enforced but the registration sets are imperative; refactor the
  runner to iterate RT_ABI rows, then coverage is a table walk);
  dead-export cleanup (print_lua_num/bool/nil). Revisit: intrinsics
  DSL only if raw .mlir authoring demonstrably hurts; full lua
  migration at the uniform-signature milestone.
- D-061 [m15/ctl] The frk.ctl v0 native lowering, settled after a
  3-designer+judge panel (transcript in the session; judge chose
  PASS-OVER-LLVM). Decision, reconciling the panel:
  (1) The ctl OPS lower inside lower-frk-kernel (all three designers
  + judge agreed): CtlPrompt → enter/apply-body/exit/resolve/load
  (branchless — resolve OVERWRITES a 2-word alloca out-slot iff this
  prompt is the target, reusing the grid-proven TableRawGet
  out-pointer recipe, which the judge scored "Tier-0 strongest");
  CtlAbort → extract dyn words + frk_rt_ctl_abort call; CtlPending →
  the real runtime flag. The body-apply carries a frk.ctl_body unit
  attr (its result is caught locally by resolve, never propagated).
  (2) Runtime twins: the pending cell (D-060 rt commit) — enter/exit/
  abort/pending/resolve, identical Rust+C, no unwinder.
  (3) GUARD PLACEMENT — the one divergence from the judge's chosen
  base: guards go in the FRONTEND (emit pending-check + cf.cond_br
  after every non-tail user call; abort → abort-call + return-dummy),
  NOT a post-LLVM block-splitting pass. Rationale: (a) melior
  block-splitting was the panel's unanimous top risk; the emitter
  builds guard blocks naturally, sidestepping it; (b) the judge's
  verifier-first objection to frontend-explicit ("hand-written native
  goldens must author guards") does NOT bind here — the hand-written
  ctl goldens are INTERP-ONLY (real unwind, no guards), and native
  verification is entirely the scheme differential (L3); (c) the
  interp evaluates the emitted guards harmlessly (frk_ctl.pending
  returns 0 in the oracle; the interp has already unwound before any
  guard is reached, so ^propagate is dead interp-side and the
  observable matches native). Heeded panel catches (all in
  ctl-calculus.md §3 fence): tokens stay OPAQUE (never printed/
  compared); aborts never cross frk_entry or a twin callback; the
  alloca-in-prompt hazard is fenced (prompts stay out of loop bodies
  in v0; hoist to entry block if forced). L1 verifier landing WITH
  the lowering: goldens/ctl/escape_direct (a direct-abort case needing
  no guards) — interp+jit+jit-rc+AOT-grid(4×2) all yield 42.
- D-060 [m15/ctl] The Rocq-anchor delegation, resolved by the human:
  "Why do I need to provide the calculus? You can do it just fine."
  §4.4's anchor is satisfied IN-REPO: docs/ctl-calculus.md (κ_frk) is
  the ctl effects design, promoted from atli (~/src/atli) — the
  human's own graded handler calculus, whose Rocq development is the
  mechanization κ_frk leans on. atli, inscription, and flexlang are
  IN-HOUSE prior art (the human's toylangs, on GitHub; frankish
  exists because their patterns kept being rewritten) — no
  attribution ceremony, promotion is the whole point; recorded in
  LANDSCAPE. κ_frk takes: effect rows, innermost-dynamic dispatch,
  the drop/resume clause taxonomy, deep one-shot continuations as
  KEYSTONE (multi-shot call/cc stays a §14 non-goal), both traps
  (one-shot violation, escape past extent), and the licenses→gates
  method. Leaves in atli: β frame-sizing, q uniqueness, ρ regions
  (revisit conditions in the doc). v0 op surface: prompt/abort with
  first-class prompt tokens (the drop-clause subset = escape
  continuations = r7rs call/ec); v1: handle/perform/resume. Native
  default: result-passing (D-011) — tagged returns threaded to the
  prompt; fence: aborts don't cross frk_entry or twin callbacks.
  WITH THIS, the r7rs_core stub's gate ("do not ratify before the
  ctl effects design lands") is SATISFIED; the manifest is ratified
  in the same commit, oracle chibi-scheme 0.9.1 (installed, pinned).
  dynamic-wind: OPEN, D-entry due when the specimen forces it.
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
