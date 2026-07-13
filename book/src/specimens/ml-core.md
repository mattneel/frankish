# ml_core — the OCaml Slice

`ml_core` is the first specimen: a MinCaml-shaped core ML, held against the
`ocaml` toplevel as oracle (pinned at 4.14.1 in `versions.env`). It was
sequenced first (D-009) to retire *abstraction risk* — the question of
whether the kernel dialects could carry a statically-typed functional
language at all — and it forced the founding dialects `frk_adt` and
`frk_closure` into existence, plus the type-inference and match-compilation
machinery.

## The subset

The manifest freezes a small but real ML: `unit`, `bool`, `int`, functions
with currying, `let` (including `let rec`), mutual recursion, `if`,
arithmetic and comparison, sum and product types, and nested pattern
`match` with exhaustiveness checking. Floats are **fenced** out of v0.1 by
the admission rule (D-038) — they carry no idiom the kernel lacked at M5,
and entered later through TS-0 where `number = f64` is specimen-faithful.
An 18-case corpus lives under `goldens/ml_core/`.

A representative case — currying and closure capture in two lines:

```ocaml
let add x y = x + y
let main () = let add40 = add 40 in add40 2
```

`ocaml` runs this file (with `print_int (main ())` appended); frankish
compiles the identical bytes through the kernel dialects; the interpreter,
both JIT strategies, and OCaml must agree on `42`.

## Hindley–Milner over `ena`

The frontend (`crates/frk-front/src/{lex,ast,infer,emit}.rs`) is
scaffolding-grade by design (D-019): a hand-written lexer and parser, then
Hindley–Milner inference with let-polymorphism built on the `ena`
union-find crate. One subtlety is recorded in the code's history:
`ena`'s unification table stores `Option<Ty>` per variable, so unifying two
variables (or a variable against a value) must go through the *fallible*
`unify_var_var` / `unify_var_value` path rather than a plain `union` — a
plain union loses the occurs-check discipline. Generalization at `let`
boundaries produces the polymorphism; instantiation copies the scheme per
use site.

## Match compilation, and the promotion that defines the method

ml_core's `match` is where the forcing loop first closed visibly. Pattern
matching compiles to **Maranget decision trees** (D-025): a
matrix-to-tree algorithm that produces a dispatch tree with no redundant
tests, over a pattern language of variants, products, and integer
literals. Occurrence typing walks the scrutinee's `!frk_adt` type to know
what each sub-position holds; the tree's interior nodes become
`frk_adt.tag_of` + `cf.switch` (or `arith.select` + `cf.switch` for a
two-variant boolean encoding); its leaves recompute occurrence values and
branch to the arm. A `Fail` node reaching emission is a *caller bug* — the
frontend must have rejected the inexhaustive match first, with a witness —
and the emitter errors loudly if one survives.

Critically, this compiler was born *inside* the ml frontend and then
**promoted** at M6 into `crates/frk-dialects/src/dtree_emit.rs`, where it
is frontend-agnostic: the kernel types carry everything, so the only thing
a frontend supplies is arm-body emission. Any match-bearing frontend now
reaches it. The module's own doc marks the event — "the extraction loop
working as designed: the specimen built it, the promotion pass moved it
down to where every match-bearing frontend can reach it." M6 also re-based
ml_core *thin* — zero private ops remaining — and confirmed D-009's
ordering was right (D-040).

## The 62-bit corpus rule

The `ocaml` oracle exposes one genuine representation difference: OCaml's
native `int` is 63-bit (one tag bit), while frankish's `int` is a full
`i64`. Rather than model OCaml's boxing, the manifest makes it a *corpus
rule*: ml_core golden programs keep their integer values within 62 bits, so
the two representations are observably identical. This is the differential
law's standard move — where a real difference exists, close it in the
corpus by construction rather than pretend it away with a tolerance. The
`OcamlOracle` in the harness records the rule in a comment at the point it
matters.

## What it left behind

ml_core is v0.1, SHIPPED. It stands as the proof that the kernel's
static-functional lane is real, and — more importantly for the project's
thesis — as the origin of two reusable assets every later specimen
inherits: the `frk_adt`/`frk_closure` dialects, and the promoted
decision-tree compiler. The [dialect chapters](../dialects/adt.md) describe
what it forced; this chapter is the language that did the forcing.
