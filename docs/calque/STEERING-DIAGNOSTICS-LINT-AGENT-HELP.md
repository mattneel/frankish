# Steering amendment: Diagnostics, agent guidance, and compiler-native linting

## 1. Authority and product contract

Calque needs first-class diagnostic data, Ariadne/Miette-quality presentation, and optional guidance for agents. The built-in linter supplies a coherent quality baseline. It diagnoses code, not its presumed author.

This amendment extends the active goal and earlier amendments. It preserves their remaining requirements.

### Immediate procedure

- Read the current constitution, state, decisions, specifications, and active amendments.
- Inspect actual implementation progress before changes.
- Store this amendment as `docs/calque/STEERING-DIAGNOSTICS-LINT-AGENT-HELP.md` or an equivalent permanent record.
- Add its integrity pin and acceptance rows without replacing historical goal documents.
- Record superseding decisions for any conflicts.
- Preserve the bootstrap chain, self-host contract, current work, and unrelated user changes.
- Deliver implementation and current-state evidence, not only a diagnostic catalog or visual prototype.

## 2. One diagnostic model, multiple presentations

A diagnostic is structured evidence about a specific program snapshot. Terminal text is one projection. Editor actions and agent guidance use the same evidence.

```text
Canonical compiler, runtime, tests, codecs, and quality rules
    -> versioned diagnostic records + source origins
        -> graphical terminal reports
        -> accessible plain text
        -> agent-oriented Markdown
        -> structured JSONL
        -> LSP diagnostics and code actions
        -> documentation and explanation catalog

Verified fix proposals -> existing shared edit-plan service
```

### Architecture requirements

- Define one shared diagnostic schema before independent text printers proliferate.
- Keep Calque-specific detection and explanation policy in the canonical Calque implementation.
- Promote reusable diagnostic transport, source handling, and host adapters into Frankish.
- Treat Ariadne and Miette as design references or replaceable presentation adapters.
- Do not require both libraries or expose their Rust types as Calque's permanent public contract.
- Do not parse pretty terminal output to construct structured diagnostics.
- Support diagnostic sinks without mandatory process-global handlers.
- Keep records independent from mutable interpreter state and MLIR object lifetimes.
- Keep diagnostics demand-driven and cacheable against complete semantic inputs.
- Preserve diagnostic provenance when cached semantic results outlive an edit.
- Keep unrelated specimens compatible through explicit adapters and tests.

### First-class library support

- Expose typed diagnostic, label, source, note, cause, and fix APIs to Calque libraries.
- Provide declarative metadata and reflection-derived registration for ordinary typed errors.
- Preserve concrete `Result` payloads instead of erasing every error into a generic report.
- Materialize expensive excerpts and explanations only when a consumer requests them.
- Support structured diagnostics from user parsers, configuration loaders, and command-line tools.
- Keep success paths free from mandatory source copying and report allocation.
- Keep graphical renderers and compiler services outside application binaries that do not use them.

## 3. Evidence, identity, and source origins

### Required record fields

- Assign stable, namespaced codes to diagnostic kinds and lint rules.
- Separate the kind code from the individual occurrence identity.
- Include schema version, producer version, category, severity, and analysis completeness.
- Include the source snapshot, target, and relevant configuration identity.
- Preserve a short message plus typed arguments for names, types, values, and constraints.
- Include primary and related locations, labeled evidence, and known causal relationships.
- Include documentation topic IDs, possible remedies, and fix-plan references.
- Distinguish established facts from heuristic findings and unresolved information.
- Preserve dependency and origin links through generic specialization, generated code, doctests, and nested `comptime`.
- Retain enough provenance to revalidate snapshot-dependent actions.
- Version the schema and define forward-compatible handling of optional fields.

### Source requirements

- Specify half-open byte ranges against immutable source snapshots.
- Convert ranges explicitly for terminal columns, CLI coordinates, and negotiated LSP encodings.
- Test tabs, combining marks, emoji, CRLF, empty spans, and end-of-file failures.
- Preserve original locations through included Markdown, escaped strings, and embedded specimen source.
- Read excerpts from the diagnosed snapshot rather than the current disk file.
- If source is unavailable, report that limitation instead of showing an unrelated excerpt.
- Keep malformed byte inputs representable without false UTF-8 assumptions.
- Escape terminal controls and bidi hazards without altering machine-readable source offsets.
- Bound report size, cause depth, and excerpt retrieval.
- Include omitted-detail counts and a route to the complete report.

