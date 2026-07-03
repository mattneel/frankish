# STATE — frankish live handoff

Updated: 2026-07-03 (M0..M5 sessions)
Phase: M5 complete (tag m5-done); M6 not started.
Tree: green — `make test` 25 blocks; clean-clone scripts/ci.sh exit 0;
`make diff`: diff[interp,jit,ocaml] 33 cases, 0 divergent;
`make dashboard`: ml_core 18 cases, 100% × 3 runners.

## Next action
M6 Promotion pass #1 per docs/SPEC.md §13: promote M5 cheats, re-base
ml_core thin, document the type kit as reusable. Exit: no private ops
in ml_core (ALREADY TRUE — zero private ops existed); conformance not
worse. The real M6 work, from the extraction report below:
1. Promote tree→IR emission out of frk-front's emitter into a
   reusable component next to adt_dtree (any frontend with matches
   needs it; it is currently entangled with ml-specific typing —
   design the seam: dtree + occurrence-typing callback → CFG).
2. Type kit reusability pass: document + de-ml-ify what generalizes
   (unification wrapper over ena, scheme/instantiate machinery);
   spans/locations threading (§6.5) is the standing debt to schedule.
3. Green-tree decision (SPEC §15, due at M5, deliberately deferred
   with evidence): plain AST sufficed for this subset; rowan-vs-custom
   gets decided when loanword (M9) or reprinting pressure arrives —
   confirm or overrule with a D-entry at M6.
4. D-009's revisit fires: "specimen order — revisit after M6
   retrospective". Write the retrospective line.

## In flight
Nothing.

## For the human
- Review ⚑ D-038 (M5 frontend rulings): three items touch the ratified
  ml_core manifest — float fenced OUT of v0.1 by the admission rule
  (the manifest listed it in scope; the corpus is float-free), match
  redundancy is an error where OCaml warns, and min-caml vendoring is
  deferred pending license verification (the 18-program hand corpus is
  the v0 conformance corpus). Strike any of these with a superseding
  entry if you want the manifest read literally instead.
- Review ⚑ D-005 (host stack ruling) in docs/DECISIONS.md — made on your
  behalf. Evidence through M4 supports it strongly: melior 0.27.2 has
  now built two IRDL dialects, an interpreter, two type-changing
  lowering passes with synthesized functions, external MLIR passes, and
  JIT symbol registration — one library bug found and shimmed
  (ArrayAttribute::try_from, LANDSCAPE), zero blocking gaps.
- RESOLVED 2026-07-02: you struck D-030 → D-031 appended (pure IRDL,
  no C++ shim, trait-free dialect designs; match de-regioned; SPEC §3
  K1 + §4.1 amended). Nothing further needed unless you also want the
  amended §4.1 wording itself adjusted.

