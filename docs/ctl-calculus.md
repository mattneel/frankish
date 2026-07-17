# κ_frk — the frankish handler calculus (frk.ctl design)

Status: **law** as of D-060. This document is the "ctl effects design"
that SPEC §4.4 anchors and the r7rs_core stub gates on. It is promoted
from **atli** (`~/src/atli`, in-house) — the handler core of λ_Atli,
stripped to what a kernel dialect needs. atli's Rocq development
(`atli/proofs/`) is the mechanization this design leans on; frankish
does not re-prove it, it re-verifies it empirically (L1–L3).

## 0. What is taken, what is left

κ_frk takes from λ_Atli:

- **Effect labels and rows** `ε ∈ 𝒫(Label)` with join `∪` — which
  handlers must be dynamically in scope. Covariant subsumption
  (a less-effectful computation is usable where a more-effectful one
  is expected).
- **The dispatch law**: `perform ℓ v` is captured by the **innermost
  dynamically enclosing** handler with a clause for `ℓ`; nested
  handlers for other labels are transparent to the search.
- **The clause taxonomy** and its two reduction rules (λ_Atli §5):
  - `H-op-drop` — the clause never mentions the continuation `k`:
    **no continuation is materialized**, no frame is captured.
  - `H-op-resume` — the clause uses `k`: a **deep** continuation κ is
    materialized (the handler reinstalls around the resumed context)
    and κ is **one-shot**.
- **The keystone axiom** (λ_Atli design axiom 3): continuations are
  affine — used at most once. This is non-negotiable; it is what
  makes frame representation, memory strategy interaction, and
  lowering simultaneously tractable. Full multi-shot `call/cc` is a
  frankish non-goal (SPEC §14) *because* of this axiom.
- **The one-shot violation trap**: `resume κ` after κ is used is
  operationally stuck; the reference interpreter *detects and traps*
  ("one-shot violation") so the differential harness can witness the
  discipline rather than trust it.
- **Grades as codegen licenses**: every static fact the frontend
  proves is cashed as an otherwise-unsafe lowering, and every license
  carries an **empirical gate** in the harness. This method — not any
  single rule — is the real import.

κ_frk leaves in atli (with revisit conditions):