### Failure classification

- Distinguish invalid user programs, configured lint violations, environment failures, resource limits, and compiler defects.
- Do not report a failed internal invariant as an ordinary user type error.
- Do not convert missing analysis or cancellation into a fabricated source defect.
- Attach MLIR and backend failures to available original source origins.
- Preserve unavailable-origin information honestly.
- Keep internal pass details available through an explicit expansion mode.
- Provide local crash reports without automatic uploads or source disclosure.

## 4. Human diagnostics must teach without overwhelming

The default report explains the failure, its evidence, and one useful next step. Additional detail remains available without a browser.

### Presentation requirements

- Support multi-file, multiline, and overlapping labels with clear priority.
- Show both expected and actual types, with the origin of each constraint.
- Explain the relevant pipeline stage rather than only the final return mismatch.
- Show missing match witnesses and the variant declaration that introduced them.
- Show struct field declarations and relevant defaults beside constructor failures.
- Show bounded compile-time traces with the failing input and target context.
- Group known downstream consequences beneath their cause without hiding independent errors.
- Keep causal order and presentation order distinct from discovery order.
- Emit deterministic ordering for identical snapshots and configuration.
- Use neutral, specific language rather than blame, jokes, or assumptions about expertise.
- Support color, monochrome Unicode, ASCII, compact, and narratable presentations.
- Respect `NO_COLOR` and explicit user configuration.
- Never rely on color alone to convey severity or label relationships.
- Prevent graphical gutters from obscuring source in narrow terminals.
- Preserve a bounded emergency reporter when normal diagnostic allocation fails.

### Illustrative target

```text
error[calque.type.unknown_field]: User has no field `scroe`

  src/main.calq
    const user = %User{name: "Matt", scroe: 42};
                                    ^^^^^ unknown field

  src/user.calq
    score: i64 = 0,
    ^^^^^ declared field with a similar name

help: Replace `scroe` with `score` if this is the intended field.
fix:  A precise edit is available. Intent requires review.
more: calque explain calque.type.unknown_field --format markdown
```

The spelling suggestion is not proof of user intent. The final catalog can choose different stable codes before its compatibility freeze.

## 5. Explanations are documentation source

### Catalog requirements

- Define each diagnostic and rule beside its structured metadata and Markdown explanation.
- Generate CLI explanations, LSP documentation links, and reference pages from that catalog.
- Include the violated contract, representative failures, corrected examples, and applicability limits.
- Include counterexamples where a tempting repair changes behavior.
- Compile or execute catalog examples through ordinary doctest and compile-fail infrastructure.
- Verify that negative examples fail for their declared reason.
- Keep documentation and library references pinned to the active toolchain and dependency versions.
- Resolve suggested APIs through the actual symbol graph before presenting concrete edits.
- Do not invent unavailable helpers or copy APIs from newer package documentation.
- Bundle core explanations for offline use outside a project.
- Keep an explanation available even when the project does not parse.
- Preserve retired code explanations and explicit redirects across compatible releases.
- Reject duplicate registrations and package attempts to impersonate core codes.

```console
calque explain calque.type.unknown_field --format markdown
calque help diagnostics/calque.type.unknown_field --format markdown
calque lint --list --format markdown
calque lint --explain calque.lint.discarded_result --format markdown
```

## 6. Optional agent quickfix-help modes

Agent help adds precise context and bounded next steps. It does not change language semantics, diagnostics, severity, permissions, or exit status.

### Command contract

```console
calque check --diagnostic-format human --help-mode brief
calque check --diagnostic-format markdown --help-mode agent
calque lint --diagnostic-format jsonl --help-mode agent
calque ci --diagnostic-format markdown --help-mode agent
calque check --help --format markdown
```

### Mode requirements

- Provide `off`, `brief`, `full`, and `agent` help modes.
- Keep ordinary concise help enabled by default.
- Let `off` remove supplementary help without hiding errors or evidence.
- Require explicit selection of agent mode rather than agent-vendor detection.
- Generate guidance deterministically from compiler facts, rule metadata, and project policy.
- Require no model, API key, network service, or telemetry.
- Keep mode-independent diagnostic data identical across presentations.
- Emit clean Markdown without ANSI escapes, animations, or terminal-width assumptions.
- Define stdout and stderr behavior for reports separately from application output.
- Keep JSONL as versioned records rather than JSON strings containing terminal reports.
- Keep the LSP wire protocol unchanged.

