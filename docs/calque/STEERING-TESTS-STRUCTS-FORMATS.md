# Steering amendment: Calque tests, Elixir-style structs, CON, and CTF

## 1. Authority and direction

This is an additive amendment to the active Calque goal. It supersedes conflicting surface examples, not the remaining goal. Calque remains a self-hosted Frankish specimen.

The required names are:

- **CON** means **Calque Object Notation**. Its extension is `.con`.
- **CTF** means **Calque Term Format**. Its extension is `.ctf`.
- **`calque test`** is the project test entry point.

The product direction is one coherent experience. Elixir-style structs define values. Tests inspect and compare those values. CON and CTF represent them through one shared serialization contract. JSON codecs reuse that contract without reducing it to JSON's value model.

### Immediate actions

- Preserve the active goal and all unrelated work.
- Read the current constitution, state, Calque specification, bootstrap plan, and acceptance matrix.
- Inspect the current implementation before amendments.
- Record this prompt as `docs/calque/STEERING-TESTS-STRUCTS-FORMATS.md` or an equivalent permanent amendment.
- Append superseding decisions for conflicting syntax and scope rulings.
- Add the amendment to the goal authority and acceptance checks.
- Preserve the original hash-pinned goal as a historical contract.
- Do not disable its integrity check to accommodate new requirements.
- Update the current specification, freeze log, manifest, and acceptance matrix.
- Retain unfinished requirements from the original goal.
- Continue implementation after the amendment, rather than stopping at revised documentation.

## 2. Elixir struct ergonomics are the new record surface

The canonical construction form is `%User{name: "Matt"}`. The canonical update form is `%User{user | active: false}`. Calque retains static field types, nominal identity, and native representations.

This replaces the public dot-initializer surface from earlier examples. It does not require a map allocation or a dynamic runtime representation.

### Required syntax

```calque
pub const User = struct {
    name: Str,
    score: i64 = 0,
    active: bool = true,
};

const original = %User{name: "Matt"};
const edited = %User{original | score: 42};

const greeting = match edited {
    %User{name: name, active: true} => "Hello, " |> str.concat(name),
    %User{active: false} => "Account disabled",
};

assert original.score == 0;
assert edited.score == 42;
```

### Construction and update

- Implement `%Type{field: value}` for named structs.
- Support qualified names and generic type applications.
- Preserve `user.name` for field access.
- Make a field without a default required at construction.
- Apply declared defaults when a constructor omits those fields.
- Do not invent implicit nulls or zero initialization.
- Require pure compile-time defaults under a documented evaluation rule.
- Reject duplicate fields, unknown fields, missing required fields, and incompatible values.
- Implement `%Type{base | field: value}` as an immutable update.
- Evaluate the base once, before explicit field expressions.
- Evaluate explicit expressions once in source order.
- Preserve untouched fields and existing aliases.
- Reject updates that introduce fields or change the nominal type.
- Keep field spelling suggestions and type origins in diagnostics.
- Support trailing commas and multiline layout.
- Do not retain two equally canonical constructor syntaxes indefinitely.

### Patterns

- Implement `%Type{field: pattern}` in matches, destructuring, and test assertions.
- Treat omitted pattern fields as unconstrained, not defaulted.
- Make `%Type{}` match the nominal type regardless of its field values.
- Reject unknown pattern fields.
- Preserve nested patterns, guards, exhaustiveness, and redundant-arm analysis.
- Support explicit pinning with `^name` where a pattern must compare an existing binding.
- Distinguish fresh bindings from pinned values.
- Preserve binding scope after successful assertion patterns.
- Extend the shared pattern machinery rather than adding a test-only matcher.

### Maps and runtime representation

- Provide `%{key: value}` for symbol-keyed maps.
- Provide `%{"key" => value}` for general key-value entries.
- Keep `:name` symbols distinct from strings and tagged-union variants.
- Preserve `.Some`, `.None`, `.Ok`, and `.Err` as union constructors.
- Do not convert Calque's result types into Elixir tuples.
- Specify heterogeneous map construction and inference before implementation.
- Keep named structs distinct from ordinary maps.
- Require explicit checked conversion from a map or term into a struct.
- Prevent nominal-type forgery through a magic map field.
- Keep native field access and layout independent of serialization metadata.
- Preserve allocation-free struct operations wherever the existing representation permits them.

### Integration and migration

