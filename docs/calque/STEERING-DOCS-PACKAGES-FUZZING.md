# Steering amendment: Docs as source, packages, and first-class fuzzing

## 1. Authority and product contract

This amendment extends the active Calque goal and the structs/tests/CON/CTF amendment. It does not replace either contract. Calque remains a self-hosted Frankish specimen.

The requested experience combines source documentation with a built-in documentation toolchain. It includes executable documentation and a searchable book. Every help topic remains available through the CLI as Markdown. Packages combine content-verified acquisition with explicit, source-aware installation. Fuzzing belongs beside ordinary tests, not in a separate developer project.

### Immediate procedure

- Read the current constitution, state, decisions, Calque specification, and active amendments.
- Inspect actual implementation progress before changes.
- Preserve unrelated user changes and ongoing compiler work.
- Store this amendment as `docs/calque/STEERING-DOCS-PACKAGES-FUZZING.md` or an equivalent permanent record.
- Add its integrity pin and acceptance rows without altering historical goal documents.
- Record superseding decisions for conflicting non-goals.
- Authorize a Hex-compatible package client and explicit package publication commands for Calque.
- Keep a new hosted registry service outside this amendment.
- Keep the existing self-hosting, verification, safety, and bootstrap obligations.
- Continue through implementation and executable evidence, not only a revised charter.

## 2. One toolchain, shared source information

Calque owns the canonical language-aware implementation. Frankish provides reusable compiler, interpreter, and host services. Documentation, tests, installers, and fuzz targets share the real frontend. None receives a second parser with approximate semantics.

```text
Calque source + Markdown guides + command declarations
    -> shared syntax, symbols, types, and source origins
    -> documentation graph
        -> API reference and book
        -> CLI Markdown and editor help
        -> doctest registrations

Resolved package + explicit installer
    -> reviewed, typed edit plan
    -> validated project transaction

Ordinary test infrastructure + registered fuzz target
    -> corpus replay and coverage-guided exploration
    -> minimized failure and permanent regression
```

### Implementation requirements

- Write language-aware policy and ordinary libraries in Calque.
- Promote reusable capabilities into Frankish through its existing contracts.
- Bundle required tool functionality with the Calque distribution.
- Do not require separate ExDoc, mdBook, Mix, Node.js, or fuzz-project setup for ordinary workflows.
- Reuse suitable libraries rather than recreating Markdown parsers, cryptography, or fuzz engines without cause.
- Pin bundled dependencies and preserve their licenses.
- Keep documentation and development instrumentation outside ordinary application runtime artifacts.
- Preserve independent artifact keys for documentation, tests, instrumented code, and release code.

## 3. Documentation is source, not a comment scrape

The required authoring forms include `@moduledoc`, `@doc`, and `@typedoc`. The compiler preserves documentation as structured source data. Markdown files are equally first-class documentation inputs.

### Intended source

````calque
@moduledoc """
# Users

Immutable user values and account operations.

See `User` for the data model and `deactivate` for an immutable update.
"""

@typedoc """
An account with a required name and an explicit activation state.
"""
pub const User = struct {
    @doc "The display name."
    name: Str,

    @doc "Whether the account accepts new work."
    active: bool = true,
};

@doc """
Returns an inactive account without changing the original value.

## Examples

```calque
const before = %User{name: "Matt"};
const after = deactivate(before);

assert before.active;
refute after.active;
```
"""
pub fn deactivate(user: User) User {
    %User{user | active: false}
}
````

### Source and semantic requirements

- Support module, function, type, field, variant, and public constant documentation.
- Attach documentation to resolved entities with stable identities and original spans.
- Preserve Markdown, metadata, examples, and author-declared visibility.
- Support version-introduced, deprecation, category, and implementation-status metadata.
- Keep documentation deprecation notices distinct from compiler deprecation diagnostics unless an explicit attribute requests both.
- Define attachment rules across attributes, generated declarations, generic definitions, and re-exports.
- Document generic declarations without duplicating every concrete instantiation.
- Include declared types, defaults, errors, effects, and verified contracts from semantic data.
- Distinguish proven contracts from descriptive prose.
- Support tracked inclusion of local Markdown documents and named source snippets.
- Preserve included-file dependencies and original diagnostic locations.
- Reject ambiguous attachment, duplicate incompatible metadata, and unresolved explicit references.
- Support `@doc false` and `@moduledoc false` for discovery visibility.
- Do not confuse hidden documentation with language privacy or a security boundary.
- Provide configurable checks for missing public documentation and broken references.
- Diagnose stale parameter or field references after API changes.
- Do not require a manually maintained second copy of signatures.

