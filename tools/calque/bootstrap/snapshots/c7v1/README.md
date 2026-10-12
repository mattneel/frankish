# c7v1 — the compiler-source snapshot post-D-136 (the string service case)

Status: the hop whose compiler KNOWS the `__calque_eval_s`
call-emission - required before calq0's comptime path may produce
STRING results (the feature-then-usage discipline).

    ordered fold: ... -> c6v1 -> c7v1 -> CURRENT

- State: the `__calque_eval_s` emission case in emit_call (the bstr
  arm of the evaluation service). NO source calls it yet - c6v1's
  compiler builds this set.
- b-0 cutoff: every feature through D-136's first half.
- Byte-pinned by `MANIFEST.sha256`; never edited after this point.
