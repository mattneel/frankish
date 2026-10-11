# Steering amendment: Built-in, first-class Calque language server

## 1. Authority and release requirement

The language server is a mode of the Calque toolchain, not an independent implementation of Calque. LSP exposes the canonical compiler's language services. Editor support is a release requirement, not a later community project.

This amendment extends the active goal and earlier steering contracts. It preserves self-hosting, Elixir-style structs, tests, documentation, packages, CON, CTF, and fuzzing.

### Immediate procedure

- Read the current constitution, state, decisions, specification, and active amendments.
- Inspect the actual implementation before changes.
- Store this amendment as `docs/calque/STEERING-LSP.md` or an equivalent permanent record.
- Add its integrity pin and acceptance rows without replacing historical goal documents.
- Record any superseding decisions explicitly.
- Add recoverable syntax and shared analysis interfaces before batch-only assumptions harden.
- Preserve current bootstrap progress and unrelated changes.
- Complete implementation and executable verification, not only a capability checklist.

## 2. One installation, one semantic engine

```console
calque lsp --stdio
calque lsp --help --format markdown
calque help editor --format markdown
```

### Distribution and architecture

- Ship the language server with every supported Calque development distribution.
- Require no separate server download, Node.js service, or editor-specific compiler.
- Write canonical Calque analysis and language-service policy in Calque.
- Reuse Frankish for generic host services and bounded execution.
- Keep transport, scheduling, and presentation separate from language semantics.
- Share parsing, resolution, inference, effects, source origins, and diagnostics with batch compilation.
- Expose library queries rather than scraping CLI output or spawning compilation for each request.
- Permit frontend startup without native emission, LLVM initialization, or whole-project compilation.
- Keep language-server code outside ordinary application binaries.
- Use one capability schema for server configuration, help, and editor settings.
- Do not require another VM, actor runtime, or production evaluator.

```text
Source snapshots + configuration + dependency identities
    -> canonical incremental compiler queries
    -> language-service API
        -> LSP adapter
        -> CLI semantic queries
        -> editor documentation and actions

The compiler's verified IR path remains a separate consumer.
```

## 3. Correct protocol behavior

### Protocol contract

- Pin the official LSP 3.18 specification and protocol model.
- Preserve interoperability through capability negotiation with older clients.
- Advertise only implemented, tested capabilities.
- Keep a capability-to-test matrix.
- Implement JSON-RPC framing with byte-correct `Content-Length` handling.
- Use the common JSON codec infrastructure without changing LSP's wire representation.
- Never substitute CON or CTF on the LSP connection.
- Reserve stdout exclusively for protocol messages during server operation.
- Send logs and traces to stderr or negotiated protocol channels.
- Handle initialization, shutdown, cancellation, and transport failure under the protocol contract.
- Complete cancelled requests with valid responses rather than orphaning request IDs.
- Bound incoming frames, queues, and outstanding requests.
- Tolerate permitted unknown fields and optional capabilities.
- Reject malformed input without corrupting later valid sessions.
- Negotiate position encodings and support the UTF-16 default.
- Test Unicode-scalar, UTF-8-byte, and UTF-16-code-unit conversions where applicable.
- Implement incremental document synchronization, with valid full-text replacement support.
- Support pull diagnostics for capable clients and a tested push fallback.
- Keep diagnostics associated with the correct document revision.
- Keep Markdown help separate from a live protocol stream.

## 4. Unsaved files and incomplete programs are ordinary inputs

The current editor buffer takes precedence over the file on disk. A request observes one coherent workspace snapshot. A parse error does not erase useful information from the rest of that snapshot.

### Snapshot and query requirements

- Implement a virtual filesystem with explicit editor overlays and disk inputs.
- Apply ordered changes against the correct document version.
- Preserve unsaved changes across disk notifications.
- Define close, save, delete, rename, and external-change behavior.
- Support standalone files, multi-root workspaces, and locked package dependencies.
- Isolate configuration and caches across distinct roots and toolchains.
- Preserve comments, missing tokens, error nodes, and partial declarations in the real syntax tree.
- Return useful local results before complete project analysis finishes.
- Keep unresolved or budget-limited information explicit.
- Never present a guessed type as a verified type.
- Use immutable snapshots and cancellation-aware query execution.
- Prioritize interactive requests over background indexing and optional analysis.
- Stop obsolete work without publishing stale diagnostics or corrupting shared caches.
- Revalidate snapshot-dependent completion resolution and edit proposals.
- Track semantic dependencies separately from source-location and documentation dependencies.
- Stop invalidation when relevant query results remain unchanged.
- Bound retained snapshots, indexes, and artifact memory.
- Report missing uncached dependencies without automatic network access.