## 4. A built-in API reference and book

`calque docs` builds one navigable publication from API documentation and authored guides. The browser and terminal use the same documentation graph. A generated site is not the only accessible representation.

### Commands

```console
calque docs
calque docs --check
calque docs --serve
calque docs --format markdown --out build/docs-markdown
calque docs --package example_codec --format markdown
calque doc User --format markdown
calque doc User.deactivate --format markdown
calque doc guide:getting-started --format markdown
calque doc --search "immutable update" --format markdown
calque docs --export --format markdown
```

### Required publication features

- Support ordered chapters, nested sections, guides, tutorials, API pages, and release notes.
- Provide a familiar Markdown summary file for authored book navigation.
- Integrate guides and API entries into one search index.
- Resolve symbol references through compiler identities rather than regex substitution.
- Generate stable permalinks, headings, source links, and version-aware dependency references.
- Pin package references to resolved versions rather than whichever release is newest.
- Support explicit redirects when public page identities change.
- Include signatures, fields, variants, defaults, and examples on appropriate API pages.
- Provide responsive layout, keyboard navigation, accessible contrast, light/dark themes, and local search.
- Bundle styles, scripts, and fonts legally without mandatory CDN requests.
- Provide copyable examples and a view of hidden test setup.
- Generate deterministic static HTML and complete Markdown exports.
- Include a Markdown index suitable for agents and an optional `llms.txt` navigation file.
- Do not substitute `llms.txt` for complete retrievable content.
- Sanitize untrusted Markdown and reject unsafe asset paths.
- Keep arbitrary scripts and executable render hooks disabled by default.
- Bind preview servers to loopback unless the user explicitly requests another interface.
- Do not open a browser without an explicit request.
- Keep the site's content usable without a JavaScript-only access path.
- Use this toolchain for Calque's own reference, standard library, and guides.

## 5. Every help topic is CLI-accessible Markdown

Markdown is a supported output contract, not an HTML-to-text fallback. Core help works outside a project and with a broken project. Installed package help remains available without a browser or package execution.

### Required interface

```console
calque --help --format markdown
calque test --help --format markdown
calque install --help --format markdown
calque fuzz --help --format markdown
calque help comptime --format markdown
calque help diagnostics/E1234 --format markdown
calque help --search "decode limits" --format markdown
calque help --index --format markdown
calque help --export --format markdown --out build/help
```

### Accessibility and completeness

- Support `--format markdown` for every command's help, including nested commands.
- Generate command help from the same declarations that define accepted arguments and flags.
- Expose language topics, APIs, guides, diagnostics, package tasks, test behavior, and format specifications.
- Include flag types, defaults, constraints, exit statuses, and working examples.
- Generate actionable alternatives for unknown topics and ambiguous symbol names.
- Use stable topic identities with toolchain, package, version, and source provenance.
- Expose full sections without silent truncation.
- Provide explicit section selection and discoverable pagination for large results.
- Emit UTF-8 Markdown to stdout without ANSI escapes, animations, terminal tables, or pagers.
- Keep diagnostics and progress on stderr.
- Make output independent from terminal width and TTY detection when Markdown is requested.
- Use Markdown for noninteractive help by default.
- Never replace requested Markdown with a browser URL.
- Keep core help bundled and available offline without compilation or LLVM initialization.
- Do not require project validity, dependency resolution, or installer execution to show core help.
- Bundle documentation with Calque package artifacts for offline access after acquisition.
- Report missing uncached dependency documentation explicitly.
- Require explicit network access before retrieval of missing remote documentation.
- Never execute dependency code merely to discover its commands or help text.
- Treat package prose as untrusted content, not instructions that grant tool permissions.
- Test that every registered command, flag, and public topic has retrievable Markdown help.
- Do not advertise nonexistent flags or unavailable platform capabilities.

## 6. Doctests use the real test runner

Examples in source documentation and guides become ordinary registered tests. They share selection, fixtures, isolation, failure reporting, and regression handling with `calque test`.

