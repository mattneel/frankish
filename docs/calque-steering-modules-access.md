# Steering amendment: Modules, companion functions, and collection access

## 1. Objective

Calque needs coherent module APIs and ordinary collection syntax. Traits are not a prerequisite for either feature.

This amendment extends the active goal and earlier amendments. The examples below define proposed source contracts, not assertions about current compiler support.

### Immediate procedure

- Read the current constitution, state, decisions, language specification, and active amendments.
- Inspect the actual source and ownership implementation before changes.
- Record this amendment without replacing historical goal documents or their integrity pins.
- Preserve frozen bootstrap snapshots and their hashes.
- Add an explicit bootstrap transition when the canonical compiler adopts new syntax.
- Preserve unrelated changes and existing specimen semantics.
- Deliver indexed access first, then migrate one coherent module API.
- Do not block either slice on a general trait solver.

## 2. Modules organize code and optionally own a nominal value type

A module is a compile-time namespace. It groups declarations and documentation. It does not create a runtime object, singleton, process, or global mutable state.

A module can declare one unnamed primary struct. That struct takes the module's qualified name. `Self` refers to that nominal type within its associated declarations.

### Proposed surface

```calque
pub module Compiler.Emitter {
    @moduledoc "Emits verified compiler artifacts."

    alias Compiler.Block;

    pub struct {
        blocks: [Block] = [%Block{}],
        current_block: i64 = 0,
        next_value: i64 = 0,
    }

    pub fn current(self: Self) Block {
        self.blocks[self.current_block]
    }

    pub fn replace_current(mut self: Self, block: Block) {
        self.blocks[self.current_block] = block;
    }

    pub fn open_block(mut self: Self, label: Str) {
        self.blocks = [..self.blocks, %Block{label: label}];
        self.current_block = self.blocks.len() - 1;
    }
}

alias Compiler.Emitter;

var emitter = %Emitter{};
Emitter.open_block(emitter, "entry");
const current = emitter.current();
emitter.replace_current(%Block{label: "loop"});
```

The index type follows the language's admitted indexing contract. The example retains `i64` to avoid an unrelated numeric-type migration.

### Module requirements

- Support qualified modules, lexical aliases, and explicit selective imports.
- Make declarations private unless `pub` exposes them.
- Keep named helper types inside their actual owner module.
- Permit modules without a primary struct.
- Permit named auxiliary structs and unions without another public module for every type.
- Bind the unnamed primary struct to the module name without creating a second nominal type.
- Preserve `%ModuleName{...}` construction, updates, defaults, and patterns.
- Reject `Self` as a value type when no associated nominal type exists.
- Keep ordinary `%Type{...}` forms valid for existing standalone types.
- Do not require a separate `impl` block for a type's own ordinary functions.
- Keep module aliases distinct from nominal type identity.
- Diagnose import collisions instead of resolving them by declaration order.
- Define module-to-file resolution against the existing package graph.
- Prefer one principal module per file without making physical file paths public type identities.
- Preserve stable symbol origins for documentation, diagnostics, and editor navigation.
- Do not add executable module-load hooks, unrestricted `use` injection, or mandatory macro machinery.

## 3. Receiver calls are a spelling for ordinary functions

`emitter.replace_current(block)` resolves to the associated module function with `emitter` as its first argument. It does not introduce virtual dispatch.

### Call requirements

- Keep `Emitter.replace_current(emitter, block)` available as the explicit qualified call.
- Resolve receiver calls through the statically known type and its owning module.
- Give built-in collection types documented associated functions through the same lookup contract.
- Do not search every imported function for a convenient first-argument match.
- Preserve argument evaluation order and exactly-once receiver evaluation.
- Honor the actual first parameter's ownership, effect, and mutation contract.
- Require a writable place for a receiver that corresponds to a `mut` parameter.
- Reject incompatible aliases through the existing scoped exclusive-access rules.
- Do not silently copy a receiver to satisfy a mutating call.
- Do not change a pure function into a mutating method because of call spelling.
- Preserve explicit receiver parameters in declarations.
- Do not introduce implicit `this`, inheritance, auto-dereference chains, or automatic runtime dictionaries.
- Report a callable-field/member collision explicitly.
- Provide `(value.field)(args)` for an explicit callable-field invocation.
- Provide `Module.function(value, args)` to disambiguate an associated function.
- Verify qualified calls, receiver calls, and admitted pipeline calls against the same resolved symbol.
- Show that symbol and its mutation contract in hover and signature help.

