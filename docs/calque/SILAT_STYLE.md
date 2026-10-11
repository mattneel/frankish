# Silat Style

## Calque's source discipline and active steering amendment

**Write the intent. Own the effects. Prove the result.**

Calque puts developer experience first. Its source must explain the program rather than expose routine compiler machinery. Performance controls remain explicit where they change contracts.

The name is a nod to Indonesian Pencak Silat. These engineering rules are Calque's own, not purported traditional martial-arts teachings. TigerStyle inspires the discipline of a coherent house style, not a copied restriction set.

This document proposes the house style and directs its adoption by the active coding agent. New surface examples are release requirements, not claims about current bootstrap support.

## 1. Authority and the immediate correction

### Actions

- Read the current constitution, decisions, specification, bootstrap plan, and active amendments.
- Inspect actual compiler sources before classification of a workaround.
- Store this document at `docs/calque/SILAT_STYLE.md` and register it as an active amendment.
- Preserve historical goal documents and their integrity pins.
- Append superseding decisions for genuine conflicts.
- Preserve unrelated user changes and current bootstrap progress.
- Adopt the companion vector-sugar contract or reconcile an equivalent existing amendment.
- Treat source ergonomics as part of feature completion, not optional final polish.
- Do not finish with only another policy document.
- Migrate representative compiler code and prove the migration through executable tests.

### The central rule

A language implementation must not make its temporary limitations the language's personality. Bootstrap constraints need explicit owners and exit conditions. They do not establish production idioms.

A collection literal must not require a constructor ceremony. Ordinary vector arithmetic must not require a tree of intrinsic calls. An error value must not hide inside a fake successful value.

### Current audit anchors

The inspected `tools/calque/compiler/parse.calq` uses one-element vectors for cursor and error state. It compares rendered token-kind names and constructs `Expr.Int("0")` as a poison value. The source documents bootstrap constraints.

The inspected `tools/calque/compiler/lex.calq` carries success and error state through parallel fields. These observations identify migration work, not unverified claims about execution failures.

### Migration actions

- Recheck these examples against the current checkout.
- Separate frozen snapshots, current bootstrap-compatible code, and canonical production code.
- Keep historical snapshots byte-identical unless the bootstrap protocol explicitly requires a versioned replacement.
- Add missing prerequisite operations through the existing verifier-first process.
- Replace bootstrap workarounds when the canonical compiler can consume the supported replacement.
- Record each temporary exception with its reason, affected files, owner, and executable removal condition.
- Do not let an exception expire merely because a date passes.
- Do not let an exception survive merely because no person remembers it.
- Keep unavailable syntax tests pending with explicit evidence requirements.
- Do not mark a source example supported because a similar lower-level program works.

## 2. Name the domain, not the scaffolding

Names form the first explanation of a program. Small local mathematical names are useful. Short opaque names for major compiler concepts are not.

### Rules

- Use `PascalCase` for types and type factories.
- Use `snake_case` for ordinary functions, values, fields, modules, and source files.
- Preserve exact foreign names and explicitly supported compatibility names.
- Migrate an established public spelling through a recorded compatibility decision.
- Use names such as `Token`, `Parser`, `BinaryExpr`, `left`, and `right` for their actual domain concepts.
- Permit conventional local names such as `i`, `x`, and `y` where the context makes their meaning immediate.
- Put units in names or types where ambiguity affects correctness.
- Keep one term for one concept throughout a subsystem.
- Name helpers for the invariant, operation, or domain boundary they own.
- Do not create `Manager`, `Processor`, `Handler`, or `Context` layers without a concrete responsibility.
- Do not add a wrapper solely to rename a call or satisfy a line-count rule.

## 3. Give every state an honest shape

A Boolean beside an error string often encodes a sum type badly. A printable name is not a semantic discriminator. A parser recovery node is not a valid integer expression.

### Rules

- Use `Result`, `Option`, structs, and tagged unions for their intended distinctions.
- Match on actual variants rather than their rendered names or undocumented integer tags.
- Keep tags and byte codes explicit at real wire or ABI boundaries.
- Do not encode failure as an empty string, negative sentinel, or dummy success without a domain contract.
- Distinguish recoverable external failures from internal invariant violations.
- Use required fields and declared defaults rather than meaningless zero initialization.
- Validate input at its trust boundary and preserve useful validation facts.
- Do not duplicate proven checks mechanically without a new trust or lifetime boundary.
- Keep parser recovery explicit through error nodes with source origins.
- Reject recovery nodes before execution can treat them as valid expressions.
- Preserve each diagnostic cause separately from shared presentation buffers.
- Use a real cursor, explicit state value, or admitted mutable cell instead of a one-element collection as a scalar box.
- Do not force functional state reconstruction when a confined mutable cursor expresses the parser more clearly.