- Update parser recovery, formatting, editor completion, rename, reflection, and diagnostics.
- Use `%Type{...}` in complete structural value displays.
- Keep truncated inspection distinct from lossless CON output.
- Migrate active examples, compiler sources, tests, and generated snippets.
- Preserve historical bootstrap snapshots and their hashes.
- If the seed already consumes frozen syntax, introduce a recorded bootstrap transition.
- If implementation has not started, amend the bootstrap grammar before it hardens.
- Verify a clean bootstrap and self-host fixed point after migration.

## 3. The test experience must feel like `mix test`

`calque test` discovers, compiles, and executes project tests without manual harness assembly. Readable failures and selective reruns are primary features.

### Required command surface

```console
calque test
calque test test/user_test.calq
calque test test/user_test.calq:24
calque test test/codecs/
calque test --failed
calque test --stale
calque test --only codec
calque test --exclude slow
calque test --include integration
calque test --seed 12345
calque test --trace
calque test --max-cases 4
calque test --timeout 5000
calque test --max-failures 1
calque test --slowest 10
calque test --watch
calque test --list
calque test --cover
```

### Discovery and selection

- Discover `test/**/*_test.calq` by default.
- Load optional `test/test_helper.calq` through a documented lifecycle.
- Integrate existing inline tests and doctests without duplicate execution.
- Support files, directories, and source-line selection.
- Select the enclosing test or describe block from a line selector.
- Print a copyable rerun command for every failure.
- Support named tests, tags, tag values, skips with reasons, and pending tests.
- Make pending tests visibly unfinished rather than successful.
- Define precedence between project filters, CLI filters, and explicit selectors.
- Report empty selections explicitly.
- Fail explicit selectors that match nothing, except documented empty `--failed` or `--stale` selections.
- Support stable test identities across ordinary line-number changes.
- Preserve failure records for tests that a partial run did not execute.
- Namespace failure records by relevant project and execution configuration.
- Provide machine-readable events alongside human output.
- Keep test events separate from captured application stdout.
- Provide nonzero statuses for failures, compilation errors, crashes, and invalid invocations.

## 4. ExUnit-style test declarations and fixtures

The following is the required ergonomic shape. Auxiliary attribute spelling can follow existing conventions through a recorded decision. Named tests, contexts, assertions, and struct syntax are mandatory.

```calque
@case(%{async: true})

describe "User serialization" {
    setup {
        %{user: %User{name: "Matt"}}
    }

    @tag(:codec)
    test "CTF preserves the typed value", %{user: user} {
        const encoded = ctf.encode(user)
            |> result.expect("the user must encode");
        const decoded = ctf.decode(User, encoded)
            |> result.expect("the user must decode");

        assert decoded == user;
        assert %User{name: name, active: true} = decoded;
        assert name == "Matt";
        refute decoded.active == false;
    }
}
```

### Fixtures and composition

- Implement named `test` declarations and `describe` groups.
- Implement file-level case configuration.
- Implement `setup`, `setup_all`, and `on_exit`.
- Run `setup` before each selected test.
- Run `setup_all` once per selected case.
- Skip fixtures when their case has no selected tests.
- Support reusable setup functions and case helpers.
- Merge fixture context under explicit precedence rules.
- Preserve useful static types for known context fields.
- If context requires dynamic values, use checked access with field-level errors.
- Do not hide unchecked casts inside fixture ergonomics.
- Support context patterns in test declarations.
- Run registered cleanup in reverse order under a documented ownership model.
- Report cleanup failures without hiding the original test failure.
- Preserve valid owner lifetimes for shared fixtures.
- Do not serialize live resources or pointers through CTF to share fixtures.
- Provide per-test temporary directories and explicit test capabilities.
- Provide capability-based test doubles without global monkey-patching.
- Capture stdout, stderr, and logs without cross-test contamination.
- Integrate doctests into selection, source locations, and failure reports.
- Support parameterized tests with stable case identities.
- Add seeded property tests with shrinking for the format and collection suites.

### Assertions