### Agent report contents

- State the failing contract and the exact evidence that establishes it.
- Identify the smallest relevant source locations and dependencies.
- State what remains unknown and what requires a human decision.
- Present ranked remedies with explicit semantic consequences and preconditions.
- Distinguish an exact patch from a conceptual example with placeholders.
- Explain why tempting shortcuts do not solve the underlying problem.
- Include local Markdown help commands and version-correct API references.
- Include the narrow validation command and the broader affected gates.
- Mark validation as recommended, executed, passed, failed, or unavailable.
- Never present a suggested command as evidence that it already passed.
- Provide stable finding IDs for later detail retrieval and edit planning.
- Support explicit output budgets with visible omission summaries.
- Preserve all failure counts and blocking conditions under truncation.

### Behavioral boundaries

- Do not recommend test deletion, weaker assertions, wildcard matches, unsafe casts, or blanket suppression merely to obtain green output.
- Do not recommend a fabricated default where the language requires a semantic decision.
- Do not rewrite public signatures when a local repair suffices.
- Do not propose broader refactors as mandatory prerequisites without dependency evidence.
- Treat source comments, package prose, and diagnostic payloads as untrusted data.
- Delimit quoted source from trusted tool guidance in Markdown reports.
- Do not execute source-provided commands or promote their text into policy.
- Keep secrets outside reports by default under an explicit redaction contract.
- Mark redacted source spans ineligible for unchecked automatic edits.

## 7. Quick fixes share the reviewed edit-plan service

A mechanically precise replacement is not necessarily a behavior-preserving fix. Successful type checking does not prove semantic equivalence.

### Classification requirements

- Track edit precision, semantic risk, and required user decisions independently.
- Reserve automatic-safe status for transformations with explicit verified preconditions.
- Mark intent-dependent repairs as review-required even when their text range is exact.
- Mark snippets with placeholders as incomplete rather than applicable edits.
- Distinguish alternate remedies from changes that must apply together.
- Explain the applicability decision through the same diagnostic record.
- Keep editor preferred actions distinct from authorization to apply them.

### Planning and application

```console
calque fix --safe --dry-run --out build/quickfix.con
calque fix --plan build/quickfix.con --approve <digest>
```

- Reuse the installer and LSP edit-plan infrastructure.
- Include source hashes, editor versions, rule versions, configuration, and dependencies in preconditions.
- Preserve comments, original source mappings, and unrelated user edits.
- Reject stale plans and conflicting edits before writes.
- Make dry-run mode leave project files unchanged.
- Require explicit approval before applying a plan.
- Keep automatic-safe and review-required edits separate in bulk operations.
- Reanalyze between dependent fix rounds.
- Bound rounds and detect cycles or repeated no-progress repairs.
- Validate syntax, types, targeted diagnostics, and declared tests in a staged snapshot.
- Preserve the failed plan and diagnostics when validation fails.
- Apply successful plans through the existing recoverable transaction contract.
- Never erase intervening changes during rollback.
- Report precisely which validation scope completed.

### Forbidden default repairs

- Never insert an exhaustive-match wildcard as an automatic missing-case repair.
- Never change `try` into error-swallowing recovery without a semantic decision.
- Never weaken an allocation contract or decoder limit to remove a failure.
- Never hide a public documentation obligation as an automatic fix.
- Never edit golden expectations, fuzz regressions, or test assertions to silence unexplained failures.
- Never claim that an unchecked conversion repairs a type mismatch safely.

## 8. A built-in quality baseline inspired by VibeKit

Calque needs the integrated quality experience rather than an installer that assembles unrelated language tools. Existing compiler facts supply type analysis. The linter adds rules over those facts and the source model.

### Core requirements

