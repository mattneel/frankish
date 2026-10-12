# c4v1 — the seed-buildable compiler-source snapshot (post-D-125)

Status: the hop that lets calq0's own sources index through FIELD
bases (`e.asts[i]`), the surface the emitter-access migration needs.

## What this is

A copy of the canonical compiler's source set at the state where the
compiler IMPLEMENTS field-base indexing but does not yet USE it.
The PREVIOUS hop's compiler (c3v1) builds this set - the frozen seed
cannot (compile.calq uses the D-124 indexed reads, surface the seed
lacks). Each hop's sources stay within the PREVIOUS hop's surface;
that is what keeps the chain short.

    frozen seed + c2v1 -> c3v1's compiler -> c4v1's compiler
    -> the CURRENT sources (chain_fold; each snapshot's compiler
    builds the next state, ending with the current sources)

The harness's `canonical_compiler()` rides the FULL fold over every
recorded snapshot (`chain_fold`; `sorted_snapshots()` is the
version-ordered list).

- b0 cutoff: every feature through D-125 (field-base index reads:
  `v[i]` where the base is `recv.field`).
- unit set: `std/str.calq` (namespace `str`), and the compiler units
  `compiler/{lex,report,parse,emit,compile}.calq` (no namespace).
- Byte-pinned by `MANIFEST.sha256`; never edited after this point.