- `β` boundedness / frame sizing (atli's genuinely novel piece).
  frankish lowers control by result-passing and, later, evidence
  passing — it does not size segmented stacks. Revisit if frk.ctl
  ever grows stack-switching coroutines with arena-placed frames.
- `q` uniqueness — frankish's mem strategies (D-041) already carry
  the memory axis per-language; grading values is a frontend concern.
- `ρ` regions / task trees — revisit at the coroutine milestone.

## 1. Kernel carrier (D-031-honest)

frk.ctl is an IR dialect, not a source language: no region-bearing
ops, no variadics (D-036). Handler bodies and handled bodies are
**outlined functions** (as closures already are); labels are symbol
attributes; handler *identity* is a first-class **prompt token**
(i64), because escape points must be values (r7rs `call/ec` hands
the escape to user code).

v0 op surface (the drop-clause subset — escape continuations):

    %r   = frk_ctl.prompt  @body (args…)     // install prompt; call
                                              // @body(token, args…);
                                              // yields @body's return
                                              // or the aborted value
    frk_ctl.abort %token, %v                  // unwind to THAT prompt;
                                              // never returns

v1 surface (labeled effects, when a specimen forces them):

    %r = frk_ctl.handle @body @clause_ℓ … (args…)
    %r = frk_ctl.perform ℓ, %v
    %r = frk_ctl.resume %k, %v                // one-shot

`abort` is exactly `perform` of a distinguished per-prompt label
whose clause is `drop`-class returning the payload — v0 is not a
different semantics, it is the taxonomy's cheapest row shipped first.

## 2. Operational semantics (the reference-interpreter contract)

Transliterated from λ_Atli §5; the interpreter implements THESE rules
and is the oracle (L3):

    prompt @f (a…)          ⇒ push fresh token t on the prompt stack;
                              run f(t, a…);
                              pop t.
    f returns v normally    ⇒ prompt yields v.            (H-return)
    abort t' v  encountered ⇒ unwind to the innermost live prompt
                              whose token = t'; that prompt yields v.
                                                          (H-op-drop)
    abort t' v, t' not live ⇒ trap "escape past extent (κ_frk)".
    resume κ v, κ unused    ⇒ run κ; mark used.        (H-op-resume, v1)
    resume κ v, κ used      ⇒ trap "one-shot violation (κ_frk)".

The v1 rung as SHIPPED (M24, D-069) — the affine ladder's tractable
classes, clause-at-the-perform-site:

    handle ℓ c @f (a…)      ⇒ push prompt t + handler H{ℓ, c, t};
                              run f(t); pop both. H-return/abort as v0.
    perform ℓ v             ⇒ innermost UNMASKED H with H.ℓ = ℓ
                              (none ⇒ trap "unhandled effect (κ_frk)");
                              mask H; κ := fresh one-shot resumer,
                              BORN UNIFORM; r := c(v, κ) at the
                              perform site; unmask H;
                              consumed(κ) ⇒ perform = r   (tail-resume:
                                the clause's return IS the resume
                                value; the mask lifting is the deep
                                reinstall)
                              else       ⇒ abort(t, r)    (abortive).
    κ applied twice         ⇒ trap "one-shot violation (κ_frk)".

FENCED to the Tier-2 stack-switching rung (named; revisit at
coroutines): full re-entrant κ — non-tail resume, stored
continuations, clause code that runs after body-rest completes.

Unwinding is *observable* only through mem effects already performed
(stores, prints) — plus the ONE finalizer form: `frk_ctl.wind`
(D-070, CLOSING the former OPEN ruling): before(); r := thunk();
after(); yield r, a crossing abort re-raised AFTER after() runs.
Escape-only (before() cannot re-run — κ is one-shot, outward);
re-entrant winds are the Tier-2 rung. Natively the D-061 guard
discipline IS the finalizer hook; the interpreter mirrors it by
catching the abort around the thunk.

## 3. Licenses → lowerings → gates

The typing/charge discipline becomes verifier obligations here. Each
row: what the frontend proves ⇒ what the backend may do ⇒ the gate
that catches a false license.

| Frontend proves | Native may | Empirical gate |
|---|---|---|
| clause drops k (v0: every `abort`) | skip continuation materialization entirely; lower by **result-passing** (D-011): tagged returns threaded to the prompt | alloc-counter golden: aborting run allocates ZERO continuation frames (D-041 counters, both twins); IR golden: no capture code |
| perform's label is in the dynamic row (frontend effect check) | omit runtime search-failure paths | interp traps "escape past extent" deterministically; differential corpus keeps rows honest |
| clause tail-resumes k exactly once (v1) | direct call, no capture | forced-general vs fast-path differential (same output byte-exact) |
| general one-shot resume (v1) | materialize once, no used-flag re-checks after checker proof | one-shot violation trap golden (interp); wedge-rejection test |
| tail position (M14 law) | musttail / frame replacement | 500k-frame fixed-stack goldens (shipped m14-done) |

v0 native lowering — result-passing, the D-011 default: in a module
containing ctl ops, an aborting computation sets a runtime **pending
cell** (flag + target token + the 2-word dyn value) and returns
immediately; every intervening frame, after a call that might have
aborted, checks the flag and returns immediately too, until a
`prompt` whose token matches clears the flag and yields the value.
`prompt` call-sites compare the pending token against their own:
match ⇒ yield the parked value; miss ⇒ leave pending set and let
their own caller keep propagating. Closure thunks participate
identically (the pack convention gives lua/scheme functions one
signature; the flag rides the runtime, not the ABI).
Fence, v0: aborts do not cross an AOT `frk_entry` boundary or a
runtime-twin callback; the corpus stays inside frankish-emitted code.

**The tail-call/guard law (the make-or-break property).** A pending-
check is code that runs *after* a call returns — so guarding a call
destroys its tail shape. But scheme's loops ARE tail recursion
(millions deep) and M14 made proper tail calls law. The resolution
is exact: **only NON-tail calls are guarded; tail calls are never
guarded.** A tail call `return f(x)` returns f's result directly, so
if f aborted, this frame returns f's dummy result with the pending
flag *still set* — propagation happens for free, and the frame stays
`musttail`. The guard is needed only where code consumes a call's
result (a non-tail call), which is exactly where the tail shape was
already absent. Consequently the ctl guard pass and the M14 tail-call
pass never contend: a call is either tail (musttail, unguarded) or
non-tail (guarded, not musttail) — the same tail-shape predicate
partitions them. This must hold in BOTH the reference semantics
(where the interpreter's real unwind is inherently tail-agnostic and
therefore already correct) and native codegen.

## 4. What r7rs_core draws (the forcing specimen)

- `call/ec` (escape continuations) = `prompt` + first-class token +
  `abort`. The specimen's ratified v0 carrier for frk.ctl.
- `error` / simple `raise` = abort to the root prompt with a message
  value; the shell prints and continues (M8 exit amendment applies:
  the offending line is echoed).
- proper tail calls: already law (m14-done); scheme makes them
  load-bearing corpus-wide.
- `dynamic-wind`: OPEN — D-entry due when the specimen forces it.
- full `call/cc`, multi-shot: FENCED (SPEC §14; keystone axiom).
- hygienic macros: v1+; the expander idiom (sets-of-scopes) is a
  separate extraction — flexlang's hygienic comptime macros are the
  in-house precedent to mine.

## 5. Implementation obligations — MET at m15-done

All bars below are green: interp reference semantics (6 K2 verifiers),
result-passing native lowering (kernel-lowered ops + frontend-explicit
guards), the r7rs_core frontend, and the 6-case differential with
chibi-scheme across all 8 runners + the grid. The original bar list
follows as the historical record.


1. Interp: prompt stack + the five rules above; both traps worded
   exactly as §2.
2. Lowering: the result-passing pass; alloc-counter gate green in
   both twins.
3. Goldens: escape-from-loop, nested prompts (inner/outer targeting),
   escape-past-extent trap, error-to-shell echo; each runs the full
   seven-runner diff with chibi-scheme as the upstream oracle once
   the frontend lands.
4. The differential law (L3) is the enforcement mechanism for every
   license row — a divergence between interp and native on any ctl
   golden is a first-rank finding.
