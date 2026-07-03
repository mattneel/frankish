# STATE — frankish live handoff

Updated: 2026-07-03 (M0..M11 sessions)
Phase: M11 complete (tag m11-done). Three languages ride the kernel,
each against its own upstream oracle.
Tree: green — `make test` 35 blocks; diff 64 cases 0 divergent
(SEVEN runners: interp, jit, jit-rc, ocaml, node, lua, repl); grid
59/59 × 4 triples × 2 strategies; canary s390x 59/59 × 2; dashboard
10 suites × 7 runners, 100% everywhere applicable.

## Next action
M11 is closed. The remaining unscheduled queue (the human's next
pick, or an L4-logged choice among peers):
1. The GC ladder's remaining rungs (D-053/D-055 sequencing): sized
   releases → THE LAYOUT-DESCRIPTOR RUNG (named, designed-not-
   discovered) → candidate buffer + trial deletion → thresholds.
   Tables/closures now exist to collect; the counters are live.
2. femto_lua v0.2 (fence lifts by admission rule: multiple returns,
   varargs, nil-fill arity, repeat/break, generic for + pairs/
   ipairs — needs multi-value call plumbing; string library slice).
3. scheme/ctl track (r7rs_core; tail calls as law).
4. Effects lowering (D-012); frk.stage; TS-1..4; gpu axis.
D-045's effects trigger is now ARMED IN PRINCIPLE: the shell cannot
yet load .lua, but the interp output buffer + Lua IO exist — the
moment the REPL grows a Lua mode, D-043's replay model must be
revisited FIRST (the ledger says so; do not let it be discovered).

## In flight
Nothing.

## For the human
- SEQUENCING: M0–M10 are done — the scheduled program is complete.
  The beyond-M10 tracks (femto_lua implementation, the GC ladder,
  scheme/ctl, effects, stage/TS-1..4/gpu) are peers; picking the
  next one is the taste call the constitution routes to you. The
  Next-action block lists them with their debt annotations. Default
  recommendation if you want one: femto_lua implementation + the GC
  ladder interleaved — they share the runtime dragon and every
  ledger debt (D-041/D-045/D-049/D-051/D-053) points at that pair.
- Review D-051 (fat values over NaN-boxing), D-052 (Lua byte-string
  ruling, v0.1 scope), D-053 (rc+cycles over MMTk; docs/gc-spike.md
  is the full argument) — the three M10 rulings, none flagged ⚑
  (all were L4 calls inside ruled territory), all cheap to strike
  now and expensive later.

## Milestone log
m11-done — Shipped: femto_lua v0.1 + the GC ladder's first rung —
the track the human picked ("Do it"), all four D-054 bars.
(1) dyn K3: fat-value lowering, boxed multi-word payloads through
the strategy allocator, straight-line rt tag checks proven at
interp (located traps) + AOT (subprocess abort); fence lifted.
(2) GC step 1: block-local liveness releases + release counters in
both twins + THE LEAK CANARY (3 allocated, 3 released, at runtime).
(3) The frontend: hand-rolled 5.1 lexer/parser; all-dyn emission;
locals as boxes (upvalue sharing = box identity; local-function
recursion through the box); lambda lifting with (_G, boxes, params);
the Lua protocols as SYNTHESIZED IR (truthiness with truthy-0,
tostring, print, tag-pair eq, concat coercion, length, __index with
BOTH forms — function form through frk_closure.apply); floor-mod
from trunc+fixup; value-returning and/or. Kernel prerequisites built
en route: frk_bstr (interned byte strings — eq is an inline pointer
compare, len an inline header load; intern tables in both twins) and
the frk_dyn raw-table surface (pure-hash dyn-keyed maps with
tombstones + border probe, out-pointer ABI, inline meta slot,
payload_word for identity). (4) canon §7: %.14g with the rounding
contract — cross-twin parity rig on deliberate half-even ties, and
the corpus tie case printing 12345678901234 through THREE printers.
RESULTS: 8/8 vs lua5.1 (bar ≥90%); diff 64 cases 0 divergent across
SEVEN runners; grid 59/59 × 5 architectures × 2 strategies;
dashboard 10 suites 100% everywhere applicable.
Verifier finds: melior's StringAttribute::value() is UB on ""
(LANDSCAPE-pinned; all text reads now via printed-form unescape);
the missing-setmetatable bug announced itself with a SOURCE LOCATION
(§6.5 paying rent live: "dyn tag mismatch at case.lua:2:29").