### Authoring and registration

- Treat Calque code fences as executable examples by default under a documented wrapper and import policy.
- Support REPL transcripts with `calque>` and multiline continuation prompts.
- Keep transcript value expectations distinct from stdout and stderr expectations.
- Support explicit compile-only, expected-error, and non-executable example modes.
- Require a reason for skipped or illustrative Calque examples.
- Report skipped examples rather than counting them as passes.
- Match expected compile errors by stable diagnostic identity and relevant source location.
- Do not accept an unrelated parser failure as proof of the intended type error.
- Support hidden setup lines and tracked snippet inclusion.
- Show hidden setup on demand in HTML and CLI Markdown.
- Keep the tested source identical to the published example plus declared setup.
- Give each example a stable identity and its original source or Markdown span.
- Scope sequential transcript bindings within one example group.
- Isolate separate example groups.

### Execution contract

- Include project doctests in ordinary test discovery without duplicate registration.
- Support source-line selection in both `.calq` and `.md` files.
- Integrate `--failed`, `--stale`, tags, trace output, timeouts, and machine-readable events.
- Provide `calque test --doctest` for explicit documentation-only selection.
- Keep dependency doctests opt-in.
- Use Frankish's interpreter and existing native test paths rather than another evaluator.
- Enforce the test runner's capabilities and resource limits.
- Deny ambient network and unrelated filesystem access by default.
- Permit explicit fixtures for examples that need effects.
- Do not execute code during help lookup or ordinary documentation rendering.
- Make execution explicit through test commands or a documented verification option.
- Track example source, setup, target, dependencies, and capabilities for invalidation.
- Reuse compiled examples, not historical pass results, when tests are selected.
- Label documentation verification with its exact revision and configuration.

## 7. Hex packages with two distinct operations

Calque needs a native package workflow over Hex-compatible repositories. Package transport compatibility does not imply executable compatibility with Erlang or Elixir code. The useful combination is verified acquisition plus explicit project integration.

### Proposed command family

```console
calque fetch hex:example_codec@1.2.3 --save
calque fetch https://example.invalid/example_codec.tar.gz --save
calque fetch git+https://example.invalid/example_codec.git --rev <commit> --save
calque fetch ./vendor/example_codec --save

calque install hex:example_codec@1.2.3 --dry-run
calque install hex:example_codec@1.2.3
calque install --plan build/install-plan.con --approve <digest>

calque deps get --locked
calque deps get --locked --offline
calque deps update example_codec
calque deps tree --format markdown
calque package build
calque package publish --dry-run
```

The package names and addresses above are fixtures, not recommendations or existing Calque packages.

### Acquisition requirements

- Implement Hex repository, registry, checksum, and package-envelope protocols against pinned official specifications.
- Verify signed registry metadata against configured trusted repository keys.
- Verify package checksums and repository/package identities.
- Do not treat a content hash alone as publisher authentication.
- Support configured public, private, and local fixture repositories.
- Preserve repository identity in dependency keys to prevent namespace confusion.
- Support explicit archive, Git, and local-path dependencies.
- Pin Git dependencies to commits in the lockfile.
- Record mutable local paths honestly rather than representing them as immutable remote artifacts.
- Preserve immutable verified downloads in a content-addressed cache.
- Separate download integrity from canonical extracted-tree identity.
- Reject archive traversal, unsafe links, decompression bombs, and path collisions.
- Keep authentication secrets outside manifests, lockfiles, docs, and command output.
- Do not forward repository credentials across untrusted redirects or mirrors.
- Implement useful version-constraint conflict diagnostics.
- Preserve exact locked versions during ordinary builds.
- Make updates explicit and minimal to the requested dependency scope.
- Record compiler compatibility, targets, build features, source identities, and transitive dependency choices.
- Define prerelease, retirement, and optional-dependency policies through conformance tests.
- Support offline reproduction after all locked inputs enter the cache.

### Manifest and language boundary

