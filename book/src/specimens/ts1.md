# TS-1 — Narrowing as Contract

TS-1 is the TypeScript specimen's second stage (D-072) and the research
slice its manifest promised from the start: *"narrowing facts are
imported as cast annotations and re-verified by our own
dominance/dataflow pass; unverifiable casts demote to frk.contract
runtime checks."* At M27 that sentence became code, and
[`frk_contract`](../dialects/contract.md) became a dialect.

## The slice

Discriminated unions in the classic TS idiom:

```typescript
type Circle = { kind: "circle"; radius: number };
type Square = { kind: "square"; side: number };
type Shape = Circle | Square;

function area(s: Shape): number {
  if (s.kind === "circle") {
    return 3.14 * s.radius * s.radius;
  } else {
    return s.side * s.side;
  }
}
```

A union value **is** an `frk_adt.sum` — variant order is declaration
order, and `kind` is *not a stored field*. In test position
`s.kind === "circle"` lowers to `tag_of` + `cmpi` (precisely the shape
the promotion pass re-derives facts from); as a value it lowers to a
tag-selected literal chain. Payload fields keep declaration order and
extract by index. The checker-as-oracle rule holds throughout: tsc
typechecks, we never re-implement the checker — the producer just asks
`getTypeAtLocation` at every union-typed use and exports the narrowing
it finds as `narrow` nodes (loanword's vocabulary extends additively
within v1, resolving D-046's "vocabulary at TS-1" revisit).

## Trust but verify

The exported facts are untrusted. Inside `area`, `s.radius` compiles to
`narrow`-then-`extract`; the interpreter checks every narrow, and the
native promotion pass re-proves them from the CFG — all four facts in
`area` promote to nothing, so the emitted native code has exactly the
checks a hand-written compiler would have: none.

The corpus then witnesses every fate the architecture claims:

- **`unions_basic` / `unions_three`** — direct `if`/`else` narrowing,
  a three-variant else-if chain, and a `!==` guard: every fact
  promotes (else-arms prove by mask *subtraction*, the chain by two of
  them).
- **`alias_demote`** — tsc narrows through an aliased discriminant
  (`const k = s.kind`), which a tag-test dominance pass honestly
  cannot see. The fact demotes to a runtime check and the output still
  matches node byte for byte. Trust-but-verify is not "reject what you
  can't prove" — it is "keep the check."
- **The tampered artifact** — flip a demoted fact's claimed variant
  where both variants share a field shape, re-seal the content id: it
  compiles cleanly, and the demoted check catches the lie at runtime
  with blame naming the claimed kind and the cast site
  (`cast to 'b' at false_fact.ts:7:…`). A false fact at a *provable*
  site never even reaches runtime — the pass refuses to promote a
  claim its dominating test contradicts, and the check traps.

## What the stage forced

Beyond the dialect itself: partially-narrowed unions (the `else` of a
three-variant chain has type `Square | Tri`, which loses the alias
symbol) taught the producer to recognize its objects by discriminant
rather than by name; and the trailing `if`/`else` whose arms both
return exposed a latent TS-0 emitter bug — the join block was born
predecessor-less and its dead ops broke LLVM translation. The join is
now lazy, and a fully-returning `if` terminates the statement stream
the same way `return` does.

TS-1 runs on all eight runners and the full AOT grid — the same four
`.ts` files execute as interpreter reference, two JIT strategies, node
oracle, and cross-compiled natives on x86-64, aarch64, riscv64, wasm32
and the s390x big-endian canary.