M11 EXTRACTION REPORT: femto_lua forced (a) frk_bstr — the second
rt-value dialect, and the interning representation that turns eq
into pointer arithmetic; (b) the raw-table surface + the
SYNTHESIZED-PROTOCOL pattern (D-056.2) — the deepest design win of
the milestone: metatable dispatch is ordinary IR, so it rode interp/
jit/aot/all-five-triples with ZERO per-runner work, and the pattern
generalizes (TS-1 narrowing checks, scheme dispatch); (c)
payload_word (identity comparison); (d) the empty-string UB find;
(e) NOTHING in adt/closure/mem — three specimens in, the M3-M7
kernel carries dynamic Lua unmodified. Private ops: zero, again.
Cheats: the fn-eq structural-vs-identity divergence (fenced), table
internals malloc-domain (owed to the layout rung), aggregates
retain-invisible under rc (owed to the same rung).

m10-done — Shipped: the gate milestone, exactly as scoped. GC gate:
docs/gc-spike.md (the written comparison SPEC demands) + D-053 —
rc+cycles wins on the two-twin runtime and the five-triple grid;
MMTk stays Tier-2 with measured revisit conditions. Tagging fork:
D-051 — fat values v0, representation-swap-later; frk_dyn K1/K2
shipped with wrong-tag traps carrying source locations from birth,
interp-fenced golden, K6 page; K3 scheduled with implementation.
Manifest: femto_lua RATIFIED (D-052) with the Lua string ruling
(interned identity-equal byte strings, NOT frk_str) and the 5.1.5
oracle installed and pinned. Exit bars: manifest ratified ✓, dyn
contract underway ✓, GC decision logged ✓. THE SCHEDULED PROGRAM
IS COMPLETE: M0 through M10, eleven milestones, one session-day,
zero red pushes surviving uncorrected. Learned: gate milestones
work — writing the spike report BEFORE code surfaced the two-twin
and grid constraints as decisive within a paragraph, where code
first would have burned a week discovering MMTk's wasm gap the
hard way.

m9-done — Shipped: loanword v1 + the whole TS-0 stage. The freeze
(D-046): canonical JSON, sha-256 content id (refusal PROVEN by
tampered fixture, D-050.2), version-gated vocabulary, self-contained
artifacts; §6.5 landed and WITNESSED (OOB trap at oob.ts:4:13,
D-050.3). Producer: tools/loanword-ts, tsc 6.0.3 checker-as-oracle +
noImplicitReturns (the D-050.1 corollary: the oracle's flags are
part of the oracle). Consumer: frk_front::loanword with span→
Location threading. TS-0 semantics: f64 (first float, SlotKind::F64),
mutable lets as boxes, arrays in frk.mem (OOB=trap+location,
stricter-than-JS by ruling), strings as rt-owned UTF-16 in both
twins (code-unit ruling fired at .length: 😀.length=2 vs V8
everywhere), three-way print protocol under canon §6's fence. Sixth
runner: node. Exit bars: TS-0 manifest 100% ✓; diff green ✓ (52
cases, 0 divergent, 6 runners); grid 47/47 × 5 architectures × 2
strategies. Startup number recorded with D-050.5's framing. Ledger:
D-046..D-050 (+D-045 amendment). Learned: melior binary_operands
insists on int widths; noLib needs the classic global-interface
prelude; wasm's exact import signatures catch ABI drift nothing else
does; reviews and implementations can cross in flight — record the
crossing honestly instead of unwinding shipped evidence.