## 4. Indexing is required source syntax

The canonical vector read is `items[index]`. The canonical indexed update is `items[index] = value` through a writable root.

### Surface requirements

```calque
const selected = items[index];
const section = items[start..end];
const count = items.len();
const optional = items.at(index);

var changed = items;
changed[index] = replacement;
changed[index] += 1;
```

- Implement checked `items[index]` reads with original source diagnostics.
- Implement indexed updates through mutable bindings and admitted `mut` parameters.
- Implement nested places such as `emitter.blocks[index].label`.
- Preserve half-open slice syntax under the collection's documented slice contract.
- Define negative indices and out-of-range indices without implicit wrapping or clamping.
- Provide `at(index)` as an explicitly optional access operation.
- Make optional access return `None` for an out-of-range admitted integer index.
- Do not silently change the historical `vector.get` function into optional access.
- Keep `len()` discoverable as a real associated function, not a magic field.
- Preserve fixed-array, managed-collection, slice, and SIMD-lane distinctions in type analysis.
- Do not extend indexing to text code points or map insertion through undocumented guessing.
- Do not introduce arbitrary user-defined index overloads as a prerequisite for built-in collection access.
- Permit low-level intrinsic names inside the runtime and lowering implementation.
- Remove routine `vector.get` and `vector.set` calls from migrated canonical source.

### Evaluation and failure contract

- Evaluate the root and selector expressions once, in source order.
- Validate the selected path before the right-hand side under the specified assignment contract.
- Evaluate the right-hand side once.
- Evaluate compound assignment's old value once.
- Preserve checked arithmetic and error behavior for compound assignments.
- Reject conflicting mutation of the selected root during an overlapping update.
- Permit safe reads of the root according to the existing exclusive-access contract.
- Commit a nested update only after its required checks succeed.
- Do not claim rollback of unrelated effects that the right-hand side already performed.
- Do not re-evaluate an index while reconstructing updated parent values.
- Test side-effectful selectors and failing right-hand sides explicitly.

## 5. Preserve value semantics, not merely prettier shared mutation

A `var` permits changes to a binding or an exclusive state parameter. It does not make every alias observe those changes.

### Required acceptance example

```calque
const original: [i64] = [10, 20, 30];
var changed = original;

changed[1] = 99;

assert original == [10, 20, 30];
assert changed == [10, 99, 30];
```

### Ownership requirements

- Preserve prior values after an indexed update through another binding.
- Support nested persistent updates along the complete place path.
- Distinguish element value retrieval from a mutable borrowed reference.
- Require an explicit scoped contract for any exposed mutable element view.
- Reuse storage only when actual ownership analysis establishes safety.
- Keep builders and foreign mutable buffers distinct from persistent collection values.
- Do not lower every bracket assignment directly to the old shared-vector mutation primitive.
- Preserve historical mutable bootstrap behavior only within its recorded snapshot or versioned compatibility contract.
- Migrate canonical collection behavior through explicit decisions and alias tests.
- Explain the logical update independently from its physical copy or reuse strategy.
- Measure hot paths before introducing another mandatory builder abstraction.

## 6. Migrate the emitter's actual domain model

A namespace fixes ownership of names. It does not justify opaque abbreviations, unnecessary mutation, or singleton collections that represent scalar state.

### Migration procedure

- Move the active emitter API into its owning module.
- Replace prefix namespaces such as `e_init` with qualified public names or direct struct construction.
- Remove constructors that do nothing beyond `%Emitter{}` unless they establish an intentional API boundary.
- Make read-only operations accept ordinary value parameters rather than unnecessary `mut` parameters.
- Keep a getter only when it protects an invariant or defines a useful operation.
- Replace mechanical field getters and setters with direct access where visibility permits it.
- Inspect every field's actual cardinality before changing its representation.
- If an AST field stores one current AST, represent that fact directly.
- If a field is genuinely a stack or history, preserve its collection semantics and name that role.
- Do not blindly flatten every collection whose current fixture contains one element.
- Replace `TV`, `InstE`, and similar opaque names with names based on their actual invariants.
- Keep helper types private unless another module needs their public contract.
- Preserve useful high-level operations such as `open_block` when they maintain multiple related invariants.
- Do not replace one emitter module with a class hierarchy or dozens of one-field wrapper modules.
- Do not manufacture protocols merely to move concrete functions out of a flat namespace.

