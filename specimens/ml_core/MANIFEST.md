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
Not started. Gated on M3 (adt) + M4 (closure).