M9 EXTRACTION REPORT: TS forced (a) f64 + the F64 slot kind (the
admission rule working exactly as designed — float entered carrying
IEEE-vs-print semantics ml_core never had); (b) arrays into frk.mem
(the first NEW kernel surface a specimen demanded since M7);
(c) frk_str — the first rt-VALUE dialect (immutable, rt-owned,
outside the strategy axis: a genuinely new kind of kernel resident);
(d) the managed/unmanaged pointer split (a CORRECTNESS hole the rc
strategy had been walking toward since M7 — strings found it);
(e) host builtins + the interp output buffer (which is also the
first thing D-045's effects trigger will point at); (f) nothing in
closures/adts — TS-0 never touched them, by design. Cheats awaiting
promotion: none private; the per-case node producer invocation is a
cost note, not a cheat. The loanword freeze held: strings/arrays
entered as ADDITIVE vocabulary, no version bump.

m8-done — Shipped: the frankish shell (SPEC §9; semantics D-043).
Bare frnksh = REPL on the reference interpreter; re-elaborate-whole
session model; typed value rendering; poly exprs as schemes without
emission; :type/:load/:emit/:profile; frnksh run FILE. MainPolicy
::OptionalAny + lenient zonk in the frontend. Transcript goldens (5)
as SourceKind::Transcript + the repl runner driving the exact library
engine. Exit bar (transcript goldens green) met AS AMENDED by the
human review (D-044.4): every shell error echoes the offending line,
proven by the division-trap golden. The review also ratified D-041
(rider: frk_rt_alloc_count in both twins), D-038 (manifest scope line
amended), D-005 (with prejudice — stack closed by evidence). ORC
per-cell redefinition: scoped out as the stretch; the re-elaboration
model makes it a later performance upgrade, not a semantic change.
Learned: NothingApplies needed downgrading to a noted skip for
kind-homogeneous corpus subsets; :load error text must name the
requested file only (resolved paths are cwd-dependent, portability);
the trap message carries the op name (arith.divsi: division by zero)
— keep it, it is the closest thing to a location until M9 spans.

m7-done — Shipped: the memory axis and the world. frk.mem (third
kernel dialect, K1-K7): box_new/get/set over !frk_mem.box<T>;
Strategy ∈ {Arena, Rc} as a LOWERING PARAMETER (D-041 ⚑ — rc v0
retains at owning stores with SSA transfer elision, no releases until
the M10 liveness pass); strategy runtimes behind the C ABI
(arena_alloc; rc_alloc/retain/release, header at ptr-8); SlotKind::
Ptr; Value::Box (shared cell, identity eq); jit-rc as fourth default
runner. The Tier-0 grid (D-042): AOT via entry-rename + mlir-translate
+ pinned-clang IR→object + zig-cc link with the C runtime mirror;
musl-static; qemu-user/wasmtime execution. RESULTS: every golden ×
both strategies byte-exact on FIVE architectures — x86_64, aarch64,
riscv64, wasm32-wasi, and the s390x big-endian canary, 37/37
everywhere. Exit bar (grid green for ml_core under both strategies):
exceeded — the whole corpus, plus the canary. Verifier finds this
milestone: (1) retain sharing decided mid-rewrite always read as
transfer (fixed: resolved pre-rewrite); (2) wasm signature_mismatch
on size_t vs i64 allocator sizes (fixed: frk ABI says u64 sizes on
every target). Learned: zig-on-PATH may be an anyzig shim (zigcc.sh
handles both); LLVM-22 .ll needs llvm-22's clang for codegen (zig
links only); wasm enforces exact import signatures — the grid's
whole point, catching ABI drift the same-word-size hosts never see.

