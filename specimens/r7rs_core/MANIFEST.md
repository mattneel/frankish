# specimen: r7rs_core — v0 SHIPPED (D-060/D-061; m15-done)

Status: v0 shipped. 6-case corpus byte-identical across interp,
jit×{arena,rc}, the AOT grid (x86_64/aarch64/riscv64/wasm32 + s390x
canary, both strategies), and chibi-scheme 0.9.1. call/ec + tail
calls proven load-bearing; frk.ctl forced into existence.


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
