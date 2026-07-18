# specimen: r7rs_core — v0.4 SHIPPED (D-081; m33-done)

Status: v0.4 shipped (M33): TOP-LEVEL VALUE DEFINES (one scm_globals
arr<dyn> behind a D-078 pointer cell — nil-filled at main entry,
late-bound reads, redefinition = same slot; the ts_queue pattern's
second frontend, zero kernel deltas), MAKE-PARAMETER/PARAMETERIZE
(parameters are closures over mutable state pairs — pack-length
protocol, exactly two live arms; parameterize desugars at the parser
onto dynamic-wind: eval ALL param exprs, ALL values, ALL olds,
raw-set all, LIFO raw restore in the after-thunk; escapes restore),
PLAIN RAISE (both raise kinds ride the one "exn" label behind a
flagged cons — a handler RETURNING from a plain raise is the
deterministic frk_rt_scm_trap on both twins), and GUARD (abortive
static clause + sentinel-identity dispatch at the handle site;
clauses run after unwinding in guard's dynamic environment; else-less
re-raise of non-continuables propagates to outer guards with payload
and flag preserved). ALSO: a PRE-EXISTING L3 divergence fell — an
abort raised in a dynamic-wind BEFORE-thunk now skips thunk AND
after natively, matching interp/chibi (goldens/scheme/
wind_before_abort pins it). THE POST-DIFF ADVERSARIAL REVIEW
(D-082) then found eleven defects the green suite could not see —
five fixes, all landed in-milestone with witness goldens (the
after()-context suspend, the rc wrap-transfer rule, guard-var/let
scope restoration, locals-before-primitives dispatch). 42-case
corpus vs chibi.

