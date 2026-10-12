# c3v1 — the seed-buildable compiler-source snapshot (post-D-122)

Status: the hop that lets calq0's own sources adopt the D-122 indexed
surface (BOOTSTRAP.md §5, D-123).

## What this is

A copy of the canonical compiler's source set (the std unit the
compiler needs, plus its five units) at the state where the compiler
IMPLEMENTS indexed access but does not yet USE it. The FROZEN seed can
build this set (verified: the `make calque-bootstrap` stage 1 runs the
seed on it); the moment the canonical sources start using `v[i]` (a
surface the pre-D-122 seed cannot parse), this snapshot is the
compiler that carries the chain:

    frozen seed + c3v1  -> artifact0 (a D-122-capable compiler)
    artifact0 + the CURRENT compiler sources -> artifact1 (the chain)
    artifact1 + the CURRENT sources -> artifact2 (fixed point)

The harness's `canonical_compiler()` prefers the direct build
(seed + current sources) while that parses, and falls back to this
chain automatically. c2v1 remains as the earlier recorded hop; the
two snapshots are independent freezes.

- b0 cutoff: every feature through D-122 (vector `v[i]` reads, the
  persistent `v[i] = e` / `+=` writes, the union `==`/`!=` tag
  fix).
- unit set: `std/str.calq` (namespace `str`), and the compiler units
  `compiler/{lex,report,parse,emit,compile}.calq` (no namespace - the
  v0 flat model).
- Byte-pinned by `MANIFEST.sha256`; never edited after this point.

## Invocation

`make calque-bootstrap` (the `calque bootstrap` driver) takes the
snapshot directory as its optional argument; the harness chain path
uses the same recipe.
