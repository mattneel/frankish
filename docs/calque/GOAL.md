# Goal: Deliver Calque as a self-hosted Frankish specimen

## 1. Objective

Calque is a Zig-like, strict, functional language on Frankish and MLIR. Calque is a specimen, not a separate compiler platform. Its canonical compiler is written in Calque from the first substantial implementation. Frankish interprets that compiler before Frankish compiles it to native code. The same Frankish interpreter supplies Calque's `comptime` execution.

The priority order is:

```text
developer experience > runtime performance > compile/iteration speed > everything else
```

Correctness and safety are requirements, not negotiable positions in that order. Fast feedback is part of developer experience. Compiler simplicity does not justify a worse language or opaque diagnostics.

This goal covers a complete, bounded first release. A design document, prototype parser, bootstrap seed, or partial feature demonstration does not complete it. The release includes the language, self-hosted compiler, shared Frankish improvements, developer tools, verification, documentation, and reproducible demonstrations.

### Execution requirements

- Deliver the implementation, not only a proposal or roadmap.
- Keep Calque inside Frankish's specimen architecture.
- Make the compiler the first substantial Calque application.
- Reuse Frankish's interpreter for runtime interpretation and compile-time execution.
- Promote reusable improvements into Frankish as Calque forces them.
- Preserve the behavior of every existing specimen within its admitted subset.
- Continue through the complete acceptance contract in this document.
- Preserve unfinished requirements across session boundaries.
- Do not invent a historical claim that Calque is the first self-hosted language.

## 2. Repository authority and authorized scope

This task authorizes the scope additions necessary for Calque. These additions do not remove Frankish's verification laws or existing specimen contracts.

### Initial procedure

1. Read `AGENTS.md` completely.
2. Read `STATE.md` completely.
3. Read `docs/DECISIONS.md` completely.
4. Read the relevant sections of `docs/SPEC.md`.
5. Read `specimens/README.md`.
6. Read each specimen manifest that the implementation will affect.
7. Inspect the actual source, toolchain pins, build targets, and CI definitions.
8. Run `git status --short`.
9. Run `git log --oneline -15`.
10. Run the required baseline tests before source changes.
11. Record the actual starting commit and baseline results.

### Scope procedure

- Reconcile stale status text against source, commits, and executable evidence.
- Do not assume that a documented capability exists because its name appears in the specification.
- Record Calque as the user-selected next workstream.
- Preserve unrelated queued work in `STATE.md`.
- Allocate milestone numbers from the current ledger rather than historical numbers in this prompt.
- Append explicit decisions for the authorized changes below.
- Update the applicable specification and constitution text through the repository's amendment process.
- Preserve historical decisions.
- Record superseding decisions explicitly.

### Authorized changes

- Admit Calque as an original specimen without a preexisting upstream implementation.
- Even when existing kernel operations express a feature, permit its admission if the Calque language contract requires it.
- Retain the stronger promotion requirement for genuinely new reusable semantics.
- Permit the self-hosted Calque frontend and its reproducible bootstrap.
- Permit editor tools, incremental frontend work, and measured performance work required by this goal.
- Permit compile-time values, type construction, reflection, and generic specialization required by this goal.
- Permit local modules, local packages, and a reproducible dependency manifest.
- Keep an online package registry outside this goal.

### Preserved constraints

- Preserve verifier-first development and the differential law.
- Preserve the K1–K7 contract for kernel dialects.
- Preserve the registered runtime ABI and applicable runtime-twin parity.
- Unless evidence requires a separately recorded change, preserve the existing Rust, melior, MLIR, and LLVM foundation.
- Unless a demonstrated blocker requires an explicit amendment, preserve the IRDL-only policy.
- Do not silently add C++ shims or unregistered kernel operations.
- Keep build, test, and release workflows independent of Oh-My-Pi.
- Preserve unrelated user changes and repository history.
- Do not force-push or publish anything to social platforms.

## 3. Product identity and implementation defaults

The name is **Calque**. The default extension is `.calq`. The default user command is `calque`. Frankish remains the platform and `frnksh` remains its driver.

These names define the requested product. Other syntax details below are implementation defaults, not excuses for repeated redesign.

### Decision procedure

- Unless an actual repository conflict requires another extension, use `.calq`.
- Implement `calque` as a thin product entry point over shared Frankish services.
- Register Calque with `frnksh` through the existing profile mechanism or its generalized replacement.
- Keep one compiler pipeline behind both entry points.
- Choose unresolved syntax details through small executable examples and ledger entries.
- If an example's exact spelling changes, preserve the requested capabilities.
- Freeze each syntax decision before its conformance corpus expands.
- Keep the standard library, examples, formatter, and documentation consistent with the final syntax.

## 4. What belongs in Calque and what belongs in Frankish

Calque owns its language semantics and canonical frontend. Frankish owns the reusable execution and compiler substrate. Self-hosting refers to Calque's compiler, not a rewrite of LLVM, MLIR, or Frankish in Calque.

### Calque implementation requirements