## 5. Required language intelligence

### Core features

| Capability | Required behavior |
|---|---|
| Diagnostics | Source errors, related spans, stable codes, and actionable explanations. |
| Completion | Contextual names, imports, struct fields, variants, and callable parameters. |
| Signature help | Active argument, generic parameters, pipeline insertion, and documented effects. |
| Hover | Resolved types, canonical signatures, Markdown docs, defaults, and verified contracts. |
| Navigation | Definition, declaration, type definition, references, and document/workspace symbols. |
| Rename | Scope-aware edits across declarations and actual references. |
| Presentation | Semantic tokens, inlay hints, folding, selection ranges, and document highlights. |
| Formatting | Shared canonical formatter with document and supported range edits. |
| Code actions | Located fixes, import edits, missing cases, and reviewed refactors. |
| Call hierarchy | Known callers and callees with explicit limits for unresolved dynamic calls. |
| Code lenses | Discoverable test, doctest, fuzz-replay, and documentation actions. |

### Semantic quality requirements

- Offer completion inside broken expressions, unfinished struct literals, and partially typed imports.
- Resolve expensive completion details lazily when the client supports it.
- Keep import edits collision-aware and comment-preserving.
- Show pipeline signatures after implicit first-argument insertion.
- Distinguish immutable values, mutable bindings, fields, variants, types, and compile-time parameters.
- Derive semantic tokens and hints from the canonical analysis.
- Respect client support for snippets, Markdown, token deltas, and lazy resolution.
- Keep inlay hints individually configurable.
- Do not invent implementations or callers that static analysis cannot establish.
- Keep syntax-based fallback information visibly distinct where semantic certainty matters.
- Avoid cascades of misleading diagnostics after one incomplete construct.
- Keep diagnostic severity consistent with `calque check` under equivalent inputs and configuration.

## 6. Calque-specific intelligence

### Structs and patterns

- Complete `%User{}` fields from the resolved nominal type.
- Distinguish required fields from fields with defaults.
- Explain missing, duplicate, misspelled, and incorrectly typed fields.
- Support `%User{user | active: false}` without duplicate suggestions for existing updates.
- Navigate fields and variants to their declarations.
- Preserve omitted-field semantics in struct patterns.
- Complete patterns from the actual scrutinee type.
- Suggest missing match arms with concrete witnesses.
- Insert explicit unfinished bodies rather than fabricated business logic.
- Keep guards and or-pattern bindings consistent with compiler semantics.
- Preserve pinning and shadowing during rename.

### Types, effects, and compile-time code

- Show the original generic declaration and useful instantiated information.
- Explain inferred effects and allocation contracts only to their proven extent.
- Expose ready compile-time constants without forced unbounded evaluation.
- Preserve source origins through generated types and specialized code.
- Offer read-only expansion or IR views through negotiated support or a thin editor adapter.
- Provide equivalent CLI inspection when a client lacks virtual-document support.
- Do not make private transport extensions prerequisites for core editing features.

### CON and schemas

- Support `.con` parsing, formatting, navigation, and diagnostics through the shared CON implementation.
- Provide schema-aware keys, defaults, and values when a schema is explicitly associated.
- Keep unknown nominal CON terms valid inert data where the format permits them.
- Do not pretend schema-free documents have known field types.
- Keep CTF binary inspection separate from text-document synchronization.

## 7. Documentation and doctests stay first-class

### Shared documentation services

