# TS-0 and loanword

TS-0 is the TypeScript specimen (M9): monomorphic functions,
`number`/`boolean`/`string`, arrays, and control flow, held against
`node`/V8 as oracle (node ≥ 20; the `typescript` package pinned at 6.0.3
in `tools/loanword-ts/package.json`). It brought two firsts to the kernel:
the first floating-point idiom (`number = f64`), and the first frontend
that reaches the kernel through a *content-addressed interchange artifact*
called **loanword**.

## Checker-as-oracle, and the producer/consumer split

TypeScript's type system is too large to reimplement, and reimplementing
it would prove nothing. So TS-0 uses a **checker-as-oracle** architecture:
the real `tsc` (via a small TypeScript program under `tools/loanword-ts/`,
run by node's native type-stripping) type-checks and lowers the program to
a typed-AST artifact; a Rust *consumer* (`crates/frk-front/src/loanword.rs`)
reads that artifact and emits kernel dialects. The frontend never parses or
type-checks TypeScript; it consumes a verified elaboration. The producer
runs under `noLib` with a synthetic prelude and `noImplicitReturns`
(D-050), so the accepted language is exactly the frozen slice.

## The cryptographic contract

The loanword interchange (D-024, frozen as v1 at D-046) is deliberately
strict:

- **Canonical encoding.** JSON with recursively sorted keys, no
  whitespace, UTF-8 — so the same program produces the same bytes.
- **Content id.** A SHA-256 over the canonical encoding, carried in the
  artifact.
- **Refusal.** The consumer *recomputes* the hash and refuses to compile
  an artifact whose content id does not match its bytes. A tampered or
  truncated artifact is rejected cryptographically, not parsed
  optimistically. This refusal has its own test — the tamper is witnessed,
  not assumed.

The effect is that the frontend boundary is auditable: what the Rust side
compiled is provably the bytes `tsc` produced, and the content id is a
stable name for a program.

## Spans that point home

The loanword artifact carries source spans, and the consumer threads them
into MLIR `FileLineColLoc` locations (SPEC §6.5). This is not cosmetic: it
is what makes a runtime trap point at the *TypeScript source line*, not at
an anonymous IR op. An out-of-bounds array access in a TS-0 program traps
in the reference interpreter with the offending source location attached —
the located-trap discipline the [mem dialect](../dialects/mem.md) enforces.
The span threading arriving as a witnessed M9 exit item is why the traps
are trustworthy.

## Floats, strings, and the idioms they carried

TS-0 admitted `number = f64` (D-013, D-047) — the first float in the
kernel, entering through the admission rule as exactly the idiom ml_core's
fenced floats had been waiting for. Number *printing* is pinned to JS
semantics in the canon, so `node` and frankish format identically.

Strings entered as **UTF-16 code units** (JS semantics) — a runtime
decision (D-049) that produced `frk_str`, the code-unit string dialect,
distinct from the byte-string dialect femto_lua would later force. The
code-unit ruling fired specifically at `.length`, where the observable
difference between code units and code points first mattered. Arrays,
being an allocation *shape* rather than a value kind, live in `frk_mem`
(`!frk_mem.arr<T>`), not in a bespoke array dialect (D-049).

## The startup framing

The manifest records a performance number, but frames it honestly (D-050):
the claim is the **Static Hermes** one — predictable performance and
instant startup, not raw throughput. A boot-dominated microbenchmark
(`fib(30)` end to end) favors an AOT binary heavily because node spends
most of its wall-clock booting V8; the number is *recorded, not gated*,
and the manifest is explicit that V8 closes the gap on steady-state hot
loops. frankish does not claim to beat V8; it claims a different point on
the startup/steady-state curve, and says so.

TS-0 is SHIPPED, running byte-identical to node across the interpreter,
both memory strategies, and the five-architecture AOT grid. Its legacy to
the kernel is the loanword interchange (reused by any elaboration-carrying
frontend) and two value kinds — f64 and UTF-16 strings — that the dynamic
specimens then built on.