- Write the canonical lexer and parser in Calque.
- Write Calque's name resolution and type-system rules in Calque.
- Write generic elaboration and specialization policy in Calque.
- Write Calque-specific transformations and source-to-kernel emission in Calque.
- Write the standard library and compiler's ordinary data structures in Calque.
- Keep source-level diagnostics under the same canonical frontend.
- Share that frontend between batch compilation, the REPL, and editor tools.

### Frankish implementation requirements

- Expose reusable module construction, verification, interpretation, and compilation services.
- Reuse existing dialects, pass infrastructure, runtime services, and harnesses.
- Add generic host services rather than Calque-specific semantic shortcuts.
- Keep handles typed, scoped, and versioned across host boundaries.
- Provide deterministic failure behavior for invalid handles and malformed requests.
- Route new runtime functions through the ABI registry.
- Separate host-only compiler services from the portable application runtime.
- Keep Calque programs free from an implicit dependency on the entire compiler toolchain.

### Prohibitions

- Do not implement the production Calque frontend in Rust and schedule a future rewrite.
- Do not hide Calque parsing or type-system rules inside a host `compile_calque` intrinsic.
- Do not implement a second production evaluator for Calque or `comptime`.
- Do not route native execution through an unrelated transpiler or VM.
- Do not turn Calque into a thin wrapper around an existing specimen's semantics.

## 5. Bootstrap and self-hosting contract

A source file cannot execute before some mechanism translates it. The bootstrap seed provides that first translation. The seed is an explicit trust root, not the canonical production compiler.

The required shape is:

```text
Auditable seed + bootstrap-compatible compiler source in Calque
    -> initial compiler kernel artifact
    -> Frankish interprets the compiler
    -> compiler translates Calque source, including compiler source
    -> Frankish compiles that artifact to native code
    -> native Calque compiler rebuilds the same compiler
```

### Seed requirements

- Choose the smallest auditable route to the initial compiler artifact.
- If its admitted subset supports the necessary work, prefer an existing specimen for the seed.
- If that route is materially simpler and explicitly justified, use a small host-language seed translator.
- Specify the bootstrap subset in a frozen manifest.
- Keep the canonical compiler source in Calque from the first substantial compiler milestone.
- Limit the seed to the bootstrap subset and required artifact emission.
- Freeze the seed after the initial self-host gate.
- Do not evolve the seed into a parallel production compiler.
- Keep every source and recipe necessary to reproduce the seed output.
- Do not use an opaque downloaded compiler binary as the only bootstrap root.

### Bootstrap evolution

- When the current compiler outgrows the seed subset, preserve a bootstrap-compatible compiler snapshot.
- Record each required source-to-source bootstrap hop.
- Distinguish frozen historical snapshots from the canonical current compiler.
- Reproduce the full bootstrap chain without network access after toolchain setup.
- Keep the chain short and document each trust boundary.
- Do not require the frozen seed to parse later language syntax.

### Fixed-point verification

1. Generate the initial compiler artifact from the seed and recorded source.
2. Run the compiler under Frankish's interpreter.
3. Compile the current canonical compiler source through the recorded bootstrap chain.
4. Build a native compiler from the resulting artifact.
5. Recompile the same current source with that native compiler.
6. Compare the canonical compiler artifacts for the same target and configuration.
7. Rebuild once more to establish a stable fixed point.
8. Run the conformance corpus through the interpreted and native compilers.
9. Compare diagnostics and emitted canonical artifacts.

### Evidence requirements

- Define the exact artifact and canonicalization rules used for fixed-point comparison.
- Preserve semantic attributes, target information, and source-origin information in that comparison.
- Exclude only documented incidental identifiers and representation noise.
- Verify the canonicalizer with intentional semantic mutations.
- Test a meaningful compiler-source edit that changes generated output.
- Test a new input program that no bootstrap fixture contains.
- Reject stale artifacts and hardcoded self-reproduction as substitutes for compilation.
- Report self-compilation as bootstrap evidence, not a proof of semantic correctness.
- Report the remaining trusted host components explicitly.

## 6. Compiler representation and reusable frontend services

The frontend needs source-oriented data before MLIR emission. The editor needs useful results even when a source file is incomplete. Typed kernel IR remains the execution boundary.

### Implementation requirements

- Implement a lossless syntax representation with comments and source spans.
- Support parser recovery and explicit error nodes.
- Assign stable identities to declarations and source entities.
- Preserve source origins through generated code and generic instantiations.
- Represent typed expressions, patterns, effects, and generic declarations before kernel emission.
- Retain collection operations long enough for sound language-level optimization.
- Keep frontend analysis independent from native code generation.
- Expose the same analysis queries to the CLI, REPL, and language server.
- Emit verified Frankish kernel IR through a versioned boundary.
- If its contract fits that boundary, reuse or extend `loanword`.
- When the interchange needs a new version, preserve existing producers.
- If direct kernel emission is the simpler bootstrap route, document that alternative.
- Avoid a new universal IR project that does not advance this goal.
- Avoid raw, untyped pointer exchange as the public frontend interface.

### Kernel promotion requirements