### Intended form

```calque
pub fn is_literal(kind: TokenKind) bool {
    match kind {
        .Int | .Str => true,
        _ => false,
    }
}
```

A wildcard is appropriate here because the function intentionally classifies a subset. It is not an automatic repair for a newly missing semantic case.

A keyword table legitimately compares input text. A compiler token-kind comparison must not depend on presentation text.

## 4. Use the language you built

Calque source favors literals, patterns, expression results, comprehensions, and typed propagation. These facilities remove mechanical work. They preserve semantic boundaries.

### Canonical collection and SIMD surface

| Spelling | Meaning |
|---|---|
| `[T]` | Immutable persistent collection, compatible with `Vector(T)`. |
| `[N]T` | Fixed-length inline array. |
| `vec[N]T` | Explicit SIMD value with `N` logical lanes. |

```calque
const users: [User] = [
    %User{name: "Matt"},
    %User{name: "Rama", active: false},
];

const active_names: [Str] = [
    for user in users
    if user.active
    => user.name
];

const rgba: vec[4]f32 = [0.2, 0.4, 0.8, 1.0];
const bgra = rgba.zyxw;
```

### Rules

- Prefer literals over `vector.new()` plus repeated pushes for a fixed set of elements.
- Prefer a comprehension for a local eager selection and transformation.
- Use pipelines when named transformations make the data flow easier to read.
- Keep explicit sequence APIs when lazy traversal or allocation behavior is part of the contract.
- Do not require `seq.iter()` merely to write an ordinary comprehension.
- Use array and collection patterns instead of length-and-index scaffolding when they express the decision directly.
- Keep explicit builders for bulk construction when their measured behavior matters.
- Use confined loops and mutation when they express the algorithm more clearly.
- Do not replace a good loop merely to increase the number of pipelines.
- Do not enforce arbitrary pipeline lengths or mandatory one-line functions.
- Bind a meaningful intermediate when it names a concept or makes a failure boundary visible.
- Do not create a temporary name for every trivial subexpression.
- Infer obvious local types and state public contracts explicitly.
- Keep a single canonical spelling in examples and generated code.

### Immutable update example

```calque
const scores: [i64] = [10, 20, 30];
var revised = scores;
revised[1] = 99;

assert scores == [10, 20, 30];
assert revised == [10, 99, 30];
```

The assignment updates the mutable binding's value. It does not mutate earlier aliases. The compiler can reuse storage only when ownership permits it.

## 5. Let vector arithmetic look like arithmetic

```calque
pub fn advance(
    position: vec[4]f32,
    velocity: vec[4]f32,
    dt: f32,
) vec[4]f32 {
    position + velocity * dt
}
```

### Rules

- Use arithmetic operators for supported SIMD operations.
- Broadcast a compatible scalar operand once under the vector-sugar type rules.
- Keep runtime numeric conversions explicit when they narrow or otherwise change the numeric contract.
- Preserve lane counts, logical lane order, masks, and checked failures.
- Do not allocate a collection to construct a contextually typed SIMD literal.
- Keep logical lanes independent from the target's physical register width.
- Preserve strict floating-point semantics unless source explicitly selects another numeric policy.
- Do not silently contract multiplication and addition into FMA under the strict policy.
- Keep mask-to-Boolean conversion explicit through `simd.any` or `simd.all`.
- Use one clear selection operation. Do not invent separate conditional syntax for every vector operation.
- Keep eager selection distinct from masked execution and masked memory access.
- Do not claim that `simd.select` protects an expression that already trapped during argument evaluation.
- Reserve explicit intrinsics for operations without an honest ordinary-language spelling.
- Preserve scalar fallback and report its actual target behavior.
- Verify the lowered operations and native code before performance claims.

The shortest source is not necessarily the clearest source. The compiler must remove routine splat and builder mechanics before users need to invent wrappers for them.

## 6. Keep the happy path direct and the failures visible

### Rules

