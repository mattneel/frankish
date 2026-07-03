# stages.md — stage dump format v0 (SPEC §7.3)

`frnksh emit --stages <file.mlir> [--out <dir>]` writes numbered per-pass
IR snapshots — the diffable pedagogy artifact. Implementation:
`frk_harness::stages`; mechanics ruled in D-028.

## Format

    <out>/
      00-parsed.mlir       the module as parsed, before any pass
      NN-<pass-name>.mlir  the module after pipeline entry N

- Two-digit, contiguous numbering from 00. Pass names come from the
  shared pipeline table (`frk_harness::pipeline::UPSTREAM_TO_LLVM`), so a
  dump sequence *is* the pipeline definition — there is no parallel truth
  to drift.
- The out dir is removed and recreated whole on every dump; stale
  snapshots from earlier pipelines cannot linger.
- Default out dir: `out/stages/<source-stem>/` (`out/` is gitignored).

## Non-guarantees

Snapshots are MLIR's default textual form. They are a debugging and
teaching artifact, deliberately **not** goldened: their bytes track
MLIR's printer, not frankish semantics. Diff adjacent stages of one
dump; do not diff dumps across MLIR versions.

mlir-reduce integration and pass-pipeline bisection (§7.3's second half)
are scheduled with the first real debugging need, not before.