- Before you add a dialect operation, attempt composition from existing operations.
- Promote a missing semantic capability rather than a Calque syntax node.
- Give every new operation reference semantics, verification, native conversion, tests, and documentation.
- Add source-mapped failures for invalid operations and unsupported combinations.
- Add negative tests at the host-service and IR boundaries.
- Demonstrate a second consumer or a language-neutral test for each substantial shared abstraction.

## 7. Surface language and evaluation model

Calque uses familiar declarations and expression-oriented bodies. Evaluation is strict and left-to-right. Ordinary function calls use ordinary argument lists. The language does not require implicit currying or pervasive laziness.

The intended surface includes examples of this form:

```text
const User = struct {
    name: Str,
    score: i64,
    active: bool,
};

pub fn activeScore(users: Vector(User)) i64 {
    users
        |> seq.iter()
        |> seq.filter(fn (user) { user.active })
        |> seq.map(fn (user) { user.score })
        |> seq.fold(0, fn (total, score) { total + score })
}
```

### Required language behavior

- Implement `const`, `fn`, `pub`, records, tagged unions, and expression blocks.
- Implement explicit local mutation without implicit shared mutation.
- Implement first-class functions, closures, and higher-order generic functions.
- Implement lexical scope and predictable shadowing rules.
- Implement recursion and mutual recursion.
- Guarantee proper direct, mutual, and closure tail calls within the Calque call convention.
- Document the tail-call guarantee.
- Do not claim that arbitrary recursion uses constant stack space.
- Implement destructuring for records, tuples, and tagged values.
- Implement optional values and typed error results.
- Implement concise error propagation and explicit error recovery.
- Implement source-level tests and documentation examples that execute.
- Keep numeric conversions and external effects visible.
- Keep ordinary application code free from mandatory allocator and lifetime bookkeeping.

## 8. Pipelines and pattern matching

Pipelines are ordinary function composition syntax. Explicit sequence values provide pull-based traversal without whole-language laziness. Pattern matching defines both value selection and destructuring.

### Pipeline requirements

- Define `x |> f(a)` as a single evaluation of `x` followed by `f(x, a)`.
- Specify argument evaluation order, operator precedence, and associativity.
- Preserve that order through every transformation.
- Provide useful type errors at the failing pipeline stage.
- Distinguish eager collection operations from explicit sequence operations.
- Implement `map`, `filter`, `fold`, `flat_map`, `take`, `zip`, and `collect`.
- Provide explicit closures for partial application.
- Avoid a separate pipeline execution engine.
- Preserve failures and observable effects during fusion.
- Do not equate purity with freedom to reorder traps or divergence.

### Pattern requirements

- Implement literals, wildcards, bindings, nested patterns, and tagged variants.
- Implement tuple patterns, record patterns, or-patterns, and guards.
- Enforce consistent bound variables across or-pattern alternatives.
- Evaluate guards in source order after their structural patterns match.
- Do not count an arbitrary guarded arm as exhaustive coverage.
- Report missing cases with concrete witnesses.
- Report unreachable or redundant arms under a documented severity policy.
- Reuse and extend Frankish's decision-tree machinery where appropriate.
- Preserve source origins in generated branches and extraction operations.
- Test nested matches, shadowed bindings, guards, and branch-local captures.

## 9. Types, generics, reflection, and compile-time types

The type system favors local inference and explicit module boundaries. Generic code specializes to concrete representations. Compile-time type values support useful type construction without a separate macro language.

The intended form includes:

```text
pub fn identity(comptime T: type, value: T) T {
    value
}

pub fn Pair(comptime T: type) type {
    struct {
        first: T,
        second: T,
    }
}
```

### Required type capabilities

- Infer local values and closure parameters from context.
- Require explicit public function signatures.
- Support useful diagnostics for unresolved type holes.
- Implement booleans and fixed-width signed and unsigned integers through 64 bits.
- Implement target-dependent size types with explicit target semantics.
- Implement `f32` and `f64` under a documented floating-point contract.
- Separate text strings from byte buffers.
- Implement tuples, records, generic tagged unions, and recursive data types.
- Support multiple concrete instantiations of one generic definition.
- Support higher-order generic functions and generic closure captures.
- Define recursive layouts and indirection without infinite inline values.
- Detect infinite specialization and type-construction cycles.
- Implement compile-time type parameters, constant parameters, and conditional type construction.
- Implement reflection over fields, variants, and function signatures.
- Support a generic codec or equivalent library that uses reflection.
- Keep type identity deterministic across interpreter, compiler, cache, and serialization boundaries.
- Avoid unrestricted trait search or general dependent typing in this release.

### Phase implementation requirements

- Separate unresolved syntax, compile-time type values, and runtime values explicitly.
- Schedule type-dependent compile-time work through an explicit dependency graph.
- Execute ready typed fragments through Frankish's interpreter.
- Return structured type descriptors rather than host pointers as compile-time type values.
- Keep Calque-specific elaboration rules in Calque.
- Do not claim that a post-typecheck constant-fold pass implements compile-time type construction.

## 10. Values, memory, effects, and resources

