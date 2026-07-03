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
Not started. TS-0 gated on M9 (loanword freeze).
