# Milestones and Extraction

Work proceeds by milestones, and the constitution fixes the loop:

> For each milestone: plan against the exit criteria, implement under
> L1–L3, run the full suite, write the milestone note in STATE.md (what
> shipped, what was learned, what cheats exist awaiting promotion), tag
> `mN-done`, push. Do not start Mn+1 with Mn red.

SPEC §13 defines M0–M10 with per-milestone exit criteria ("Each ends with:
suite green, STATE.md milestone note, tag `mN-done`, push"). Beyond M10 the
tracks are unscheduled peers — femto_lua implementation, the GC ladder,
scheme/ctl, effects, staging — and picking the next one is a taste call the
constitution routes to the human. Each beyond-M10 milestone therefore
begins with a ledger entry that *is* its contract: D-054 (M11), D-057
(M12), D-058 (M13), D-059 (M14), D-060/D-061 (M15) each record the human's
pick or the L4 call, the scope, and the exit bars, before the code.

The milestone note is not a changelog. Its required parts — what shipped,
what was learned, what cheats exist awaiting promotion — plus, for specimen
milestones, an **extraction report** (SPEC §8 law: "the deliverable is the
promotion pass"), make every milestone auditable against its own claims.
The notes also carry the negative results: bugs the verifiers found, with
the finding order preserved, and per-session **landmine lists** for the
next agent ("a TAIL-SHAPED infinite recursion is now an infinite loop, not
a depth trap"; "rcword arithmetic is UNSIGNED-ONLY"; "never call
interpret_entry on a default 2 MiB test thread"). Cadence is law L8: commit
at every green step, push at minimum every three green steps and at every
milestone exit.

## The record, m0 → m15

All sixteen milestones are tagged in the repository; summaries below are
condensed from the milestone log in `STATE.md`. M0 through M10 — the
scheduled program — completed in one session-day, "zero red pushes
surviving uncorrected."

| Tag | Shipped |
|---|---|
| `m0-done` | Workspace skeleton (7 crates + sandbox/); `versions.env` as the single pin point (Rust 1.96.0, LLVM/MLIR 22, melior 0.27.2); `make setup\|build\|test\|ci`; smoke verifier `jit_add_i64` asserting 40+2=42 through a real ExecutionEngine. |
| `m1-done` | `docs/canon.md` v0 + the single canon filter; the golden engine (case layout, strict directives, bless reporting changed/unchanged); 6-case upstream corpus with outputs computed by hand before the runner ran; differential scaffold; stage dumps (D-027/D-028). |
| `m2-done` | frk-interp: the K2 Eval trait + derived interpreter over func/arith/scf/cf; D-029 total-and-deterministic trap semantics (UB traps, 1024-frame depth cap) and the UB-free corpus rule; interp installed as reference runner (D-008); L3 enforced in CI, 8 cases two-way. |
| `m3-done` | frk.adt, the first kernel dialect, full K1–K7 under D-031 (pure IRDL, trait-free, no match op); the Maranget decision-tree pass with ten byte-exact tree goldens (D-034); external lowering pass (D-032); the `runners=` machinery caught a real jit/interp divergence on day one. |
| `m4-done` | frk.closure K1–K7 — and the discovery-driven redesign of frk.adt to packed surfaces after the IRDL variadic-unification ceiling (D-036, first-rank finding); heap envs via `frk_rt_alloc`; synthesized thunks; church encoding `two(inc)(40)=42` under both runners. |
| `m5-done` | The first specimen end to end: ml_core lexer/parser, HM over ena with real let-polymorphism, typed-AST emission through the decision tree; the OCaml oracle joins; 18-program corpus, 100% three-way on first contact; SlotKind::Words widening forced within minutes of real programs. |
| `m6-done` | Promotion pass #1: tree→dispatch-IR emission promoted out of frk-front into `frk_dialects::dtree_emit`; `docs/type-kit.md` documents what travels vs what stays per-frontend; D-039 (green tree deferred, named trigger), D-040 (specimen order confirmed). |
| `m7-done` | frk.mem (third dialect) with Strategy ∈ {Arena, Rc} as a lowering parameter (D-041 ⚑); the Tier-0 AOT grid (D-042): zig-cc-linked musl-static binaries under qemu/wasmtime — every golden × both strategies byte-exact on five architectures including the s390x big-endian canary. |
| `m8-done` | The frankish shell (SPEC §9, D-043): bare `frnksh` = REPL on the reference interpreter, re-elaborate-whole session model; transcript goldens; first human ⚑ review integrated (D-044) including the error-echo exit amendment. |
| `m9-done` | loanword v1 frozen (D-046: canonical JSON, SHA-256 content id, tamper refusal proven) + the whole TS-0 stage: f64, boxes, arrays, UTF-16 strings, span→location threading witnessed by a source-mapped OOB trap; node is the sixth runner; grid 47/47 × 5 × 2. |
| `m10-done` | The gate milestone: `docs/gc-spike.md` + D-053 (rc+cycles beats MMTk on the two-twin runtime and the grid); D-051 fat dyn values; D-052 femto_lua ratified with the Lua string ruling. The scheduled program complete. |
| `m11-done` | femto_lua v0.1 + GC ladder rung 1 (D-054): dyn K3 fat-value lowering; block-local releases + the leak canary; frk_bstr (interned byte strings); raw tables; the synthesized-protocol pattern (D-056.2); canon §7 %.14g with the tie-parity rig; 8/8 vs lua5.1, 64 diff cases across seven runners. |
| `m12-done` | The GC ladder's remaining rungs (D-057), both twins: three-word headers, sized cascading frees, layout descriptors computed by the lowering, Bacon–Rajan trial deletion; grid 59/59 × 5 × 2 **with frees live** — every rc golden a standing use-after-free detector. Three collector bugs found by verifiers within minutes. |
| `m13-done` | femto_lua v0.2 (D-058): the pack convention `fn<[arr<dyn>],[arr<dyn>]>` — multiple returns, nil-fill, destructuring, break/repeat, generic `for` over the real (f, s, ctrl) protocol; 12/12 vs lua5.1; the kernel paid exactly one widening (two-slot `arr<dyn>` elements). |
| `m14-done` | Tail calls as law, first rung (D-059): the interpreter trampolines every tail-shaped call (frame replacement; the D-029 cap counts non-tail entries only); native musttail for identical-signature direct tails; wasm `-mtail-call`; 500k-frame fixed-stack goldens that fail without each rung, green on all five architectures. |
| `m15-done` | frk.ctl v0 (escape continuations — prompt/abort/pending, κ_frk per D-060, native lowering per D-061) + the r7rs_core specimen that forced it; the reference interpreter really unwinds, native is result-passing through a runtime pending cell; 6-case scheme corpus byte-identical across all 8 runners and the grid. |

The suite as of `m15-done`: `make test` runs 38 result blocks; the
differential matrix holds 77 cases at zero divergence across eight runners
(`interp`, `jit`, `jit-rc`, `ocaml`, `node`, `lua`, `scheme`, `repl`); the
AOT grid is green over {x86_64, aarch64, riscv64, wasm32-wasi} × {arena,
rc} with s390x as the big-endian canary.

## Extraction is the thesis working

The specimen law (SPEC §8) permits a specimen's first implementation to
cheat — private ops, ad-hoc lowerings — because the *deliverable* is the
promotion pass: extract what the specimen forced into kernel dialects, then
re-base the specimen and show conformance unchanged. "A specimen still fat
after promotion is evidence the abstraction is wrong; file it, don't paper
it." Three extractions show the loop at three scales.

### M6 — the decision-tree emitter leaves ml_core

M5's extraction report named exactly one component built a layer too high:
tree→IR emission, constructed inside `frk-front` because ml_core was its
first consumer (D-034 had deliberately deferred emission until a consumer
existed — "never speculatively"). M6 promoted it into
`frk_dialects::dtree_emit`, and the promotion's shape is the interesting
part: the seam is **arm-emission-only**, because occurrence typing derives
from the kernel types themselves — the component walks the scrutinee's
`!frk_adt` type through `decode_sum`/`decode_product`, so the only thing a
frontend supplies is a callback emitting arm bodies. Bool dispatch and the
single-variant-inline rule moved down with it; `frk-front` shrank by five
private functions; the component gained a frontend-free verifier
(hand-built module, callback arms, interpreted). Exit bars held: zero
private ops in ml_core, conformance not worse (33 cases, 0 divergent,
three-way). Every match-bearing frontend since — none of which existed when
the code was written — calls the same emitter.

### M13 — the pack convention, priced exactly

femto_lua v0.2's headline was a *calling-convention* change: every Lua
function becomes `fn<[arr<dyn>], [arr<dyn>]>` — one argument pack in, one
values pack out, parameters read through a bounds-checked nil-fill helper.
D-058 predicted the change would be frontend-only, and the milestone note
confirms it with an itemized bill: the kernel paid **one widening**
(two-slot `arr<dyn>` elements, stride-addressed) — which M12's
`ARRAY_DYN` layout tracer already knew how to walk, so the collector
handled argument packs with zero new GC code — plus `frk_dyn.table_next`
(the first two-result kernel op) and `frk_bstr.sub/rep`. What v0.2 did
*not* force is listed with equal care: closures, adts, mem, the dyn core,
the collector — untouched. The extraction report then looks forward:
the pack convention is "promotable thinking" — TS-1's union-narrowing
calls and scheme's multi-value continuations will both want it — a note
that became load-bearing two milestones later.

### M15 — the fork that resolved both ways

When r7rs_core arrived, the obvious move was to reuse femto_lua's pack
convention for scheme procedures. The M15 extraction report records the
fork resolving the **opposite** way: the scheme frontend lambda-lifts
procedures to direct `func.func` calls — real M14 tail calls, no per-call
pack allocation — "because scheme leads with tail-recursion where lua led
with arity." Same kernel, two frontends, two deliberately different
conventions, each justified by what its language makes load-bearing. The
same report banks the next promotion candidate (the escape-closure pattern
— a closure capturing a prompt token, its body aborting — "is exactly what
a general one-shot `resume` will generalize at effects-v1") and fences the
remaining gap honestly: closure-apply tail calls are not yet trampolined in
the interpreter, so deep scheme recursion beyond direct call chains is
interp-capped and the corpus stays shallow by design, until the
uniform-signature convention (the D-059 ledgered gap) lifts it.

The pattern across all three: the specimen forces, the extraction names
what was forced and what was not, and the un-forced list is as much a
result as the shipped code — it is the evidence that the kernel's existing
abstractions carried the load.