Ordinary Calque values have immutable value semantics and automatic lifetimes. Local mutation is an implementation tool. External resources have explicit lifetimes independent of heap reclamation.

### Memory requirements

- Use the existing RC strategy as the initial managed application profile where its verified contract fits.
- Preserve compatibility with other supported strategies where their admission rules apply.
- Implement immutable vectors, maps, and sets with structural sharing.
- Provide explicit mutable builders for efficient bulk construction.
- Preserve existing aliases after an immutable collection update.
- Keep managed views alive with their backing storage.
- Define equality and hashing consistently for admitted value types.
- Specify deterministic collection iteration behavior where program output depends on it.
- Implement escape analysis and uniqueness-based reuse where evidence supports them.
- Avoid mandatory borrow annotations in ordinary Calque code.
- Preserve the runtime's retain, release, trace, and layout invariants.
- Test cyclic structures whenever the admitted representation permits them.
- Keep allocation failure behavior explicit and tested.

### Effect and resource requirements

- Unless their signatures or explicit capabilities permit external effects, make ordinary functions pure.
- If it cannot affect caller-visible state, permit private local mutation.
- Track effects sufficiently to enforce purity and compile-time restrictions.
- Propagate closure effects through higher-order calls and generic specialization.
- Represent file access, clocks, and randomness through explicit capabilities.
- Prevent implicit ambient authority in pure functions.
- Provide scoped resource cleanup and a documented `defer` mechanism.
- Execute required cleanup on ordinary returns and error propagation.
- Document behavior for traps, process termination, and unsupported control transfers.
- Do not promise deterministic file cleanup from garbage collection.
- Reject resource escape or report deterministic closed-resource errors under the chosen contract.
- Unless existing machinery makes them necessary, keep advanced user-defined effect handlers outside the critical path.

### Safety requirements

- Define integer overflow, division errors, shifts, bounds failures, and invalid conversions.
- Keep safe-source behavior consistent across interpreter, JIT, and AOT.
- Unless the compiler proves them unnecessary, preserve safety checks in ordinary release builds.
- Provide explicit wrapping operations rather than accidental LLVM undefined behavior.
- Do not enable unrestricted floating-point reassociation by default.

## 11. The interpreter is `comptime`

Calque's compile-time execution uses Frankish's existing evaluation machinery. It executes typed code with functions, recursion, closures, and managed values. It is not a second expression evaluator with a smaller hidden language.

### Shared service requirements

- Expose a reusable compile-time evaluation service in Frankish.
- Execute through the same per-operation semantics as ordinary interpretation.
- Add missing evaluator capabilities to Frankish rather than a Calque-only evaluator.
- Accept a verified module, entry point, arguments, target, capabilities, and resource limits.
- Return a structured value or a source-mapped failure.
- Preserve the caller's state across nested evaluation requests.
- Isolate interpreter frames, runtime state, allocations, and effect channels per request.
- Restore isolation after errors, cancellation, and budget exhaustion.
- If the service needs reusable staging semantics, register operations for those semantics.
- When `frk.stage` or an equivalent shared interface becomes necessary, preserve the kernel contract.

### Required compile-time behavior

- Implement explicit `comptime` expressions and blocks.
- Reject runtime dependencies inside required compile-time expressions.
- Do not silently replace required compile-time execution with runtime execution.
- Preserve untaken branches without premature evaluation of their failures.
- Execute ordinary pure library functions at compile time.
- Support aggregates, recursive values, strings, and compiler-relevant data structures.
- Support compile-time reflection and the type values required by Section 9.
- Execute nested compile-time calls with bounded resources.
- Provide compile-time assertions and custom source-level diagnostic messages.
- Preserve call traces and instantiation traces.
- Define target arithmetic independently from the build host's pointer size and endianness.
- Test target-sensitive expressions against corresponding target runtime results.

### Reification requirements

- Freeze interpreter values into deterministic compiler-owned representations.
- Materialize runtime constants through verified IR emission.
- Handle shared substructure and cycles under an explicit serialization policy.
- Reject raw pointers, live resources, and interpreter-owned handles at the boundary.
- If their code and captures have a complete supported representation, permit closure reification.
- Emit an actionable diagnostic for unsupported values.
- Do not retain pointers into a destroyed compile-time arena.
- When no runtime reference remains, verify that compile-time-only functions disappear from final application artifacts.

### Security and reproducibility requirements

- Deny network access, shell execution, ambient environment access, clocks, and randomness by default.
- Permit file reads only through explicit tracked build inputs.
- Enforce capabilities at the execution boundary, not only through frontend annotations.
- Even when another specimen admits them at runtime, reject unsupported foreign effects during compile-time execution.
- Bound evaluation steps, memory, recursion, and nested requests.
- Support deterministic cancellation and useful limit diagnostics.
- Keep default compile-time execution independent of JIT availability.
- Cache results by complete semantic inputs and configuration.
- Include compiler identity, target, dependencies, capabilities, and resource policy in cache validity.
- Invalidate results after transitive input changes.
- Keep failed or partial evaluations outside the result cache.

