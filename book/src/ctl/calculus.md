# κ_frk — The Handler Calculus

`frk_ctl` is the control-effects dialect: escape continuations today,
labeled effect handlers next. Unlike the other dialects, it was required
to arrive with a written calculus — the project's spec anchors the
control design to a typing/charge discipline whose rules become the
dialect's verifier obligations. That calculus is **κ_frk**, recorded
normatively in `docs/ctl-calculus.md` (D-060); this chapter is its guided
tour.

## Provenance

κ_frk is promoted from **atli**, an in-house graded handler calculus
(effects, uniqueness, frame-boundedness, and regions as grades; a Rocq
mechanization; the operational rules this chapter uses). frankish takes
atli's *handler core* and its *method* — every static fact is a "codegen
license" cashed as an otherwise-unsafe optimization, and every license
carries an empirical gate. What κ_frk deliberately leaves in atli, with
recorded revisit conditions: the frame-boundedness grade `β` (frankish
lowers control by result-passing, not sized stack segments), the
uniqueness grade `q` (frankish's memory axis lives in the mem
strategies), and the region/task grade `ρ` (waits for coroutines).

## The keystone: one-shot continuations

κ_frk's non-negotiable axiom, inherited intact: **continuations are
affine** — used at most once. This single restriction keeps four things
simultaneously tractable: frame representation, memory-strategy
interaction, lowering without an unwinder, and (in atli, where it is
proven) totality analysis. Multi-shot `call/cc` is a frankish non-goal
(SPEC §14), and the calculus is why the fence will hold: it is a design
axiom, not a deferred feature.

## The dispatch law and the clause taxonomy

A `perform ℓ v` is captured by the **innermost dynamically enclosing**
handler with a clause for `ℓ`; nested handlers for *other* labels are
transparent to the search. Handler clauses divide into two classes, and
the division is the entire optimization story:

- **`H-op-drop`** — the clause never mentions the continuation `k`. Then
  **no continuation is materialized at all**: no frame capture, no
  allocation. Escapes, early exits, and exceptions are all drop-clause
  instances.
- **`H-op-resume`** — the clause uses `k`. A *deep* continuation κ is
  materialized (the handler reinstalls itself around the resumed
  context), and κ is marked **one-shot**. Resuming a used κ is
  operationally stuck; the reference interpreter turns that stuck state
  into a detectable trap — *"one-shot violation"* — so the discipline is
  witnessed by tests, not trusted.

As of effects-v1 (M24, D-069) the shipped rung covers the affine
ladder's tractable clause classes — drop (v0), **abortive** (the
clause returns without consuming κ; the handle yields its value),
and **tail-resume** (the clause consumes κ exactly once; its return
IS the resume value) — with the clause running *at the perform
site*: dispatch masks the handler for the clause call (performs
inside the clause go outward) and the mask lifting is the deep
reinstall. κ is *born uniform*: a closure over a one-shot marker
whose application marks-or-traps and returns its pack. Full
re-entrant κ — non-tail resume, stored continuations — is the named
Tier-2 stack-switching rung. The v0 drop-clause ops:

| Op | Signature | Semantics |
|---|---|---|
| `frk_ctl.prompt` | `(body: !frk_closure.fn<[i64],[!frk_dyn.dyn]>) -> !frk_dyn.dyn` | install a fresh prompt; call `body(token)`; yield its return, or the aborted value if an abort targeted **this** prompt |
| `frk_ctl.abort` | `(token: i64, value: !frk_dyn.dyn)` | unwind to the live prompt whose token matches; never returns |
| `frk_ctl.pending` | `() -> i64` | the result-passing carrier (see [the lowering chapter](lowering.md)); answers 0 in the reference semantics |

Prompt identity is a first-class **token** (an `i64`), because Scheme's
`call/cc` hands the escape to user code as a value. Tokens are monotonic
and never reused within a run, so a stale escape can never alias a fresh
prompt.

## The operational rules

The interpreter implements exactly these, and is the oracle:

```
prompt @f (a…)          ⇒ push fresh token t; run f(t, a…); pop t.
f returns v normally    ⇒ prompt yields v.                    (H-return)
abort t' v encountered  ⇒ unwind to the innermost live prompt
                          with token t'; that prompt yields v. (H-op-drop)
abort t' v, t' dead     ⇒ trap "escape past extent (κ_frk)".
resume κ v, κ unused    ⇒ run κ; mark used.              (H-op-resume, v1)
resume κ v, κ used      ⇒ trap "one-shot violation (κ_frk)".
```

Six interpreter verifiers pin the rules down (`crates/frk-dialects/tests/
ctl_eval.rs`): normal pass-through; an abort landing at its own prompt;
an abort fired 1,000 frames deep in a **tail-recursive** chain (proving
the escape threads correctly up through the tail-call trampoline's frame
replacement); an inner abort caught by an inner prompt while the outer
continues; an inner abort *targeting the outer* prompt, unwinding through
a passed-over inner prompt; and the escape-past-extent trap — whose test
round-trips the token as a value first, proving tokens really are values.

## Licenses → lowerings → gates

The atli method, applied: each row is a static fact, the unsafe thing it
licenses, and the harness gate that catches a false license.

| Frontend proves | Native may | Gate |
|---|---|---|
| clause drops `k` (v0: every abort) | skip continuation materialization; lower by result-passing | allocation counters: an aborting run allocates **zero** continuation frames |
| tail position (M14 law) | `musttail` / frame replacement | 500k-frame fixed-stack goldens |
| tail-resume exactly once (v1) | direct call, no capture | forced-general vs fast-path differential |
| general one-shot resume (v1) | materialize once, no re-checks | one-shot violation trap golden |

The enforcement mechanism for every row is [the differential
law](../method/differential.md): the interpreter's real unwinding and the
native result-passing lowering are two maximally different
implementations of the same rules, and the corpus holds them
byte-identical — against `chibi-scheme` as the outside witness.

## What r7rs_core draws from this

Scheme is the forcing specimen: `call/cc` used escape-wise becomes
`prompt` + a first-class token + `abort`; deep loops are tail recursion
under the M14 law; `dynamic-wind` closed escape-only at M25 (D-070 —
"wind is THE finalizer form", and D-081.0 later fixed the native path
so an abort raised in the *before*-thunk skips thunk and after, as the
reference always did); the exception surface grew in three steps —
tail-resumptive `raise-continuable` (M26), then plain `raise` and
`guard` (M33) as the flagged-payload and value-carrying-abortive
idioms over the same one label; full multi-shot `call/cc` stays fenced
by the keystone, and everything that needs re-entry (guard's true
re-raise environment, coroutines, generators) is the named Tier-2
stack-switching rung. The [specimen chapter](../specimens/r7rs.md)
shows the surface side of the same story.