- Implement `assert expression` and `refute expression` for Boolean expressions.
- Capture the source expression and evaluated operands.
- Evaluate operands exactly once.
- Implement `assert pattern = expression` with useful binding and mismatch reports.
- Support approximate floating-point comparisons with explicit tolerances.
- Support expected error results without conversion into exceptions.
- Support expected language traps through an isolated test boundary.
- Never count arbitrary memory corruption as a successful expected trap.
- Show structural differences for structs, maps, collections, strings, and bytes.
- Identify the first relevant nested path and retain full detail on demand.
- Truncate large values under documented display budgets.
- Keep assertion introspection in source-aware compilation or reusable metadata.
- Do not build an unrelated assertion interpreter.

### Execution and isolation

- Default cases to serial execution unless they explicitly opt into concurrency.
- Run concurrent cases under a bounded worker limit.
- Keep tests within one case serial unless a separate contract explicitly permits parallelism.
- Isolate interpreter contexts, runtime state, random seeds, and captured output.
- Use supervised workers when native traps or global state prevent safe in-process isolation.
- Stop timed-out native work rather than leaving an abandoned thread active.
- Report a crashed worker as a test failure.
- Allow unaffected tests to continue.
- Distinguish guaranteed supervisor cleanup from callbacks lost after a hard process failure.
- Do not require a BEAM clone or actor runtime to implement these guarantees.
- Make `--trace` serial and readable.
- Keep finite trace timeouts unless the user explicitly disables them.
- Record randomized order and the seed.
- Do not claim that a seed reproduces operating-system scheduling races.
- Run a shared test corpus under interpreter and native execution.
- Preserve the existing JIT/AOT differential gates.

## 5. Incremental tests without false cache hits

Compiled tests are cacheable artifacts. An ordinary successful test is not a cached promise that the world remains unchanged.

### Required behavior

- Reuse unchanged analysis and compiled test artifacts.
- Execute selected tests on every ordinary `calque test` invocation.
- Do not replace execution with previously successful results.
- Use `--stale` only as an explicit selection optimization.
- Track source, fixtures, helpers, imports, compile-time inputs, and relevant configuration.
- Include fixture and doctest dependencies in stale selection.
- If dependency coverage is incomplete, select the affected case conservatively.
- Never silently omit affected tests to improve latency.
- Debounce watch events without losing edits.
- Handle cancellation, stale results, and edits during a run.
- Print the reason for stale selection through an explanatory mode.
- Keep property-test seeds and failure reproduction information.
- Report compile, discovery, execution, and total elapsed time separately.
- Add query and artifact counters beside latency measurements.
- Keep coverage instrumentation in distinct artifact keys.
- Map coverage to source rather than generated framework glue.
- Do not enable expensive coverage instrumentation by default.

## 6. One common codec foundation

CON is the human-readable notation. CTF is the binary term representation. JSON is an interoperability projection. The common foundation is a term model, typed schema contract, and streaming codec interface.

The dynamic term model is an optional representation, not a mandatory intermediate allocation.

```text
Calque value + derived schema
             |
       shared codec contract
        /       |       \
      CON      CTF      JSON

Optional dynamic Term values use the same contract.
Typed paths can encode and decode directly.
```

### Shared implementation

- Design one reflection-derived codec contract for all three formats.
- Provide direct typed encoders and decoders without a required intermediate `Term` tree.
- Provide dynamic term APIs for unknown schemas and inspection tools.
- Support buffer, reader, writer, and incremental decoder interfaces.
- Share field metadata, validation, error paths, and schema evolution rules.
- Preserve format-specific lexical and wire rules in separate backends.
- Keep existing `json.Codec(T)` APIs as facades when practical.
- Generate specialized paths through Calque's existing compile-time machinery.
- Keep codec policy and ordinary algorithms in Calque.
- Promote reusable host and runtime primitives through Frankish's existing contracts.
- Do not route every typed value through `frk_dyn` merely for serialization.
- Exclude file I/O from pure in-memory codec operations.

### Common API shape

```calque
const packet = try ctf.encode(user);
const restored = try ctf.decode(User, packet);

const document = try con.encode(user);
const fromDocument = try con.decode(User, document);

const jsonText = try json.encode(user);
const fromJson = try json.decode(User, jsonText);

const unknown = try ctf.decodeTerm(packet);
const readable = try con.encode(unknown);
```

### Shared validation