- Use `try` for ordinary propagation of a compatible error type.
- Match when branches represent meaningful recovery or distinct outcomes.
- Adapt unrelated error types explicitly at the actual boundary.
- Keep required diagnostic payloads and source spans intact.
- Render a diagnostic where the tool or application owns presentation.
- Do not build nested concatenation trees throughout semantic code to format errors.
- Use `expect` for deliberate invariant checks, test setup, or bounded examples where termination is the stated contract.
- Do not turn recoverable library errors into traps merely to shorten source.
- Keep fallback policy explicit and test its consequences.
- Never swallow an error just to obtain green output.
- Never insert a wildcard merely to quiet exhaustiveness diagnostics.
- Assert meaningful internal invariants and test invalid external inputs as errors.
- Do not require an assertion quota per function.

### Preferred boundary

```calque
pub fn load_config(fs: Fs, path: Path) ConfigError!Config {
    const text = try read_config_text(fs, path);
    decode_config(text)
}
```

In this example, both helpers deliberately expose `ConfigError`. Their implementations preserve the underlying failure as structured data. No implicit conversion between unrelated error types is implied.

## 7. Keep ownership and concurrency visible

### Rules

- Own every task through its declared scope.
- Keep explicit awaits, cancellation boundaries, deadlines, and cleanup visible.
- Do not hide detached tasks behind an innocent-looking helper.
- Keep CPU parallelism distinct from ordinary sequence evaluation.
- Reject unsafe cross-thread captures. Do not prescribe unchecked annotations as a repair.
- Prefer immutable messages or exclusive transfer where they fit the problem.
- Keep shared mutation behind its actual synchronization contract.
- Avoid a new thread pool for each library or nested operation.
- Preserve a resource until all children that can access it complete.
- Do not call a cancellation request proof that foreign work stopped.
- Keep test and compiler execution contexts isolated.
- Use the shared runtime contracts. Do not add a parallel convenience subsystem.

## 8. Explain cost without call-site ceremony

Safety and semantic defaults belong in versioned APIs. Repeated defaults at every call are not automatically clearer. Explicit annotations belong where they state a necessary contract or override a default.

### Rules

- State representation, resource, ordering, and numeric choices when they affect observable behavior.
- Keep collection concatenation distinct from SIMD arithmetic.
- Keep eager allocation distinct from explicit sequence consumption.
- Avoid quadratic accumulation when an admitted builder or algorithm solves the actual workload.
- Preserve explicit no-allocation, alignment, CPU-feature, and parallelism contracts where required.
- Use the cost report to support allocation or fusion claims.
- Measure a performance-sensitive change against its actual workload.
- Keep correctness evidence independent from speed measurements.
- Do not use unsafe casts, global fast-math, or weakened bounds to satisfy a style suggestion.
- Keep physical layout and machine tuning separate from portable CON/CTF identities.
- Permit a justified longer implementation when it gives a materially better algorithm or diagnostic.
- Record the reason beside non-obvious optimization constraints.

## 9. Documentation carries knowledge. Tests carry evidence.

### Rules

- Use source documentation for public contracts, important defaults, errors, effects, and examples.
- Comment invariants, surprising constraints, and rejected alternatives that matter to maintenance.
- Do not narrate obvious statements or add a project-history preamble to every function.
- Put session history and milestone status in their designated repository records.
- Preserve license notices and relevant compatibility explanations.
- Include concise executable examples rather than repeated motivational prose.
- Use ordinary assertions, doctests, properties, and fuzz regressions through the shared runner.
- Test failure behavior and boundaries as well as representative success cases.
- Keep a regression input for every repaired semantic defect.
- Distinguish code generation evidence, tests, benchmarks, and bounded schedule exploration.
- Do not claim that a test suite proves properties it does not observe.
- Keep style examples accessible through CLI Markdown and editor help.

## 10. No cargo-cult restrictions

Silat Style does not adopt TigerStyle's entire policy set. Calque has a different product contract.

### Non-rules

- Do not ban recursion in a language that deliberately supports it.
- Do not ban closures, higher-order functions, or managed allocation.
- Do not force one thread for a language with explicit multicore execution.
- Do not require two assertions per function or an arbitrary fixed function length.
- Do not prohibit early returns when they clarify a real failure boundary.
- Do not demand a wrapper or interface for every operation.
- Do not extract an abstraction solely because two snippets resemble each other.
- Keep one invariant together when a function splits into helpers.
- Do not rename established foreign APIs to satisfy local aesthetics.
- Do not classify source as AI-authored or punish an author's presumed identity.
- Do not make a metric's green status more important than a readable, correct program.

## 11. Enforcement: grammar, semantic rules, and judgment are different

The formatter owns mechanical presentation. Compiler checks own semantic safety. Lints explain supported patterns. Review handles taste that lacks reliable machine evidence.

