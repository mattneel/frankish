# calque examples — frozen surface forms

These files are part of the M36 (C0) freeze (SPEC §12): their surface
forms define Calque syntax and MUST NOT be redesigned. "Frozen" means
the spelling is fixed; it does NOT mean executable yet — each file
becomes executable at the stage below, at which point it joins the
corpus (`goldens/calque/`) with independently derived expected
output as a comment in the file.

| File | Surface it pins | Frozen by | Executable at |
|---|---|---|---|
| `active_score.calq` | records, `pub fn`, pipeline chains, `seq.*`, closure literals with inferred params | GOAL §7; D-086 | C2 (M38) |
| `identity_pair.calq` | `comptime T: type` parameters, functions returning types, type expressions | GOAL §9; D-086 | C3 (M39) |
| `fib.calq` | b0 function syntax, `if` as expression, recursion, `io.print`, `str.fromI64` | D-086 | C1 (M37) |
| `squares.calq` | b0 `var`/assignment, `while`, `Vector(i64)`, `vector.*` | D-086 | C1 (M37) |

All examples are ASCII (SPEC F18). Expected outputs are recorded as
comments ONLY for the b0 cases, where the expected bytes were
derived by hand from SPEC semantics (the no-upstream-oracle rule,
MANIFEST "Conformance").

The fuller surface-intent source is the user's demo playbook
(`docs/calque-demo-playbook.md`; adopted by D-090) — it is the
storyboard of record for C6 and the spelling source for the C2+
forms (dot construction, `union(enum)`, `try`/`catch`, attributes,
test blocks, and the rest). Where a revision landed, SPEC §12's
freeze log cites the revising entry.