- Distinguish malformed input from valid data that violates the requested schema.
- Report byte offsets, textual spans where available, and nested value paths.
- Reject duplicate fields and duplicate map keys by default.
- Reject unknown typed fields by default.
- Make compatibility policies explicit rather than silently permissive.
- Apply declared struct defaults under the same typed-decoding policy across formats.
- Preserve omitted fields as omitted in schema-free dynamic data.
- Do not invent defaults for an unknown schema.
- Reject missing required fields and numeric narrowing failures.
- Resolve types through explicit registries or compile-time schemas.
- Never load modules or execute arbitrary constructors from serialized type names.

## 7. The portable term model exceeds JSON

CTF targets ETF-class utility for Calque's portable values. It is a Calque format, not an ETF wrapper or a claim of Erlang wire compatibility.

### Required portable values

- Support unit, booleans, and an explicitly defined null term.
- Keep null distinct from unit and optional absence unless an explicit projection says otherwise.
- Support signed, unsigned, and arbitrary-precision integers.
- Support `f32` and `f64`, including special values and negative zero.
- Support valid UTF-8 text, arbitrary byte strings, and bitstrings.
- Support bounded symbols without an immortal global atom table.
- Support tuples, sequences, proper lists, and improper lists in the dynamic term model.
- Keep tuples distinct from lists.
- Support maps with arbitrary admitted term keys.
- Support nominal structs, tagged variants, and concrete generic identities.
- Preserve Option and Result variants rather than flattening them into null or ordinary values.
- Preserve collection kinds when a typed round trip requires them.
- Define exact round-trip guarantees for both typed values and schema-free terms.

### Equality, identity, and unsupported values

- Specify integer-width preservation versus mathematical integer normalization.
- Specify float bit preservation, NaN equality, and canonical NaN policy.
- Define map-key equality and canonical ordering for every admitted key type.
- Keep text, symbols, and bytes distinct.
- Use logical type identities and explicit schema versions rather than compiler-local IDs.
- Include concrete type arguments where nominal generic identity requires them.
- Keep logical schema identity separate from native memory layout.
- Specify recursive values and repeated references.
- Reject cycles deterministically in the initial portable profile unless a complete graph-reference contract exists.
- Bound expansion of shared substructures.
- Reject raw pointers, open resources, live continuations, and executable closures by default.
- Do not fabricate portable equivalents for runtime features Calque does not implement.
- Reserve extension mechanisms without claiming support for unimplemented term kinds.
- Treat any future process or resource reference as an explicit capability protocol.

## 8. CON: Calque Object Notation

CON is safe data syntax with Calque's visual identity. It is not executable Calque and not JSON with a comment preprocessor.

### Intended surface

```con
// Human-editable data, not code.
%Demo.User{
    name: "Matt",
    score: 42,
    active: true,
}
```

```con
%{
    status: :ok,
    labels: [:functional, :native],
    payload: bytes"00 01 fe ff",
    settings: %{"theme" => "dark"},
}
```

The byte-literal spelling above is a proposed lexical form. The `%Type{field: value}` struct form is mandatory.

### Required behavior

- Freeze a complete grammar and versioned term mapping before broad adoption.
- Reuse source literal spelling wherever its semantics match the term model.
- Support comments, trailing commas, multiline layout, and escaped Unicode text.
- Support every admitted portable term through an explicit lossless textual form.
- Specify byte, bitstring, large-integer, special-float, variant, and generic-type spellings.
- Distinguish typed struct terms from ordinary maps.
- Parse unknown nominal types as inert data for dynamic consumers.
- Require explicit schema resolution for typed decoding.
- Reject expressions, calls, interpolation, imports, environment expansion, and filesystem effects.
- Keep declared build-input access outside CON parsing.
- Implement a lossless concrete syntax tree for comment-preserving edits.
- Separate comment-preserving formatting from value decoding and re-encoding.
- Provide readable pretty output and a defined canonical output mode.
- Make canonical output locale-independent and independent of map insertion order.
- Do not use truncated inspection output as serialized data.
- Add editor highlighting and useful parse-recovery diagnostics.

## 9. CTF: Calque Term Format

CTF is a versioned, self-describing binary format for portable Calque values. It supports fast local serialization, persistent data, and framed transport payloads.

ETF-class performance is a release objective that requires benchmark evidence. A round-trip implementation alone does not complete CTF.

### Wire contract