M7 EXTRACTION: the grid forced (a) the u64-size ABI clause — a real
portability law found by the narrowest target; (b) the C runtime
mirror pattern (per-triple zig compile beats rust cross-toolchains at
this runtime size; revisit when frk-rt grows past trivial); (c) the
entry-rename protocol. Nothing else bent: the SAME lowered IR that
JITs on x86_64 runs interpreted, JIT'd, and AOT'd on five targets
with zero per-target conditionals in the lowering — the strategy/
triple matrix is pure configuration.

m6-done — Shipped: Promotion pass #1, light exactly as the M5
extraction report predicted. The centerpiece: tree→dispatch-IR
emission promoted from frk-front into frk_dialects::dtree_emit — the
seam is arm-emission-only because occurrence typing derives from the
kernel types themselves (decode_sum/decode_product); bool dispatch
and the single-variant-inline rule moved down with it; the component
is verified frontend-free (hand-built module, callback arms,
interpreted). frk-front shrank by five private fns. Exit bars: no
private ops in ml_core (true before, truer now); conformance not
worse (33 cases, 0 divergent, three-way; dashboard 100% × 3
unchanged). docs/type-kit.md documents the kit split: what travels
(ena unification pattern, schemes with recorded instantiations, the
value-restriction predicate, zonking discipline) vs what stays
per-frontend (the Ty language, constructor law, kernel spelling) —
with the M9 rule: don't abstract the type language on one data point.
Ledger: D-039 (green tree deferred with a named M9 trigger — one
specimen's evidence is coin-flipping), D-040 (D-009 retrospective:
specimen order CONFIRMED; abstraction risk retired; dragon still
asleep). Cheats awaiting promotion: none — the queue is empty.

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

    Session end: 2026-07-03 (nineteenth entry)
    Milestone/step: M11 complete, tagged m11-done
    Green? yes — 35 blocks; 64 cases 0 divergent (7 runners); grid
    59/59 × 5 × 2; lua 8/8 vs lua5.1
    Did:
    - canon §7 + parity rig (D-055 executed); frk_bstr; raw tables;
      payload_word; lua frontend + synthesized protocols; LuaOracle;
      8-case corpus; melior empty-string UB pinned + dodged
    Next: unscheduled queue (human pick) per Next-action
    Landmines:
    - StringAttribute::value() is UB on "" — ALWAYS read text attrs
      via attr_util::string_attr_bytes
    - lua stdlib globals are SEEDED (print/tostring/setmetatable/
      getmetatable); an unseeded global read is nil → tag-mismatch
      trap AT THE CALL SITE (correct but puzzling if you forget)
    - the lua REPL mode is BLOCKED on D-045 revisit (replay model)
      before any implementation

    Session end: 2026-07-03 (eighteenth entry)
    Milestone/step: M11 bars 1–2 (dyn K3 + GC step 1)
    Green? yes — 34 blocks; 54 cases 0 divergent; native grid 49/49
    Did:
    - D-054 (milestone contract from the human's pick); dyn K3 with
      the boxed-payload arm + AOT abort-path subprocess test; GC
      step 1 releases + release_count in both twins + leak canary
    Next: bars 3–4 per Next-action (canon fence first, then the
      frontend, then bstr, then tables)
    Landmines:
    - dyn tag mismatch under IN-PROCESS JIT aborts the harness;
      corpus law: no mismatch cases in jit-run goldens (the checked
      path is verified at interp + AOT levels instead)
    - the release pass anchors on TERMINATORS (they survive
      rewriting); if a future plan ever replaces a terminator, the
      anchors die — revisit the anchor scheme then
    - escape analysis counts func.call operands as escaping (callee
      may store); releasing across calls needs the real liveness
      pass (ladder step 2+)

    Session end: 2026-07-03 (seventeenth entry)
    Milestone/step: M10 complete, tagged m10-done — SCHEDULED PROGRAM
    COMPLETE (M0..M10)
    Green? yes — 32 blocks; 53 cases 0 divergent; grid/canary green
    Did:
    - docs/gc-spike.md + D-053 (rc+cycles; MMTk Tier-2)
    - D-051 fat values + frk_dyn K1/K2 + located traps + fenced
      golden + K6; D-052 manifest ratification + Lua string ruling;
      lua5.1 oracle installed/pinned/doctored
    Next: BLOCKED-BY-DESIGN on sequencing (For the human) — peer
    tracks need a human pick or a logged L4 call; recommendation
    recorded (femto_lua impl + GC ladder interleaved)
    Landmines:
    - frk_dyn has NO K3: dyn cases must carry runners=interp until
      the femto_lua implementation milestone lands the lowering
    - the dyn tag space is CLOSED at six; TS-1 unions will want tags
      — that widening is a D-entry, not an edit
    - lua5.1 number printing is %.14g — a canon fence like TS-0's
      §6 is REQUIRED before the first femto_lua golden

    Session end: 2026-07-03 (sixteenth entry)
    Milestone/step: M9 complete, tagged m9-done
    Green? yes — 31 blocks; 52 cases 0 divergent; grid 47/47 × 5 × 2;
    TS-0 manifest 100%
    Did:
    - strings (UTF-16 both twins, surrogate goldens), arrays (frk.mem,
      OOB trap + location), Ptr managed/unmanaged split, D-049/D-050,
      second review integrated (noImplicitReturns, tamper refusal,
      §6.5 witness, Static Hermes framing, fuzz-fence note)
    Next: M10 femto_lua per Next-action — GC SPIKE REPORT FIRST
    Landmines:
    - the strings steer crossed implementation in flight; D-050.4
      records the crossing — check STATE timing before deferring
      rulings the code may already have made
    - frk_str pointers are UNMANAGED (no rc header); any new
      rt-owned pointer type must join the unmanaged arm or rc
      corrupts ptr-8
    - noLib preludes need the full classic global-interface set or
      array literals type as {}

    Session end: 2026-07-03 (fifteenth entry)
    Milestone/step: M9 first half — loanword frozen, TS-0 slice-1
    green everywhere, startup number taken
    Green? yes — 30 blocks; 48 cases 0 divergent (6 runners); grid
    43/43 × 5 architectures × 2 strategies
    Did:
    - D-046 (freeze), D-047 (ts conventions), D-048 (green tree
      resolved: not adopted), D-045 (effects-trigger amendment on
      D-043, human directive)
    - producer/consumer/corpus/oracle/prints as summarized in
      Next-action; fib 17.7× vs node
    Next: strings + arrays → manifest 100% → m9-done
    Landmines:
    - binary_operands INSISTS on int widths — float evaluators use
      float_operands; adding int ops that touch floats will bite
    - interp builtins answer only ABSENT-or-BODYLESS symbols; a
      module-level func.func with a body always wins
    - the AOT shim is per-kind (ts = void variant); the i64 shim on
      a void entry prints garbage — found by the ts0 grid
    - producer runs are per-case node invocations (~60ms); cache if
      corpus growth makes it felt
    - canon §6 fence: printed values 0 or |v| ∈ [1e-4, 1e15) finite;
      widening it is a canon change (D-entry)

    Session end: 2026-07-03 (fourteenth entry)
    Milestone/step: M8 complete, tagged m8-done
    Green? yes — 30 blocks; transcripts 5/5
    Did:
    - frk-repl crate (engine, pretty types, typed rendering,
      transcript driver); frnksh bare=REPL + run FILE; harness
      Transcript kind + repl runner; five transcript goldens
    - frontend: MainPolicy::OptionalAny, lenient zonk, main_result,
      emit generalized to any concrete main result
    - integrated the first human ⚑ review (D-044): three
      ratifications + riders + the M8 error-echo exit amendment
    Next: M9 (loanword + TS-0) per Next-action — big one: green-tree
    decision fires, span threading due, first float
    Landmines:
    - REPL classification is parser-driven (decl-parse then
      expr-wrap) — never token-sniff; `let x = 1 in x` is an EXPR
    - lenient_zonk is REPL-only; batch compilation still hard-errors
      on ambiguity — do not leak OptionalAny into compile_ml
    - transcript expected.out contains OS error text ("No such file
      or directory (os error 2)") — Linux/macOS agree; Windows would
      not (frontier concern, not Tier-0)

    Session end: 2026-07-03 (thirteenth entry)
    Milestone/step: M7 complete, tagged m7-done
    Green? yes — 27 blocks; 37 cases 0 divergent 4-way; grid 37/37 ×
    4 triples × 2 strategies; canary s390x 37/37 × 2
    Did:
    - D-042; AotRunner (entry rename, mlir-translate, pinned-clang
      object, zigcc.sh link, C runtime mirror); frnksh grid
      [--canary|--native]; make grid/grid-native/canary; ci native
      slice; doctor entries (mlir-translate/zig/qemu/wasmtime);
      ZIG_VERSION=0.16.0 pin (user: 0.14 is ancient — 0.16.0 is
      current stable, verified against ziglang.org)
    - grid found the size_t/i64 ABI mismatch on wasm; frk ABI now u64
      sizes everywhere
    Next: M8 (the shell) per Next-action
    Landmines:
    - zig on PATH here is an anyzig shim: bare `zig version` FAILS;
      zigcc.sh resolves plain-vs-shim against ZIG_VERSION — never
      call zig directly in scripts
    - .ll from LLVM 22 must be compiled by $MLIR_PREFIX/bin/clang
      (zig's bundled LLVM may lag); zig only links
    - AOT renames entry→frk_entry PRE-lowering; corpus entry symbols
      must stay externally-invoked-only (goldens README notes it)
    - wasmtime lives at ~/.wasmtime/bin if not on PATH (runner
      resolves both)

    Session end: 2026-07-03 (twelfth entry)
    Milestone/step: M7 first half — frk.mem + strategy knob shipped
    Green? yes — 27 blocks; diff[interp,jit,jit-rc,ocaml] 37 cases 0
    divergent; dashboard 100% × 4 × 5
    Did:
    - D-041 (⚑); frk_mem dialect K1-K7; Strategy{Arena,Rc} lowering
      param; frk-rt strategy ABI (arena/rc); rc retain + transfer
      elision; SlotKind::Ptr; Value::Box; jit-rc runner; mem goldens
    - bug found by verifier: sharing decided mid-rewrite always read
      as transfer; now resolved pre-rewrite
    Next: the grid per Next-action (AOT runner first)
    Landmines:
    - retain sharing MUST be decided before any rewriting: op
      replacement rewrites operands in place; mid-rewrite use-count
      lookups miss and silently elide every retain
    - retain assertions must match "llvm.call @frk_rt_rc_retain" —
      the declaration alone contains the bare symbol name
    - frk_rt_alloc is GONE; four strategy symbols registered in JIT

    Session end: 2026-07-03 (eleventh entry)
    Milestone/step: M6 complete, tagged m6-done
    Green? yes — 26 blocks; 33 cases 0 divergent three-way; clean
    clone verified at commit time
    Did:
    - promoted dtree emission (frk_dialects::dtree_emit) + its
      frontend-free verifier; frk-front delegates
    - docs/type-kit.md; D-039 (green tree, M9 trigger), D-040 (D-009
      retrospective: order confirmed)
    Next: M7 per the Next-action block — frk.mem surface design FIRST
    (unadjudicated; ledger before code), grid second
    Landmines:
    - dtree_emit callbacks receive (arm_entry, arm, bindings) and must
      return the EXIT block — nested matches inside arms split blocks
    - four M7 debts are now due together (arena-behind-frk_rt_alloc,
      by-ref captures, recursive ADTs, boxed reps) — resist solving
      them piecemeal; one memory design covers all four

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