## 7. Protocols address shared behavior, not ordinary member access

Protocols define operations across unrelated value types. Module callback contracts describe replaceable implementations. Neither mechanism is required for indexing or a concrete emitter API.

### Follow-on contract

- Reuse the existing dictionary-passing boundary for genuine generic protocol calls.
- Resolve concrete protocol implementations statically where the type is known.
- Keep runtime-erased dispatch explicit in the source or API contract.
- Define unique implementation selection and diagnose overlap without import-order dependence.
- Preserve module ownership and package coherence in implementation lookup.
- Use protocols for real common behavior such as inspection or codec participation.
- Use typed module callback contracts for replaceable passes or service providers when a consumer requires them.
- Avoid parallel trait and protocol solvers with duplicated semantics.
- Do not require protocol conformance to access built-in collections.
- Do not expand this immediate milestone into unrestricted specialization, inheritance, or metaprogramming.

## 8. Tools, identity, and compatibility

### Integration requirements

- Extend the canonical lexer, parser, resolver, type checker, and emitter.
- Preserve source nodes for brackets, places, receiver calls, aliases, and module declarations.
- Reuse these nodes in diagnostics, formatters, documentation, lints, and LSP queries.
- Provide module and member completion against incomplete source.
- Show indexed bounds failures at the bracket expression rather than a runtime helper.
- Show write-through-immutable-root failures at the root and its declaration.
- Preserve references and rename targets across aliases and receiver-call spelling.
- Derive module documentation and typed struct documentation from the same semantic identities.
- Keep CLI Markdown help available for all new syntax and rules.
- Do not rename serialized nominal identities merely because files or source modules move.
- Record any deliberate CON, CTF, JSON, ABI, or artifact-schema migration explicitly.
- Preserve existing wire identities through the supported metadata mechanism when required.
- Do not require identical old and new artifact bytes after intentional symbol-identity changes.
- Preserve behavior and document legitimate golden changes.
- Reestablish exact compiler-stage parity within the new recorded target and artifact contract.

## 9. Verification and completion

### Required tests

- Test direct, nested, and generic collection access.
- Test empty, negative, boundary, and out-of-range access.
- Test optional access and slice endpoints.
- Test exactly-once selectors, right-hand sides, and compound updates.
- Test alias preservation and rejection of immutable-root writes.
- Test independent emitter instances and retained prior state.
- Test same-spelled private functions in different modules.
- Test alias collisions, private access, and callable-field ambiguity.
- Compare qualified, receiver, and pipeline calls for equivalent evaluation and effects.
- Verify concrete receiver calls in generated IR without mandatory runtime lookup.
- Test invalid `mut` receiver use and mismatched argument effects.
- Test parser recovery and source mapping for incomplete brackets and module declarations.
- Run the admitted interpreter, native, bootstrap, and self-host gates.
- Preserve every existing specimen's admitted behavior.

### Delivery order

1. Land checked reads and indexed writes with ownership and alias tests.
2. Migrate several real emitter accesses through the new syntax.
3. Land the module and companion-call surface with resolver tests.
4. Migrate one complete emitter API and its callers.
5. Update tools, documentation, and bootstrap transitions.
6. Add protocols only when the next actual polymorphic consumer requires them.

### Completion evidence

- Show the new emitter declaration, initialization, read operation, and stateful update.
- Show at least one migrated caller through qualified and receiver syntax.
- Show the alias-preservation test and current compiler-stage results.
- Record exact commands and unavailable checks honestly.
- Update the acceptance matrix and `STATE.md` before handoff.
- Do not mark syntax documentation as implementation evidence.
- Do not substitute a roadmap for the first indexed-access slice.
- Keep the complete original goal active while unrelated mandatory work remains.

**Modules own names. Values own data. Brackets express access. Protocols express genuinely shared behavior.**