- Write the wire specification before tag assignments become de facto compatibility rules.
- Define magic, versioning, tags, lengths, endianness, and extension handling.
- Keep wire layout independent of compiler layouts, pointer width, and machine endianness.
- Specify limits and overflow checks for every length and count.
- Provide compact encodings for common small values.
- Keep large payloads length-delimited.
- Permit full decoding of a standalone term without a hidden external schema dictionary.
- Support optional in-frame dictionaries for repeated symbols, field names, and type descriptors.
- Bound dictionary growth and validate every reference.
- Preserve unknown extensions only under an explicit opaque-value policy.
- Reject unknown required versions or features.
- Provide exact decoding that rejects trailing data.
- Provide prefix decoding that returns consumed bytes.
- Provide incremental decoding that distinguishes incomplete input from invalid input.
- Support framed sequences of terms for files and transports.
- Keep framing separate from any future distributed-runtime protocol.
- Implement compression as an opt-in envelope with bounded decompression.

### Performance paths

- Implement contiguous-buffer and reusable-buffer encoding.
- Implement scatter/gather output that can retain existing immutable byte payloads.
- Keep referenced buffers alive until the output consumer releases them.
- Provide buffered writer APIs without an intermediate contiguous copy.
- Implement direct typed decoding and dynamic term decoding.
- Provide an explicit view API where payload slices retain their source buffer safely.
- Document retained-buffer memory costs.
- Keep an owning decode path for callers that need independent storage.
- Bound tiny-slice retention of oversized input buffers.
- Do not advertise universal zero-copy decoding.
- Optimize ordinary terms, not only one large binary benchmark.

### Fast and canonical profiles

- Separate the ordinary fast profile from a canonical profile for hashes and reproducible artifacts.
- Define canonical bytes independently from incidental encoder traversal order.
- Define canonical map ordering, integer forms, floats, dictionaries, and schema metadata.
- Verify canonical identity across supported targets and runtime implementations.
- Hash canonical uncompressed content under a versioned domain.
- Do not equate deterministic compressed output with canonical semantic content.
- Keep existing bootstrap and interchange formats until an explicit compatibility migration proves safe.
- Do not silently replace `loanword` or the current compiler artifact format with CTF.

## 10. Safe decoders by default

### Required protections

- Bound nesting, term count, bytes, container sizes, symbols, and aggregate allocation.
- Bound decompressed size and expansion ratios.
- Apply limits before expensive allocation or recursion.
- Use checked arithmetic for offsets and lengths.
- Reject malformed UTF-8 text while preserving arbitrary bytes in byte terms.
- Reject malformed bitstrings and invalid dictionary references.
- Prevent unbounded global symbol interning.
- Prevent deserialization-triggered code execution and authority creation.
- Validate borrowed-view ownership across interpreter and native paths.
- Test cancellation and error paths for leaks and use-after-free defects.
- Fuzz truncation at every byte boundary and adversarial length prefixes.
- Keep safe decoding the ordinary API rather than an optional obscure flag.
- Distinguish structural validity, schema validity, and application authorization.
- Do not treat successful decoding as application-level trust.

## 11. JSON builds on the common ground

JSON remains an interoperability format. Rich Calque values do not gain an automatic lossless JSON representation merely because a common codec exists.

### Required projection rules

- Implement a standards-conforming JSON backend over the shared codec contract.
- Preserve object keys as strings.
- Reject unsupported key types unless the caller selects an explicit mapping.
- Reject nonfinite numbers by default.
- Define integer precision behavior for common external consumers.
- Do not silently round large integers through a floating-point intermediate.
- Require explicit projections for bytes, symbols, tuples, variants, and nominal metadata where JSON cannot preserve identity.
- Keep omitted fields distinct from explicit nulls under the typed schema.
- Do not hide CTF bytes inside base64 and call that a general JSON codec.
- Reuse field validation and structured diagnostic paths across all formats.
- Reuse reflection-derived codecs rather than maintaining three incompatible derivation systems.
- Test JSON round trips on the declared projection domain, not on unsupported arbitrary terms.

## 12. Prove ETF-class usefulness and performance

### Benchmark procedure

1. Pin an Erlang/OTP reference and record its serialization options.
2. Freeze representative workloads and pass criteria before optimization.
3. Compare equivalent logical data and equivalent decode completion.
4. Measure encode, decode, round-trip, encoded size, allocations, peak memory, and copied bytes.
5. Run repeated trials in persistent processes after warm-up.
6. Exclude process startup, shell pipes, and fixture construction from codec throughput measurements.
7. Record hardware, compiler flags, runtime strategy, and the reference version.
8. Report medians, tail latency, variability, and individual workload results.
9. Compare ordinary, canonical, compressed, and scatter/gather paths separately.
10. Publish scripts, fixtures, raw results, and reproducible summaries.

