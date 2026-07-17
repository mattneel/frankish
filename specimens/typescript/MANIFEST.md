# specimen: typescript — staged TS-0..TS-4 (each stage frozen separately)

## Identity & pin
TypeScript, checker-as-oracle architecture: we never reimplement the type
checker. tools/loanword-ts drives the stable TS 6 compiler API (Corsa API
migration at TS 7.1 — LANDSCAPE watch item), queries types and control-flow
narrowing facts per node, emits **loanword**. frankish consumes loanword;
narrowing facts are imported as cast annotations and re-verified by our own
dominance/dataflow pass; unverifiable casts demote to frk.contract runtime
checks (trust-but-verify).

## Rulings inherited
D-013 number = f64 specimen-faithful (i32/i64 annotations = profile
extension); D-014 sealed-world profile switch; D-015 gradual boundary =
contract ops; strings are UTF-16 code units (JS semantics) — rt decision.

## Stages
- TS-0 (M9): monomorphic functions, number/boolean/string, arrays, control
  flow. Needs arith/scf + closure-lite. Demo golden: fib.ts → native, node
  as oracle, startup number recorded (not gated).
- TS-1: discriminated unions + narrowing → frk.adt + imported-flow-facts
  verifier. The research slice.
- TS-2: classes, structural interfaces (itabs, D-026), object closures —
  GC goes live (Tier 2).
- TS-3: async/await via the ported tsc downlevel state-machine transform
  (MIT; reference transform + baselines exist upstream), exceptions.
- TS-4: generics (monomorphized), sealed-world switch, `any`/gradual
  boundary as contract ops.

## Fences (all stages)
No eval/with/proxies/prototype mutation; no `any` in codegen paths (TS-4
admits it only at contract boundaries); no decorators; module system minimal
(single-file → ES-module subset later); no Node stdlib — console + math
only until a stage says otherwise.

## Conformance
Curated test262 slice per stage (license: BSD) + tsc baseline-derived cases
+ hand corpus per idiom. node/V8 is ground truth through canon filter.

## Status
TS-2 SHIPPED, STAGE FROZEN (2026-07-17, m29-done; D-073/D-074/
D-075, over two milestones):
- Classes core (m28-done): monomorphic classes as MANAGED BOXES OF
  PRODUCTS (frk_mem.field_get/field_set), all-assigning
  constructors with the `this.next = this` knot, methods as
  `this`-first plain functions, recursive class types via D-074's
  type-erased recref, GC LIVE (record layouts trace; both twins
  collect the record ring; a live object cycle runs under rc on
  all five architectures).
- Interfaces + object closures (m29-done, D-075): STRUCTURAL
  interfaces on D-026's itabs — no `implements` anywhere, shape is
  the contract; iface value = {obj, itab} pair; interp runs the
  dictionary representation, native a real itab with indirect
  calls, the matrix arbitrates. Arrows lambda-lift onto
  frk_closure verbatim; captures by binding (params by value,
  lets by box) give JS mutation visibility.
Stage fences (carried forward, NOT TS-2 debt): inheritance/extends,
static members, getters/setters, optional + union-typed fields,
field initializers, method VALUES (unbound `this` — refused;
arrows are the spelling), iface stores (borrows-only v0),
interface properties/extends, block-bodied arrows, this-in-arrow.

TS-1 SHIPPED (2026-07-17, m27-done, D-072): discriminated unions +
the imported-flow-facts verifier — THE RESEARCH SLICE, exactly as
the identity paragraph above promised. Unions of `kind`-discriminated
object aliases ride frk_adt sums (kind not stored: tests are tag
compares, reads are tag-selected literals); tsc's control-flow
narrowing exports as narrow cast annotations (loanword additive
within v1); frk_contract is BORN (narrow op, blame from the span
table); the native promotion pass re-derives every provable fact
and DELETES its check, unprovable facts demote to runtime checks.
Corpus: 4 cases node-diffed on all runners + the full grid; the
demotion witness (aliased discriminant), the promotion counts, and
the tampered-fact blame trap are standing tests. Slice fences
(D-072): switch narrowing, union-typed LOCALS (box reads have no
SSA identity — facts would silently demote; admit with the demotion
named), optional props, nested payloads, >64-variant unions.

TS-0 SHIPPED (2026-07-03): the full stage scope — monomorphic
functions, number/boolean/string, arrays, control flow — compiles
through loanword v1 (frozen, D-046) into the kernel dialects and runs
byte-identical to node across interp, jit×{arena,rc}, and the five-
architecture AOT grid. Conventions and fences: D-047 (entry protocol,
JS semantic mappings, canon §6 print fence), D-049 (arrays in
frk.mem, OOB traps stricter-than-JS with source locations, strings
as rt-owned UTF-16 — the code-unit ruling fired at .length), D-050
(noImplicitReturns as checker-as-oracle corollary; tamper-refusal
and §6.5 witnesses). TS-2 (classes, structural interfaces, object
closures — the GC goes live for TS) is the next stage, unscheduled.