## Milestone log
m5-done — Shipped: the first specimen, end to end. frk-front: lexer +
recursive-descent parser (pattern-let desugaring, multi-param → nested
funs), HM over ena with real let-polymorphism (value restriction,
recorded instantiations, ≤1-instantiation monomorphic emission per
D-038), typed-AST emission into the kernel dialects — match through
the Maranget dtree (D-034's emission met its consumer), universal
closure calling convention, lambda lifting with the rec re-make
pattern, bools as two-variant sums, pure cf-CFG. Kernel lowering grew
SlotKind::Words (adt values cross closure boundaries; nested-adt
fence lifted for finite types). Harness: source kinds (.ml cases,
OCaml-comment directives), Runner::applicable, the OcamlOracle (same
file + print_int (main ()), LC_ALL=C, canon-filtered), the dashboard
(SPEC §8). Corpus: 18 hand OCaml-compatible programs, 100% three-way
— diff[interp,jit,ocaml]: 33 cases, 0 divergent, FIRST CONTACT.
Exit bars: ≥90% conformance → 100%; dashboard row live; extraction
report below. Ledger: D-038 (⚑). Learned: melior cf helpers spell
pre-MLIR-22 segment attr names (build branches generically);
single-variant dispatch must not emit cf.switch; ocaml 4.14 runs
decl+print files with no ;; needed.

M5 EXTRACTION REPORT (specimen discipline): ml_core forced (a)
SlotKind::Words in the kernel lowering — the adt-at-closure-boundary
gap D-035 fenced was hit within minutes of real programs, widened
honestly; (b) tree→IR emission — built INSIDE frk-front, the one
component that belongs lower (promotion candidate #1 for M6: any
match-bearing frontend needs it); (c) nothing else — zero private
ops, zero ad-hoc lowerings: the M3/M4 dialects carried a whole ML
subset unmodified, which is the thesis doing its job. Cheats awaiting
promotion: the dtree emission location (above); §6.5 span threading
(every location is unknown — diagnostics point at nothing); the
in-emitter bool-as-sum trick could sink into the dtree layer. The
specimen is already thin; M6 is promotion + documentation, not
surgery.

m4-done — Shipped: frk.closure, the second kernel dialect, K1–K7
under D-031/D-035/D-036/D-037 — and the discovery-driven redesign of
frk.adt that made it possible. First-rank finding: IRDL-22 unifies
constraint variables across ALL positions including within variadic
groups → heterogeneous variadics inexpressible → D-036 packed
surfaces (product_new/product_snoc/make_sum(payload); closure
make(env)/apply(closure, args) → one result); mixed_fields golden
proves the previously-inexpressible case both ways. Closure K1: IRDL
with cross-dialect product refs (combined-module load), deep verifier
resolving callees through a module symbol table. K2: Value::Closure;
apply re-enters eval_function (D-029 guard verified through closure
re-entry); church two(inc)(40)=42 in the interpreter. K4 LIVE:
frk_rt_alloc in frk-rt (extern C, 8-aligned, leaks by design v0;
same-symbol replacement at M7). K3 (D-037): ONE lower-frk-kernel pass
(mutual value nesting forces the merge); slot model int=1/closure=2
(ptrtoint/inttoptr); make → heap env + slot stores + synthesized
thunk (address via func.constant + unrealized cast — addressof can't
name func.func; FuncToLLVM+reconcile fold it); apply → indirect
llvm.call with exact generic attrs (validated against mlir-opt before
writing). Church + counter_fold green under BOTH runners: 15 cases 0
divergent, first contact. K6: docs/dialects/closure.md (incl. the
honest Tier-0-with-asterisk portability note). Learned: mlir-opt
--mlir-print-op-generic is the oracle for builder-emitted attribute
sets; "#builtin.symbol_ref" is FlatSymbolRef's base name; by-value
capture can't tie a recursive knot (that's a feature until frk.mem).
Cheats awaiting promotion: none; deferrals all ledgered (defunc,
by-ref captures, adt-at-closure-boundaries layout oracle, multi-
result closures — every one gated on M7 or a demanding profile).

m3-done — Shipped: frk.adt, the first kernel dialect, full K1–K7
under D-031 (pure IRDL, trait-free, no match op). K1: IRDL definition
(sum/product parametric types; make_sum/tag_of/extract/make_product/
get) + frk semantic verifier (index ranges, result-type-equals-field,
arity/type-vs-shape) wired into every runner's front half. K2: Eval
impls over Value::Adt (wrong-variant extract traps); cf.switch joined
the upstream interpreter; Interp grew the register_eval hook +
eval_util authoring kit. K3 (D-032): lower-frk-adt as an external
MLIR pass, stage 01 in dumps — sum → {i64 tag, i64×K} struct, product
tagless, extui/trunci slot adaptation; integer-fields-≤64 fence until
frk.mem. K5: goldens/adt (4 cases) green under interp AND jit — the
runners= flip caught a real divergence day one (missing
llvm.emit_c_interface: jit had no ciface symbol while interp
answered). Decision-tree pass (D-025/D-034): Maranget matrix→tree,
typed columns, ten byte-exact tree goldens; exhaustiveness/usefulness
tree-derived behind the PatternAnalysis boundary
(rustc_pattern_analysis deferred to M5); IR emission deferred to its
first consumer (ml_core). K4 vacuous (flat by-value structs, zero
runtime; revisit M7) — documented with the rest in
docs/dialects/adt.md (K6). K7: D-031/032/033/034. Learned: melior
0.27.2's ArrayAttribute::try_from is miswired to is_dense_i64_array
(LANDSCAPE-pinned; mlir-sys shim in adt.rs; all other attribute
try_froms audit clean); IRDL constraint variables unify VALUES
(separate vars per independent attribute); external passes + IrRewriter
(via as_rewriter_base) suffice for type-changing lowerings without
DialectConversion. Cheats awaiting promotion: none — but two explicit
deferrals with M5 revisits (rustc_pattern_analysis, tree→IR emission)
and one fence (integer-only adt fields) with an M7 revisit.