- Use CON for human-edited package metadata after the existing CON implementation supports the required schema.
- Use a documented lock format with a readable inspection command.
- Preserve comments and user layout during dependency edits.
- Migrate older manifests through an explicit compatibility transition.
- Define Calque package metadata and build-tool identification within the Hex envelope.
- Keep required Hex wire formats unchanged rather than replacing them with CTF.
- Reject unsupported package languages and build tools with specific diagnostics.
- Do not run Mix, Rebar, NIF builds, or arbitrary shell hooks implicitly.
- Treat native foreign-library builds as separate explicit capabilities.
- Do not claim that arbitrary Hex packages run on Frankish.
- Implement package building and publication validation against a local fixture repository.
- Provide real publication only as an explicit authenticated user command.
- Do not publish packages or modify remote accounts during this implementation task.

## 8. Installers produce reviewable source edits

`fetch --save` records a dependency without package-code execution. `install` additionally proposes integration changes. The installer is a bounded Calque program or declarative plan, not an unrestricted shell hook.

### Required installer contract

- Expose typed project-edit operations over the shared source and CON syntax trees.
- Preserve comments, imports, formatting, and unrelated edits.
- Support dependency, configuration, source, test, guide, and example additions.
- Require installers to declare their intended capabilities and parameters.
- Run installer code through a capability-restricted service before any project mutation.
- Give the installer a bounded project snapshot rather than raw host filesystem access.
- Keep network acquisition in the resolver, not the installer evaluator.
- Reject undeclared filesystem, process, network, and credential access.
- If the host cannot enforce isolation, reject the installer rather than trusting an annotation.
- Return a structured edit plan with explanations, dependencies, file preconditions, and validation actions.
- Render the same plan as a human diff, CLI Markdown, and canonical machine-readable data.
- Include package identity, installer identity, inputs, capability grants, and preconditions in the plan digest.
- Make `--dry-run` leave project files unchanged.
- Disclose permitted cache writes during acquisition and planning.
- Require approval before application of project edits.
- In noninteractive operation, require an explicit approval mode rather than a hidden prompt fallback.
- Prefer approval of an exact plan digest for agent workflows.
- Do not let a generic `--yes` grant capabilities absent from the plan.

### Transaction and failure behavior

- Revalidate source hashes and relevant filesystem preconditions before application.
- Reject a stale plan instead of overwriting intervening edits.
- Apply changes through staged writes and a recoverable journal.
- Validate syntax, types, manifest consistency, and configured tests in an isolated worktree or equivalent snapshot.
- Define transaction guarantees without claiming that multiple filesystem writes form one atomic operation.
- Recover cleanly after validation failure, interruption, or process termination.
- Never delete unrelated modifications during rollback.
- Make repeated installation idempotent when package and requested configuration remain unchanged.
- Record managed changes and report conflicts during later upgrades.
- Preserve explicit follow-up actions instead of silently executing them.
- Keep package task metadata and Markdown help declarative and readable without task execution.

## 9. Documentation travels with the package

A fetched Calque package carries the documentation for its exact release. An installer can add configuration without separating users from the documentation that explains it.

### Required integration

- Package the API documentation, guides, command help, schemas, source maps, and Markdown index.
- Support CLI lookup of dependency symbols and installation guides without a browser.
- Keep package documentation associated with the locked version and integrity-verified package identity.
- Record and validate provenance for separately distributed documentation archives.
- Do not assume every external documentation archive has the same integrity guarantees as the source package.
- Provide local source-derived fallback for admitted Calque packages when necessary.
- Never build unknown dependency code automatically just to read its documentation.
- Reuse versioned CTF for structured documentation artifacts once its stable profile exists.
- Keep CON and Markdown representations inspectable without native object-file decoding.
- Do not make basic toolchain help depend on a newly compiled serializer.
- Include dependency documentation in offline help exports on explicit request.
- Preserve license and source attribution through exports.

## 10. Fuzzing is a first-class test mode

Calque needs raw-input fuzz targets and typed property targets. It needs real coverage feedback, reproducible cases, minimization, and regression promotion. Random tests alone do not satisfy this requirement.

### Intended source shape

```calque
fuzz "CTF round trips portable terms", value: Term {
    const encoded = ctf.encode(value)
        |> result.expect("the generated term must encode");
    const decoded = ctf.decodeTerm(encoded)
        |> result.expect("the encoded term must decode");

    assert term.equivalent(decoded, value);
}

fuzz "CTF handles arbitrary input", bytes: Bytes {
    match ctf.decodeTerm(bytes) {
        .Ok(value) => {
            const encoded = ctf.encode(value)
                |> result.expect("a decoded term must encode");
            const decoded = ctf.decodeTerm(encoded)
                |> result.expect("the encoded term must decode");
            assert term.equivalent(decoded, value);
        },
        .Err(_) => {},
    }
}
```