## 12. Compiler-as-library and cross-specimen compile-time execution

The compiler is a Calque library as well as a command. Frankish's shared interpreter also permits explicit compile-time use of other admitted specimen languages. This does not imply unrestricted cross-language runtime interoperability.

### Compiler library requirements

- Expose source-to-artifact compilation through a typed Calque library interface.
- Keep filesystem access in the caller's explicit capabilities.
- Permit compilation from in-memory source values.
- Produce diagnostics as structured data before rendering them.
- Support a Calque program that invokes the compiler library under Frankish's interpreter.
- Support a `comptime` program that compiles a small Calque program and evaluates the resulting artifact.
- Materialize that evaluation's result into the outer program.
- Enforce the same nested-request limits and target contract throughout this path.
- Do not special-case the demonstration's source or result.

### Cross-specimen requirements

- Expose an explicit profile-based compile-and-evaluate interface.
- Use the registered frontend and interpreter path for each admitted specimen.
- Define the portable value subset accepted across that interface.
- Support simple pure entry points from each existing registered specimen that admits such an entry point.
- Add a conformance case for every supported profile.
- Reject unsupported values and effects with located diagnostics.
- Track embedded source and imported modules as build dependencies.
- Keep optional external frontend tools pinned and explicit.
- Do not claim support beyond a specimen's manifest.
- Demonstrate at least one foreign-specimen result embedded into a native Calque executable.

## 13. Standard library, modules, and application boundaries

The release needs enough library support for its own compiler and useful small applications. A global package ecosystem is not a prerequisite.

### Library requirements

- Provide text, bytes, numeric utilities, collections, and sequence operations.
- Provide `Option` and `Result` helpers that compose with pipelines.
- Provide source spans, diagnostics, parsing utilities, and deterministic serialization.
- Provide paths and scoped file access for compiler and command-line applications.
- Provide argument parsing, buffered terminal output, and test helpers.
- Provide builders for efficient text and collection construction.
- Provide a reflection-based codec with round-trip tests.
- Keep intrinsic functions narrow and documented.
- Implement ordinary library algorithms in Calque.

### Module and foreign-interface requirements

- Implement local modules with explicit imports and public boundaries.
- Detect import cycles under a documented policy.
- Make dependency resolution reproducible and independent of the current shell directory.
- Support local package dependencies and a lockable manifest.
- Keep network package discovery outside this release.
- Provide a minimal documented C ABI for supported scalar and explicit-layout types.
- Separate managed values from foreign layout types.
- Specify ownership and lifetime rules at foreign boundaries.
- Isolate raw pointers and unchecked foreign operations behind an explicit unsafe boundary.
- Reject unsupported foreign signatures instead of implicit layout guesses.
- Keep arbitrary FFI unavailable during default compile-time execution.

## 14. Developer tools are release features

The language's developer experience is part of the implementation contract. Documentation cannot substitute for diagnostics, navigation, or reliable local workflows.

### CLI requirements

- Provide commands for execution, compilation, analysis, tests, formatting, and the REPL.
- Provide artifact emission and compiler-stage inspection.
- Provide structured diagnostic output for tools.
- Provide deterministic exit codes and useful command help.
- Hide routine MLIR and LLVM setup from normal program execution after installation.
- Share implementation between `calque` and the Frankish profile entry point.

### Formatter and language-server requirements

- Implement a comment-preserving canonical formatter.
- Verify formatter idempotence and semantic preservation.
- Implement diagnostics, hover, completion, definition lookup, references, and rename.
- Make rename respect scope and shadowing.
- Return useful partial results for incomplete source.
- Reuse the canonical frontend rather than a second editor parser and type checker.
- Handle cancellation and stale document versions.
- Add protocol-level tests for multi-file edits and incomplete programs.
- Ship a minimal editor integration with installation instructions.

### REPL requirements

- Provide typed value display and multiline input.
- Provide type queries, module load, source diagnostics, and IR inspection.
- Define declaration replacement and state lifetime explicitly.
- Preserve resource isolation after failed evaluations.
- Golden-test complete REPL transcripts.

### Diagnostics and debug requirements

- Attach original source spans to user-facing errors.
- Include the primary cause and relevant secondary locations.
- Show generic, compile-time, and generated-code origins without an unreadable trace dump.
- Avoid raw IR-only diagnostics for ordinary source failures.
- Preserve source mapping through native debug information where the backend supports it.
- Add one native debugger smoke test for source-level breakpoints and a call stack.
- Distinguish source-mapped compiler diagnostics from native debug-information support.

### Incremental work requirements

- Track dependencies at useful declaration or module boundaries.
- Reuse unchanged analysis results after local edits.
- Invalidate transitive dependents correctly.
- Report cold and warm analysis times separately.
- Add counters that prove cache hits and limited recomputation.
- Preserve complete diagnostics and necessary analysis during iteration-speed optimization.

## 15. Performance with a visible cost model

The native path must earn its performance claims. MLIR is infrastructure, not evidence of efficient generated code.

### Required optimization work

