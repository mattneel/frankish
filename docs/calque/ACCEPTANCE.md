# calque — Acceptance Matrix

Status: live document, maintained by the Calque workstream. Checker:
`sh scripts/calque-check.sh` (part of `make calque-test`), which
validates structure (unique IDs, valid statuses, evidence paths for
`done` rows) — it does NOT judge prose.

Reading guide. Columns: **ID** (stable), **Requirement** (the goal's
normative item, §-referenced to `GOAL.md`), **Stage** (the milestone
that owns it), **Evidence** (the verifier/command/artifact — for
pending rows this names the PLANNED verifier, which is C0's
"concrete tests and file locations" deliverable), **Status**.

Status values: `done` (evidence exists in-tree NOW and was exercised),
`pending (MN)` (owned by that milestone), `ongoing` (a discipline
that must hold at every milestone exit), `never` (an explicit
non-goal; audited at C6 so nobody "completes" it by accident).

Milestone map: C0=M36 (charter), C1=M37 (interpreted compiler),
C2=M38 (language), C3=M39 (comptime), C4=M40 (native self-host +
performance), C5=M41 (developer experience + portability), C6=M42
(release + demos). Stage exit bars: MANIFEST "Exit bars".

---

## §1 Objective (GOAL §1)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-1.1 | Canonical compiler written in Calque from the first substantial implementation; Frankish interprets it before native compilation; same interpreter supplies comptime | C1/C3/C4 | `make calque-selfhost` (chain + fixed point, BOOTSTRAP §3); sources under `tools/calque/compiler/` | pending (M37) |
| AC-1.2 | Calque stays inside the specimen architecture (not a separate platform) | C0 | MANIFEST; SPEC §8 entry; specimen table row in frk-harness at M37 | done |
| AC-1.3 | Priority order honored: DX > runtime perf > compile/iteration speed; correctness+safety non-negotiable | ongoing | Stage exit bars; C6 audit row AC-23.1; safety fenced in SPEC §6.7/§10 | ongoing |
| AC-1.4 | No claim that Calque is the "first self-hosted language" anywhere | C6 | repo-wide doc audit in the C6 sweep (grep for the claim class) | pending (M42) |

## §2 Repository authority and scope (GOAL §2)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-2.1 | Authorized scope amendments recorded as explicit decisions | C0 | `docs/DECISIONS.md` D-085 (+ D-086/D-087/D-088); SPEC §14 carve-out text; AGENTS.md workstream paragraph | done |
| AC-2.2 | Starting commit + baseline recorded | C0 | this file §Baseline; STATE.md M36 entry | done |
| AC-2.3 | Calque recorded as the user-selected next workstream; unrelated queue preserved | C0 | STATE.md "Next action"/queue section | done |
| AC-2.4 | Milestone numbers allocated from the current ledger, not the prompt | C0 | M36–M42 map (this file, MANIFEST); SPEC §13 amendment | done |
| AC-2.5 | Original-specimen admission (no upstream) + relaxed admission rule for Calque | C0 | D-085(a); MANIFEST "Role" | done |
| AC-2.6 | Self-hosted frontend + reproducible bootstrap permitted and specified | C0/C1 | BOOTSTRAP.md; D-086 | done (specified) |
| AC-2.7 | Editor tools, incremental frontend work, measured performance work permitted | C0 | D-085(c); SPEC §14 carve-out; stages C4/C5 rows | done (authorized) |
| AC-2.8 | Compile-time values, type construction, reflection, generic specialization permitted | C0 | D-085(d); SPEC §8/§9 rows | done (authorized) |
| AC-2.9 | Local modules/packages + lockable manifest permitted; online registry excluded | C0 | D-085(e); §7 rows; non-goal row AC-21.7 | done (authorized) |
| AC-2.10 | Preserved constraints: L1–L3, K1–K7, ABI registry/parity, IRDL-only, no C++ shims, no Oh-My-Pi dependence, no force-push/social, unrelated user changes preserved | ongoing | per-stage rows (AC-4.*, AC-16.*); AGENTS.md paragraph; C6 audit | ongoing |

## §3 Product identity (GOAL §3)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-3.1 | Name Calque, extension `.calq`, command `calque` | C0/C1 | SPEC F1 (frozen); binary name lands at M37 per D-087(2) | done (frozen) |
| AC-3.2 | `calque` is a thin product entry over shared Frankish services | C1 | D-087(2): lib services + thin bin; review at M37 | pending (M37) |
| AC-3.3 | Registered with frnksh via the profile mechanism or its generalized replacement | C1 | D-087(1): minimal dispatcher table + behavior-preservation tests for all existing specimens | pending (M37) |
| AC-3.4 | One compiler pipeline behind both entry points | C1 | frnksh `calque` subcommand over the same `crates/calque` lib (no second pipeline) | pending (M37) |
| AC-3.5 | Unresolved syntax decided by executable examples + ledger; frozen before corpus expansion | C0/ongoing | SPEC §12 freeze log; examples/ README; every new freeze = D-entry + row | done (process), ongoing |