The declaration grammar is an ergonomic target. Freeze its final spelling through the language decision process. Keep generation policy and decoder limits explicit in registered target metadata.

### Commands

```console
calque test
calque test --fuzz --runs 10000 --seed 12345
calque fuzz test/ctf_test.calq --time 60s
calque fuzz --list --format markdown
calque fuzz replay fuzz/regressions/ctf-case.bin --target ctf-decode
calque fuzz minimize fuzz/findings/case.bin --target ctf-decode
calque fuzz promote fuzz/findings/case.bin --target ctf-decode
calque fuzz --help --format markdown
```

### Authoring and execution

- Register fuzz targets through the same discovery and source identity system as ordinary tests.
- Support byte-oriented inputs, typed generators, bounded recursive terms, and state-machine action sequences.
- Allow custom generators, shrinkers, dictionaries, and structured mutators.
- Derive useful generators from admitted schemas without treating arbitrary bytes as valid typed values.
- Keep valid-value generation separate from malformed-wire-input generation.
- Support deterministic corpus replay under ordinary `calque test`.
- Keep ordinary test duration finite and predictable.
- Make new exploration explicit through `calque fuzz` or `calque test --fuzz`.
- Provide finite defaults and explicit limits for runs, time, input size, memory, and recursion.
- Report discarded generated cases and reject vacuous properties with excessive rejection.
- Record exact inputs as the replay authority.
- Do not promise identical parallel exploration from a seed alone.
- Preserve the target, engine, version, build, seed, and environment metadata.
- Deny ambient capabilities unless a target declares deterministic fixtures.
- Keep time, randomness, and external services reproducible through explicit test capabilities.
- Reuse supervised workers for crashes and hard timeouts.
- Do not swallow assertions, memory defects, or unrelated traps as ordinary decoder errors.

## 11. Native coverage, interpreter semantics, and sanitizers

The primary exploration path needs feedback from generated Calque code. Interpreter execution supplies complementary semantic and differential checks. Instrumentation of the host compiler alone does not satisfy application fuzz coverage.

### Backend requirements

- Integrate a maintained or appropriately supported pinned coverage-guided engine behind a shared host interface.
- Reuse an existing engine instead of inventing a mutation scheduler without evidence.
- Instrument generated Calque functions and relevant runtime paths through the actual MLIR/LLVM pipeline.
- Verify executable edge or branch feedback in the final fuzz target.
- Enable comparison feedback where the selected engine and backend support it.
- Demonstrate coverage inside an imported Calque module, not only the harness.
- Make an instrumentation canary fail when required callbacks or counters are absent.
- Keep coverage identities associated with their instrumented build.
- Retain corpus inputs across compiler changes even when coverage metadata needs reconstruction.
- Integrate supported memory-safety instrumentation on the primary native development target.
- Prove actual instrumentation rather than merely linking a sanitizer runtime.
- Distinguish language safety checks, sanitizers, and interpreter errors in reports.
- Document unsupported sanitizer/target combinations explicitly.
- Do not claim that Clang flags automatically instrument code emitted directly through LLVM.
- Keep fuzz instrumentation and its runtime absent from normal release artifacts.
- Provide corpus replay on other admitted targets even when exploration engines differ.

### Correctness checks

- Compare interpreter and native results for admitted generated programs and values.
- Compare relevant memory strategies and target representations.
- Reset test state between cases or record an explicit stateful sequence.
- Include cancellation, allocator reuse, cleanup, and nested evaluator isolation.
- Normalize only documented incidental differences.
- Preserve real differences in errors, failure order, bytes, and resource behavior.
- Use independent wire vectors and semantic oracles where both execution paths share frontend code.
- Do not treat agreement between two paths as a complete correctness proof.

## 12. Corpus lifecycle and actionable failures

A finding becomes useful when another developer can reproduce it without the exploration engine. Minimization must preserve the failure, not merely produce a smaller crashing file.

### Required lifecycle