CORPUS LAWS (D-081, chibi-witnessed): no w-e-h handler may RETURN
normally from a plain raise, even under an enclosing guard (chibi
turns handler-returned into a secondary CATCHABLE exception and
exits 0 — "catch everything" is insufficient; we trap); no else-less
re-raising guard across a user dynamic-wind with OBSERVABLE thunks
(R7RS 4.2.7 re-raises in the ORIGINAL dynamic environment — chibi
re-fires winds twice, in-out-in-out; unimplementable before Tier-2;
parameterize interposed is byte-safe and allowed); parameterize
binds genuine parameter objects only; (p v) setter spellings hit the
arity trap; use-before-define of a top-level value reads
deterministic nil on both twins where chibi errors (off-corpus, the
harness's nonzero-exit exclusion covers it automatically).

Fences v0.5+: the make-parameter CONVERTER (admission tests recorded
in D-081 — all converts precede any set; a raising converter leaves
earlier bindings unbound), guard (test => proc) and expression-less
(test) clauses, continuable re-raise through an else-less guard
(Tier-2 stack switching — a LOUD trap, never silent), error objects,
string-ref/chars, string-set!, symbol?/string?/vector?, #(...)
literals, list->vector, internal defines, define-syntax.

## Admission justifications v0.4 (L5)

| Feature | Idiom the kernel lacked |
|---|---|
| top-level value defines | module-level mutable state — D-078's rung consumed by a second frontend |
| make-parameter/parameterize | dynamic binding — the first composition of wind + module state |
| guard | post-unwind value-carrying clause dispatch — D-076's marker generalized to sentinel identity |
| plain raise | the continuable/non-continuable class distinction; guard re-raise parity needs it |

Previously — v0.3 (D-077; m31-done):

Status: v0.3 shipped (M31): PAIR MUTATION (set-car!/set-cdr! as
frk_mem.field_set on the boxed cons cell — the representation went
honest: aliases share the cell on both twins; cyclic cons rings
collect, drilled in both collectors), STRINGS (tag-3 bstrs like
symbols — interned, so string=? is a pointer compare even for
dynamic strings; append/length/=?/substring; the symbol?/string?
predicates are fenced until the tag split), and VECTORS
(TAG_VECTOR = 7 over arr<dyn> — the third D-051 widening;
make-vector with required fill, vector, -ref/-set!/-length).
16-case corpus vs chibi. Fences v0.4+: string-ref/chars,
string-set!, symbol?/string?/vector?, #(...) literals,
list->vector, make-parameter/parameterize, guard, plain raise,
top-level value defines, define-syntax.

Previously — v0.2 (D-071; m26-done): R7RS exceptions —
with-exception-handler + raise-continuable over the v1 handlers
(handle{label=exn}; the handler's return IS raise-continuable's
value; nested handlers delegate outward via the D-069 masking rule)
— and FIRST-CLASS PROCEDURES de-fenced (lambdas as uniform pack-fn
closures; procedure-valued application through the uniform
convention). 15-case corpus vs chibi, all runners, all triples.
Fences v0.3+: plain raise (handler-returns semantics), guard sugar,
error objects, make-parameter/parameterize (wants top-level value
defines), set-car!/set-cdr!, strings, vectors, define-syntax.

Previously — v0.1 (D-070; m25-done):

Status: v0.1 shipped (M25): pairs/lists (TAG_PAIR = 6 — the D-051
widening — as wrapped product<[dyn,dyn]> carriers), quote + symbols
(interned tag-3 bstrs; eq? via frk_bstr.eq), and DYNAMIC-WIND closed
escape-only as frk_ctl.wind (afters run innermost-first exactly once
on normal and escape exits; re-entrant winds = the Tier-2 rung).
11-case corpus byte-identical across interp, jit×{arena,rc}, the
grid (4 triples + s390x, both strategies), and chibi 0.9.1. Fences
v0.2+: set-car!/set-cdr! (pair mutation reopens the cycle question),
strings-as-strings, vectors, chars, top-level value defines,
define-syntax (the expander).

Previously — v0 (D-060/D-061; m15-done): 6-case corpus; call/ec +
tail calls proven load-bearing; frk.ctl forced into existence.


Pin: R7RS-small, core sublanguage. Oracle: **chibi-scheme 0.9.1**
(`chibi-scheme -q`, versions.env `CHIBI_VERSION_TESTED`); chez
remains the performance ceiling reference, not a harness oracle.
Role: **tortures frk.ctl** — this specimen exists to force the
κ_frk design (docs/ctl-calculus.md) into ops, and to make proper
tail calls load-bearing corpus-wide. The stub's ratification gate
("do not ratify before the ctl effects design lands") is satisfied
by κ_frk landing in-repo.

## Frozen subset, v0

- Fixnum integers (i64; corpus stays in range), booleans.
- `define` (top-level values and procedures), `lambda`, `if`,
  `let`, `let*`, `letrec` (mutual recursion), `begin`.
- Arithmetic/comparison: `+ - * quotient remainder = < > <= >=`.
- **Proper tail calls** — the law is already paid (m14-done);
  scheme makes it observable everywhere: deep loops in the corpus
  are written as tail recursion, no loop syntax exists.
- **`call/ec`** (escape-only continuation; in the oracle spelled
  via `call/cc` used one-shot-escapewise) and **`error`** — the
  frk.ctl v0 carriers (prompt/abort). THE reason this specimen is
  admitted at all.
- `display`, `newline` — output per canon §7 (fixnums print as
  decimal; booleans as `#t`/`#f`).

## Admission justifications (L5)

| Feature | Idiom the kernel lacked |
|---|---|
| call/ec, error | frk.ctl prompt/abort — escape continuations, the drop clause |
| tail-call-only loops | forces M14's law from "golden" to "load-bearing" |
| lambda/letrec | none new — admitted as carriers (closure dialect exists) |

## Fences (not TODO lists — L5)

- Pairs/lists, `quote`, symbols-as-values — v0.1, as frk_adt/bstr
  carriers, when a corpus case needs structured data.
- Strings, chars, vectors, ports — carry nothing new yet.
- `dynamic-wind` — OPEN ruling due when forced (κ_frk §2).
- Full/multi-shot `call/cc` — NON-GOAL (SPEC §14; κ_frk keystone).
- Hygienic macros (`define-syntax`) — v1+; sets-of-scopes expander
  is its own extraction (flexlang precedent in LANDSCAPE).
- Ratios, flonums, bignums; `eqv?` identity subtleties; TCO across
  `apply`/`map` (no higher-order stdlib in v0).

## Oracle protocol

`chibi-scheme -q case.scm`, LC_ALL=C, stdout byte-compared after
canon. Corpus law: every case runs interp, jit×{arena,rc}, AOT
grid, and chibi; disagreement is a first-rank finding (L3).
