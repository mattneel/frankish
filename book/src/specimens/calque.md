# calque — the Self-Hosted Product Language

Every other specimen in this book pins a subset of an existing
language and borrows its upstream implementation as an oracle.
**Calque** is the first ORIGINAL specimen: a strict, Zig-like
functional language with no upstream, whose canonical compiler is
written in Calque itself (D-085).

The acceptance contract is the goal document itself
(`docs/calque/GOAL.md`, stored verbatim and sha-pinned); the language
is specified in `docs/calque/SPEC.md` (v0.1 freezes the bootstrap
subset **b0** and its syntax decisions in a freeze log); the live
evidence matrix is `docs/calque/ACCEPTANCE.md`, checked by
`make calque-test`.

## The bootstrap shape

A source file cannot execute before something translates it — so the
chain starts from a small **auditable seed**: a Lua 5.1 translator
running on the already-verified femto_lua specimen
(`docs/calque/BOOTSTRAP.md`; D-086). The seed translates the
bootstrap-compatible compiler source into **kernel IR text** — the
artifact — which Frankish then parses, verifies, and either
interprets or compiles natively. The same derived interpreter that
runs ordinary programs supplies Calque's `comptime` execution: there
is no second evaluator anywhere in the stack.

Because there is no upstream oracle, the verification story is
deliberately *more* verifiers, not fewer: expectations derive from
the spec by hand before the compiler runs; interpreter, JIT, and AOT
must agree on every golden; interpreted and native compilers must
emit canonical-equal artifacts; the fixed-point comparator is itself
mutant-tested. Self-compilation is reported as bootstrap evidence —
agreement between execution paths — never as a correctness proof.

## Status

Workstream milestones (from the repository's current ledger):
M36 = C0 charter (this chapter's milestone: goal record, manifest,
spec + b0 freeze, bootstrap route, acceptance matrix,
`make calque-test`), M37 = C1 interpreted compiler, M38 = C2 the
useful functional language, M39 = C3 shared comptime, M40 = C4
native self-host + performance, M41 = C5 developer experience +
portability, M42 = C6 release + demonstrations. Nothing on this page
claims to run yet; `docs/calque/ACCEPTANCE.md` is the authoritative,
command-backed status.