### Required workload coverage

- Include tiny control messages, nested records, mixed maps, tuples, and proper or improper lists.
- Include large binaries, repeated symbols, Unicode text, and arbitrary-precision integers.
- Include compiler-shaped trees and schema-rich application messages.
- Include dynamic term materialization and direct typed decoding as separate comparisons.
- Include boundary sizes, invalid inputs, and configured safety limits.
- Compare scatter/gather behavior with the corresponding ETF API.
- Compare deterministic modes under explicitly stated canonicalization differences.
- Avoid comparisons between a lazy view and a fully materialized result without clear labels.
- Do not use a JSON-shaped subset as proof of general ETF-class utility.
- Target equivalent or better throughput and latency on the principal representative workloads.
- Record statistical tolerances before optimization, not after unfavorable results.
- Keep performance acceptance open when the agreed targets remain unmet.
- Do not waive the target through an unrepresentative aggregate score.
- Report unsupported runtime term kinds separately from portable-data performance.

## 13. Shared verification and adoption

### Required properties

```text
ctf.decode(T, ctf.encode(value)) == value
con.decode(T, con.encode(value)) == value

con.parse(con.encode(term)) == term
ctf.decodeTerm(ctf.encode(term)) == term

ctf.decodeTerm(ctf.encode(con.parse(document))) == con.parse(document)
```

These laws describe successful results after explicit error propagation. They use the specified equivalence for special floats and other non-reflexive values. They do not authorize loss of nominal identities or term kinds.

### Verification requirements

- Add positive, negative, malformed, and cross-version wire vectors.
- Preserve vectors independently from the production encoder.
- Add a small independent reference decoder or external conformance implementation.
- Use the native test framework for Calque conformance and library tests.
- Use generated terms and shrinking to test recursive round trips.
- Compare interpreter, JIT, and AOT results where the target supports them.
- Verify canonical bytes across 32-bit, 64-bit, and established big-endian targets.
- Add mutation tests that prove schema, wire, and assertion verifiers reject defects.
- Test duplicate keys, unknown structs, bad defaults, schema evolution, and projection failures.
- Test fixture ordering, failure reruns, filtering, seeds, timeouts, worker crashes, and stale dependencies.
- Verify that one failed test does not poison later interpreter or native work.
- Keep existing Frankish harness tests and unrelated specimens green.
- Run full self-host verification after compiler syntax and schema changes.

### Required adoption

- Use the new structs in active compiler data types and examples.
- Use the test framework for its own behavior and codec conformance.
- Use CON for real fixtures and at least one checked-in human-edited data file.
- Use CTF for at least one versioned internal data consumer after an explicit migration.
- Keep executable artifacts separate from portable data.
- Add demonstrations for a field typo, a structural assertion failure, and `--failed` reruns.
- Add a CON-to-CTF-to-typed-value demonstration.
- Add a compile-time decode demonstration that embeds a value without runtime parsing.

## 14. Delivery and completion

### Integration order

1. Record the amendment and new acceptance rows.
2. Land struct syntax with negative tests and bootstrap migration.
3. Land the minimal named-test runner and structural assertions.
4. Use that runner for the shared term model and schema tests.
5. Land CON, the safe CTF baseline, and shared JSON derivation.
6. Complete fixtures, selection, isolation, incremental execution, and coverage.
7. Optimize CTF against the frozen ETF comparison suite.
8. Complete format consumers, documentation, demos, and release verification.

### Required artifacts

- Document the struct grammar and migration contract.
- Document the test command, fixtures, assertions, and execution guarantees.
- Publish CON grammar, CTF wire specification, and versioned conformance vectors.
- Document schemas, JSON projections, ownership, and decoder resource policies.
- Add reproducible performance gates and raw benchmark results.
- Wire everything through discoverable `make calque-*` targets and the existing acceptance matrix.
- Preserve all original Calque deliverables.
- Update `STATE.md` before handoff.
- Mark this amendment complete only with executable evidence for every required capability.
- Keep the full active goal unfinished until its remaining requirements also pass.

**Deliver the Elixir feel without sacrificing Calque's types, native representations, shared interpreter, or compiler architecture.**
