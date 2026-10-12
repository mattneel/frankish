# c2v1 — the seed-buildable compiler-source snapshot

Status: the freeze point of the seed (BOOTSTRAP.md §1.3, D-119).

## What this is

A copy of the canonical compiler's source set (the std unit the
compiler needs, plus its five units) taken at the moment the seed was
frozen, such that the FROZEN seed can build it. Snapshots are inputs
to the bootstrap chain, never the product; the canonical compiler
lives in `tools/calque/compiler/` and evolves independently.

- b0 cutoff: every feature through D-118 (imports, `usize` as the
  i64 alias, module-member visibility, the `[T]` spelling).
- unit set: `std/str.calq` (namespace `str`), and the compiler units
  `compiler/{lex,report,parse,emit,compile}.calq` (no namespace — the
  v0 flat model).
- The files are byte-pinned by `MANIFEST.sha256` and by the
  `CALQUE_SNAPSHOT_*` checks in `scripts/calque-check.sh`. They are
  never edited after this point; a new feature that the frozen seed
  cannot parse gets a NEW snapshot directory (`c3v1`, ...).

## The hop it serves

    frozen seed + c2v1 sources -> artifact0 (the calq0 compiler)
    artifact0 + the CURRENT tools/calque/compiler sources -> artifact1
    artifact1 + the CURRENT sources -> artifact2

`make calque-bootstrap` runs all three stages, writes the artifacts
and their sha256s under `out/calque/`, and reports whether artifact1
equals artifact2 (the fixed point, in embryo; C4 completes the native
and mutation arms) plus whether the snapshot still equals the current
sources (an exact file diff). Today they are byte-identical, so the
seed also builds the current sources directly; the moment a source
feature outgrows the frozen seed, only the c2v1 hop builds it.

Note: the artifact BYTES still differ between those two builds even
today, because the sources' own test blocks embed their paths as
string constants. The comparisons of record are the source diff and
the artifact1 == artifact2 fixed point — not artifact0 against a
direct-seed build.

## Seed invocation

The seed is `tools/calque/bootstrap/seed.lua`, invoked through
`frk_harness::calque::seed_translate` (BOOTSTRAP.md §1.1's channel:
the driver assembles a Lua source with the unit table and runs it on
the femto_lua pipeline; stdout carries the artifact).