- If function identity is known, preserve direct calls.
- Avoid closure environments for functions without captures.
- If the proof permits it, remove nonescaping closure allocations.
- Implement generic specialization without unresolved dynamic dispatch on ordinary concrete calls.
- Optimize representative record and tagged-union layouts.
- Preserve efficient exhaustive dispatch for pattern matches.
- If semantics permit it, fuse explicit sequence pipelines into ordinary loops.
- Eliminate intermediate sequence allocations in the demonstrated fusion case.
- Elide redundant retain and release operations where ownership analysis proves safety.
- Use uniqueness analysis for demonstrably safe internal updates.
- Preserve proper tail calls after optimization.
- Keep optimization passes independently testable and inspectable.

### Performance controls

- Provide a source-level cost report for allocations, closure captures, and unresolved dynamic calls.
- Provide an enforceable no-heap-allocation contract for supported functions.
- Include reachable calls in that contract.
- If allocation freedom cannot be established, reject unresolved call targets under the no-allocation contract.
- Explain rejected contracts with a source-level call path.
- Do not label an unproven function allocation-free.

### Measurement procedure

1. Establish correct baseline implementations before performance changes.
2. Record hardware, toolchain, target, build mode, and runtime strategy.
3. Measure compiler startup separately from program execution.
4. Measure cold compilation separately from warm incremental work.
5. Measure allocation counts and peak memory alongside elapsed time.
6. Use repeated trials and report variability.
7. Compare generated code against an equivalent direct implementation for representative cases.
8. Record performance budgets before the optimization milestone.
9. Preserve benchmark inputs and measurement scripts.
10. Investigate statistically meaningful regressions before release.

### Required evidence

- Demonstrate an allocation-free specialized sequence loop with equivalent observable behavior.
- Demonstrate a noncapturing function without a heap environment.
- Demonstrate a recursive workload with bounded stack use under the tail-call contract.
- Demonstrate actual reuse after an unchanged frontend query.
- Demonstrate a compile-time cache hit and a correct dependency-triggered miss.
- Publish measured results without invented targets or unsupported comparisons against other languages.

## 16. Verification strategy

Frankish's interpreter remains the kernel reference. Calque has no external upstream compiler. Agreement between execution paths does not detect every shared frontend error.

### Required verification layers

- Add source-level semantic examples with independently derived expected results.
- Add negative syntax, type, effect, and compile-time tests.
- Add kernel verifier tests for malformed operations and type combinations.
- Compare interpreter, JIT, and AOT results across the admitted corpus.
- Compare supported memory strategies where their contracts admit the same program.
- Compare runtime evaluation with compile-time evaluation for pure expressions.
- Compare interpreted and native compiler output for the same source.
- Add bootstrap fixed-point tests from Section 5.
- Add property tests for collections, codecs, substitutions, and pattern compilation.
- Add typed program generation and grammar-aware malformed-input tests.
- Add metamorphic tests for semantics-preserving source transformations.
- Use independent subset oracles where a valid translation to an existing specimen provides useful evidence.
- Do not build a second production Calque interpreter to satisfy this section.

### Adversarial cases

- Test shadowing and captured values across nested scopes.
- Test multiple generic instantiations and recursive generic layouts.
- Test guard order and failures inside sequence pipelines.
- Test escaped closures and persistent collection aliases.
- Test runtime layout descriptors against actual retained and traced fields.
- Test cleanup across error propagation and nested evaluations.
- Test deterministic compile-time budgets and failed-cache isolation.
- Test target-dependent integers and serialization boundaries.
- Test source paths, synthesized symbols, and module names for collisions.
- Test truncated artifacts and incompatible interchange versions.
- Test long-lived compiler and REPL processes for memory growth.
- Test malformed editor requests and stale document updates.

### Test discipline

- Land a verifier with or before each feature.
- Keep canonicalization narrow enough to expose semantic differences.
- Never normalize away a real error, failure-order difference, or missing output.
- Verify that deliberate semantic mutants fail the relevant tests.
- Preserve all existing specimen tests.
- Explain every intentional golden change in the required commit record.
- Record skipped and unavailable checks as unavailable, not passed.

## 17. Portability and deployment

Calque applications and the Calque compiler have different host requirements. The application portability grid does not imply that the entire compiler runs freestanding.

### Requirements

- Use Frankish's current target grid as the application's portability baseline.
- Preserve existing cross-compilation and runtime-twin obligations.
- Include the established big-endian canary where applicable.
- Exercise target-sensitive compile-time behavior on both 32-bit and 64-bit targets.
- Support the current WASI application path without a WasmGC dependency.
- Test the compiler on the repository's supported development hosts.
- Document host-only compiler services and optional external frontend dependencies.
- Keep generated applications independent from build-host absolute paths.
- Separate required local gates from scheduled architecture gates.
- Report exact target coverage instead of claiming universal portability.

## 18. Delivery sequence

The labels below are local goal stages. Map them onto available repository milestone numbers. The complete goal includes every stage.

### C0: Charter and executable contract

