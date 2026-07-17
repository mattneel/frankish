# specimen: r7rs_core — v0.1 SHIPPED (D-070; m25-done)

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
