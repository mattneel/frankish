# specimen: ml_core — v0.1 (frozen at ratification; amend via D-entry)

## Identity & pin
A MinCaml-shaped core-ML. Executable oracle: `ocaml` (version pinned in
versions.env at M5) running the subset. Readable reference: the min-caml
sources (~2kloc, the de-facto spec of this slice).

## Role
Forces: frk.adt (sums/products/match, decision trees, exhaustiveness),
frk.closure (capture analysis, env lowering), HM inference via the type kit,
let-polymorphism, recursion incl. mutual `let rec`. Runtime stays malloc-only
by design — abstraction risk first (D-009).

## Scope grammar (v0.1)
unit, bool, int (63-bit ok: match OCaml's boxed/unboxed story is FENCED —
we use i64 and note divergence in canon filter), float; tuples; algebraic
datatypes (non-polymorphic constructors v0, polymorphic at v0.2);
`let`/`let rec`/`fun`/application; `match` with nested patterns, guards
FENCED; `if`; arithmetic/comparison; arrays FENCED to v0.2; strings FENCED;
no modules, no exceptions (arrive with frk.ctl), no objects, no labels.

## Conformance
Vendored: min-caml test programs (check license before vendoring) + a hand
corpus targeting decision-tree shapes (nested, redundant, non-exhaustive-
rejection). Exclusions listed here, never edited into files.

## Oracles & canonicalization
`ocaml` output normalized per docs/canon.md; int-width divergence (63 vs 64)
documented as a canon rule, revisited at v0.2.

## Exit bars
M5: ≥90% corpus conformance, dashboard row live, extraction report written.
M6: re-based thin (zero private ops), conformance not worse.

## Status
M5 SHIPPED (2026-07-03): frontend (lex/parse/HM-with-let-poly/emit
through the decision-tree pass) + 18-program hand conformance corpus,
100% three-way (interp = jit = ocaml 4.14.1), dashboard row live.
Operative rulings and fences: D-038 (float fenced by the admission
rule ⚑; recursive ADTs to v0.2/M7; poly emission ≤1 instantiation;
redundancy = error; min-caml vendoring deferred pending license
verification — exclusion list empty because nothing is vendored yet).
Zero private ops — the M3/M4 kernel dialects carried the whole
specimen; see the M5 extraction report in STATE.md.