- Ship `calque lint` and `calque ci` with the toolchain.
- Share the canonical parser, semantic queries, docs graph, effects, and source identities.
- Do not add a second approximate parser or duplicate the type checker.
- Keep core rules deterministic, versioned, documented, and available offline.
- Classify findings as correctness, reliability, performance, maintainability, architecture, or documentation.
- Separate compiler errors from configurable lint findings.
- Keep heuristic confidence distinct from severity and project enforcement.
- Provide a high-signal default profile and an explicit stricter profile.
- Keep optional prose and style heuristics outside default correctness failures.
- Show effective policy and the origin of each override.
- Keep lint execution read-only except documented cache artifacts.
- Do not execute tests, installers, arbitrary package hooks, or network requests during lint discovery.
- Keep dependency-provided rules disabled until an explicit trust policy permits them.
- Bound trusted extension execution through existing capability mechanisms.
- Expose typed read-only rule APIs and the same diagnostic/fix contract to extensions.

## 9. Required rule families and honest limits

### Reliability and unnecessary structure

- Diagnose implicitly discarded `Result` values unless the compiler already supplies that diagnostic.
- Recognize documented explicit-discard APIs without duplicate warnings.
- Report error information that a broad recovery branch discards, with legitimate fallback counterexamples.
- Detect provably redundant Boolean branches and identity transformations.
- Preserve effects, iteration order, checked arithmetic, traps, and divergence during rewrite analysis.
- Report obvious placeholder bodies and vacuous assertions only under explicit applicability rules.
- Exclude declared compile-fail fixtures and intentional unfinished examples from misleading reports.
- Identify unnecessary wrappers without treating every API boundary as redundant.
- Avoid duplicate reports when compiler analysis already establishes the same issue.

### Performance and duplication

- Diagnose repeated immutable append or concatenation only when the relevant data structure justifies the cost claim.
- Suggest existing builders or collection operations from resolved library metadata.
- Do not label amortized mutable operations quadratic through syntax alone.
- Report repeated materialization only when it survives relevant optimization or has a source-level consequence.
- Identify structural duplicate candidates through resolved syntax with locations and measurable similarity.
- Preserve meaningful literal and operator differences in clone reports.
- Keep clone detection advisory by default rather than enforcing zero clones universally.
- Never manufacture an abstraction merely to satisfy a duplication score.
- Respect generated code, wire layouts, independent test oracles, and intentionally parallel implementations.

### Architecture and documentation

- Enforce declared module, package, layer, and capability boundaries.
- Explain violations with concrete import or call paths and the policy source.
- Model public exports, FFI entries, tests, reflection, and generated roots in reachability analysis.
- Treat unresolved dynamic reachability conservatively.
- Do not automatically delete code merely because a partial graph lacks callers.
- Check missing required docs, broken symbol links, stale field references, and unverified example status.
- Keep prose-quality heuristics optional and evidence-specific.
- Do not classify code as AI-authored or shame its author.
- Do not penalize the required technical-writing conventions through blanket comment-style rules.
- Provide legitimate counterexamples and local opt-outs for heuristic findings.

## 10. Policy, suppression, and `calque ci`

### Project policy

- Define versioned quality policy through CON and documented defaults.
- Keep the active rule profile reproducible across developer machines and CI.
- Add explicit allow, warn, and deny controls for lint rules.
- Require a reason for scoped suppressions and baseline entries.
- Diagnose unknown, obsolete, and unused suppressions.
- Make suppression scope and precedence visible through CLI Markdown.
- Keep compiler soundness failures outside lint suppression controls.
- Do not auto-create a baseline that conceals current failures.
- Match accepted findings by stable semantic evidence rather than line numbers alone.
- Ensure accepted historical findings cannot hide distinct new defects.
- Require explicit approval before changing quality policy or agent instruction files.
- Never overwrite `AGENTS.md` with generated guidance during an ordinary lint invocation.

### Unified quality gate

- Run formatter verification, compiler analysis, tests, doctests, lint rules, and documentation checks through `calque ci`.
- Include checked-in fuzz regressions without unbounded exploration.
- Include declared architecture and duplication policies.
- Reuse analysis and compiled artifacts within the same source snapshot.
- Execute selected tests rather than trusting cached successful outcomes.
- Do not fetch dependencies or update lockfiles implicitly.
- Report each gate as passed, failed, blocked, skipped, or unavailable.
- Preserve dependency ordering without hiding independent actionable failures.
- Provide fast local and complete release profiles with explicit coverage differences.
- Never label a partial profile as complete release verification.
- Keep agent help optional on the same gate rather than creating a second quality definition.

## 11. LSP, runtime, tests, and codec integration

### Shared consumers

