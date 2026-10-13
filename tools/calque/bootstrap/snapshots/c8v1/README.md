# c8v1 — the compiler-source snapshot post-D-151 phase A (limited service)

Status: the hop whose compiler KNOWS the `__calque_eval_i64_limited` /
`__calque_eval_s_limited` call-emission - required before calq0's own
comptime path may pass explicit step limits (the feature-then-usage
discipline).

    ordered fold: ... -> c7v1 -> c8v1 -> CURRENT

- State: everything through D-151 phase A - the emitter's owning-module
  migration (D-142), nested comptime statements (D-143), comptime value
  parameters, type functions, reflection, anonymous structs, `comptime
  if`, field access by comptime index (D-144..D-149), and the limited
  service emission case with the `comptime(steps: N)` surface parsed
  but refused. NO source calls the limited entries yet - c7v1's
  compiler builds this set.
- Byte-pinned by `MANIFEST.sha256`; never edited after this point.