m0-done — Shipped: SPEC §12 workspace skeleton (7 crates + sandbox/);
versions.env as the single pin point (RUST_TOOLCHAIN=1.96.0,
LLVM_MAJOR=22, LLVM_VERSION_TESTED=22.1.8, MELIOR_VERSION=0.27.2) with
scripts/check-pins.sh asserting every mirror stays in sync; make
setup|build|test|ci over POSIX scripts (L6); frk-core context() with
eager dialect loading (LANDSCAPE segfault warning); smoke verifier
jit_add_i64 — builder-API add(i64,i64) → convert-to-llvm →
ExecutionEngine, asserts 40+2=42; clean-clone make ci green. Learned:
apt.llvm.org hosts need libmlir-22-dev AND libpolly-22-dev
(llvm-config --libs names Polly; tblgen-rs fails to link without it —
the cairo-native footgun; setup.sh doctor checks both). melior 0.27
API notes: invoke_packed requires llvm.emit_c_interface on the func;
ExecutionEngine::new takes 5 args (enable_pic new); verify() lives on
the OperationLike trait; conversion passes are macro-generated,
create_to_llvm() is the one-shot lowering. Cheats awaiting promotion:
none — no dialect work happened; frnksh is an honest placeholder that
prints its pre-M8 status.

m1-done — Shipped: docs/canon.md v0 + frk_harness::canon (the single
§7.4 filter; CRLF/CR→LF, one trailing LF, i64 rendering, float policy
pinned); golden engine (goldens/<suite>/<case>/ layout, strict
`// frk-case:` directives, check writes gitignored output.actual,
bless reports changed/unchanged, zero-case corpus is an error);
6-case upstream corpus (arith/scf.for-i64/scf.if/cf.cond_br/func.call)
with expected outputs computed by hand before the runner ran, syntax
validated via mlir-opt 22; differential scaffold (diff_corpus, BTreeMap
matrix, L3 escalation text in the report) live on the corpus via
default_runners(); stage dumps (00-parsed + NN-<pass> per shared
pipeline table, out dir recreated whole, never goldened — docs/
stages.md); frnksh test/bless/diff/emit + make bless|diff. Ledger:
D-027 (custom golden runner, format, entry protocol, pipeline),
D-028 (single-pass-manager stage dumps). Learned: scf.for takes i64
induction directly (no index detour needed); the pipeline
scf-to-cf → convert-to-llvm → reconcile-unrealized-casts covers all
six cases; melior pass constructors are macro-named from C API symbols
(strip Conversion/Convert/Pass, snake-case). Cheats awaiting
promotion: none. Known wart (accepted for v0): `emit --stages` default
out dir uses the source file stem, so corpus files (all named
case.mlir) collide on out/stages/case — pass --out for those.