- Use the documentation graph for hover, completion details, signature help, and links.
- Show resolved dependency documentation for the locked package version.
- Provide local documentation access without a browser or network request.
- Honor negotiated Markdown support with a readable plain-text fallback.
- Sanitize untrusted documentation and disable executable links without explicit approval.
- Complete and resolve symbol references inside `@doc`, `@moduledoc`, and authored guides.
- Analyze Calque examples through the same doctest extraction and setup rules.
- Map errors, navigation, and edits back into the original string or Markdown fence.
- Preserve escaping, indentation, hidden setup, and transcript structure during mapped edits.
- Reject ambiguous mapped edits instead of altering the wrong source location.
- Reuse the Markdown parser and source graph rather than building a parallel embedded-language parser.
- Do not execute doctests to display their documentation.
- Support documented editor routing for Calque fences without replacing an unrelated Markdown language server globally.

## 8. Tests, fuzz targets, and package actions

LSP carries ordinary editor features. Rich test explorers and specialized workflows also need declared commands or negotiated extensions. They must use the existing services rather than duplicate runners.

### Execution and discovery

- Discover tests, describes, fixtures, doctests, fuzz targets, and checked-in regressions through the shared test registry.
- Provide run, rerun, trace, and failure-location actions.
- Require an explicit action before tests, fuzz exploration, or installer execution.
- Use existing output events, seeds, capabilities, timeouts, and supervised workers.
- Keep long test and fuzz work outside the interactive request lane.
- Stop child work on cancellation under the runner's documented cleanup contract.
- Label results with their exact source snapshot and configuration.
- Support unsaved snapshot runs through the shared runner when its effect contract permits them.
- Otherwise, require a save and explain the boundary rather than running different disk contents silently.
- Keep failure replay and minimized-input inspection available without a new exploration campaign.
- Expose test explorers through thin adapters over versioned service contracts.
- Do not claim a custom explorer protocol is standard LSP.

### Package integration

- Read locked package metadata and cached source without dependency execution.
- Show missing dependencies and version conflicts as actionable project diagnostics.
- Reuse installer edit plans for explicitly requested dependency changes.
- Require approval before acquisition, plan application, or expanded capabilities.
- Include editor overlays in installer preconditions.
- Reject stale plans instead of overwriting unsaved edits.
- Never run package hooks to obtain completion, hover, or command help.

## 9. Refactors are reviewed edits, not hidden filesystem writes

### Edit safety

- Return capability-compatible workspace edits rather than modifying open files behind the editor.
- Use versioned document edits where the client supports them.
- Revalidate unopened-file and project preconditions before an action applies.
- Share edit planning with the CLI and installer framework.
- Implement rename preparation and collision detection.
- Preserve comments, aliases, public boundaries, and unrelated same-spelled identifiers.
- Update actual source and documentation references through resolved identities.
- Do not rewrite arbitrary strings, serialized field names, or binary fixtures during symbol rename.
- Warn when a public field or type rename changes CON, CTF, or JSON schema compatibility.
- Keep wire migrations separate from ordinary source refactors.
- Preserve narrow client capabilities without claiming unsupported cross-file atomicity.
- If an edit cannot remain safe, return an explanation rather than a partial destructive refactor.

## 10. Safe analysis of untrusted projects

A hover request does not authorize an arbitrary build. Calque's compile-time execution needs an explicit editor policy because types can depend on evaluation.

### Trust and evaluation

- Default to source analysis, trusted built-ins, and bounded pure evaluation through Frankish's existing service.
- Keep cached results subject to the current target, toolchain, dependency, and capability policy.
- Do not bypass current permissions through a cache hit.
- Require an explicit workspace policy before compile-time reads beyond approved inputs.
- Deny ambient network, shell, secrets, installers, and arbitrary native plugins during automatic analysis.
- Enforce restrictions at the host-service boundary, not only in annotations.
- Bound evaluation steps, memory, recursion, and wall time.
- Make nested compiler evaluation cancellable and isolated.
- If evaluation exceeds its budget, retain useful analysis and report deferred semantic information.
- Do not report deferred analysis as a source type error or invent its result.
- Provide an explicit larger-budget analysis action with visible permissions.
- Keep transport responsive during pathological compile-time programs.
- Sanitize workspace commands and use structured arguments rather than shell concatenation.
- Avoid public network listeners by default.
- Keep logs bounded and free from source contents or secrets unless explicit tracing permits them.

## 11. CLI parity for agents

Agents need the same answers without an editor or a second semantic engine. Markdown remains the required help and report format. The LSP connection itself remains JSON-RPC.

### Intended commands

