# STATE — frankish live handoff

Updated: 2026-07-03 (M0..M14 sessions)
Phase: M29 complete (tag m29-done). TS-2 SHIPPED AND FROZEN
(D-075): structural interfaces on D-026's itabs (dictionary interp
/ real-itab native, the matrix arbitrates) + object closures
(arrows onto frk_closure, captures by binding). Five specimens,
five languages, one kernel.
Tree: green — `make test` 53 blocks; diff 104 cases 0 divergent (8
runners); grid 99/99 × BOTH strategies × 4 triples + s390x canary.

## Next action
M29 closed; TS-2 frozen. The queue:
1. parameterize: r7rs v0.3 — make-parameter/parameterize (global
   cells rung), guard sugar, plain raise.
2. pairs-mut: r7rs structured data — set-car!/set-cdr!, strings,
   vectors.
3. tier-2: the stack-switching rung (re-entrant κ / winds), at
   coroutines.
4. ts-3: async/await via the ported tsc downlevel state-machine
   transform + exceptions (the manifest's next stage; wants the
   effects lane it now has).

## In flight
Nothing.

## For the human
- RESOLVED (2026-07-03): the Rocq anchor — delegated back ("Atli is
  my code… you don't need attribution"); κ_frk authored in-repo,
  D-060. inscription + flexlang also registered as minable in-house
  art. If any κ_frk ruling misreads atli's intent (esp. the
  keystone: multi-shot call/cc stays fenced), strike via D-entry.
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
m29-done — Shipped: TS-2's second half; THE STAGE FREEZES (D-075).
Structural interfaces cash D-026: !frk_dyn.iface = {obj, itab}
two-slot; iface_make(box){methods} at sealed-world conversion
sites; iface_call(iface, pack){k}. TWO REPRESENTATIONS, ONE
SEMANTICS: interp = dictionary (product of bound closures), native
= real itab (stack table hoisted to entry, method addresses via
func.constant+cast carrying POST-RETYPE types, one load + indirect
call typed from the interface def — no wrapper thunks, the Go
move). v0 iface values are BORROWS (params only) — the retain
design waits for a real consumer. OBJECT CLOSURES: arrows lambda-
lift onto frk_closure.make/apply with ZERO new kernel ops (the
dialect's fourth frontend); captures by BINDING — params by value,
lets by box — so arrows over objects share the alias (JS law,
node-witnessed: mutation between calls visible). Producer computes
captures (tsc knows the bindings); method VALUES refused (unbound
this — arrows are the honest spelling). Corpus: ifaces (two
layouts, one interface, no implements anywhere) + arrows. Suite
53; diff 104/0; grid 99/99 × 5 × 2.

M29 EXTRACTION: (1) the dictionary/itab pair is the K-contract's
sharpest demo yet — REPRESENTATION IS A LOWERING DETAIL is now
witnessed by a case where interp and native use structurally
different data (closures vs tables) and the matrix cannot tell.
(2) The closure dialect absorbed its fourth frontend with zero
ops added — arrows, ml lambdas, lua functions, scheme lambdas all
ride make/apply; the M3-M7 kernel keeps refusing to grow for new
languages, which is the thesis. (3) Coercion sites (obj literals,
iwrap) all hang off checker.getContextualType — the checker-as-
oracle architecture keeps paying: assignability is tsc's problem,
representation is ours, and the boundary is one function call.

m28-done — Shipped: TS-2 classes core (D-073/D-074). The record
idiom lands in frk_mem: field_get/field_set on boxes of products
(gep+load/store, retain-new under rc, owned-operand exclusion, the
leak-biased no-release-old frontier); kinds_layout's product
recursion goes SLOT-KIND-DRIVEN (managed fields code 1 — records
holding strings/arrays/records trace; retain==trace held). THE TYPE
KNOT (D-074): recursive classes cannot type structurally (no
μ-types in MLIR) — class-ref fields erase to !frk_mem.recref with
identity rec_ref/rec_cast; `this.next = this` seeds recref_null and
back-patches after box_new. Frontend: classes/ctors/methods/new/
this/pset/mcall through loanword (additive within v1 again);
methods are this-first plain functions (direct calls). Corpus:
counter/identity/linked — the linked ring is a LIVE OBJECT CYCLE
under rc, green on all five architectures; both collector twins
drill the record-shaped dead ring to the same free count (the
2/2/4/4/6 parity story gains its 8). Suite 51; diff 102/0; grid
97/97 × 5 × 2. Fenced to TS-2's second half: itabs (D-026), method
values/object closures, inheritance, optional/union fields.

M28 EXTRACTION: (1) the record surface cost TWO ops because
identity/shape/tracing were already separately owned (box/product/
layout word) — the dialect factoring paid out; composition, not new
machinery. (2) Type erasure is the kernel's answer to μ-types, and
it is CHEAP precisely because layout rides allocations, not types —
a design decision made for the collector (D-057) turned out to be
what makes recursive data typeable. (3) The construction knot
(self-reference needs the box before its own ref) is a LANGUAGE-
INDEPENDENT fact — Scheme letrec closures hit it (D-035 solved it
by value-capture), TS hits it at object graphs; recref_null is the
record-shaped instance of the same bootstrap pattern.

m27-done — Shipped: TS-1 (D-072), the manifest's research slice
verbatim: tsc's control-flow narrowing IMPORTED as cast annotations,
RE-VERIFIED by our own dominance/dataflow pass, unverifiable casts
DEMOTED to frk.contract runtime checks. frk_contract is BORN (SPEC
§4.6 / D-015's first ops): narrow = checked cast, identity on
success, blame trap on refutation; interp ALWAYS checks (reference
semantics is maximal checking); native runs promote_narrows at
lower_kernel entry — forward must-dataflow, possible-tag bitmasks
per sum root, edge constraints from cond_br on tag_of cmpi, no kill
set (sums are pure values), union-at-joins, fixpoint; proven narrows
DELETED, so a wrong promotion is an L3 divergence by construction.
Representation: unions ARE frk_adt sums; kind NOT stored (tests =
tag compares — the promotion pass's food; reads = tag-selected
literal chains). Producer: type-alias tables, contextual-type
construction, prop nodes, narrow export; loanword additive within
v1 (D-046 revisit resolved). Corpus 4 cases × 8 runners; witnesses:
promotion counts (≥9 facts, 0 surviving on direct cases; exactly
(0,1) on alias_demote), tampered-fact blame trap. Found: latent
TS-0 dead-join bug (trailing if/else both-return → predecessor-less
block breaks LLVM translation) — join now lazy. Suite 49; diff
99/0; grid 94/94 × 5 × 2.

M27 EXTRACTION: the promotion pass is the first REAL optimization
in the repo, and it cost nothing semantically because the interp
NEVER runs it — "reference = maximal checking, native = proof-
elided" turns the entire differential matrix into an automatic
soundness auditor for any future pass with the same shape. Second:
the DEMOTION fate is what makes imported-fact architectures honest
— alias_demote encodes "tsc knows things our pass doesn't" as a
GREEN test, not a limitation note. Third: the tag-test lowering
choice (kind not stored) is what made verification tractable — the
frontend lowers the discriminant INTO the vocabulary the verifier
speaks; representation choices and verifier design are one decision.

m26-done — Shipped: handler consumption (D-071). R7RS
with-exception-handler + raise-continuable over the v1 handlers as
PURE CONSUMPTION: the clause and body are STATIC wrapper intrinsics
(per-site closures differ only in env); the handler's return IS the
resume value (R7RS 6.11 = the tail-resume ABI); nested delegation =
the D-069 masking rule, zero new semantics. The consuming idiom
forced FIRST-CLASS PROCEDURES (L5 working): lambdas lift as uniform
pack-fn closures (wind-thunk machinery generalized to n params via
__scm_arg), procedure-valued vars apply through the uniform
convention + guard. THE LEDGERED DELTA against the zero-kernel bar:
perform_end clobbered an in-flight abort when a clause ESCAPED
through an enclosing prompt — the interp's Err propagation had
escape-wins right, native overwrote the pending cell. Both twins
fixed; exn_escape (escape from a handler crossing a dynamic-wind,
afters firing) witnesses it on every runner. Corpus 15/15 vs chibi;
suite 45; diff 95/0; grid 90/90 × 5 × 2.

M26 EXTRACTION: consumption is a VERIFIER CLASS. Hand-written op
goldens exercise what the author imagined; a real surface exercises
COMPOSITIONS nobody wrote down — escape-from-clause-across-wind
existed in no golden and broke native. The milestone's design
lesson: static wrapper intrinsics over per-site closures (env
carries the variance) kept the whole feature at two IR functions —
the seed-module surface absorbing what would have been emitter
builder-code sprawl. First-class procedures cost ~80 emitter lines
because the uniform convention already existed everywhere below.

m25-done — Shipped: r7rs_core v0.1 (D-070; the human picked
"r7rs_core"). THE D-051 WIDENING FIRED: TAG_PAIR = 6 — pairs as
wrapped product<[dyn,dyn]> through EXISTING ops (no cons kernel op;
the manifest's adt-carriers promise kept). The D-057 symmetry law
named every site: masked_dyn_ptr → the 4..=6 range compare, six
tracer arms across both twins, kinds_layout recursing into product
fields (the all-zero fallback = a latent rc UAF), TAG_LIMIT 7. No
pair mutation ⇒ no new cycles ⇒ trial deletion untouched. Symbols =
interned tag-3 bstrs; eq? compares symbols through frk_bstr.eq
(byte-equal in the oracle, pointer-equal natively — the
payload_word shortcut DIVERGED interp-side and the differential law
caught it). DYNAMIC-WIND CLOSED escape-only as frk_ctl.wind:
before(); r := thunk(); after(); yield r, crossing aborts re-raised
AFTER after() — natively the D-061 guard discipline IS the
unwind-finalizer hook; the interp mirrors by catching the abort
around the thunk. Wind thunks lift (captures…, pack)→pack with the
RetShape generalization. Corpus 11/11 vs chibi; suite 45; diff
91/0; grid 86/86 × 5 × 2.

M25 EXTRACTION: two lessons for the ledger of lessons. (1) A tag
widening is a FRONTIER walk, not a constant bump — D-057's symmetry
law turned "add a tag" into a checklist of named sites, and the one
site NOT on the checklist (kinds_layout's product fallback) was
exactly where the latent UAF sat. Laws that enumerate frontiers pay
off when the frontier moves. (2) Identity is a SEMANTIC contract,
not a representation accident: payload_word equality was true
natively (interning) and false in the reference (fresh values) —
the differential law surfaced a divergence NO single implementation
would ever notice. eq? now goes through the op whose two
implementations CONVERGE. The wind design is the milestone's gift:
zero new runtime state, because the propagation discipline built
for D-061 already WAS a finalizer mechanism waiting to be named.

m24-done — Shipped: effects-v1 (D-069; the human picked "effects").
frk.ctl grows handle/perform/resume — κ_frk's H-op-resume rung,
scoped to the affine ladder's tractable classes with THE CLAUSE AT
THE PERFORM SITE: drop (v0), abortive (clause returns without
consuming κ → the handle yields it via the v0 abort machinery), and
tail-resume (clause consumes κ once; ITS RETURN IS THE RESUME VALUE;
the dispatch mask lifting is the deep reinstall). κ is BORN UNIFORM:
a closure over a one-shot marker; application marks-or-traps and
returns its pack — Apply special-case interp-side, a synthesized
identity-on-pack thunk native-side. Native dispatch = an evidence
stack in both twins (labels interned ⇒ find is pointer compare) with
BRANCH-FREE perform: begin masks + mints the marker; end reads the
clause pack's head and decides consumed-else-abort IN THE RUNTIME.
Full re-entrant κ = the named Tier-2 stack-switching rung. Six new
K2 verifiers landed red-first (L1); the license gate (forced-general
interp vs evidence-stack native) held: suite 45; diff 86/0; grid
81/81 × 5 × 2.

M24 EXTRACTION: the grid earned its keep TWICE pre-commit. (1)
func.func addresses need the func.constant+cast recipe — addressof
is llvm-symbols-only at pass time. (2) THE WASM32 κ-BOX: hand-rolled
i64 slot stores vs unwrap's native {ptr,ptr} struct read — on 32-bit
pointers the env came back as the fn-pointer's high half (zero), the
resumer loaded a garbage marker FROM LINEAR-MEMORY ADDRESS 0 (valid
on wasm — no crash!), and tail-resume silently became abortive
(40≠42). s390x passed; ONLY a 32-bit-pointer target could see it.
Lesson for the ledger of lessons: never hand-roll a layout the
kernel already owns a recipe for — struct stores exist so pointer
width stays the kernel's problem.

m23-done — Shipped: femto_lua v0.3 (D-068). The explist ADJUSTMENT
engine: one emitter mechanism (non-final truncates, final call/`...`
expands, single-call forwarding preserved for the tail law) consumed
by returns, destructuring, call args, constructor tails, and the
generic-for explist — so varargs, multi-RHS, and explicit iterator
triples fell out of one design. Varargs are pack-native (`...` tail
copied at the prologue BEFORE the D-067 dispose; __lua_pack_tail /
__lua_pack_copy_into as frk.borrows IR intrinsics — rc discipline
inherited from the kernel, zero hand-written retains). __newindex =
__lua_setindex: luaV_settable faithful (existing keys raw; table
form re-enters as a TAIL call — chains ride the trampoline/musttail
like __index). Expansion made pack LENGTHS observable: print() went
multi-value (tab-joined) and next() returns ONE nil at exhaustion —
both oracle-ruled. arith.maxsi joined the interp registry with test.
Corpus 18/18 vs lua5.1; suite 45; diff 84/0; grid 79/79 × 5 × 2.

M23 EXTRACTION — two kernel ownership theorems the corpus forced,
both jit-rc segfaults caught by the differential law pre-commit:
(1) THE BORROW GATE IS ABOUT SHAPE, NOT PROVENANCE: D-067 gated
received packs; the explicit GenFor triple proved CREATED packs with
cross-block borrowed-out reads need the identical gate (ArrayNew now
gated). Any container whose reads are borrows must not die while a
borrow lives — regardless of where the container came from.
(2) TRANSFER REQUIRES AN OWNED PRODUCER: sole-use retain elision is
only sound when the stored value's producer FORFEITS ownership
(wrap/array_new/box_new/table_new/apply/make). A borrow (box_get,
array_get, a frk.borrows call, a block arg) leaves its source
owning — its store must retain. produces_owned() is now the rule;
the elision keeps its wins on the fresh-allocation fast path.
Together they harden the same lesson as M22: ownership facts attach
to EDGES (who produced, who borrowed), never to op categories.

m22-done — Shipped: the pack terminal-count ruling (D-067; closes
D-064's observation). Ruled OWNED: frk_mem.dispose (K2 no-op; Rc
release; Arena erased) ends the callee's ownership of its incoming
pack after the boxing prologue; frk.borrows (declared in the callee's
own IR — __lua_arg in intrinsics.mlir) exempts borrowing reads from
the escapes-conservatism; received packs join die_at behind the
DERIVED-BORROW LOCALITY gate. The gate exists because the harness
caught its absence: generic_for's iterator closure freed mid-loop —
a jit-rc segfault the differential law surfaced before any commit.
Measured: 1000-call leak 2026 → 24; disposed packs reclaim as
Bacon–Rajan deferred frees at collect. Permanent witness:
pack_reclamation.rs (no O(calls) term may return). Suite 45 blocks;
diff 79/0; grid 74/74 × both strategies × 5 triples.

M22 EXTRACTION: "borrows" is a fact about operands, not results —
conflating the two was the milestone's one real bug, and the
derived-borrow locality gate is the reusable lesson: releasing a
container needs the borrows OUT of it to be provably dead or
independently retained. That gate pattern will matter again the day
effects-v1 hands out references from resumed frames.

m21-done — Shipped: D-062 closed (D-066; "finish D-062"). Registry-
driven registration: jit_symbol/builtin_for tables supply addresses
and closures, frk-abi rows drive the SETS, coverage witnessed in both
directions (missing AND stale) plus loud panics at registration. Dead
exports print_lua_num/bool/nil deleted from both twins + registry.
The last u8 → i64 (frk_rt_print_bool): loanword declares i64 and
extui-widens at call sites; capture shim + interp builtin follow
(the builtin's as_bool() would have broken on the widened value —
caught in the rewrite); AbiTy::U8 removed from the vocabulary. The
migration was DRIVEN BY THE MACHINERY: registry first, then the C
header, the Rust assertions, and the capture assertions each refused
in turn, naming their own fix sites. Suite 44 blocks; diff 79/0;
grid 74/74 × both strategies × 5 triples.

M21 EXTRACTION: registry-first migration is the payoff of authoring
surfaces — the enforcement points aren't just guards, they are the
MIGRATION PLAN (edit the truth, follow the refusals). The coverage
witnesses' stale direction mattered immediately: deleting the dead
rows would have left stale jit/builtin bindings undetected without it.

m20-done — Shipped: the lua intrinsics migration completes (D-065;
D-062's follow-up, unfenced by D-063 exactly as the sequencing rule
planned). The _v pack wrappers, next/pairs/ipairs(+iter), string
sub/rep, and __lua_index moved from emitter builder code into
lua/intrinsics.mlir — 442 lines of reviewable kernel IR carrying the
whole lua protocol library and its runtime declarations. emit_helpers
DELETED; dead builder utilities (lua switch/pack_dyns, scheme switch)
removed. The lua emitter now: parse seed → append program. Suite 43
blocks; diff 79/0; grid 74/74 × both strategies × 5 triples.

M20 EXTRACTION: the sequencing-rule bet from the M17 panel settled at
full value — signatures were rewritten ONCE (in builder code at M18)
and frozen into text ONCE (here). Rule of thumb for the ladder:
migrate representation-stable code to data immediately; migrate
convention-riding code only after the convention lands.

m19-done — Shipped: tail-aware release scheduling (D-064; D-063's
fence resolved). Evidence-first: the rc-lowered lua tail loop showed
exactly ONE release between call and return, and it was half of a
retain/release pair — the args pack retained by its owning snoc,
released by the frame at the terminator. The rule: in a tail-shaped
block, a paired frame release relocates to before the call. Soundness
in two legs: the pair witnesses a second owner (crossing count >= 1;
only pure ops sit between the relocated release and the call), and no
caller code runs after a tail call. Unpaired releases stay put
(conservative). Terminal counts unchanged — no accounting drift.
Implementation: one anchor function in the releases loop
(tail_release_anchor: return-fed-by-previous-call + SSA-identity
retain scan). Both deep goldens unfenced; jit-rc runs 100k tail
frames in-process; grid 74/74 × BOTH strategies × 5 triples.

M19 EXTRACTION: the fix was 80 lines because the evidence pass came
first — dumping the actual rc IR showed the offending release was
PAIRED, which turned a scary ownership-convention redesign
(Perceus-style callee-owned params) into a local scheduling rule with
a mechanical soundness check. Record for the ladder: when the next
discipline change looks like it needs a convention rewrite, dump the
IR first. Also surfaced (ledgered, not fixed): packs terminally leak
at count 1 under rc — the owned-params question is now a named
follow-up rather than an unknown.

m18-done — Shipped: the uniform-signature convention (D-063,
D-059's gap closed). KERNEL: !frk_closure.envref + closure.env_load
(index + carried env product; verifier + interp eval + lowering that
reuses the exact thunk-prologue slot math); closure.make skips thunk
synthesis for uniform callees (the closure struct holds the callee's
address); the two conventions coexist per-callee. INTERP: the tail
law generalized with ZERO frk-interp changes — Apply returns
Step::TailCall on the tail shape (both conventions, every frontend).
NATIVE: frk-tail-calls marks INDIRECT tail calls whose callsite
prototype equals the caller's type — found and fixed the type-
spelling trap (standalone "!llvm.ptr" vs bare "ptr" inside
!llvm.func<…>; x86 masked the miss via opportunistic sibcall, wasm
exposed it with stack exhaustion). LUA: lifted fns are (envref, pack)
via env_load, _v wrappers gained the ignored envref param (exactly
the rewrite D-062's sequencing rule predicted), zero thunks anywhere.
Goldens: closure/uniform_tail (100k apply-tails, kernel-level) +
lua/tail_recursion (100k frames vs PUC lua5.1) — which required
widening the frk-case directive to lua/scheme comment forms (`--`,
`;;`) since the oracle runs the same file. The M14 depth-cap lesson
replayed ON SCHEDULE: the runaway-closure fixture was tail-shaped,
became a legitimate infinite loop, now consumes its result. FENCED
(D-063): rc-native TCO — block-exit releases break the tail shape;
release scheduling is its own rung; deep goldens fence rc runners.

M18 EXTRACTION: Step::TailCall as the interp's tail channel proved to
be THE right abstraction — closure tails cost zero interpreter
changes because evaluators already speak Step. The uniform convention
is the pack convention's logical completion: M13 unified the ARGS,
M18 unified the FUNCTIONS. Promotable: scheme's call/cc receivers can
adopt uniformity for free; effects-v1's resume closures should be
born uniform.

m17-done — Shipped: intrinsics + runtimes as first-class authoring
surfaces (D-062; the human's directive), panel-reviewed (3 adversarial
lenses; strongest finding — the type-erased JIT capture shims — fixed
with generated typed assertions). SURFACE A: intrinsics modules —
language primitives as kernel IR in .mlir seed files (include_str!,
L6); scheme migrated fully (builder code deleted), lua's nine
plain-dyn protocol helpers migrated (the _v wrappers wait on D-059
per the sequencing rule). SURFACE B: crates/frk-abi, the runtime ABI
registry — 39 symbols, 8-variant vocabulary incl. PtrPayload
(void*/mut-u8 asymmetric rendering); enforcement at five layers, all
L1-witnessed: Rust twin (build.rs fn-pointer assertions), C twin
(generated frk_rt_abi.h included by frk_rt.c — the tamper test
REPLAYS the M15 display_bool bug and proves compiler refusal), capture
shims, derived kernel_lower declarations (hand tables deleted, dedup
vs module-declared symbols), and the semantic verifier's declaration
projection (i1/i8↔u8 widening pinned; 5 witnesses). Day-one catches:
11 fns of latent void*/uint8_t* drift; 3 dead print exports;
loanword's unchecked declarations now guarded. INCIDENTAL FIRST-RANK
FIND (L3): node 26 colorizes PIPED console.log under COLORTERM, and
FORCE_COLOR (ambient in agent shells) overrides NO_COLOR — the node
oracle env now pins color off unconditionally (NO_COLOR=1 +
env_remove FORCE_COLOR/COLORTERM). Book gains the authoring-surfaces
chapter. Suite 43 blocks; diff 77/0; grid 72/72 × 5 × 2.

M17 EXTRACTION: the two surfaces are the same idea at two levels —
author once as DATA (a .mlir file, a const table), then derive or
compile-time-check every consumer. The registry's day-one catches
(latent drift, dead exports) repeated the M12 lesson: the value of a
contract is what it finds the day you write it down. Promotable next:
the seed-module pattern generalizes to any frontend-supplied IR
(prelude modules, user dialects at v2); the registry's lane column is
the hook for specimen twin extensions.

m15-done — Shipped: frk.ctl v0 (escape continuations) + the
r7rs_core specimen, closing the Rocq-anchor escalation by delegation
(the human: "you can do it just fine" — atli IS his handler calculus;
D-060 promotes its core as κ_frk, D-061 records the native lowering
chosen after a 3-designer+judge panel). The dialect: prompt/abort/
pending. Reference semantics REALLY unwind (EvalError::Abort threads
up; monotonic tokens, LIFO prompt stack, both traps) — 6 K2
verifiers incl. a 1000-deep tail-chain abort and both nested-prompt
targetings. Native is result-passing (D-011, no unwinder → Tier-0/
wasm): the ctl ops kernel-lower to a runtime pending-cell (both
twins), and the FRONTEND emits the guards (pending-check + early-
return after every NON-tail call; tail calls unguarded so they stay
musttail and propagate for free — the tail-call/guard law). The
r7rs_core frontend lambda-lifts procedures to direct func.func calls
(real M14 tail calls, the manifest's headline) and lowers call/cc to
a prompt over an escape-closure; escape continuations are apply-only
in v0. 6-case corpus byte-identical across ALL 8 runners + the AOT
grid (6/6 × 4 triples × 2 + s390x). A grid-found wasm signature bug
(display_bool u8 vs i64 call) reprised the u64-everywhere lesson.

M15 EXTRACTION: the interp trampoline (M14) and the ctl reference
semantics compose for free — a deep tail-recursive abort threads up
through frame replacement with no special case. The pack-vs-lift
fork resolved toward lambda-lifting for scheme (direct calls, real
TCO, no per-call allocation) — the OPPOSITE of femto_lua's pack
choice, because scheme leads with tail-recursion where lua led with
arity. Promotable: the escape-closure pattern (a closure capturing a
prompt token, its body aborting) is exactly what a general one-shot
`resume` will generalize at effects-v1; the frontend-explicit guard
discipline is the thing to revisit (a backstop pass, per the design
panel's judge, if a second ctl frontend arrives). Fenced honestly:
closure-apply tail calls are NOT yet TCO'd in the interp, so deep
scheme recursion beyond direct func.call chains is interp-capped
(corpus stays shallow); the uniform-signature convention lifts it.

m14-done — Shipped: tail calls as law, first rung (D-059). The
interpreter trampolines every tail-shaped call (frame REPLACEMENT
threading Step::TailCall to eval_function's loop; the D-029 depth
cap counts non-tail entries only — its exemption clause, finally
cashed). Natively, frk-tail-calls joins the pipeline as a fifth
stage: identical-signature direct tails get musttail (self-recursion
always, equal-signature mutual too); wasm32 gains -mtail-call. The
law's verifiers are 500k-frame fixed-stack goldens that FAIL without
each rung — and they hold on ALL FIVE architectures including
big-endian s390x and wasm's tail-call instruction (grid 65/65 × 2).
The semantic depth of the change surfaced immediately: the old
runaway-recursion test was TAIL-SHAPED and became a legitimate
infinite loop — its fixture now uses the call result (non-tail),
guarding the depth cap for the world it still governs. LEDGERED GAP:
indirect + cross-signature tails (lua-level `return f(x)` chains)
await the uniform-signature convention. ESCALATED: r7rs ratification
blocked on the human's Rocq handler calculus per the stub's own law.

M14 EXTRACTION: reference-semantics-leads-native-follows held its
shape — the trampoline is total and general, musttail is a
conservatively-gated subset, and the corpus goldens measure exactly
the difference. The tail-shape detector (call-feeds-adjacent-return)
is the same pattern at both levels, interp and LLVM — one semantic
idea, two carriers.

m13-done — Shipped: femto_lua v0.2 (D-058), in two waves. Wave 1,
the PACK CONVENTION: every Lua function is fn<[arr<dyn>],
[arr<dyn>]> — one argument pack in, one values pack out, params read
through bounds-checked nil-fill (__lua_arg) — which dissolved the
exact-arity fence and made multiple returns a surface feature
instead of plumbing. The kernel paid with ONE widening (two-slot
arr<dyn> elements, stride-addressed) that M12's ARRAY_DYN tracer
already knew how to walk: the collector handled argument packs with
zero new GC code. Wave 2: return explists, destructuring
locals/assignments, tail-position pack forwarding, break (loop-exit
stack), repeat/until (condition sees body locals), generic for over
the real (f, s, ctrl) protocol with pairs/ipairs/next seeded
(table_next walks slot order; canon keeps pairs output
order-independent), and string.sub/rep as frk_bstr ops + a module
table. Corpus 12/12 vs lua5.1 (bar ≥90); diff 68/0 across seven
runners; grid 63/63 × 5 × 2 with real frees under everything.
Learned: the convention change was FRONTEND-ONLY exactly as D-058
predicted — the deepest surface change femto_lua has had cost the
kernel one element-width widening; and the interp/native iteration-
order split (insertion vs slot order) is invisible under the canon
aggregation rule, which is the rule working, not luck.

M13 EXTRACTION: what v0.2 forced — (a) two-slot array elements (the
only kernel change; arr<dyn> is now fully general for pairs);
(b) frk_dyn.table_next — the first TWO-RESULT kernel op, and the
lowering's replace-both-results pattern that came with it;
(c) frk_bstr.sub/rep. What it did NOT force: closures, adts, mem,
the dyn core, the collector — all untouched. The pack convention is
promotable thinking: TS-1's union-narrowing calls and scheme's
multi-value continuations will both want it; note for the frk.ctl
design.

m12-done — Shipped: the GC ladder's remaining rungs (D-057), both
twins. Sized releases (three-word headers [layout][size][rcword];
cascaded REAL frees); the layout-descriptor rung exactly as D-055
demanded — designed, then climbed: per-site layout words from the
lowering's slot kinds (wordmap/table/array encodings,
lockstep-tested); Bacon–Rajan trial deletion over purple candidates
with an explicit deterministic collect(). Cross-twin: the C
collector, driven through zigcc, reports the byte-identical
cascade/dead-cycle/live-cycle free counts (2/2/4/4/6). Grid: 59/59
× 5 architectures × 2 strategies WITH FREES LIVE — every rc golden
is a standing UAF detector, green. Verifier finds, in order: (1)
the arithmetic-shift color smear (purple read as -1; the C twin is
unsigned throughout because of it); (2) the retain/trace frontier
ASYMMETRY (tracer saw dyn edges retains never counted → core dump →
RetainKind + branch-free masked dyn retains + raw_set/set_meta
ownership discipline); (3) the transfer-vs-release DOUBLE-SPEND
(sole-use owning stores transfer their reference; block-exit
releases spent it twice → no die_at for transferred values). All
three were found by tests/corpus within minutes of frees becoming
real — D-057.4 predicted exactly this and it held.

M12 EXTRACTION: the milestone's law-shaped lesson — RETAIN COVERAGE
MUST EQUAL TRACE COVERAGE, and lifetime analyses must respect
ownership TRANSFER; the frontier widens symmetrically or not at
all. The layout-descriptor design (header words computed by the
compiler, walked by a runtime that never heard of SlotKind) is the
D-049 split made physical, and the lockstep test is what keeps the
two sides honest. Remaining leak-bias (deliberate, counted):
deleted table keys, overwritten box payloads, >30-word payloads,
non-dyn aggregate interiors.

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

    Session end: 2026-07-17 (thirty-first entry)
    Milestone/step: M29 complete, tagged m29-done; TS-2 frozen
    Green? yes — 53 blocks; 104/0 (8 runners); grid 99/99 × 5 × 2
    Did:
    - D-075; frk_dyn.iface + iface_make/iface_call (IRDL/verify/
      dictionary eval/itab lowering + K3 JIT test); producer
      interfaces/iwrap/imcall/arrows/fn-types/fcall; consumer
      Iface/Fn types + emissions + arrow lambda-lifting; 2 goldens;
      manifest TS-2 FROZEN; book dyn itab section
    Next: queue to the user (parameterize / pairs-mut / tier-2 /
    ts-3)
    Landmines:
    - iface tables are STACK values hoisted to function entry —
      sound ONLY under the borrows-only fence; lifting the fence
      (iface stores) requires the static-global cache + a retain
      story FIRST
    - func.constant references must carry POST-RETYPE types
      (declared_fn_type) or verification fails after signature
      rewriting — any new address-taking site must go through it
    - melior ArrayAttribute::try_from does not accept real
      ArrayAttrs — use attr_util::array_elements (that is WHY it
      exists)
    - arrow captures are BY BINDING; capturing a let means the BOX
      travels — releasing that box while a closure holds it would
      be a UAF; closures escaping into fields/arrays stay fenced
      until the capture-lifetime rung

    Session end: 2026-07-17 (thirtieth entry)
    Milestone/step: M28 complete, tagged m28-done
    Green? yes — 51 blocks; 102/0 (8 runners); grid 97/97 × 5 × 2
    Did:
    - D-073/D-074; frk_mem field_get/field_set + recref/rec_ref/
      rec_cast/recref_null (IRDL/verify/eval/lowering); slot-kind
      product layouts; record-ring drill in BOTH collector twins;
      producer classes (fields/ctor/methods/new/this/pset/mcall);
      consumer TsTy::Class + @C__new/@C__m emission; 3 goldens incl.
      the live-cycle ring; manifest TS-2-in-progress; book mem
      records section
    Next: queue to the user (ts-2b itabs / parameterize / pairs-mut
    / tier-2)
    Landmines:
    - field_set does NOT release the old value (leak-biased, mirrors
      box_set) — closing that frontier is a deliberate future rung,
      not a bug fix to slip in
    - recref_null must NEVER survive a constructor; the back-patch
      happens in the SAME block as box_new — do not let a future
      ctor form (option-b bodies) move statements between them
    - TsTy::Class is an INDEX into the artifact rows — cloning
      artifacts or merging modules would invalidate it; the index is
      artifact-scoped by design
    - class methods mangle as {Class}__{method}; a user function
      named the same collides silently — producer-side guard is owed
      when TS-2 completes

    Session end: 2026-07-17 (twenty-ninth entry)
    Milestone/step: M27 complete, tagged m27-done
    Green? yes — 49 blocks; 99/0 (8 runners); grid 94/94 × 5 × 2
    Did:
    - D-072; frk_contract dialect (IRDL/verify/eval/lowering);
      promote_narrows dataflow + 12 CFG verifiers; frk_rt_contract_
      check (Contract lane, both twins, make abi); producer type
      aliases + obj/prop/narrow nodes; consumer Union/Variant +
      kind-test tag lowering + blame from spans; 4 goldens; 3
      witness tests (promotion counts, demotion, tampered fact);
      TS-0 dead-join fix; manifest TS-1 SHIPPED; book chapters
      (dialects/contract, specimens/ts1)
    Next: queue to the user (parameterize / pair mutation / Tier-2 /
    TS-2)
    Landmines:
    - promote_narrows runs on NATIVE paths only — never "unify" it
      into the interp path; the asymmetry IS the auditor
    - narrow is identity-on-success and kind is NOT a stored field;
      storing kind would break both the verifier vocabulary and the
      sum layout — TS-2 objects must not casually regress this
    - union-typed LOCALS are fenced (box reads have no SSA
      identity); admitting them silently demotes every fact —
      that admission needs its own D-entry naming the cost
    - the false-fact witness needs variants sharing a field shape;
      differently-shaped variants are refused at EMISSION (the
      consumer's typing), which is the cheaper defense firing first

    Session end: 2026-07-17 (twenty-eighth entry)
    Milestone/step: M26 complete, tagged m26-done
    Green? yes — 45 blocks; 95/0 (8 runners); grid 90/90 × 5 × 2
    Did:
    - D-071; __scm_arg intrinsic; wind-thunk lifting generalized to
      n params; first-class lambdas + procedure-valued application
      (guarded); __scm_exn_clause/__scm_exn_body static wrappers;
      with-exception-handler + raise-continuable primitives; the
      escape-wins perform_end fix (both twins); 4 chibi-validated
      corpus cases; manifest v0.2; book current
    Next: queue to the user (parameterize+global cells / pair
    mutation+strings+vectors / Tier-2 / TS-1)
    Landmines:
    - perform_end preserves an in-flight abort BY DESIGN (escape
      wins over abortive-clause) — do not "simplify" the pending
      check away; the interp's Err propagation is the spec
    - first-class application reads result HEAD via __scm_arg,
      which breaks the apply-tail shape — deep first-class tail
      recursion is not TCO'd (direct calls still are); note before
      any corpus case leans on it
    - __scm_exn_clause runs the user handler INSIDE the clause
      frame: a handler that itself installs handlers works (masking
      is per-entry), but a handler RESUMING TWICE would hit the
      one-shot trap — R7RS-legal (raise-continuable resumes once)

    Session end: 2026-07-16 (twenty-seventh entry)
    Milestone/step: M25 complete, tagged m25-done
    Green? yes — 45 blocks; 91/0 (8 runners); grid 86/86 × 5 × 2
    Did:
    - D-070; TAG_PAIR widening (mask range, six tracer arms,
      kinds_layout product recursion, TAG_LIMIT 7); frk_ctl.wind op
      (IRDL/verify/eval/lowering + 2 K2 verifiers); scheme quote
      sugar + Expr::Quote + symbols; cons/car/cdr/null?/pair?/eq?/
      list primitives as IR intrinsics; display str/'()/pair/dotted
      arms; wind-thunk job flavor + RetShape; rt rows scm_display_str
      + ctl_pack_head; arith.andi eval (+ sentinel repointed);
      __scm_eq via frk_bstr.eq (the interning-divergence fix);
      5 chibi-validated corpus cases; manifest v0.1; κ_frk OPEN
      ruling struck; book current
    Next: queue to the user (handler-consuming surface / r7rs v0.2
    with pair mutation / Tier-2 stack switching)
    Landmines:
    - set-car!/set-cdr! REOPEN the cycle question: mutable pairs
      must join the purple/candidate machinery before v0.2 ships
      mutation (today pairs cannot cycle — that is WHY trial
      deletion needed no change)
    - eq? on numbers is payload-BIT equality (f64 bits): fine for
      fixnums; (eq? 0.0 -0.0) would diverge from chibi — fenced
      with flonums, note before unfencing
    - payload_word is identity ONLY natively; never use it for
      cross-implementation equality — route through an op whose
      interp/native semantics converge (the __scm_eq lesson)

    Session end: 2026-07-16 (twenty-sixth entry)
    Milestone/step: M24 complete, tagged m24-done
    Green? yes — 45 blocks; 86/0 (8 runners); grid 81/81 × 5 × 2
    Did:
    - D-069 + κ_frk v1 rung; six K2 verifiers red-first; interp
      handler stack/masking/markers + Apply resumer special-case;
      5 registry rows + both twins (evidence stack, branch-free
      perform_end deciding consumed-else-abort in rt); kernel
      lowering (handle/perform/resume arms, intern_label,
      synthesize_resumer); 2 native goldens incl. the hand-written
      D-061 guard; two grid finds fixed (func.constant; wasm32 κ-box
      struct layout); book ctl chapters updated
    Next: queue to the user (r7rs v0.1+dynamic-wind / a handler-
    consuming surface / Tier-2 stack-switching)
    Landmines:
    - κ's clause ABI is v1-tail-resume semantics: the clause's
      RETURN is the resume value; code after k(v) in a clause does
      NOT run after body-rest (that's the Tier-2 rung). Frontends
      must emit tail-resume-shaped clauses or abortive ones only.
    - the evidence stack masks by INDEX; handler_pop is plain pop —
      an abortive unwind crossing MULTIPLE handles relies on each
      handle's pop running in its own frame (guards make that true
      natively; the interp pops in Handle's eval). Do not reorder.
    - never hand-roll {ptr,ptr} boxes as i64 slots (the wasm32 find);
      use closure_struct + store_op — pointer width is the kernel's.

    Session end: 2026-07-16 (twenty-fifth entry)
    Milestone/step: M23 complete, tagged m23-done
    Green? yes — 45 blocks; 84/0 (8 runners); grid 79/79 × 5 × 2
    Did:
    - D-068; lexer Dots; parser explists + vararg capability stack;
      the explist engine + pack_with_tail; vararg prologue copy;
      __lua_pack_tail/copy_into/ctor_append/setindex intrinsics;
      multi-value print; next() exhaustion arity; arith.maxsi eval;
      5 oracle-validated corpus cases; TWO kernel ownership fixes
      (created-pack borrow gate; produces_owned transfer rule)
    Next: queue to the user (r7rs v0.1 / effects-v1 / lua v0.4)
    Landmines:
    - pack LENGTHS are surface semantics now: any _v wrapper that
      builds a return pack fixes an arity the oracle can observe —
      check lua5.1 before choosing 1-pack vs 2-pack returns
    - retain elision: NEVER add an op to produces_owned() unless its
      result truly carries a forfeitable +1; when unsure, retain
      (over-retain balances; under-retain frees live objects)
    - gated packs LEAK by design (conservative D-067 posture); a
      corpus case putting a GenFor in a 1000-call hot loop would put
      an O(calls) term back in pack_reclamation — the release-at-
      function-exit rung is the named fix if that day comes

    Session end: 2026-07-16 (thirtieth entry)
    Milestone/step: M22 complete, tagged m22-done
    Green? yes — 45 blocks; 79/0 (8 runners); grid 74/74 × 2 × 5
    Did:
    - D-067; frk_mem.dispose (IRDL/verify/eval/lowering); frk.borrows
      escape exemption; lua emitter + intrinsics disposes; received-
      pack die_at + derived-borrow locality gate (added after the
      generic_for jit-rc segfault); pack_reclamation witness; book
      strategies chapter section
    Next: queue to the user (see Next action)
    Landmines:
    - frk.borrows means the callee doesn't KEEP OPERANDS — it says
      NOTHING about results; anything derived from a borrowed read
      must be block-local or independently retained before the
      container may release (the generic_for lesson)
    - dispose is for RECEIVED ownership only (parameters); disposing
      a frame-created value double-releases with die_at
    - disposed packs free at COLLECT (buffered zeros — Bacon-Rajan
      defers); leak measurements must run frk_rt_rc_collect first

    Session end: 2026-07-16 (twenty-ninth entry)
    Milestone/step: M21 complete, tagged m21-done; D-062 CLOSED
    Green? yes — 44 blocks; 79/0 (8 runners); grid 74/74 × 2 × 5
    Did:
    - D-066; registry-driven JIT/builtin registration + has_builtin +
      two-directional coverage witnesses; dead exports deleted; u8→i64
      (twins, shim, loanword decl + extui, interp builtin); AbiTy::U8
      removed; book enforcement table gains the registration row
    Next: queue to the user (see Next action)
    Landmines:
    - adding a runtime fn now = ONE registry row + the twins; the
      panics/witnesses NAME every remaining site — follow the errors,
      do not pre-edit consumers
    - interp builtins read WIDENED integer flags (as_signed != 0),
      never as_bool — call sites extui before the ABI boundary

    Session end: 2026-07-16 (twenty-eighth entry)
    Milestone/step: M20 complete, tagged m20-done
    Green? yes — 43 blocks; 79/0 (8 runners); grid 74/74 × 2 × 5
    Did:
    - D-065; wrappers/iterators/string/__lua_index into
      intrinsics.mlir (dump-extract, same as M17); emit_helpers +
      dead utilities deleted; book authoring-surfaces chapter updated
    Next: queue to the user (see Next action)
    Landmines:
    - lua protocol changes are now EDITS TO intrinsics.mlir — do not
      reintroduce builder helpers; the file carries its own rt decls
      (kernel_lower dedups against them)
    - brace-scan deletions of emitter methods: cut EXACTLY the method
      (the neighboring-method over-cut cost one revert this session)

    Session end: 2026-07-16 (twenty-seventh entry)
    Milestone/step: M19 complete, tagged m19-done
    Green? yes — 43 blocks; 79/0 (8 runners); grid 74/74 × 2 × 5
    Did:
    - D-064; tail_release_anchor in the releases loop (paired-retain
      relocation); deep goldens unfenced (directives dropped); book
      tail-calls chapter fence paragraph -> resolution; ledgered the
      pack terminal-count follow-up
    Next: queue to the user (see Next action)
    Landmines:
    - the relocation rule REQUIRES the paired retain: never widen it
      to unpaired releases without re-proving the crossing count
      (a borrowed operand released early = use-after-free)
    - the pair check is SSA-identity on the LOWERED values — masked
      dyn retains (different SSA value) deliberately do not match
    - packs cross calls at terminal count 1 that nobody releases
      (pre-existing rc leak, now written down; see Next action 5)

    Session end: 2026-07-16 (twenty-sixth entry)
    Milestone/step: M18 complete, tagged m18-done
    Green? yes — 43 blocks; 79/0 (8 runners); grid 74/74+72/72 × 5
    Did:
    - D-063; envref + env_load (dialect/verify/eval/lowering); uniform
      make path (no thunks); Apply Step::TailCall; indirect musttail
      (+ the type-spelling normalization); lua on the convention;
      directive comment forms for .lua/.scm; two 100k law goldens;
      runaway-closure fixture non-tail; book chapter updated
    Next: queue to the user (see Next action)
    Landmines:
    - !llvm.func<…> prints types in BARE LLVM shorthand ("ptr") while
      standalone types print "!llvm.ptr" — never string-compare across
      those contexts without normalizing (cost: wasm-only stack
      exhaustion that x86 masked via opportunistic sibcall)
    - a TAIL-SHAPED runaway closure apply is now an infinite loop, not
      a depth trap — depth-cap tests must consume the apply result
    - deep-recursion goldens fence rc-native runners (D-063); when
      adding one, list runners explicitly incl. aot-* arena names
    - frk-case directives: `--` (.lua) and `;;` (.scm) forms exist now;
      a directive in an unrecognized comment form is SILENTLY ignored

    Session end: 2026-07-13 (twenty-fifth entry)
    Milestone/step: M17 complete, tagged m17-done
    Green? yes — 43 blocks; 77/0 (8 runners); grid 72/72 × 5 × 2
    Did:
    - D-062 + SPEC K4 amendment + §6.6; crates/frk-abi (registry +
      generators + gen-header bin + make abi); both twins compile-time
      pinned (build.rs assertions / generated header); capture-shim
      assertions (frk-harness build.rs); kernel_lower declarations
      derived + dedup'd; verifier declaration projection + 5
      witnesses; scheme + lua intrinsics.mlir seed modules (builder
      code deleted); node-oracle color pinning; book chapter
    Next: queue to the user (see Next action)
    Landmines:
    - node >= 25 colorizes PIPED console.log when COLORTERM is set,
      and FORCE_COLOR (set ambiently by agent shells) OVERRIDES
      NO_COLOR — any subprocess whose stdout is protocol bytes must
      env_remove both. Found as 8 phantom diff divergences.
    - intrinsics .mlir files are SEED modules: emitters append into
      them; kernel_lower skips re-declaring their symbols. Do not
      re-add declare_runtime-style builders to frontends.
    - the _v pack wrappers stay emitter-built until D-059 (their
      signatures ride the closure convention) — do not migrate them
      to intrinsics files yet.
    - frk-rt/frk-harness have build.rs deps on frk-abi; a registry
      edit rebuilds both (by design — that IS the enforcement).

    Session end: 2026-07-13 (twenty-fourth entry)
    Milestone/step: the Book — deep docs on GitHub Pages (interview artifact)
    Green? yes — mdbook builds clean, 0 broken links; CI build+deploy green;
    https://mattneel.github.io/frankish/ serves 200
    Did:
    - book/ mdbook (0.5.2 pinned in versions.env; make book / book-serve);
      .github/workflows/book.yml (download-pinned-mdbook → build → Pages);
      GitHub Pages set to Actions source (gh api); README + AGENTS.md map
      updated. 34 chapters, ~35k words: method / architecture / dialects /
      memory / control effects / specimens / provenance / 3 appendices
      (incl. the full D-001..D-061 ledger table, generated exact).
    - Fact-checked GC bit layout, dyn tags, decision numbers, version pins.
    Next: kernel queue unchanged (femto_lua v0.3 / uniform-sig convention /
    r7rs v0.1 / effects-v1) — the book is orthogonal to M15's queue.
    Landmines:
    - four Fable-5 subagents hit the account usage limit mid-run; they wrote
      16 chapters before dying, the rest were written directly. If drafting
      fleets again, checkpoint per-file — a mid-run credit death loses
      unwritten files silently.
    - book/book/ is gitignored; CI rebuilds from book/src/. The Pages
      workflow downloads mdbook by MDBOOK_VERSION — bump both together.

    Session end: 2026-07-03 (twenty-third entry)
    Milestone/step: M15 complete, tagged m15-done
    Green? yes — 38 blocks; 77/0 (8 runners); scheme grid 6/6 × 5 × 2
    Did:
    - D-060 κ_frk (promoted from atli) + D-061 native lowering
      (3-designer+judge panel); frk_ctl dialect + interp (6 verifiers)
      + both twins (pending cell) + kernel lowering (prompt/abort/
      pending); r7rs_core frontend (reader/ast/emit, lambda-lifting,
      call/cc→prompt, frontend-explicit guards); chibi oracle; 6-case
      corpus green everywhere; wasm display_bool i64 fix
    Next: queue to the user (femto_lua v0.3 / uniform-sig convention /
    r7rs v0.1 / effects-v1)
    Landmines:
    - scheme closure-apply tail calls NOT interp-TCO'd — deep scheme
      recursion beyond direct func.call is interp-capped (corpus
      shallow by design); uniform-signature convention lifts it
    - frk_ctl.pending returns 0 in the interp BY DESIGN (real unwind
      happens first); the emitted guard's ^propagate is interp-dead —
      do not "fix" this divergence, it is the correctness argument
    - escape continuations are APPLY-ONLY in v0 (k in operator
      position); k-as-value is fenced (needs real reified continuations)
    - wasm enforces exact import signatures: every runtime arg is i64
      (the display_bool bug); never declare a twin fn with u8/char

    Session end: 2026-07-03 (twenty-second entry)
    Milestone/step: M14 complete, tagged m14-done
    Green? yes — 37 blocks; 70/0 (7 runners); grid 65/65 × 5 × 2
    Did:
    - D-059; interp trampoline (Step::TailCall + CfgOutcome + the
      eval_function loop); frk-tail-calls pass (5th stage);
      -mtail-call for wasm; 500k-frame law goldens; runaway test
      re-fixtured non-tail; Rocq escalation filed
    Next: BLOCKED-BY-DESIGN at queue top (Rocq anchor — For the
    human); unblocked: uniform-signature convention
    Landmines:
    - a TAIL-SHAPED infinite recursion is now an infinite loop, not
      a depth trap — depth-cap tests must use non-tail fixtures
    - musttail rewrites ONLY identical-LLVM-type direct calls; do
      not widen without the uniform convention (stack-arg ABIs)
    - pkill -f with a pattern matching your own command line is
      suicide (cost one shell this session)

    Session end: 2026-07-03 (twenty-first entry)
    Milestone/step: M13 complete, tagged m13-done
    Green? yes — 37 blocks; 68/0 (7 runners); grid 63/63 × 5 × 2
    Did:
    - D-058; pack convention (wave 1, regression-first); two-slot
      arr<dyn> elements; multis/break/repeat/genfor/pairs/ipairs/
      next/string (wave 2); 4 new corpus cases; MANIFEST v0.2
    Next: queue per Next-action — scheme/ctl is queue-top
    Landmines:
    - THE fn type is fn<[arr<dyn>],[arr<dyn>]> — any helper seeded
      as a global MUST be a pack-convention _v wrapper or unwrap{5}
      type-mismatches at every call site
    - pairs order differs interp (insertion) vs native (slot) —
      LEGAL under canon's aggregation rule; never print raw pairs
      sequences in corpus cases
    - repeat's until scope: emit the condition BEFORE restoring the
      env (Lua sees body locals there)

    Session end: 2026-07-03 (twentieth entry)
    Milestone/step: M12 complete, tagged m12-done
    Green? yes — 37 blocks, 0 warnings; 64/0 (7 runners); grid 59/59
    × 5 × 2 with the collector live
    Did:
    - D-057; three-word headers + sized frees; layout descriptors +
      lockstep test; trial deletion both twins + zigcc parity rig;
      RetainKind symmetry; transfer-vs-release exclusion
    Next: queue per Next-action (human pick)
    Landmines:
    - rcword arithmetic is UNSIGNED-ONLY (color in the sign bits;
      arithmetic shifts smear — both twins document it)
    - retain coverage == trace coverage, ALWAYS; widening one side
      of the Words frontier without the other corrupts or leaks
    - a value whose only use is an owning store has TRANSFERRED its
      reference — never emit a block-exit release for it
    - explicit collect() only; no automatic thresholds yet (D-053)

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
