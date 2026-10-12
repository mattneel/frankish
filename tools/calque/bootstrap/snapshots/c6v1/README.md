# c6v1 — the compiler-source snapshot post-D-134 (comptime surface staged)

Status: the hop whose compiler KNOWS the `__calque_eval_i64`
call-emission - required before calq0's own comptime path may CALL the
service (the feature-then-usage discipline).

    ordered fold: ... -> c5v1 -> c6v1 -> CURRENT

- State: the parse surface for `comptime { ... }` / `comptime e`, the
  staged sub-module builder (the error until the usage lands), and the
  `__calque_eval_i64` emission case in emit_call. NO source calls the
  service yet - c5v1's compiler builds this set.
- b0 cutoff: every feature through D-134's first half.
- Byte-pinned by `MANIFEST.sha256`; never edited after this point.
