# The Toylang Quarry

frankish did not appear from nothing, and its prior art is unusually
close at hand: three complete language implementations by the same
author, each of which independently rebuilt the same middle layer this
project exists to make reusable. They are recorded in
`docs/LANDSCAPE.md` as **in-house prior art** — a design quarry to mine,
with promotion preferred over reinvention.

## The pattern that demanded a workbench

Lay the three side by side and the repetition is the argument:

| | atli | inscription | flexlang |
|---|---|---|---|
| Character | graded, coeffectful functional systems language | deterministic prose-punctuation systems language | F#/Elm-flavored functional systems language |
| Backend | MLIR → LLVM | MLIR → LLVM 22 | MLIR → LLVM 22 |
| Reference semantics | oracle interpreter, differential vs native | deterministic tooling end to end | pure-Python interpreter, differential vs native |
| Effects | effect rows + handlers, one-shot continuations | — | explicit effects |
| Memory | regions/arenas sized by the type system | owned buffers, move analysis | region-based allocation |
| Metaprogramming | — | comptime evaluation | hygienic comptime macros |

Three times: an interpreter held against a native MLIR backend as the
semantic reference. Three times: an effects-or-ownership discipline
enforced by a checker. Three times: the ADT/closure/memory middle layer,
built from scratch. frankish is the decision to stop — build the middle
layer once, as kernel dialects, and let *specimens* rather than whole new
languages be the forcing function.

## atli — the calculus source

atli is the deepest of the three: a graded type theory where a capability
row travels with every computation — effects `ε`, uniqueness `q`,
frame-boundedness `β` (the checker literally computes the frame
allocation native code will use), and regions `ρ` — with a Rocq
mechanization and a discipline the project calls **grades as codegen
licenses**: every static fact is cashed as an otherwise-unsafe backend
optimization, and every license has an empirical gate.

Its contribution to frankish is direct and load-bearing. When the
control-effects design came due, the spec's anchor — "the human's
handler calculus" — turned out to *be* atli. [κ_frk](../ctl/calculus.md)
(D-060) promotes its handler core: the innermost-dynamic dispatch law,
the drop/resume clause taxonomy, deep one-shot continuations as the
keystone axiom, both runtime traps, and the licenses-with-gates method
itself. What κ_frk deliberately left behind — `β` frame-sizing, `q`
uniqueness, `ρ` regions — is recorded with revisit conditions, so the
unmined veins stay mapped.

## inscription — the determinism discipline

inscription is a systems language with prose-punctuation syntax whose
distinguishing obsession is *deterministic tooling*: canonical
formatting, stable diagnostic codes with an `explain` catalog,
deterministic release archives with checksums, a source index and LSP
built on reproducible queries. Its influence on frankish is the
tooling-honesty posture — pinned toolchains, byte-exact goldens,
stage dumps as first-class artifacts — and LANDSCAPE keeps its
diagnostic-catalog design flagged as the model for a future `frnksh`
diagnostics rung. (This book's mdbook + GitHub Pages arrangement is
inscription's, adopted wholesale.)

## flexlang — the expander precedent

flexlang pairs explicit `Result`-based failure and explicit effects with
**hygienic comptime macros** — a working expander in a
reference-interpreter-plus-MLIR architecture. Two future frankish
obligations point straight at it: r7rs_core's fenced `define-syntax`
(the sets-of-scopes expander is its own planned extraction), and the
`frk.stage` staging dialect, for which flexlang's comptime evaluation is
the in-house precedent.

## The delegation, on the record

One ledger entry makes this chapter's relationship official. The control
design was originally blocked *by its own gate*: the r7rs_core specimen
stub forbade ratification "before the ctl effects design lands," and the
spec anchored that design to the author's calculus — an artifact only he
could supply. The block was escalated through the proper channel
(`STATE.md`, "For the human") and resolved by delegation: *the calculus
was already written; it was atli all along*. D-060 records the
resolution, κ_frk landed the same day, and the specimen gate opened.

The lesson frankish takes from its own provenance: good prior art is not
a bibliography, it is inventory. The quarry is mapped, the extraction
conditions are written down, and every promotion so far — the
differential method, the handler calculus, the book you are reading —
has been cheaper and sounder than reinvention would have been.