```console
calque query hover src/user.calq:12:8 --format markdown
calque query definition src/user.calq:12:8 --format markdown
calque query references src/user.calq:12:8 --format markdown
calque query rename src/user.calq:12:8 --to Account --dry-run --format markdown
calque query --help --format markdown
calque lsp --help --format markdown
```

### Requirements

- Expose core read-only semantic queries and reviewed edit plans through the shared library.
- Define CLI line and column units separately from negotiated LSP positions.
- Support explicit input overlays without requiring an already-running editor.
- Include source revision, toolchain, target, and completeness in semantic reports.
- Keep Markdown stable, uncolored, and free from terminal-only layout.
- Return exact locations and explicit ambiguity instead of silently choosing a symbol.
- Keep core help available outside a project, offline, and before server initialization.
- Generate CLI and editor configuration help from the same schema.

## 12. Responsiveness is measured behavior

### Performance contract

- Measure startup-to-initialize separately from startup-to-first-useful-result.
- Measure warm completion, hover, rename, diagnostics, and cancellation latency.
- Record p50, p95, peak memory, queue depth, query executions, and artifact reads.
- Freeze representative fixtures and latency budgets before optimization.
- Benchmark realistic edit traces rather than only unchanged requests.
- Include comment edits, local body edits, signature changes, and same-result compile-time changes.
- Verify limited recomputation for unrelated declarations.
- Keep interactive analysis independent from mandatory native emission or linking.
- Make background indexing incremental, cancellable, and visibly in progress.
- Recover from corrupted caches without losing user buffers.
- Verify bounded memory under long edit sessions and repeated project switches.
- Publish measured results rather than unsupported millisecond promises.

## 13. Verification and editor delivery

### Required gates

- Add `make calque-lsp` with library, wire-protocol, and end-to-end tests.
- Exercise a real subprocess over stdin and stdout.
- Test fragmented frames, malformed JSON, invalid ranges, cancellation, and shutdown races.
- Test capabilities from minimal clients and feature-rich clients.
- Test emoji, combining marks, CRLF, escaped URIs, and supported filesystem path behavior.
- Test unsaved multi-file edits, same-spelled symbols, shadowing, and failed parses.
- Test stale completion resolution, diagnostics, rename plans, and workspace edits.
- Compare clean LSP diagnostics with batch diagnostics for identical snapshots and configuration.
- Verify documentation, CON, tests, fuzz replay, and package-plan integration.
- Fuzz document-event sequences and protocol decoding with bounded resources.
- Compare incremental query results with fresh analysis after generated edit sequences.
- Promote minimized protocol and edit-sequence failures into ordinary regression tests.
- Ship a thin VS Code integration and a documented Neovim configuration.
- Smoke-test both clients against the shipped binary.
- Preserve a usable baseline for other conforming LSP clients.
- Avoid mandatory marketplace access or editor-specific server forks.
- Keep editor versions and capability limitations explicit in evidence.

### Acceptance demonstrations

1. Complete required fields inside an unfinished `%User{}` constructor.
2. Rename a field across source references and receive a wire-compatibility warning.
3. Add a union variant and navigate every affected exhaustive match.
4. Hover a pipeline stage and inspect its effective signature and local Markdown documentation.
5. Break a doctest and navigate to the original documentation line after explicit execution.
6. Replay a fuzz regression from an editor action without another exploration campaign.
7. Edit a compile-time computation and verify cancellation of obsolete analysis.
8. Retrieve dependency definitions and documentation with the network disabled.
9. Inspect the same symbol through LSP and CLI Markdown against an identical snapshot.
10. Recover from malformed source without restarting the server or losing unsaved work.

## 14. Completion

- Publish the protocol capability matrix, trust policy, extension contracts, and Markdown configuration reference.
- Attach executable evidence to every required feature.
- Integrate the language server into release packaging and the acceptance matrix.
- Preserve the bootstrap chain, self-host fixed point, and existing specimen tests.
- Update `STATE.md` with current evidence and the next executable action.
- Keep unfinished requirements explicit across session boundaries.
- Mark this amendment complete only after the delivered server passes its gates.
- Keep the full Calque goal active while any other mandatory work remains.

**One compiler understands Calque. The CLI, editor, documentation system, and agents all ask that compiler.**