## §4 Calque vs Frankish split (GOAL §4)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-4.1 | Lexer, parser, name resolution, type rules, generic elaboration, Calque-specific transforms, source→kernel emission written in Calque | C1/C2/C3 | `tools/calque/compiler/*.calq`; C1 gate compiles them via the chain | pending (M37) |
| AC-4.2 | Stdlib + compiler's ordinary data structures in Calque | C1/C2 | `tools/calque/std/*.calq`; corpus cases | pending (M38) |
| AC-4.3 | Source-level diagnostics under the same canonical frontend | C1/C5 | SPEC §11 wire format emitted by the compiler; M37 corpus diagnostics compare | pending (M37) |
| AC-4.4 | Same frontend shared by batch compile, REPL, editor tools | C5 | `make calque-dx`; LSP request tests reuse compiler queries | pending (M41) |
| AC-4.5 | Frankish exposes reusable module construction, verification, interpretation, compilation services | C1/C3 | M37 host services (D-087.3); C3 comptime service (D-087.4) | pending (M37) |
| AC-4.6 | Add generic host services, not Calque-specific semantic shortcuts | C1 | host contains no Calque parsing/typing (grep-audited at M37 review); services parameterized | pending (M37) |
| AC-4.7 | Handles typed, scoped, versioned across host boundaries; deterministic failures for invalid handles/requests | C3 | comptime service request/response types; error tests (malformed requests) | pending (M39) |
| AC-4.8 | New runtime functions routed through the ABI registry | C1 | `crates/frk-abi` rows + `make abi`; both twins compile | pending (M37) |
| AC-4.9 | Host-only compiler services separated from portable application runtime | C1/C4 | apps link only their runtime lane; compiler needs the host (documented in BOOTSTRAP §6 + SPEC §10) | pending (M40) |
| AC-4.10 | Calque programs free of implicit dependence on the compiler toolchain | C4 | AOT app run without host services; grid legs | pending (M40) |
| AC-4.11 | Prohibitions: no Rust production frontend (also: no scheduled rewrite), no hidden `compile_calque` intrinsic, no second evaluator, no transpiler/VM, not a wrapper around another specimen | ongoing | stage reviews; C6 audit row; grep for host-side Calque parsing in the M37/M38 reviews | ongoing |

## §5 Bootstrap and self-hosting (GOAL §5)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-5.1 | Required chain shape implemented end to end | C1/C4 | BOOTSTRAP §3 steps 1–8; `make calque-bootstrap`, `make calque-selfhost` | pending (M37) |
| AC-5.2 | Seed = smallest auditable route; existing specimen preferred; justification recorded | C0 | D-086(1); BOOTSTRAP §1 (+ priced alternatives) | done (specified) |
| AC-5.3 | Bootstrap subset frozen in a manifest | C0 | SPEC §12 (b0); MANIFEST "Scope grammar (b0)" | done |
| AC-5.4 | Canonical compiler source in Calque from the first substantial milestone | C1 | M37 gate: build artifacts regenerate from `tools/calque/compiler/` only | pending (M37) |
| AC-5.5 | Seed limited to the subset + emission; frozen after the initial self-host gate | C1 | D-086(1); seed freeze recorded in STATE at M37 exit | pending (M37) |
| AC-5.6 | Every source + recipe to reproduce seed output kept; offline after setup | C1 | `tools/calque/bootstrap/`; `make calque-bootstrap` runs with no network | pending (M37) |
| AC-5.7 | No opaque downloaded compiler binary as the only root | C0 | seed is in-repo Lua source (D-086) | done |
| AC-5.8 | Growth: bootstrap-compatible snapshot preserved; hops recorded; snapshots vs canonical distinguished; seed never learns new syntax | C2+ | `tools/calque/bootstrap/snapshots/<v>/README`; BOOTSTRAP §5 | pending (M38) |
| AC-5.9 | Chain short; trust boundaries documented; trusted host components reported | C0/C4 | BOOTSTRAP §6 (component list); §5 hop discipline | done (documented) |
| AC-5.10 | Fixed-point verification steps 1–9 executed | C1/C4 | BOOTSTRAP §3; `make calque-selfhost` report in `out/calque/` | pending (M37→M40) |
| AC-5.11 | Artifact + canonicalization rules defined; semantics/target/origin preserved; exclusions documented; comparator mutant-verified; source-edit test; new-program test; stale-artifact rejection; bootstrap-evidence framing | C1 | BOOTSTRAP §4; mutant/unit tests with the comparator (L1) | pending (M37) |

## §6 Compiler representation and frontend services (GOAL §6)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-6.1 | Lossless syntax representation (comments + spans), parser recovery + error nodes | C2/C5 | Calque-source CST in the compiler; formatter (C5) preserves comments; incomplete-source tests | pending (M38) |
| AC-6.2 | Stable declaration/source identities; origins preserved through generated code + instantiations | C2/C3 | identity tests across recompiles; locations in artifacts; instantiation-origin tests | pending (M38) |
| AC-6.3 | Typed expression/pattern/effect/generic representations before kernel emission; collection ops for sound optimization; analysis independent of codegen | C2 | compiler pipeline module boundaries; LSP queries reuse analysis | pending (M38) |
| AC-6.4 | Verified kernel IR through a versioned boundary; loanword reuse decision recorded; no raw pointer interchange; no new universal IR | C0/C1 | D-087(5) (loanword NOT used; text boundary + version marker); BOOTSTRAP §2 | done (decided) |
| AC-6.5 | New dialect ops: composition first; semantic (not syntax) promotion; full K1–K7 incl. verifier, tests, docs; source-mapped failures; negative tests; second consumer or language-neutral test | C1+ | D-085(a)+(b); per-op ledger entries; new-op checklist (kernel_lower map from the M17 pattern) | ongoing |
| AC-6.6 | `frk.stage` ops admitted only if the comptime service needs staging semantics, preserving the kernel contract | C3 | D-087(4); C3 design entry before any op lands | pending (M39) |