- Project diagnostics into LSP codes, related information, documentation, tags, and code actions.
- Preserve snapshot versions and capability negotiation.
- Generate CLI and editor fixes from identical plans and applicability decisions.
- Map doctest failures to original source documentation rather than generated wrappers.
- Reuse structural value differences for test assertions and codec failures.
- Include CTF byte offsets, CON/JSON source spans, and typed schema paths when available.
- Preserve nested cross-specimen and compile-time source identities.
- Report allocation-contract paths from actual analysis rather than inferred prose.
- Expose dependency-resolution and installer failures through the same core report model.
- Preserve foreign raw messages as labeled detail when a structured adapter lacks information.
- Keep normal runtime errors, language traps, and native process failures distinguishable.
- Avoid duplicate terminal diagnostics when a supervisor already owns the failure report.

## 12. Verification and responsiveness

### Required diagnostic tests

- Add independent tests for the record schema, source mapping, renderers, catalog, and exit statuses.
- Golden-test representative terminal layouts and Markdown reports under pinned presentation configuration.
- Verify structured fields independently from visual snapshots.
- Test multiline, multi-file, generic, embedded-source, and nested compile-time failures.
- Test unavailable source, invalid spans, control characters, redaction, and oversized values.
- Verify that human, agent, JSONL, and LSP projections preserve the same facts.
- Test cause grouping without loss of independent errors.
- Verify that every registered code has offline Markdown help.
- Compile catalog examples and verify their expected diagnostic identities.
- Fuzz record decoding, report formatting, source mapping, and edit-plan validation.
- Preserve a minimal emergency diagnostic after a controlled renderer failure.

### Required rule and fix tests

- Give every default rule positive, negative, and intentional-counterexample fixtures.
- Mutation-test rules and verifiers to prove that deliberate defects fail.
- Test incomplete analysis without false certainty.
- Test all source and configuration dependencies for incremental invalidation.
- Verify safe fixes for evaluation order, effects, errors, and alias preservation where relevant.
- Reject stale, overlapping, placeholder-bearing, and unauthorized edits.
- Test unsaved buffers and concurrent file changes.
- Test repeated-fix convergence and no-progress termination.
- Verify that suggested commands exist in the current CLI and use registered tests.
- Audit legitimate compiler, standard-library, and documentation code for false positives.
- Measure report latency, allocation volume, and incremental lint recomputation on realistic edit traces.
- Keep optional whole-project rules off the interactive LSP path until their budgets permit them.
- Preserve source semantics and all existing specimen tests during host adapter changes.

## 13. End-to-end acceptance

1. Misspell a struct field and inspect the same evidence in the terminal, CLI Markdown, and LSP.
2. Follow its local explanation without a network connection or valid project build.
3. Review a precise spelling repair without an unsupported claim of known intent.
4. Add a union variant and receive concrete missing cases without an automatic wildcard.
5. Trigger a generic pipeline mismatch and locate both conflicting type origins.
6. Trigger a nested `comptime` failure and navigate to the original embedded source.
7. Trigger a real lint violation and retrieve its rationale, counterexample, and narrow validation command.
8. Run an intentional counterexample and verify that policy handles it without false correctness claims.
9. Preview a verified safe fix and reject it after an intervening source edit.
10. Apply an approved fresh plan and inspect actual post-edit validation results.
11. Run `calque ci` with agent help and distinguish completed gates from blocked or unavailable gates.
12. Inject hostile text into source or dependency docs and verify that no command or policy change occurs.

### Delivery requirements

- Publish the diagnostic schema, catalog conventions, rule SDK, applicability taxonomy, and policy reference.
- Document all help and quickfix modes through the built-in documentation system.
- Add discoverable `make calque-diagnostics` and `make calque-lint` gates or equivalent existing targets.
- Integrate evidence with the active acceptance matrix and appropriate CI tiers.
- Migrate Calque's own compiler and tools onto the shared diagnostic system.
- Preserve an explicit compatibility path for existing Frankish diagnostics and bootstrap artifacts.
- Update `STATE.md` before handoff.
- Keep unfinished obligations active across session boundaries.
- Mark this amendment complete only after its current-state verification passes.
- Keep the full goal active while any earlier mandatory work remains.

**Explain the failure. Show the evidence. Offer the smallest justified repair. Prove what the repair actually fixes.**