- Record the authorized scope amendments.
- Freeze the initial language examples and bootstrap subset.
- Add a feature-to-evidence acceptance matrix.
- Establish the repository baseline and implementation plan.
- Create the specimen manifest and permanent goal record.

Exit: The next stage has concrete tests, file locations, and a documented bootstrap route.

### C1: Interpreted compiler written in Calque

- Implement the minimal seed and reusable host bridge.
- Write the canonical compiler core in Calque.
- Compile and interpret that compiler through Frankish.
- Compile a small independent corpus through it.
- Establish the initial self-compilation gate.

Exit: The compiler exists as Calque source and runs under Frankish's interpreter. A host-language production frontend does not exist.

### C2: Useful functional language

- Complete pipelines, matches, closures, errors, generics, and recursive data.
- Complete the compiler's required standard library and module support.
- Expand the self-hosted compiler through the recorded bootstrap chain.
- Keep the seed frozen.
- Promote required kernel, runtime, and frontend services.

Exit: The compiler and representative applications use the actual functional language rather than permanent bootstrap restrictions.

### C3: Shared compile-time execution

- Complete ordinary compile-time evaluation and reification.
- Complete type construction, reflection, capabilities, and cache validity.
- Complete nested compiler-library execution and cross-specimen entry points.
- Add target and failure-isolation tests.

Exit: The compiler, ordinary programs, and supported foreign profiles use the same bounded Frankish evaluation service.

### C4: Native self-host and performance

- Build the native compiler from the canonical Calque source.
- Establish the final reproducible bootstrap chain and fixed point.
- Complete the required optimizations and cost controls.
- Publish reproducible measurements and artifact evidence.

Exit: Interpreted and native compilers agree across the release corpus. Performance claims have direct evidence.

### C5: Developer experience and portability

- Complete the formatter, language server, REPL, and shared analysis queries.
- Complete incremental invalidation and native debug support.
- Complete the applicable target and runtime matrix.
- Complete clean-install and offline-after-setup workflows.

Exit: A new user can edit, analyze, test, and compile Calque without internal compiler knowledge.

### C6: Release and demonstration package

- Complete the demonstrations in Section 20.
- Complete the reference documentation and architecture explanations.
- Audit every acceptance item against the current tree.
- Resolve release defects and record remaining non-goals.

Exit: Every mandatory requirement has direct evidence from the final repository state.

## 19. Repository artifacts and commands

The following paths are defaults. Existing conventions take precedence when they provide equivalent, discoverable artifacts.

```text
specimens/calque/MANIFEST.md
specimens/calque/examples/
specimens/calque/conformance/
tools/calque/compiler/
tools/calque/std/
tools/calque/bootstrap/
docs/calque/GOAL.md
docs/calque/SPEC.md
docs/calque/BOOTSTRAP.md
docs/calque/ACCEPTANCE.md
docs/calque/PERFORMANCE.md
docs/calque/DEMOS.md
book/src/specimens/calque.md
```

### Artifact requirements

- Store this complete goal in the repository.
- Keep the canonical compiler source and historical bootstrap snapshots distinguishable.
- Keep the acceptance matrix current with commands and evidence paths.
- Document the language grammar, evaluation order, effects, and memory contract.
- Document compile-time phases, target behavior, and sandbox boundaries.
- Document the bootstrap trust root and reproduction recipe.
- Document every shared Frankish improvement and its consumers.
- Integrate the book, README, and conformance dashboard.
- Keep unsupported capabilities distinct from unfinished mandatory requirements.

### Build interface requirements

- Provide `make calque-bootstrap` for the reproducible bootstrap chain.
- Provide `make calque-test` for Calque's required local conformance gates.
- Provide `make calque-selfhost` for fixed-point and compiler-parity verification.
- Provide `make calque-comptime` for compile-time and cross-specimen conformance.
- Provide `make calque-dx` for formatter, editor, and REPL verification.
- Provide `make calque-bench` for reproducible measurements.
- Provide `make calque-demos` for demonstration verification and artifact production.
- Provide an aggregate Calque release gate.
- If equivalent existing targets avoid duplicate infrastructure, reuse them.
- Document any target-name differences in the acceptance matrix.
- Keep all required operations reproducible without the coding agent.

## 20. Demonstrations with executable evidence

The demonstrations prove the product. They are not substitutes for the test suite. Each terminal session has a short readable version and a complete verification command.

### Demo A: The compiler is the program

- Show the canonical compiler source in Calque.
- Run that compiler through Frankish's interpreter.
- Build the native compiler from its output.
- Rebuild the compiler with the native result.
- Show verified canonical artifact equality.
- Label any abbreviated bootstrap output explicitly.

### Demo B: Functional source, ordinary machine loop

- Show a readable pipeline over records.
- Produce a deterministic result from a generated dataset.
- Show the specialized loop and allocation evidence.
- Compare the result with a direct reference implementation.
- Demonstrate one useful pipeline type error and its source location.

### Demo C: The runtime work disappeared

- Compute a nontrivial table or parser structure with an ordinary Calque function at compile time.
- Embed the result in a native executable.
- Verify the result against runtime evaluation of the same function.
- Show artifact evidence that the compile-time-only computation is absent at runtime.
- Show a real compile-time error with its call trace.