m2-done — Shipped: frk-interp — the K2 Eval trait + derived
interpreter over func/arith/scf/cf (value domain: two's-complement
ints ≤64 bits; per-call Frame keyed by MLIR value identity; multi-
block CFG execution; single-block structured regions; symbol-indexed
func.call). Semantics ruled in D-029: total & deterministic — UB
traps (div-by-zero, signed-div overflow, non-positive for-step), call
depth caps at 1024; corollary: corpus must be UB-free (goldens/README
rule added). Harness: InterpRunner on a STACK_SIZE thread;
default_runners()=[interp, jit]; reference_runner()=interp (D-008
assumed in full — blessing writes reference bytes). Corpus at 8 cases
(+fib_rec recursion, +add_wrap modulo-2^64 canary). L3 IS ENFORCED IN
CI: make test two-way-diffs every golden; make diff prints the
matrix (8 cases, 0 divergent). Learned: interpreted frames cost a few
KiB of host stack each — depth-ceiling programs need the STACK_SIZE
thread (libtest's 2 MiB default overflows); melior successor operand
splitting needs no segment attribute (destination arg counts
suffice); i1 sign-extends to -1 (tested, it will bite someone).
Cheats awaiting promotion: none — but note the interpreter recurses
on the host stack; if a specimen needs unbounded depth, that's a
rework flag, not a knob.

## Handoff template (copy for every session end)
    Session end: <date>
    Milestone/step: <where>
    Green? <yes/no — if no, why and where>
    Did: <bullets>
    Next: <single concrete action>
    Landmines: <anything the next agent must not step on>

## Session log

    Session end: 2026-07-03 (tenth entry)
    Milestone/step: M5 complete, tagged m5-done
    Green? yes — 25 blocks; clean-clone exit 0; 33 cases 0 divergent
    three-way; dashboard 100% × 3 for ml_core
    Did:
    - harness source kinds + OcamlOracle + Runner::applicable +
      dashboard; versions.env OCAML pin + setup check
    - 18-case ml_core corpus; first oracle triangulation green on
      first contact
    - D-038 (⚑ ×3), MANIFEST status, extraction report
    Next: M6 promotion pass per the Next-action block
    Landmines:
    - the ocaml oracle appends print_int (main ()) — a corpus file
      that already prints would double-print; keep corpus files pure
    - dashboard denominators exclude skips; NothingApplies runners
      simply lose their column — do not "fix" that into zeros

    Session end: 2026-07-03 (ninth entry)
    Milestone/step: M5 first half — frk-front built, interp-green
    Green? yes — 25 blocks; diff 15 cases 0 divergent (one
    intermediate commit pushed red for a stale fence test; corrected
    and repushed in the next commit — the fence had LIFTED by design)
    Did:
    - frk-front: lexer, parser (desugaring), types+infer (ena, real
      let-poly with recorded instantiations), emit (dtree consumer,
      cf-CFG, lambda lifting + rec re-make, Words boundary crossing)
    - kernel_lower: SlotKind::Words (nested adts + closure-boundary
      adts as verbatim word copies); fence tests track the new law
    - ten e2e batteries; found+fixed zero-case-switch emission bug
    Next: M5 second half per the Next-action block (harness .ml,
    ocaml oracle, corpus, dashboard, D-038, close)
    Landmines:
    - melior cf::cond_br/switch helpers use pre-MLIR-22 attr names
      (operand_segment_sizes); emit.rs builds branches generically
      with operandSegmentSizes — do not "simplify" back to the helpers
    - single-variant dispatch must not emit cf.switch (zero-case
      vector<0xi64> is illegal) — emitted inline; test covers it
    - ocaml oracle wrapper appends print_int (main ()) — corpus files
      MUST define main () and stay ocaml-runnable verbatim

    Session end: 2026-07-03 (eighth entry)
    Milestone/step: M4 complete, tagged m4-done
    Green? yes — make test 24 blocks; clean-clone ci.sh exit 0;
    make diff 15 cases 0 divergent; runners= rot check clean
    Did:
    - K2: Value::Closure + closure evaluators; church 42 in interp;
      depth guard proven through closure re-entry
    - K4 live: frk_rt_alloc (leaks by design v0, D-035)
    - K3 (D-037): merged lower-frk-kernel pass — slot model
      int=1/closure=2, heap envs, synthesized thunks, func.constant+
      cast for addresses, exact-attr indirect llvm.call; JIT registers
      frk_rt_alloc; church + counter_fold flipped and green BOTH ways
    - K6 page docs/dialects/closure.md; D-037 ledgered
    Next: M5 ml_core per the Next-action block — READ
    specimens/ml_core/MANIFEST.md first (protocol step 5 is now real)
    Landmines:
    - the closure {ptr,ptr} inside adt slots is ptrtoint'd — any
      future pass reordering around lower-frk-kernel must keep that
      pass FIRST in the table
    - mlir-opt --mlir-print-op-generic before hand-building ANY llvm
      op with attributes; the attr sets are non-obvious and versioned
    - thunk names __frk_thunk_N are reserved; frontends must not emit
      colliding symbols

    Session end: 2026-07-02 (seventh entry this session)
    Milestone/step: M4 in flight — D-035/D-036 ruled; K1 done for
    frk.closure; frk.adt REDESIGNED to the packed surface
    Green? yes — make test 23 blocks; make diff 13 cases 0 divergent
    Did:
    - discovered the IRDL variadic-unification ceiling (first-rank:
      mixed-type make_sum NEVER worked; M3 corpus was uniform by luck);
      pinned in LANDSCAPE, ruled D-036 (no variadics — packed surfaces)
    - redesigned frk_adt: product_new/product_snoc/make_sum(payload);
      rewrote IRDL, verify, eval, lowering, all tests, all goldens;
      added the mixed_fields golden (the previously-inexpressible case)
    - frk.closure K1 on the packed surface: IRDL (cross-dialect
      product refs via combined-module load), deep semantic verifier
      (symbol table in the verify driver), 6 smoke tests incl. the
      full church shape verifying clean
    - D-035 ruled ahead of the remaining work (env+fnptr on frk-rt
      heap; defunc/by-ref deferred; boundary fence)
    Next: M4 step 1 in the Next-action block (K2 closure eval)
    Landmines:
    - IRDL variables unify EVERYWHERE, including within a variadic
      group and across operand/result positions — never share one, and
      never add a variadic group to a kernel op (D-036)
    - both dialects must load as ONE IRDL module (cross-dialect
      @frk_adt::@product refs); register() already does this
    - "#builtin.symbol_ref" is the base name for FlatSymbolRef; there
      is no flat_symbol_ref registered name

    Session end: 2026-07-02 (sixth entry this session)
    Milestone/step: M3 complete, tagged m3-done
    Green? yes — make test 22 blocks; clean-clone ci.sh exit 0;
    make diff 12 cases 0 divergent
    Did:
    - K1 (IRDL + semantic verifier in every runner), K2 (Eval impls,
      Value::Adt, cf.switch), K3 (D-032 external lowering pass +
      corpus flip to two-way — caught the emit_c_interface divergence),
      K5 (adt corpus + D-033 runners= machinery), decision-tree pass
      (D-025/D-034, ten tree goldens), K4 vacuous note, K6 page
      (docs/dialects/adt.md), K7 (D-031..034)
    - melior ArrayAttribute::try_from bug found, shimmed, pinned
    Next: M4 frk.closure per the Next-action block (trait-free op
    design, by-value captures v0, defunctionalization deferred —
    ledger both; church-encoding + counter goldens are the exit)
    Landmines:
    - adt lowering fences integer fields ≤64; closures capturing adt
      values will hit it — decide widen-vs-fence EARLY, ledger it
    - unguarded frk_adt.extract: interp traps, lowered code reads
      garbage — never admissible as a golden (D-032)
    - dtree goldens are literal strings in tests/adt_dtree.rs; a
      heuristic change re-blesses them with an L2 justification
    - the runners= directive rot check (D-033): grep goldens for
      runners= at every milestone exit — today it greps clean

    Session end: 2026-07-02 (fifth entry this session)
    Milestone/step: D-030 struck per human ruling; D-031 appended
    Green? yes — docs-only change; make test green (17 result blocks)
    Did:
    - appended D-031 (supersedes D-030): IRDL-only registration,
      trait-free dialect designs, match de-regioned to pure ops +
      cf.switch + matrix→IR decision-tree pass
    - amended SPEC §3 K1 (no C++/ODS in v1; pass-hosted verifier for
      deep invariants) and §4.1 (frk.adt op set)
    - LANDSCAPE tier wording fixed; registration.rs header now cites
      D-031; STATE next-action rewritten as the 6-step M3 build order
    Next: M3 step 1 — frk_adt IRDL definition + register() +
    parse/verify smoke (extend registration.rs's pattern)
    Landmines:
    - D-030 stays in the ledger struck-but-visible; never edit it —
      the strike lives in D-031's first line
    - kernel dialect designs must stay trait-free (no custom
      terminators/successors/region ops) — check D-031 before
      sketching any new dialect's op set

    Session end: 2026-07-02 (fourth entry this session)
    Milestone/step: M3 step 0 — dialect-registration ruling (D-030)
    Green? yes — make test green (registration spike adds 5 tests)
    Did:
    - spiked IRDL end to end via mlir-opt AND melior: definitions,
      dynamic parametric types, generated verifiers (arity + type
      variables + attribute kind), builder-path construction — all work
    - found the ceiling: LLVM 22 IRDL has no trait declarations —
      dynamic ops can't be terminators or carry successors; that blocks
      region-based match under pure IRDL
    - verified the C++ escape hatch is provisioned (headers +
      MLIRConfig.cmake + cmake/ninja on apt; brew ships same)
    - ruled D-030 (Tier A IRDL / Tier B C++ shim); pinned the ceiling
      in LANDSCAPE + a watch item to re-fold B into A
    Next: Tier-B shim skeleton (cmake lib + frk_adt ODS + handle
    registration in frk_core::context()), smoke-verified, then K2–K7
    Landmines:
    - `irdl.is i64` constrains attribute-EQUALS-type-i64; use
      `irdl.base "#builtin.integer"` for "an integer attribute"
    - irdl.parametric needs fully nested refs (@dialect::@type)
    - the registration spike test doubles as D-030's standing evidence;
      if an MLIR bump breaks it, revisit the ruling before patching it

    Session end: 2026-07-02 (third entry this session)
    Milestone/step: M2 complete, tagged m2-done
    Green? yes — make test green (incl. two-way diff on all 8 goldens);
    clean-clone scripts/ci.sh exit 0
    Did:
    - frk-interp: Eval trait, value domain, CFG/structured execution,
      upstream evaluators (arith/func/cf/scf), 20 verifiers
    - D-029 (trap semantics + depth ceiling); UB-free corpus rule
    - InterpRunner wired as reference; corpus → 8 cases; L3 live in CI
    Next: M3 frk.adt — but FIRST settle the custom-dialect registration
    mechanism (IRDL vs unregistered ops vs C++ shim) as a D-entry; it
    shapes everything K1 onward
    Landmines:
    - deep interpretation needs frk_interp::STACK_SIZE threads; never
      call interpret_entry on a default 2 MiB test thread for recursive
      programs
    - blessing now writes INTERPRETER bytes; if jit disagrees afterward
      that is an L3 first-rank finding, not a blessing mistake
    - i1 as_signed() is -1, not 1 — use as_bool()/as_unsigned() unless
      you really mean sign extension

    Session end: 2026-07-02 (second entry this session)
    Milestone/step: M1 complete, tagged m1-done
    Green? yes — make test green; clean-clone scripts/ci.sh exit 0
    Did:
    - canon contract + filter; golden engine + 6-case corpus; diff
      scaffold; stage dumps; frnksh subcommands; make bless|diff
    - D-027, D-028 appended; goldens/README.md + docs/canon.md +
      docs/stages.md written
    Next: M2 — Eval trait + interpreter over func/arith/scf/cf; append
    it to default_runners() and flip reference_runner() (D-008); exit =
    L3 enforced in CI
    Landmines:
    - run cargo via make (exports MLIR_SYS_220_PREFIX/TABLEGEN_220_PREFIX)
    - melior is alpha: build contexts via frk_core::context() only
    - never bless without an L2 justification line in the commit; the
      bless report prints changed/unchanged per case to keep you honest
    - goldens comparison happens ONLY through frk_harness::canon — do
      not add a second normalization anywhere

    Session end: 2026-07-02
    Milestone/step: M0 complete, tagged m0-done
    Green? yes — make test green; clean-clone make ci exit 0
    Did:
    - workspace skeleton per SPEC §12; versions.env + check-pins.sh
    - melior =0.27.2 pinned; frk-core context(); smoke jit_add_i64 green
    - installed libmlir-22-dev + libpolly-22-dev on this host; setup.sh
      doctor now checks for both so the next machine gets named hints
    - clean-clone make ci verified; README status refreshed; tagged; pushed
    Next: M1 harness v0 (SPEC §7): golden runner + bless + docs/canon.md
    v0 + stage dumps + differential scaffold
    Landmines:
    - run cargo via make (it exports MLIR_SYS_220_PREFIX /
      TABLEGEN_220_PREFIX); bare cargo without those exported will pick a
      wrong or absent LLVM
    - melior is alpha: build contexts via frk_core::context() (eager
      dialect loading) — unloaded-dialect access can segfault (LANDSCAPE)
