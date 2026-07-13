# Landscape — Pinned Facts

`docs/LANDSCAPE.md` is the project's memory of the world outside the
repository: verified prior art, pinned facts about the substrate,
time-sensitive watch items, and the hard-won pitfalls that must not be
rediscovered. Where the [ledger](../method/ledger.md) records *decisions*,
LANDSCAPE records *facts* — and the discipline is that a fact enters only
once verified, with the date and the evidence.

## The substrate, pinned

frankish rides **melior / mlir-sys / tblgen-rs** (the mlir-rs org's Rust
MLIR bindings) against LLVM/MLIR 22. LANDSCAPE names the load-bearing peer:
**cairo-native** (LambdaClass), a production language on melior, whose
existence is evidence the bindings are viable at scale. Every version is a
single pin point in `versions.env`; LANDSCAPE's standing rule is that
MLIR/LLVM major bumps are taken *deliberately, never implicitly*, because
melior tracks them with lag.

## The pitfalls (so they cost once)

The most valuable thing in LANDSCAPE is the list of bindings defects and
MLIR-22 surprises that each cost real debugging time, pinned so they cost
it once:

- **`StringAttribute::value()` is UB on the empty string.** In melior
  0.27.2 the raw `StringRef` is null for `""`, and `value()` calls
  `slice::from_raw_parts` on it — which the Rust runtime's UB check
  aborted the first time a Lua `#""` golden ran. Every frankish
  text-attribute read now goes through `attr_util::string_attr_bytes`,
  which unescapes the printed form instead. This is a real upstream bug
  worth patching; until then the dodge is load-bearing.
- **`binary_operands` insists on integer widths**, so f64 operations must
  be built through the float-operand path.
- **The `cf` dialect helpers use pre-MLIR-22 attribute names**, so
  `cf.switch` / `cf.cond_br` are built with explicit `operandSegmentSizes`
  (and `case_operand_segments`) rather than the helper defaults.

Every entry carries the date it was verified and the golden or milestone
that surfaced it. Together they are the reason a new contributor does not
re-lose a day to the empty-string abort.

## Watch items

LANDSCAPE tracks a few time-sensitive externalities with scheduled
reactions rather than vague intentions:

- **Mojo open-sourcing (committed, fall 2026).** When KGEN lands it becomes
  the largest readable corpus of *exactly* frankish's kernel-dialect layer;
  a study milestone is scheduled for when it drops.
- **TypeScript 7.0 (Corsa, Go-native).** No stable programmatic API until
  7.1, so `tools/loanword-ts` builds on the TS 6 API (pinned 6.0.3) until
  then; migration is a planned follow-up.
- **Upstream IRDL trait support.** If IRDL learns to declare traits
  (terminators and the like), region-bearing op designs become expressible
  in pure IRDL again — at which point [D-031's](../method/ledger.md)
  de-regioning could be reopened, but *only* with a dialect demonstrably
  suffering under it. Checked at every LLVM major bump.

A **paper crib list** pins the sources behind the designs: Maranget for
decision trees, Xie & Leijen (Koka) for evidence-passing effect handlers,
Siek & Taha for gradual typing and blame, Go's itab dispatch for dynamic
interfaces, Tiger Style for the contract dialect's sensibility.

## In-house prior art

The final section names the three languages frankish exists to stop
rewriting — **atli**, **inscription**, **flexlang** — as a design quarry,
with each one's *unmined* veins listed explicitly: atli's β-certified frame
sizing and the ε×β handler fixpoint; inscription's diagnostic-catalog and
deterministic-release discipline; flexlang's hygienic-macro expander and
comptime evaluation. They are checkouts under `~/src/`, deliberately *not*
vendored (L6 forbids machine-path dependence in build or test) — design
sources, not dependencies. [The toylang quarry](toylangs.md) tells that
story in full; LANDSCAPE is where the extraction conditions are kept
current.