### Demo D: Calque compiles Calque during compilation

- Invoke the compiler library inside a bounded `comptime` expression.
- Compile a small in-memory Calque program.
- Evaluate its emitted artifact through Frankish.
- Embed the result into the outer application.
- Prove that an input change changes the resulting constant.

### Demo E: One interpreter, another specimen

- Evaluate an admitted pure program from another registered specimen at compile time.
- Cross the documented value boundary explicitly.
- Embed the result into a native Calque program.
- Show a rejected foreign effect or unsupported boundary value.
- Avoid any claim of unrestricted interoperability.

### Recording requirements

- Record actual terminal sessions with asciinema.
- Preserve the `.cast` files, scripts, input fixtures, and expected outputs.
- Produce MP4 exports through a documented, pinned rendering toolchain.
- Verify that each MP4 decodes and contains the expected demonstration.
- Keep the main clips readable without audio.
- Show actual timings rather than hardcoded performance numbers.
- Disclose cuts, acceleration, and shortened logs.
- Keep full benchmark evidence available outside the short clips.
- Do not fabricate output or use prerecorded results as live execution.
- Do not publish the videos or create social accounts.

## 21. Non-goals for this release

The bounded release does not include every possible functional-language feature. These exclusions do not remove any mandatory capability elsewhere in this goal.

- Do not rewrite Frankish, MLIR, or LLVM in Calque.
- Do not build another VM or production interpreter.
- Do not build an online package registry.
- Do not require whole-language lazy evaluation or implicit currying.
- Do not require unrestricted dependent types or general trait search.
- Do not require multi-shot continuations, a distributed actor runtime, or a new scheduler.
- Do not require a GPU backend or WasmGC backend.
- Do not require arbitrary runtime interoperability between every specimen.
- Do not treat a manual-memory profile as a prerequisite for ordinary managed applications.
- Do not spend the release budget on unrelated syntax experiments or branding assets.

## 22. Work loop and handoff

### Per-feature procedure

1. Identify the source behavior and shared compiler obligation.
2. Add a failing verifier or independently specified test.
3. Record any unresolved design decision through the ledger.
4. Implement the smallest complete vertical slice.
5. Run the relevant Calque and shared Frankish gates.
6. Investigate every differential mismatch before feature expansion.
7. Inspect the generated artifacts when the claim concerns code generation.
8. Update the acceptance evidence and documentation.
9. Commit the verified step under repository conventions.
10. Update `STATE.md` with the next executable action.

### Continuation requirements

- Use parallel agents only for bounded tasks with explicit interfaces and integration gates.
- Keep tool-specific coordination outside build and test requirements.
- When practical, review state isolation, ABI changes, and semantic transforms independently.
- Fix prerequisite defects that directly block the release.
- Avoid unrelated refactors without demonstrated value to this goal.
- Batch genuine human-only questions.
- Resolve ordinary implementation choices through the ledger.
- Preserve evidence for environmental blockers and unavailable tools.
- Do not treat a toolchain failure as a semantic test pass.
- Leave the tree documented and recoverable at each session boundary.
- When time or token limits interrupt the work, keep the full objective active.

## 23. Final acceptance and completion

### Required final audit

- Verify that Calque is a registered Frankish specimen with a versioned manifest.
- Verify that the canonical compiler implementation is written in Calque.
- Verify that the compiler runs under Frankish's interpreter.
- Verify that the native compiler derives from that same canonical source.
- Verify the auditable bootstrap chain and fixed-point comparison from clean inputs.
- Verify all required functional-language and standard-library capabilities.
- Verify shared interpreter execution for runtime and compile-time code.
- Verify compile-time types, reflection, reification, isolation, and reproducibility.
- Verify compiler-library execution during `comptime`.
- Verify the supported cross-specimen compile-time boundary.
- Verify source-level diagnostics, formatter, language server, REPL, and incremental behavior.
- Verify the required cost reports, allocation contracts, and performance evidence.
- Verify the applicable target and memory-strategy matrix.
- Verify that existing specimens retain their admitted behavior.
- Verify that new shared capabilities satisfy their kernel and ABI contracts.
- Verify documentation, clean setup, and offline-after-setup reproduction.
- Verify every demonstration script, `.cast` recording, and MP4 export.
- Verify that mandatory work does not remain behind placeholder implementations or permanent bootstrap fences.
- Verify all claims against current files and current command output.

### Completion procedure

- Produce a final evidence report with the exact revision and commands.
- Include the bootstrap trust root and stage hashes.
- Include conformance, portability, developer-tool, and performance results.
- Include paths to the executable examples and demonstration artifacts.
- List limitations honestly.
- Keep unfinished requirements marked as unfinished.
- Update `STATE.md` and the acceptance matrix before the final report.
- Call `goal({op:"complete"})` only after every mandatory deliverable has direct current-state evidence.
- Otherwise, preserve the active goal and the next executable action.

**Finish with a language people can use, a compiler written in that language, and a stronger Frankish underneath both.**