| Concern | Enforcement |
|---|---|
| Literal, pattern, update, and SIMD spelling | Parser, types, formatter, and conformance tests. |
| Wrong variant handling or unsafe task escape | Existing compiler diagnostics. |
| Enum-to-display-string comparisons | Type-aware lint with positive and negative fixtures. |
| Constant collection constructor ceremonies | Semantic lint and verified equivalent edits. |
| In-band error sentinels | Evidence-based advisory, not a blanket ban on valid domain values. |
| One-element collections used as scalar cells | Advisory with escape and mutation context. |
| Redundant branches and identity transforms | Verified effect-aware suggestions. |
| Naming and helper boundaries | Review guidance with narrow mechanical checks. |
| Function size and abstraction quality | Review hints, not universal failure thresholds. |
| Bootstrap exceptions | Explicit ledger and migration evidence. |

### Tool integration

```console
calque fmt --check
calque lint --profile silat
calque lint --profile silat --help-mode agent --diagnostic-format markdown
calque help style --format markdown
calque ci
```

These commands are integration targets. Reuse existing equivalent flag contracts. Do not create a second linter or CI definition.

### Lint requirements

- Integrate the Silat profile with the existing compiler-native lint system.
- Make its deterministic high-signal rules the house default for canonical Calque sources.
- Leave subjective advisory rules independently configurable.
- Keep public application defaults documented and stable within their declared compatibility policy.
- Share diagnostic IDs, Markdown explanations, and reviewed edit plans with LSP and CLI.
- Include a preferred example, an intentional counterexample, and applicability conditions for every rule.
- Prove an edit preserves evaluation count, order, error behavior, aliasing, and the relevant resource contract.
- Mark intent-dependent edits as review-required.
- Do not offer unsafe automatic repairs for an inferred style problem.
- Require specific reasons for suppressions.
- Keep a missing prerequisite distinct from a waived final requirement.

## 12. Prove the house style in the compiler

### Required work

1. Audit the active compiler, standard library, examples, and generated snippets.
2. Identify actual bootstrap constraints before a style judgment.
3. Land prerequisite syntax and typed operations through the current bootstrap plan.
4. Migrate one complete lexer or parser path to the supported idiomatic forms.
5. Replace display-name comparisons with semantic variant analysis.
6. Replace one-element cursor or error storage with an admitted explicit state representation.
7. Replace fake successful poison values with explicit recovery states where recovery is needed.
8. Replace parallel success/error fields with a coherent error or recovery contract.
9. Preserve full diagnostics, accepted source, and error behavior under the documented migration.
10. Run the source corpus and independent negative cases.
11. Run formatter, style, doctest, self-host, and relevant runtime gates.
12. Record the remaining exceptions and their executable removal conditions.

### Acceptance evidence

- Demonstrate `[T]`, `[N]T`, and `vec[N]T` without constructor boilerplate.
- Demonstrate a comprehension and a nested vector/struct pattern through the actual compiler.
- Demonstrate scalar-broadcast SIMD arithmetic without per-call splat boilerplate.
- Verify immutable element updates preserve old aliases.
- Verify that contextual SIMD literals do not allocate persistent collections.
- Verify arithmetic order, mismatch diagnostics, masks, swizzles, and scalar fallback.
- Preserve public error contracts and parser recovery behavior after the style migration.
- Show before-and-after source for one meaningful compiler function rather than only a toy example.
- Keep explicitly frozen historical sources exempt from automatic rewrites.
- Preserve the existing self-host fixed point and specimen matrix.
- Publish the preferred style examples as executable source and CLI Markdown.
- Reject intentionally regressed idioms through the applicable rule tests.
- Keep human judgment explicit where no sound mechanical rule exists.
- Mark this amendment complete only after the actual source and tests demonstrate its rules.

## Reference anchors

The naming reference is UNESCO's *Traditions of Pencak Silat* entry. The engineering reference is TigerBeetle's `docs/TIGER_STYLE.md`. These rules are an original Calque proposal.

The vector contrast is Zig's versioned language reference: lane operators are familiar, but scalar/vector mixing requires an explicit splat. Calque deliberately proposes contextual scalar broadcast instead.

The repository audit anchors are `tools/calque/compiler/lex.calq` and `tools/calque/compiler/parse.calq`, inspected on 2026-10-11. The active agent must recheck them because work continues.

**Do not make Calque look like the temporary compiler that first learned to compile it.**
