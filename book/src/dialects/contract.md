# frk_contract — Trust but Verify

`frk_contract` is the checked-cast dialect (SPEC §4.6), forced into
existence by TS-1 (D-072) as the gradual-typing boundary D-015 promised:
casts are contract ops **with blame**. Its v0 surface is one op:

```
frk_contract.narrow(%s) {variant = 1, blame = "cast to 'square' at case.ts:7:15"}
    : (!frk_adt.sum<[[f64],[f64]]>) -> !frk_adt.sum<[[f64],[f64]]>
```

`narrow` asserts that a sum value's tag is `variant`. It is
**identity-on-success** — the result type equals the operand type,
because narrowing is a *fact about* a value, not a representation
change. The claimed variant rides as the attribute that downstream
`frk_adt.extract` sites rely on. On refutation it traps
deterministically, and the trap message carries the blame string —
frontends build it from the artifact line table, so a failed cast names
its source site.

## Where the facts come from

The op exists to carry **imported flow facts**. The TS-1 producer asks
tsc's checker for its control-flow narrowing at every use site and
exports each fact as a `narrow` node in the loanword artifact. That
fact is *untrusted input* — the artifact may be stale, hand-written, or
hostile. The architecture is trust-but-verify:

1. The frontend emits every imported fact as a real `narrow` op.
2. The **interpreter executes every check.** Reference semantics is
   maximal checking; there is no promotion on the interp path.
3. Native paths run the **promotion pass** at `lower_kernel` entry: a
   forward must-dataflow over `cf` edges that re-derives the facts from
   scratch and deletes every narrow it can prove. What it cannot prove
   **demotes** to a runtime check — `frk_rt_contract_check`, a
   straight-line abort-on-mismatch in both runtime twins, blame bytes
   in a module global.

Because only native paths promote, the differential law does the
auditing: a wrong promotion elides a check the interpreter still
performs, and the divergence surfaces on the next run of the matrix.

## The promotion dataflow

State is a **possible-tag bitmask per sum root** (roots resolve through
narrow results to the underlying SSA value — sums are pure values, so a
fact about a value never invalidates; the transfer function has no kill
set). A `cf.cond_br` whose condition is `arith.cmpi eq/ne` of
`frk_adt.tag_of(root)` against a constant constrains its edges: the
eq-true edge intersects the tested tag, the eq-false edge subtracts it,
`ne` mirrored. Block entry state is the union over predecessor
contributions, iterated to fixpoint. A narrow claiming variant `v`
promotes iff its block's entry mask is a subset of `{v}`.

Subtraction is what makes the pass match tsc in practice: the `else` of
a two-variant test proves the other variant, and an else-if chain over
three variants proves the final arm by two subtractions. The TS-1
corpus witnesses both, plus the honest failure: tsc narrows through an
*aliased* discriminant (`const k = s.kind; if (k === "circle") …`), and
the dominance pass — which only speaks tag tests — cannot. That fact
demotes, the check runs, and the output still matches node byte for
byte. A tampered artifact claiming the *wrong* variant at such a site
compiles cleanly and is caught at runtime with blame naming the kind
and the cast site.

## Verifier surface

K1 semantic verification enforces what IRDL cannot: the identity typing
rule, the variant index in range, the blame attribute present. K2 is
the always-checking evaluator. K3 is the pair of fates — a proven
narrow lowers to *nothing*; a demoted one to an extract of the tag word
plus one runtime call. The op's twelve dialect verifiers pin each fate
against a hand-written CFG: eq-true domination, else-implication,
three-way chains, joins that widen, loop bodies that keep facts, chained
narrows collapsing to their root, and claims that contradict their
dominating test surviving to trap.