## §7 Surface language (GOAL §7)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-7.1 | `const`, `fn`, `pub`, records, tagged unions, expression blocks | C0/C1 | SPEC §3–§5 frozen (b0); corpus lands at M37 | done (frozen) |
| AC-7.2 | Explicit local mutation; no implicit shared mutation | C0/C1 | SPEC §6.1 (var + Vector's one aliasing surface); corpus; C2 removes Vector mutability | done (frozen) |
| AC-7.3 | First-class functions, closures, higher-order generic functions | C1/C2 | SPEC §6.4 (b0 closures); generics at C2; corpus cases | pending (M37) |
| AC-7.4 | Lexical scope; predictable shadowing | C0/C1 | SPEC §4 (reserved modules, no same-scope redeclare); shadowing tests | done (frozen) |
| AC-7.5 | Recursion + mutual recursion | C1 | SPEC §6.4; corpus (fib, mutual chains) | pending (M37) |
| AC-7.6 | Tail-call guarantee (direct, mutual, closure); documented | C1 | SPEC F10/§6.4; deep-recursion goldens (500k-class legs join the corpus at M37) | pending (M37) |
| AC-7.7 | No claim that arbitrary recursion is constant-stack | C0/ongoing | SPEC F24 (documented); C6 doc audit | done (documented) |
| AC-7.8 | Destructuring for records, tuples, tagged values | C2 | SPEC §5.2 roadmap; corpus: destructuring cases | pending (M38) |
| AC-7.9 | Optional values, typed error results, concise propagation + explicit recovery | C0/C1 | SPEC §6.5 (`try`/`catch`, Option/Result); corpus incl. trap-vs-error distinction | done (frozen) |
| AC-7.10 | Source-level tests and doc examples that execute | C2/C5 | `test` blocks in Calque + `make calque-test` runs them; doc-example extraction test | pending (M38) |
| AC-7.11 | Numeric conversions + external effects visible | C2/C3 | SPEC §6.9 (no implicit conversions, explicit casts at C2); effect rows | pending (M38) |
| AC-7.12 | Ordinary application code free of allocator/lifetime bookkeeping | C1/ongoing | SPEC §9; review of corpus + demos | ongoing |

## §8 Pipelines and patterns (GOAL §8)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-8.1 | Pipeline (SPEC §6.3 desugar): single evaluation of the piped value, then the call; order/precedence/associativity specified | C0/C1 | SPEC §5.1/§6.3 (frozen); corpus | done (frozen) |
| AC-8.2 | Order preserved through every transform; type errors at the failing stage | C2/C4 | desugar-at-parse (structural, order-preserving); type-error cases at C5 quality | pending (M38) |
| AC-8.3 | Eager collections vs explicit sequences distinguished; map/filter/fold/flat_map/take/zip/collect | C2 | `seq` stdlib in Calque + corpus `seq_*` cases | pending (M38) |
| AC-8.4 | Explicit closures for partial application; no separate pipeline engine | C2 | Desugar-only pipelines (frozen design); closure tests | pending (M38) |
| AC-8.5 | Failures/effects order preserved under fusion; no trap/divergence reordering | C4 | fusion pass tests with trap witnesses (`pipeline_trap_order`) | pending (M40) |
| AC-8.6 | Patterns: literals, wildcards, bindings, nested, tagged variants; tuples/records/or/guards; or-bound consistency; guard order | C1/C2 | b0 subset live at M37 (SPEC §5.2); full set C2; pattern corpus incl. negative cases | pending (M37→M38) |
| AC-8.7 | Arity: guarded arm does not fake exhaustiveness; missing cases reported with witnesses; redundancy policy documented | C0/C1 | SPEC §5.2 (error + witness; redundancy = error); negative tests | done (policy) / pending (M37 impl) |
| AC-8.8 | Reuse/extend decision-tree machinery where appropriate; source origins in branches; tests incl. shadowed bindings + branch-local captures | C1/C2 | match lowering (D-088.4) with locations; corpus | pending (M37) |

## §9 Types, generics, reflection, comptime types (GOAL §9)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-9.1 | Local inference; explicit public signatures; hole diagnostics | C2/C5 | C2 inference design (SPEC F25 due at M38 planning); `pub` always explicit; hole-diagnostic tests | pending (M38) |
| AC-9.2 | bool + fixed-width signed/unsigned ints through 64 bits; target-dependent size types | C2 | integer suite corpus; target-size cases | pending (M38) |
| AC-9.3 | f32/f64 under a documented contract; no default reassociation | C2 | SPEC float clause at M38 (D-entry); float contract tests | pending (M38) |
| AC-9.4 | Str vs Bytes separation | C0/C2 | SPEC §6.8 (Str now); Bytes at C2 | done (Str) / pending (M38) |
| AC-9.5 | Tuples, records, generic tagged unions, recursive data types | C1/C2 | b0 records/unions/recursion live at M37 (D-088.2); tuples/generic unions C2 | pending (M37) |
| AC-9.6 | Multiple instantiations; higher-order generic functions; generic closure captures | C2 | monomorphization corpus (`gen_multi`, `gen_hof`); IR inspection evidence | pending (M38) |
| AC-9.7 | Recursive layouts without infinite inline values | C1 | D-088.2 recref erasure; layout tests | pending (M37) |
| AC-9.8 | Infinite specialization / type-construction cycle detection | C2/C3 | negative tests (`gen_cycle`), loud diagnostics | pending (M38) |
| AC-9.9 | comptime type/const params; conditional type construction | C3 | SPEC §8; corpus (`type_values`, `cond_type`) | pending (M39) |
| AC-9.10 | Reflection over fields/variants/signatures | C3 | compiler-emitted descriptors; reflection tests | pending (M39) |
| AC-9.11 | Generic codec (or equivalent) with round-trip tests | C3/C5 | stdlib codec + property round-trips | pending (M39) |
| AC-9.12 | Type identity deterministic across interpreter/compiler/cache/serialization | C3 | identity tests across paths; cache-key tests | pending (M39) |
| AC-9.13 | No unrestricted trait search / dependent types | never | SPEC §8/§14; C6 non-goal audit | never |
| AC-9.14 | Phases: unresolved syntax vs type values vs runtime values separated; dependency graph scheduling; structured descriptors (no host pointers); Calque-in-Calque rules; no "post-typecheck fold" claim | C3 | D-087(4) service shape; descriptors as data; doc audit | pending (M39) |

## §10 Values, memory, effects, resources (GOAL §10)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-10.1 | RC as the initial managed profile; strategy stays a parameter; other strategies where admitted | C1/C4 | D-088 emission on frk_mem; corpus × {arena,rc} where admitted | pending (M37) |
| AC-10.2 | Immutable vectors/maps/sets with structural sharing; builders; aliases preserved after update | C2 | C2 collection design + corpus (`coll_alias`) | pending (M38) |
| AC-10.3 | Managed views alive with backing storage; eq/hash consistency; deterministic iteration where observable | C2 | collection corpus + property tests | pending (M38) |
| AC-10.4 | Escape analysis / uniqueness reuse where evidence supports; no mandatory borrow annotations | C4 | C4 optimization rows + no-annotation audit | pending (M40) |
| AC-10.5 | retain/release/trace/layout invariants preserved; cycles tested where the representation admits them | C1/C2 | M20/M31/M35 drill pattern reused; rc corpus legs green | pending (M37) |
| AC-10.6 | Allocation-failure behavior explicit and tested | C4 | C4 documented behavior + test | pending (M40) |
| AC-10.7 | Purity by default; private local mutation permitted; effect tracking suffices for enforcement | C3 | C3 effect model (SPEC §6.6); purity-rejection tests | pending (M39) |
| AC-10.8 | Closure effects propagate through HOFs + specialization | C3 | specialization-effect tests | pending (M39) |
| AC-10.9 | Capabilities for file/clock/random; no ambient authority; boundary enforcement | C3/C5 | comptime sandbox tests + capability tests | pending (M39) |
| AC-10.10 | `defer` scoped cleanup on returns AND error propagation; trap/termination behavior documented; no GC-based file cleanup promise; resource escape/closed-resource behavior | C2 | SPEC §6.1 roadmap → C2 entries + tests | pending (M38) |
| AC-10.11 | Safety: overflow/div/shift/bounds/conversion definitions; consistent across interp/JIT/AOT; checks preserved in release unless proven; explicit wrapping ops; no default fp reassociation | C0/C1/C4 | SPEC §6.7 (frozen); D-088.3 checked-arith lowering; safety corpus incl. traps on all runners; C4 no-UB review | done (defined) / pending (M37 impl) |
| AC-10.12 | Advanced user-defined effect handlers off the critical path unless machinery makes them necessary | C6 | audit row; no handler feature in spec | never (unless forced; then D-entry) |

## §11 comptime (GOAL §11)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-11.1 | Reusable Frankish comptime service over the SAME interpreter (no Calque-only evaluator, no second evaluator); missing capabilities added to Frankish | C3 | D-087(4); service tests; review | pending (M39) |
| AC-11.2 | Request/response contract: module+entry+args+target+capabilities+limits → value or source-mapped failure | C3 | service API + tests (incl. malformed requests) | pending (M39) |
| AC-11.3 | Caller state preserved across nested requests; frames/state/allocs/effect channels isolated per request; restored after errors/cancellation/exhaustion | C3 | nested-evaluation tests; isolation tests | pending (M39) |
| AC-11.4 | Registered staging ops only if needed, preserving kernel contract | C3 | AC-6.6 | pending (M39) |
| AC-11.5 | `comptime` exprs/blocks; runtime deps rejected; no silent downgrade; untaken branches preserved | C3 | SPEC §8; corpus (`ct_reject`, `ct_branch`) | pending (M39) |
| AC-11.6 | Compile-time execution of pure library fns; aggregates/recursive values/strings/compiler data structures; nested bounded calls; comptime asserts + custom diagnostics; traces preserved | C3/C5 | comptime corpus; trace tests | pending (M39) |
| AC-11.7 | Target arithmetic defined independently of host; tested vs target runtime results | C3/C5 | 32/64-bit target tests | pending (M39) |
| AC-11.8 | Reification: deterministic frozen values; materialization via verified IR; sharing/cycles policy; refuse pointers/live resources/handles; closure reification only with complete representation; actionable diagnostics; no dangling arena pointers | C3 | reification corpus + negative tests | pending (M39) |
| AC-11.9 | Compile-time-only functions absent from final artifacts | C4 | IR/object inspection evidence | pending (M40) |
| AC-11.10 | Security/reproducibility: deny network/shell/env/clocks/random; file reads ONLY through declared named build inputs (`calque.toml` `[build.inputs]`, D-090) whose content participates in cache validity; boundary enforcement; foreign-effect rejection; step/memory/recursion/nesting budgets; deterministic cancellation + limit diagnostics; JIT-independent | C3 | sandbox tests; budget/cancellation tests; capability-denial test (playbook #10); default-path without JIT | pending (M39) |
| AC-11.11 | Cache/query engine (D-093): resident live query DB (dependency revisions; validation ≠ deserialization) + persistent action index (input-derived keys; lazy restart validation) + content-addressed artifact store (action key ≠ output hash); validity contract = implementation identity (compiler/evaluator/intrinsics/foreign frontends) + program deps + target config + granted inputs + result interpretation; conservative complete-toolchain-identity namespacing first; no failed/partial results; counters | C3 | cache/query tests (hit/miss/invalidate/red-green); counters; AC-15.9 | pending (M39) |

## §12 Compiler-as-library + cross-specimen (GOAL §12)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-12.1 | Typed library interface for source→artifact; in-memory source; filesystem only via caller capabilities; structured diagnostics before rendering; `compiler.compile` stays the convenient public interface with TRACKED QUERIES at the D-093 boundaries internally | C1/C2 | `crates/calque` lib API + tests; query-boundary tests | pending (M37) |
| AC-12.2 | A Calque program invokes the compiler library under the interpreter | C3 | demo/test calling compile on a source value | pending (M39) |
| AC-12.3 | `comptime` compiles a small program, evaluates its artifact, materializes the result (the playbook #2 inception shape: `compiler.compile` + `frankish.eval`, D-090/F31); same limits/target; no special-casing | C3 | demo test + mutant of the embedded program (live edit changes the constant) | pending (M39) |
| AC-12.4 | Profile-based compile-and-evaluate for other specimens; registered frontend/interpreter path; portable value subset defined; per-specimen pure entry points; conformance case per profile; located rejections; deps tracked (embedded source/imports); pinned external frontends | C3 | `cross_specimen` suite (ml/scheme/lua profiles) | pending (M39) |
| AC-12.5 | Foreign-specimen result embedded into a native Calque executable | C3/C6 | Demo E (recorded) | pending (M39/M42) |

## §13 Standard library, modules, application boundaries (GOAL §13)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-13.1 | Text, bytes, numeric, collections, sequence operations; Option/Result helpers composable with pipelines | C2 | stdlib corpus | pending (M38) |
| AC-13.2 | Spans, diagnostics, parsing utilities, deterministic serialization; builders; reflection codec | C2/C3 | stdlib tests + round-trips | pending (M38) |
| AC-13.3 | Paths + scoped file access; CLI parsing; buffered terminal output; test helpers | C2/C5 | stdlib + dx tests | pending (M38) |
| AC-13.4 | Intrinsics narrow + documented; ordinary algorithms in Calque | C0/C1 | intrinsics census in review; algorithms-in-Calque review | ongoing |
| AC-13.5 | Local modules with explicit imports; cycle policy; reproducible resolution independent of cwd; local packages + lockable manifest; `calque.toml` build manifest with named `[build.inputs]` (D-090) | C2/C5 | module/package tests; lockfile determinism test; manifest-input tests | pending (M38) |
| AC-13.6 | No network package discovery | never | non-goal audit | never |
| AC-13.7 | Minimal documented C ABI for scalars/explicit-layout types; managed vs foreign layout separation; ownership rules; raw pointers behind explicit unsafe boundary; unsupported foreign signatures rejected; no FFI during default comptime | C5/C3 | FFI suite + comptime FFI-refusal tests | pending (M41) |

## §14 Developer tools (GOAL §14)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-14.1 | CLI: subcommands check/build/run/test/explain (+ formatting/REPL at C5); `--explain-comptime`, `--runtime-artifact`, `--function` surfaces (D-090/F30); artifact emission + stage inspection; structured output; deterministic exit codes + help; MLIR/LLVM setup hidden; shared impl with the frnksh entry | C1/C5 | `calque --help`; exit-code tests; `emit`; `make calque-dx` | pending (M37/M41) |
| AC-14.2 | Comment-preserving canonical formatter; idempotence + semantic-preservation verified | C5 | `calque fmt` + property tests | pending (M41) |
| AC-14.3 | Language server: diagnostics, hover, completion, definition, references, rename (scope/shadowing aware); partial results for incomplete source; one frontend; cancellation/stale versions; protocol tests; editor integration docs | C5 | `calque lsp` + protocol tests | pending (M41) |
| AC-14.4 | REPL: typed display, multiline, type queries, module load, diagnostics, IR inspection; declaration replacement + state lifetime defined; isolation after failed evals; transcript goldens | C5 | `calque repl` + transcript corpus | pending (M41) |
| AC-14.5 | Diagnostics: original spans; cause + secondary locations; generic/comptime/generated origins; no raw IR-only diagnostics; source-mapped native debug info; one debugger smoke test; distinction documented | C1/C5 | SPEC §11 + rendered form at C5; debugger test | pending (M37→M41) |
| AC-14.6 | Incremental (D-093): red-green — invalidation stops at unchanged observable results; resident live-query warm path with NO per-unchanged-function disk transactions; separate fingerprint classes (signature/body/derived summaries; frozen value vs diagnostic provenance); dynamic dependencies preserve read order; cold and warm times reported separately; counters proving hits + limited recomputation; diagnostics preserved; `calque build --explain-incremental` retrace with dependency classes | C5 | `make calque-bench` incremental section + counters; D-093 | pending (M41) |

## §15 Performance (GOAL §15)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-15.1 | Direct calls preserved; closure env elision; nonescaping closure alloc removal; specialization without dynamic dispatch | C4 | optimization pass tests + IR inspection | pending (M40) |
| AC-15.2 | Record/union layout optimization; efficient exhaustive dispatch; pipeline fusion; intermediate alloc elimination | C4 | layout + fusion tests with allocation counters | pending (M40) |
| AC-15.3 | retain/release elision where proven; uniqueness-based updates; tail calls preserved after optimization | C4 | rc IR diffs + tail goldens post-opt | pending (M40) |
| AC-15.4 | Optimization passes independently testable/inspectable | C4 | per-pass tests; `calque emit` stages | pending (M40) |
| AC-15.5 | Cost report (allocations, captures, unresolved dynamic calls); enforceable no-heap contract incl. reachable calls; reject unresolved targets; explain with source call path; no unproven labels | C4 | cost-report output + contract tests | pending (M40) |
| AC-15.6 | Measurement procedure 1–10 (baseline first, env recorded, startup vs exec, cold vs warm, allocs+memory, repeats+variability, direct-impl comparison, budgets pre-recorded, inputs preserved, regressions investigated) | C4 | `make calque-bench` + PERFORMANCE.md | pending (M40) |
| AC-15.7 | Evidence: alloc-free specialized loop; noncapturing fn without env; bounded-stack tail recursion; frontend reuse; cache hit + dependency-triggered miss; honest reporting | C4/C3 | demo tests + measured outputs | pending (M40) |
| AC-15.8 | No invented targets or unsupported language comparisons | C4/C6 | audit | pending (M40) |
| AC-15.9 | Incremental targets live in the BENCHMARK PLAN (`docs/calque/PERFORMANCE.md`, D-089/D-093): work counts + elapsed, warm and cold separate, the seven-edit matrix, counters (queries/deserialization/interpreter steps/partitions); planned targets (warm body-local tens-of-ms class; medium 50–200 ms; cold ≈ affected-subgraph compile; inception second-run reuse) become claims only after C4 measurement | C4/C5 | `docs/calque/PERFORMANCE.md` + measured report | pending (M40) |

## §16 Verification strategy (GOAL §16)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-16.1 | Source-level semantic examples with independently derived results (no oracle) | C1+ | `goldens/calque/*/expected.out` written from SPEC before running (MANIFEST rule) | pending (M37) |
| AC-16.2 | Negative syntax/type/effect/comptime tests; kernel verifier tests for malformed ops; truncated-artifact/incompatible-version tests | C1+ | negative corpora per layer | pending (M37) |
| AC-16.3 | interp/jit/aot comparison on the corpus; memory strategies where admitted; interpreted vs native compiler parity | C1/C4 | `make calque-test` (diff), `make calque-selfhost` | pending (M37/M40) |
| AC-16.4 | Runtime vs comptime comparison for pure expressions; bootstrap fixed-point tests | C3/C4 | comptime parity tests; fixed-point target | pending (M39/M40) |
| AC-16.5 | Property tests (collections, codecs, substitutions, pattern compilation); typed program generation + malformed input; metamorphic tests | C2/C4 | proptest-style suites in the corpus | pending (M38/M40) |
| AC-16.6 | Independent subset oracles where a faithful transliteration exists | C1+ | `oracle_ml`/`oracle_scheme` transliteration cases | pending (M37) |
| AC-16.7 | No second production interpreter built to satisfy this section | ongoing | review at M37/M39 | ongoing |
| AC-16.8 | Adversarial cases: shadowing/captures; multi-instantiation + recursive generics; guard order + pipeline failures; escaped closures/persistent aliases; layout descriptors vs real fields; cleanup across errors/nested evals; comptime budgets/failed-cache isolation; target ints/serialization; path/symbol/module collisions; truncated artifacts; long-lived process memory; malformed editor requests; red-green boundary edits (value-vs-provenance split; comptime branch read-order changes) | C1–C5 | named cases enter the corpus at the owning stage (`adv_*`) | pending |
| AC-16.9 | Test discipline: verifier with/before each feature; narrow canon; never normalize away real errors; deliberate mutants fail; existing specimen tests preserved; golden changes explained; skipped checks recorded | ongoing | per-commit discipline; C6 audit; M36 baseline green | ongoing |

## §17 Portability and deployment (GOAL §17)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-17.1 | Application portability = the current target grid; existing cross obligations preserved; big-endian canary exercised | C1/C4/C5 | grid legs for Calque corpus | pending (M40) |
| AC-17.2 | Target-sensitive comptime on 32- and 64-bit; WASI without WasmGC | C3/C4 | target tests; wasm32 legs | pending (M39) |
| AC-17.3 | Compiler tested on supported dev hosts; host-only services + optional external tools documented; no build-host absolute paths in generated apps | C4/C5 | doc + grep tests; clean-clone run | pending (M40/M41) |
| AC-17.4 | Required local gates vs scheduled architecture gates separated; exact coverage reported | C5/C6 | PERFORMANCE/ACCEPTANCE coverage table | pending (M41) |

## §18 Delivery sequence (GOAL §18)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-18.1 | C0 exit: concrete tests, file locations, bootstrap route documented | C0 | this file (§3 file map, planned verifiers), BOOTSTRAP, SPEC §12; `make calque-test` | done |
| AC-18.2 | C1 exit: compiler in Calque runs under the interpreter; no host-language production frontend | C1 | `make calque-selfhost` initial gate; AC-4.11 audit | pending (M37) |
| AC-18.3 | C2 exit: functional language + stdlib + modules; compiler self-hosts the grown language; seed frozen | C2 | stage exit review + corpus | pending (M38) |
| AC-18.4 | C3 exit: compiler, programs, and foreign profiles share one bounded evaluation service | C3 | service conformance + cross-specimen suite | pending (M39) |
| AC-18.5 | C4 exit: interpreted and native compilers agree; performance claims have direct evidence | C4 | fixed-point report + PERFORMANCE.md | pending (M40) |
| AC-18.6 | C5 exit: a new user can edit/analyze/test/compile without internal knowledge | C5 | DX dry-run script (`make calque-dx`) | pending (M41) |
| AC-18.7 | C6 exit: every mandatory requirement has direct final-state evidence | C6 | §23 audit table below | pending (M42) |

## §19 Repository artifacts and commands (GOAL §19)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-19.1 | This goal stored in-repo; canonical sources vs historical snapshots distinguishable; matrix current; docs for grammar/eval/effects/memory, comptime phases/targets/sandbox, bootstrap root+recipe, shared improvements+consumers; book/README/dashboard integrated; unsupported vs unfinished kept distinct | C0+ | `docs/calque/GOAL.md` (sha pinned in the checker); SPEC/BOOTSTRAP/ACCEPTANCE; snapshots under tools/calque/bootstrap; book chapter + README at C1/C6 | done (goal record) / ongoing |
| AC-19.2 | Build interface: `make calque-bootstrap`, `calque-test`, `calque-selfhost`, `calque-comptime`, `calque-dx`, `calque-bench`, `calque-demos` + aggregate; reuse equivalent targets where possible; documented target-name differences; reproducible without the coding agent | C1–C6 | Makefile targets land with their stages (calque-test live now); ACCEPTANCE §2 target map | pending (M37+) |

## §20 Demonstrations (GOAL §20)

The storyboard of record is the user's demo playbook
(`docs/calque-demo-playbook.md`; D-090/F28): ten demos, a chosen
opening order (its "What earns the first four clips" table), and
REQUIRED executable proofs — compiler-edit-changes-output; lowered-code
+ allocation verification for fusion; stack instrumentation for the
tail-call clip (a large input alone proves nothing); a live edit that
changes the embedded constant; cache reuse/invalidation on a real
dependency change; capability denial; no invented timing in any clip.
The goal's A–E map onto the playbook as A→#1, B→#5, C→#4, D→#2, E→#3;
its console blocks are release targets, never captured-run substitutes.

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-20.1 | Demo A (compiler is the program; playbook #1): canonical source shown, interpreted run, native build, rebuild, canonical artifact equality, labeled abbreviation | C6 | `demos/` + `.cast` + MP4; `make calque-selfhost` | pending (M42) |
| AC-20.2 | Demo B (functional source, ordinary loop; playbook #5): pipeline over records, deterministic dataset, fused loop + allocation evidence, direct reference comparison, one pipeline type error with location | C6 | `demos/` | pending (M42) |
| AC-20.3 | Demo C (the runtime work disappeared; playbook #4): comptime-constructed table/parser, embedded result, runtime cross-check, absence evidence, real comptime error with call trace | C6 | `demos/` | pending (M42) |
| AC-20.4 | Demo D (Calque compiles Calque during compilation; playbook #2): compiler library in bounded comptime, in-memory program, evaluate artifact, embed result, live-edit proof (6*7→6*9 changes the constant) | C6 | `demos/` | pending (M42) |
| AC-20.5 | Demo E (one interpreter, another specimen; playbook #3 "NOW KITH"): foreign pure program at comptime via `frankish.evalSpecimen`, explicit value boundary, embed in native Calque, rejected effect/boundary value, no interop over-claim | C6 | `demos/` | pending (M42) |
| AC-20.6 | Recording: real terminal sessions, `.cast` + scripts + fixtures + expected outputs, pinned MP4 toolchain, decode-verified, readable without audio, real timings, cuts disclosed, full logs outside clips, no fabrication, no publishing/social | C6 | `demos/verify.sh` (decode + content checks) | pending (M42) |
| AC-20.7 | Playbook demos #6–#10 also ship: exhaustiveness catch + or-pattern repair; reflection codec round-trip with precise runtime error; persistent-map undo; million-tail-call with stack instrumentation; named build input with cache reuse/invalidation + capability denial | C6 | `demos/` (each with its executable proof) | pending (M42) |
| AC-20.8 | Storyboard discipline: the playbook's opening order and captions honored; every terminal claim backed by its own executable test, not a decorative summary | C6 | review at C6 | pending (M42) |
| AC-20.9 | Red-green incremental demo (D-093; the strongest public claim): the inception program edited 6*7→21*2 — nested queries/eval re-execute, the frozen result stays 42, downstream consumers stay valid ("I changed the program that runs during compilation. Its answer stayed the same. The application did not rebuild."); then 6*9→54 invalidates genuinely; `--explain-incremental` retrace recorded with dependency classes separated; real timings only | C6 | `demos/` records | pending (M42) |

## §21 Non-goals (GOAL §21) — audited to NOT be done

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-21.1 | No rewriting Frankish/MLIR/LLVM in Calque | never | audit | never |
| AC-21.2 | No second VM / production interpreter | never | audit; AC-16.7 | never |
| AC-21.3 | No online package registry | never | audit; AC-13.6 | never |
| AC-21.4 | No whole-language laziness / implicit currying | never | SPEC §1/§6.2 | never |
| AC-21.5 | No unrestricted dependent types / general trait search | never | AC-9.13 | never |
| AC-21.6 | No multi-shot continuations / actor runtime / new scheduler | never | audit | never |
| AC-21.7 | No GPU/WasmGC backend for this release | never | audit | never |
| AC-21.8 | No unrestricted runtime interop between specimens | never | AC-12.4 boundary definition | never |
| AC-21.9 | Manual-memory profile not a prerequisite for managed apps | never | audit | never |
| AC-21.10 | No unrelated syntax experiments / branding spend | never | audit | never |

## §22 Work loop and handoff (GOAL §22)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-22.1 | Per-feature procedure 1–10 (verify first, ledger, smallest slice, gates, investigate divergence, inspect artifacts, update docs, commit, STATE) | ongoing | commit discipline; reviews at stage exits | ongoing |
| AC-22.2 | Parallel agents only for bounded tasks with explicit interfaces + gates; tool coordination outside build/test | ongoing | delegation notes in commits | ongoing |
| AC-22.3 | Independent review where practical: state isolation, ABI changes, semantic transforms | C1+ | planel/reviewers per the M32/M33/M35 precedent; isolation review at M37 (interp reuse) | ongoing |
| AC-22.4 | Prerequisite defects fixed; no unrelated refactors; human questions batched; choices via ledger; env blockers evidenced; toolchain failure ≠ pass; tree recoverable (L7) | ongoing | STATE.md discipline | ongoing |

## §23 Final acceptance (GOAL §23)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-23.1 | Audit: registered specimen + versioned manifest; compiler written in Calque; runs under the interpreter; native compiler from same source; auditable bootstrap + fixed point from clean inputs | C6 | audit table in the final evidence report + `make calque-release` | pending (M42) |
| AC-23.2 | Audit: language + stdlib capabilities; shared interpreter for runtime+comptime; comptime types/reflection/reification/isolation/reproducibility; compiler-library in comptime; cross-specimen boundary | C6 | audit + AC-7/9/11/12 rows | pending (M42) |
| AC-23.3 | Audit: DX (diagnostics/formatter/LSP/REPL/incremental); cost reports + allocation contracts + perf evidence; target/memory matrix; existing specimens retain behavior | C6 | audit + AC-14/15/17 rows + baseline-vs-final suite diff | pending (M42) |
| AC-23.4 | Audit: new shared capabilities satisfy kernel/ABI contracts; docs + clean setup + offline reproduction; demos verified; no placeholder implementations or permanent bootstrap fences | C6 | audit | pending (M42) |
| AC-23.5 | Completion: final evidence report (revision, commands, trust root, stage hashes, conformance/portability/DX/perf results, artifact paths, honest limitations, unfinished marked); STATE + matrix updated BEFORE completion | C6 | final report file + updated records | pending (M42) |

---

## File map (planned locations; defaults per GOAL §19)

| Component | Path | Stage |
|---|---|---|
| Specimen law | `specimens/calque/MANIFEST.md` | M36 (exists) |
| Frozen examples | `specimens/calque/examples/` | M36 (exists) |
| Canonical compiler source | `tools/calque/compiler/` | M37+ |
| Calque stdlib | `tools/calque/std/` | M38+ |
| Seed + snapshots + driver recipe | `tools/calque/bootstrap/` | M37+ |
| Calque corpus | `goldens/calque/` | M37+ |
| Calque intrinsics (kernel IR seed) | `crates/calque/src/intrinsics.mlir` (crate-local, `include_str!` per the D-062 pattern) | M37 |
| Host lib + CLI | `crates/calque/` (bin `calque`) | M37 |
| Spec / route / matrix / goal | `docs/calque/{SPEC,BOOTSTRAP,ACCEPTANCE,GOAL}.md` | M36 (exist) |
| Performance and incrementality plan | `docs/calque/PERFORMANCE.md` | M37 (plan opened) → measured at C4 |
| Demos | `docs/calque/DEMOS.md` + `demos/` | M42 |
| Book chapter | `book/src/specimens/calque.md` | M36 (stub) → M42 |
| Charter checker | `scripts/calque-check.sh` | M36 (exists) |

## Make targets (planned; only `calque-test` exists at M36)

| Target | Stage | Purpose |
|---|---|---|
| `calque-test` | M36 | charter checks now; the corpus gate (goldens across runners) from M37 |
| `calque-bootstrap` | M37 | rebuild artifact0 from seed + snapshot, offline |
| `calque-selfhost` | M37 (interp legs) / M40 (native + fixed point) | chain verification + comparisons |
| `calque-comptime` | M39 | comptime + cross-specimen conformance |
| `calque-dx` | M41 | formatter/LSP/REPL/incremental verification |
| `calque-bench` | M40 | reproducible measurements |
| `calque-demos` | M42 | demo verification + artifact production |
| `calque-release` | M42 | aggregate gate (all of the above) |

## Baseline

Starting point of the workstream (recorded at M36, before any Calque
source change):

| Item | Value |
|---|---|
| Starting commit | `791709d` (`[M35] lua: docs — v0.4 coroutines …`); tree otherwise clean, with the incoming goal document untracked |
| Toolchain | pins per `versions.env` (Rust 1.96.0, LLVM/MLIR 22, melior 0.27.2); `sh scripts/check-pins.sh` → ok |
| Environment finding | `tools/loanword-ts` node deps were absent in this checkout; `npm ci` (as `scripts/ci.sh` does) installed them |
| `make test` | green — workspace suite, 554 s wall (gating run 2026-10-11) |
| `make diff` | 162 cases, 0 divergent — diff[interp,jit,jit-rc,ocaml,node,lua,scheme,repl] |
| `make grid` | 157/157 × BOTH strategies × 4 triples (x86_64, aarch64, riscv64, wasm32) |
| `make canary` | s390x 157/157 × BOTH strategies |
| `make calque-test` | green — 28/178 acceptance rows done (charter checks) |
| `make book` | green with the calque chapter |

## How this stays current

- Every stage flips its rows to `done` WITH the evidence path in the
  same commit that lands the evidence (L1 couple).
- New goal-relevant decisions add rows (or amend existing ones) with
  a D-entry; the freeze log (SPEC §12) is the companion record.
- The checker (`scripts/calque-check.sh`) validates: unique IDs,
  known statuses, section coverage (§1–§23 each present), and that
  every `done` row's evidence names an existing in-repo path or a
  make target that exists.
- The C6 audit walks §23 rows against the final tree; unfinished
  rows are reported as unfinished, never silently dropped.