- Deduplicate inputs by content identity under the target's corpus contract.
- Preserve arbitrary input bytes exactly, including malformed CTF or invalid UTF-8.
- Store reproduction metadata separately from the input bytes.
- Use CON for inspectable manifests and CTF where a stable binary metadata format provides value.
- Do not require the decoder under test to parse its own crash metadata.
- Distinguish regression inputs, exploration corpus, new findings, and generated scratch artifacts.
- Deduplicate failures by meaningful signatures without merging unrelated defects.
- Minimize byte inputs and typed values through appropriate independent strategies.
- Preserve a specific assertion, sanitizer failure, or differential mismatch during minimization.
- Retain the original input and the minimized input.
- Replay minimized failures in fresh processes before classification.
- Report flaky findings explicitly instead of silently discarding them.
- Promote an approved finding into a checked-in regression with a stable identity.
- Require review before intentional changes to tracked regression files.
- Run promoted cases during ordinary tests without another exploration campaign.
- Print the original source location, structural diff, exact rerun command, and metadata path.
- Provide Markdown failure reports suitable for CLI agents.
- Keep corpus growth, merges, and stored findings within explicit disk budgets.

## 13. Required targets and end-to-end evidence

The new features must test each other. Documentation rendering, package installation, and binary decoding all accept complex input. Shared test infrastructure gives them one failure lifecycle.

### Mandatory target families

- Fuzz Calque lexing, parsing, formatter preservation, and recovery.
- Fuzz Markdown parsing, source includes, symbol links, and documentation artifact readers.
- Fuzz CON, CTF, JSON, schema conversion, and codec round trips.
- Fuzz persistent collections and symbol handling.
- Fuzz package envelopes, archive extraction, registry decoders, and version constraints.
- Fuzz installer plans, edit conflicts, idempotence, journal recovery, and stale-plan rejection.
- Fuzz compiler queries, nested compile-time evaluation, and source-origin propagation.
- Exercise bounded malformed inputs without network access or persistent host mutation.

### Acceptance demonstrations

1. Add an API example and find it through HTML, editor hover, and CLI Markdown.
2. Break that example and reproduce its doctest failure at the original Markdown or source line.
3. Edit prose and verify that unrelated runtime machine-code partitions remain valid.
4. Edit an API signature and verify appropriate documentation, link, and doctest invalidation.
5. Fetch a verified fixture package without any installer execution.
6. Preview its installation as a source-aware diff and Markdown plan.
7. Approve the plan and verify its idempotent result.
8. Modify a precondition file and verify rejection of the stale plan.
9. Retrieve the package's exact-version docs with the network disabled.
10. Detect a controlled fuzz defect with generated-code coverage.
11. Minimize the finding and promote it into an ordinary regression test.
12. Verify that the repaired implementation passes the retained input across applicable execution paths.

### Negative gates

- Reject tampered registry signatures, package contents, documentation identities, and installer preconditions.
- Verify that Markdown help never executes a package hook or requires a valid project.
- Verify that documentation lookup does not execute doctests.
- Reject malicious includes, traversal paths, and unauthorized installer capabilities.
- Detect disabled fuzz instrumentation and intentionally broken properties.
- Distinguish unavailable environmental checks from successful tests.
- Preserve the existing compiler bootstrap, self-host fixed point, and specimen matrix.

## 14. Completion and handoff

### Required artifacts

- Publish the documentation source contract, graph schema, doctest rules, and Markdown CLI contract.
- Publish package metadata, lock, trust, installer-plan, and recovery specifications.
- Publish fuzz-target APIs, engine pins, coverage evidence, corpus policy, and reproduction instructions.
- Add `make calque-docs`, `make calque-packages`, and `make calque-fuzz` or equivalent discoverable gates.
- Integrate all new gates with the existing acceptance matrix and appropriate CI tiers.
- Keep short deterministic checks separate from explicitly budgeted exploration campaigns.
- Use signed local registry fixtures rather than public publication for release tests.
- Include generated documentation and help in release packaging.
- Preserve the full original goal and every earlier amendment.
- Update `STATE.md` with current evidence and the next executable action.
- Mark this amendment complete only after its implementation and current-state verification pass.
- Keep the overall goal active while any mandatory deliverable remains unfinished.

**One source of truth: documentation explains the API, examples test it, packages deliver it, and fuzzing attacks its assumptions.**
