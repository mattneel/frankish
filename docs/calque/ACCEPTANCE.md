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
| AC-1.1 | Canonical compiler written in Calque from the first substantial implementation; Frankish interprets it before native compilation; the same interpreter DEFINES comptime semantics (execution may use any conforming runner — JIT default — with byte-exact equivalence, D-110) | C1/C3/C4 | `make calque-selfhost` (chain + fixed point, BOOTSTRAP §3); sources under `tools/calque/compiler/` | done (M37: fixed point green; the D-112 state migration kept every differential byte-identical) |
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
| AC-7.5 | Recursion + mutual recursion | C1 | SPEC §6.4; corpus (fib, mutual chains) | done (M37: `goldens/calque/fib`, `goldens/calque/mutual` green interp/jit/jit-rc/AOT) |
| AC-7.6 | Tail-call guarantee (direct, mutual, closure); documented | C1 | SPEC F10/§6.4; deep-recursion goldens (500k-class legs join the corpus at M37) | done (M37: `goldens/calque/tco` 500k tail calls green under jit/jit-rc/AOT; the reference interpreter's 1024-frame ceiling skips it explicitly, reported) |
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
| AC-8.6 | Patterns: literals, wildcards, bindings, nested, tagged variants; tuples/records/or/guards; or-bound consistency; guard order | C1/C2 | b0 subset live at M37 (SPEC §5.2); full set C2; pattern corpus incl. negative cases | done (b0, M37: corpus `goldens/calque/union_match`/`badge`/`destr` + the hand-asserted error tests in crates/calque/tests/seed.rs; tuples/or/guards remain C2) |
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
| AC-9.9 | comptime type/const params; conditional type construction | C3 | Type parameters D-113 (`calq0_matches_the_seed_on_generics`); value parameters D-144 (`goldens/calque/comptime_values`, `calq0_comptime_value_parameters`); type functions with comptime-conditional bodies D-145 (`goldens/calque/cond_type`, `calq0_conditional_type_construction`, incl. cycle/unbounded-construction refusals); anonymous `struct {}` construction (the goal's `Pair(T)`) remains | pending (M39; type, value, and conditional-selection arms implemented) |
| AC-9.10 | Reflection over fields/variants/signatures | C3 | D-146: `reflect.*` queries over fields, variants, and signatures reified as data, incl. inside generic instances (`goldens/calque/reflect`, `calq0_reflection`); record-valued descriptor lists await aggregate reification | pending (M39; scalar-descriptor arm implemented) |
| AC-9.11 | Generic codec (or equivalent) with round-trip tests | C3/C5 | stdlib codec + property round-trips | pending (M39) |
| AC-9.12 | Type identity deterministic across interpreter/compiler/cache/serialization | C3 | D-144/D-145/D-146: typed instance keys, cached type-application identity, and the canonical `type_key` text are pure functions of the elaborated types; cross-path identity and cache-key tests remain | pending (M39) |
| AC-9.13 | No unrestricted trait search / dependent types | never | SPEC §8/§14; C6 non-goal audit | never |
| AC-9.14 | Phases: unresolved syntax vs type values vs runtime values separated; dependency graph scheduling; structured descriptors (no host pointers); Calque-in-Calque rules; no "post-typecheck fold" claim | C3 | D-087(4) service shape; descriptors as data; doc audit | pending (M39) |

## §10 Values, memory, effects, resources (GOAL §10)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-10.1 | RC as the initial managed profile; strategy stays a parameter; other strategies where admitted | C1/C4 | D-088 emission on frk_mem; corpus × {arena,rc} where admitted | done (M37: `make calque-test` grid leg green 11/11 on both strategies; jit-rc in the diff matrix) |
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
| AC-11.1 | Reusable Frankish comptime service over the SAME interpreter (no Calque-only evaluator, no second evaluator); missing capabilities added to Frankish | C3 | D-133/D-136; `goldens/probe/eval_service` and `goldens/probe/eval_service_bstr`, exercised by `make test`; ordinary and nested evaluation reuse the reference interpreter | done (service; the full request/policy contract remains AC-11.2/11.10) |
| AC-11.2 | Request/response contract: module+entry+args+target+capabilities+limits → value or source-mapped failure | C3 | service API + tests (incl. malformed requests) | pending (M39) |
| AC-11.3 | Caller state preserved across nested requests; frames/state/allocs/effect channels isolated per request; restored after errors/cancellation/exhaustion | C3 | nested-evaluation tests; isolation tests | pending (M39) |
| AC-11.4 | Registered staging ops only if needed, preserving kernel contract | C3 | AC-6.6 | pending (M39) |
| AC-11.5 | `comptime` exprs/blocks; runtime deps rejected; no silent downgrade; untaken branches preserved | C3 | D-134/D-135; `goldens/calque/comptime` and `calq0_comptime_evaluates_and_reifies` prove evaluation/reification and runtime-local refusal; dedicated untaken-branch failure tests remain | pending (M39; explicit forms and runtime-dependency refusal implemented) |
| AC-11.6 | Compile-time execution of pure library fns; aggregates/recursive values/strings/compiler data structures; nested bounded calls; comptime asserts + custom diagnostics; traces preserved | C3/C5 | D-135/D-136/D-139/D-140/D-143; `goldens/calque/comptime`, `goldens/calque/comptime_str`, `goldens/calque/comptime_assert`, `goldens/calque/comptime_nested`, and `calq0_comptime_flushes_lifted_closures`; i64/Str/functions/closures/assertions/expression nesting implemented; D-143 fixed D-139's statement/const nesting defect (`goldens/calque/comptime_nested_stmt`, `calq0_nested_comptime_statements`); aggregates, explicit budgets, and traces remain | pending (M39; partial implementation, not complete) |
| AC-11.7 | Target arithmetic defined independently of host; tested vs target runtime results | C3/C5 | 32/64-bit target tests | pending (M39) |
| AC-11.8 | Reification: deterministic frozen values; materialization via verified IR; sharing/cycles policy; refuse pointers/live resources/handles; closure reification only with complete representation; actionable diagnostics; no dangling arena pointers | C3 | D-135/D-136; `calq0_comptime_evaluates_and_reifies` verifies i64 and Str constants and execution; aggregate/sharing/cycle/resource/closure-result boundaries remain | pending (M39; i64 and Str implemented) |
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

## STEERING AMENDMENT (structs, tests, formats — D-095/D-096/D-097)

Authority: `docs/calque/STEERING-TESTS-STRUCTS-FORMATS.md` (the user's
amendment, stored verbatim; its own requirement set is the source —
these rows are the tracking index). The ORIGINAL goal stays hash-pinned
and untouched; the checker does not weaken its integrity check.

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-S1 | Amendment recorded permanently; goal untouched; integrity pin intact | C1 | this file + `docs/calque/STEERING-TESTS-STRUCTS-FORMATS.md`; `scripts/calque-check.sh` pins both hashes | done |
| AC-S2 | Struct surface: `%Type{f: v}` construction, `%Type{base \| f: v}` immutable update (base once, source-order fields, aliases preserved), pure comptime defaults, required fields, `%Type{f: pat}`/`%Type{}`/`^name` patterns through the shared machinery | C1/C2 | SPEC §3/§5/§5.2 + F35; negative tests (duplicate/unknown/missing/incompatible fields); bootstrap grammar check (no records in seed v0 today) | pending (M38) |
| AC-S3 | Maps `%{k: v}` / `%{"k" => v}` + `:name` symbols distinct from strings and union constructors; nominal-vs-map separation with explicit checked conversion; heterogeneity/inference specified before implementation | C2 | SPEC §3 roadmap; corpus + inference spec (D-entry at freeze) | pending (M38) |
| AC-S4 | Migration: examples/compiler sources/tooling use `%Type{...}`; retired dot-initializer forms never reappear; display uses it (truncated inspection ≠ lossless CON) | C1/C2 | example sources; parser-recovery/formatting/completion/rename/reflection tests at C2/C5 | pending (M38) |
| AC-S5 | `calque test` discovery + selection + statuses + events: `test/**/*_test.calq`, `test_helper.calq`, file/dir/line, name/tag, the §3 flag set, copyable reruns, stable identities, empty-selection reporting, nonzero statuses, machine-readable events | C2/C5 | runner + protocol tests; `--failed` demo | pending (M38) |
| AC-S6 | Declarations/fixtures: `test`/`describe`/`@case`/`@tag`, `setup`/`setup_all`/`on_exit` with selection-aware skipping, precedence rules, reverse cleanup with reported failures, context patterns, doctests, parameterized + seeded property tests | C2/C5 | fixture-order/failure-rerun tests (steering §13 list) | pending (M38) |
| AC-S7 | Assertions: `assert`/`refute` + source/operand capture ONCE; `assert pattern = expr` with binding + structural diffs (first nested path, display budgets); float tolerances; expected errors + expected traps under isolation; no second interpreter | C2 | assertion tests incl. mutation-proof verifiers | pending (M38) |
| AC-S8 | Execution/isolation: serial default, bounded opt-in concurrency, per-case isolation, supervised workers for traps/global state, stopped timeouts, worker crashes as failures, `--trace` serial/finite, recorded seed, interpreter+native corpus with existing gates | C5 (+native C4) | worker-crash/timeout/isolation tests; trace transcripts | pending (M41) |
| AC-S9 | Incremental tests WITHOUT false hits: ordinary runs always execute; `--stale` explicit with conservative dependency coverage (sources/fixtures/helpers/imports/comptime inputs/config/doctests); watch debouncing; reason reporting; separated timings + counters; coverage keyed distinctly and off by default | C5 | stale-selection tests + counters (AC-14.6 machinery) | pending (M41) |
| AC-S10 | One reflection-derived codec contract: typed direct paths + optional dynamic `Term`; buffer/reader/writer/incremental interfaces; shared validation/errors/schema evolution; `json.Codec(T)` facades; comptime specialization; codec policy in Calque; no frk_dyn routing; no I/O in pure ops | C2/C3 | codec tests; API review | pending (M39) |
| AC-S11 | CON: frozen versioned grammar + term mapping, lossless CST, comment-preserving formatting, canonical mode (locale/insertion-order independent), inert unknown nominals, explicit typed-decode schema resolution, parse-recovery diagnostics, editor highlighting | C3/C5 | CON grammar doc + conformance vectors + formatter tests | pending (M39) |
| AC-S12 | CTF: versioned self-describing wire (spec'd before tag freeze), layout/endianness independence, overflow-checked lengths, standalone term decode, bounded in-frame dictionaries, exact/prefix/incremental decoders, framed sequences, opt-in bounded decompression, fast vs canonical profiles | C2/C3 | wire spec + conformance vectors (independent of the encoder) | pending (M39) |
| AC-S13 | Safe decoders by default: resource bounds before allocation, checked arithmetic, strict UTF-8 (bytes arbitrary), malformed-bitstring/dictionary rejection, no deserialization-triggered authority, cancellation/leak tests, every-byte truncation fuzzing; validity ≠ schema validity ≠ authorization | C3 | safety test suite + fuzz vectors | pending (M39) |
| AC-S14 | JSON projection: standards-conforming; explicit mappings for bytes/symbols/tuples/variants/nominals; no silent big-int rounding/nonfinite by default; omitted ≠ null under schema; projection-domain round-trips | C3 | JSON backend tests on the projection domain | pending (M39) |
| AC-S15 | Round-trip laws (§13) + preserved conformance vectors + a small independent reference decoder + generated/shrunk recursive tests + interp/JIT/AOT comparison + canonical bytes across 32/64-bit and big-endian | C3 | `make calque-test` format suite; reference decoder | pending (M39) |
| AC-S16 | ETF-class performance as a benchmarked release objective: pinned OTP reference, frozen workloads/pass criteria BEFORE optimization, medians/tails/variability, separated profiles, workload coverage incl. compiler-shaped trees; targets open until met | C4 | `make calque-bench` + raw results + scripts | pending (M40) |
| AC-S17 | Adoption evidence: structs in compiler data types; self-used test framework; checked-in CON hand-edited fixture; one versioned CTF internal consumer post-migration; demos for field typo, structural assertion failure, `--failed` rerun, CON→CTF→typed, comptime-embedded decode | C6 | demos/ records + committed fixtures | pending (M42) |
| AC-S18 | Targets wired: `calque-test` carries the project-test + format suites as they land; codec benches ride `calque-bench`; documented in this matrix | C2–C6 | Makefile + this file | pending (M38) |

### STEERING AMENDMENT 2 (docs, packages, fuzzing — D-099/D-100/D-101/D-102)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-S19 | Documentation (module/function/type/field/variant/const) attaches to RESOLVED entities with stable identities + original spans; Markdown/metadata/author visibility preserved; includes keep provenance; ambiguous/duplicate/unresolved attachment rejected; `@doc false` = discovery visibility only (never a security boundary); missing-doc + broken/stale-reference checks; signatures/defaults/errors/effects/contracts come from semantic data (no second copy) | C2 | doc-preservation tests; SPEC §13 mapping (D-099) | pending (M38) |
| AC-S20 | `calque docs` builds ONE documentation graph → deterministic static HTML + complete Markdown export + one search index + ordered book/guides/release notes; symbol references resolve through compiler identities; stable permalinks + version-pinned dependency references; bundled assets (no mandatory CDN), sanitized Markdown, no executable render hooks, loopback preview, content usable without JavaScript; agent Markdown index (+ optional `llms.txt`, never a substitute) | C5 | graph/site/book/export tests; byte-identical rebuild check | pending (M41) |
| AC-S21 | Every command's help obtainable as `--format markdown`, generated from the declarations that define its flags; core help offline, project-free, compilation-free; UTF-8, no ANSI/pagers/TTY dependence, diagnostics/progress on stderr; actionable unknown-topic alternatives; nothing advertised that does not exist; help NEVER executes package code or doctests | C5 | CLI help conformance sweep; offline run | pending (M41) |
| AC-S22 | Doctests register through the SHARED test discovery/source-identity system (no second evaluator); Calque fences executable by default under a documented wrapper; REPL transcripts keep value vs stdout/stderr expectations distinct; compile-only/expected-error modes matched by stable diagnostic identity + location; skipped examples require a reason and are REPORTED as skips; hidden setup shown on demand; stable identities + `.calq`/`.md` line selection; group-scoped transcript bindings; `calque test --doctest`; dependency doctests opt-in; invalidation tracks example source/setup/target/caps; reuse = compiled artifacts, never historical pass results | C2 (registration) / C5 (full) | doctest registration/failure/selection tests (D-100) | pending (M38) |
| AC-S23 | `fetch` = verified acquisition WITHOUT package-code execution: Hex registry/envelope/checksum protocols against pinned specs; signed metadata verified against configured trusted keys; checksums AND repository/package identities verified; content hash alone ≠ publisher authentication; traversal/unsafe links/bombs/path collisions rejected; repository identity in dependency keys; git pinned to commits; local paths recorded honestly; content-addressed cache (download integrity ≠ extracted-tree identity); credentials never in manifests/lockfiles/output, never forwarded across untrusted redirects; useful constraint-conflict diagnostics | C4 | fetch tests over signed local fixture repositories; tamper-rejection tests | pending (M40) |
| AC-S24 | `install` = reviewable TYPED edit plan over the shared source/CON trees (comments/formatting preserved); declared capabilities enforced by a capability-restricted service over a bounded snapshot (unenforceable isolation ⇒ rejection); plan digest carries package/installer identities, inputs, grants, preconditions; `--dry-run` leaves files unchanged; stale plans REJECTED; staged writes + recoverable journal; isolated-snapshot validation; idempotent re-install; managed-change tracking + upgrade conflicts; follow-up actions preserved, never auto-executed; `fetch --save` executes nothing | C4 | installer tests over fixture plans (dry-run/stale/rollback/idempotence) | pending (M40) |
| AC-S25 | Package documentation travels with the exact locked release (API docs, guides, command help, schemas, source maps, Markdown index) bound to the integrity-verified identity; offline CLI lookup of dependency symbols; versioned CTF once its stable profile exists; CON/Markdown inspectable without native decoding; core help never depends on a new serializer; unknown dependency code never built just to read docs; offline exports include dependency docs on explicit request with license/attribution | C4/C5 | offline lookup + export fixtures | pending (M40) |
| AC-S26 | Fuzz targets register through the shared discovery/source-identity system (byte + typed/state-machine targets, generator policy + limits in metadata); deterministic corpus replay under ordinary `calque test`; exploration explicit (`calque fuzz`, `calque test --fuzz`) with finite defaults + explicit limits; vacuous properties rejected/reported; maintained pinned engine behind a shared interface; instrumentation through the actual MLIR/LLVM pipeline with an instrumentation canary AND demonstrated coverage in an imported module; comparison feedback where supported; coverage identities bound to the instrumented build | C4 | `make calque-fuzz` quick tier; canary + coverage tests | pending (M40) |
| AC-S27 | Finding lifecycle: exact bytes preserved (malformed/invalid UTF-8 included); content-identity dedup; metadata separate from bytes; regression/exploration/findings/scratch separated; signature-based failure dedup; failure-preserving minimization (original + minimized retained, fresh-process replay before classification); flaky findings reported; reviewed promotion into checked-in regressions that run in ordinary tests; actionable reports (location, diff, exact rerun, metadata path, Markdown); explicit disk budgets | C4/C5 | lifecycle fixture tests; promotion flow | pending (M40) |
| AC-S28 | Mandatory target families exist (lex/parse/formatter recovery; Markdown/docs artifact readers; CON/CTF/JSON/schema codecs; collections/symbols; package envelopes/extraction/registry/constraints; installer plans/conflicts/journal; compiler queries/comptime/origins) under bounded malformed inputs without network or persistent host mutation; the amendment's §13 acceptance demonstrations 1–12 run end-to-end | C4–C6 | target inventory; demo records | pending (M40) |
| AC-S29 | Negative gates: tampered registry signatures/package contents/documentation identities/installer preconditions rejected; Markdown help never executes package hooks and never requires a valid project; documentation lookup never executes doctests; malicious includes/traversal/unauthorized installer capabilities rejected; disabled fuzz instrumentation and intentionally broken properties detected; unavailable environmental checks distinguished from passes; the existing bootstrap/self-host fixed point/specimen matrix preserved | C4–C6 | negative-gate suite; preserved gates | pending (M40) |
| AC-S30 | Gates wired: `make calque-docs` / `make calque-packages` / `make calque-fuzz` (or equivalent) exist, joined to this matrix + CI tiers; short deterministic checks separated from budgeted exploration campaigns; signed local registry fixtures stand in for publication; generated documentation + help ship in release packaging; GOAL + every amendment preserved (pins intact) | C4–C6 | Makefile targets; this file; checker pins | pending (M40) |

### STEERING AMENDMENT 3 (language server — D-103)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-S31 | The language server is a toolchain MODE shipped with every supported distribution (no separate download/Node service/editor-specific compiler); analysis + service policy written in Calque; shared library queries (no CLI scraping, no per-request compilation); startup without native emission/LLVM init/whole-project compile; one capability schema for config/help/editor settings; no new VM/actor runtime; server code outside ordinary binaries | C5 | `calque lsp --stdio`; architecture tests (D-103) | pending (M41) |
| AC-S32 | Protocol: LSP 3.18 pinned; negotiated capabilities only (advertised == implemented == tested; capability-to-test matrix); byte-correct Content-Length framing (JSON codec; NEVER CON/CTF on the wire); stdout protocol-only; cancellation completes valid responses; bounded frames/queues/outstanding; unknown fields tolerated; malformed input cannot corrupt later sessions; position encodings negotiated (UTF-16 default) + conversion tests; incremental sync with full fallback; pull diagnostics + tested push fallback; revision-keyed diagnostics | C5 | wire tests (fragmented frames, malformed JSON, shutdown races, minimal/full clients, Unicode/CRLF/URI) | pending (M41) |
| AC-S33 | Snapshots: virtual filesystem with editor overlays; buffer wins over disk; one coherent snapshot per request; error-tolerant trees preserving comments/missing tokens/partial declarations; local results before full analysis; incompleteness explicit (never a guessed type as verified); immutable snapshots + cancellation-aware queries; interactive priority; obsolete work stopped without stale publication; bounded retained state; uncached dependencies reported without auto-network | C5 | overlay/snapshot/isolation tests | pending (M41) |
| AC-S34 | Intelligence: the §5 capability table implemented with its semantic-quality rules (completion inside broken expressions/literals/partial imports; lazy resolution where supported; collision-aware comment-preserving imports; pipeline signatures after implicit insertion; value/field/variant/type/comptime distinctions; canonical-derived tokens/hints; individual hint configurability; no invented implementations/callers; syntax fallback visibly distinct; severity parity with `calque check`) | C5 | capability-to-test matrix; batch-vs-LSP comparison | pending (M41) |
| AC-S35 | Calque-specific: `%User{}` completion/diagnostics from resolved nominals (required vs defaulted; update without duplicate suggestions); pattern completion + missing-arm witnesses; explicit unfinished bodies; pin/shadow-preserving rename; generics as declaration + useful instantiations; effects/contracts to proven extent only; origins through specialization; read-only expansion/IR via negotiated support (equivalent CLI inspection otherwise); CON shared implementation + schema-aware when explicitly associated (inert unknowns; no pretended schema-free types); CTF separate from text sync | C5 | struct/pattern/CON integration tests | pending (M41) |
| AC-S36 | Docs/doctests: hover/completion/signature/links read the documentation graph (D-099); locked-version dependency docs offline; sanitized untrusted docs, executable links disabled; symbol completion inside doc strings/guides; doctest extraction + setup SHARED with D-100; mapped edits preserve escaping/indentation/hidden setup/transcripts and reject ambiguity; display never executes examples | C5 | doc-graph + mapped-edit tests | pending (M41) |
| AC-S37 | Tests/fuzz/packages: discovery through the shared registry; run/rerun/trace/failure actions with explicit execution; output events/seeds/capabilities/timeouts/supervised workers reused; long work outside the interactive lane with documented cancellation; snapshot+config labels; unsaved-snapshot runs only under the runner's effect contract (else require save and explain); thin adapters (never claimed as standard LSP); locked metadata read without dependency execution; missing deps/conflicts as diagnostics; installer plans with approval + overlay preconditions + stale rejection; no hooks for completion/hover/help | C5 | registry/runner/installer integration tests | pending (M41) |
| AC-S38 | Edits: capability-compatible workspace edits returned (nothing written behind the editor); versioned edits where supported; preconditions revalidated; edit planning shared with CLI + installer; rename preparation + collision detection; comments/aliases/boundaries/unrelated same-spelled identifiers preserved; resolved-identity updates only (no string/serialized-name/binary rewrites); CON/CTF/JSON schema-compat warnings on public renames; no overclaimed cross-file atomicity; explanation over partial destructive refactor | C5 | edit-plan tests (workspace edits, versioned edits, collisions) | pending (M41) |
| AC-S39 | Trust: automatic analysis is bounded PURE evaluation over trusted built-ins + approved inputs through Frankish's existing service (host-boundary enforcement); caches obey current policy (no bypass via cache hit); explicit workspace policy beyond approved inputs; ambient network/shell/secrets/installers/native plugins denied; bounded steps/memory/recursion/time; nested evaluation cancellable + isolated; over-budget retains useful analysis + reports deferred (never invented/as source error); explicit larger-budget action with visible permissions; responsive transport; structured arguments; loopback default; bounded logs without source contents | C5 | trust-policy tests; budget/cancellation tests | pending (M41) |
| AC-S40 | CLI parity: `calque query hover/definition/references/rename` (+ `--dry-run`) over the shared library; Markdown stable/uncolored; CLI line/column units distinct from LSP encodings; input overlays without a running editor; revision/toolchain/target/completeness in reports; exact locations + explicit ambiguity; help from the same schema, offline, pre-init. Performance: measured per §12 (startup-to-initialize vs first-useful-result; warm completion/hover/rename/diagnostics/cancellation p50/p95; peak memory; queue depth; frozen fixtures + budgets BEFORE optimization; realistic edit traces; limited recomputation; bounded memory over long sessions); results published | C5 | `calque query` tests; PERFORMANCE.md measurements | pending (M41) |
| AC-S41 | Gates: `make calque-lsp` carries library + wire + end-to-end tests (real subprocess; the §13 gate list incl. fuzzed document-event sequences under bounded resources and incremental-vs-fresh equivalence; minimized failures promoted to regressions); thin VS Code integration + documented Neovim config smoke-tested against the shipped binary (no marketplace requirement); release packaging; §13 demonstrations 1–10; bootstrap chain/self-host fixed point/specimen matrix preserved | C5/C6 | Makefile target; client smoke records; preserved gates | pending (M41) |

### STEERING AMENDMENT 4 (diagnostics, lint, agent help — D-104)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-S42 | ONE versioned diagnostic schema before independent printers: namespaced kind codes separate from occurrence identity; schema/producer versions, category, severity, completeness, snapshot/target/config identities; typed arguments; labeled locations + causal links; doc topics/remedies/fix refs; established vs heuristic facts; provenance through specialization/generated code/doctests/nested comptime; revalidatable snapshot actions; forward-compatible optionals; Calque policy in Calque; reusable transport/adapters promoted to Frankish; no parsing pretty output into records; sinks without global handlers; records decoupled from interpreter/MLIR lifetimes; demand-driven + cacheable; first-class typed APIs incl. declarative/reflected registration preserving concrete Result payloads; success paths allocation-free | C2 | schema tests + migration notes (D-104) | pending (M38) |
| AC-S43 | Source model: half-open byte ranges over immutable snapshots; explicit terminal/CLI/LSP coordinate conversions; tabs/combining marks/emoji/CRLF/empty spans/EOF tested; original locations through includes/escapes/embedded specimen source; excerpts from the diagnosed snapshot (unavailable source REPORTED); malformed bytes representable; terminal controls/bidi escaped without altering machine offsets; bounded reports with omission counts | C2 | source-mapping tests (§12) | pending (M38) |
| AC-S44 | Failure classification: invalid-program vs lint vs environment vs resource vs COMPILER-DEFECT kinds distinct (no invariant failures as user type errors; no fabricated defects from missing analysis/cancellation); MLIR/backend failures attached to origins when available, honest when not; internal detail behind explicit expansion; local crash reports (no uploads/source disclosure) | C2 | classification tests | pending (M38) |
| AC-S45 | Human presentation: failure + evidence + one next step by default; multi-file/multiline/overlapping labels; expected vs actual with constraint origins; offending pipeline stage; missing witnesses + introducing declaration; struct decls/defaults beside constructor failures; bounded comptime traces; cause grouping without hiding independents; deterministic ordering; neutral language; color/mono/ASCII/compact/narratable honoring NO_COLOR + config; never color-alone; narrow-terminal safe; bounded emergency reporter | C5 | golden layouts under pinned config (§12) | pending (M41) |
| AC-S46 | Catalog-as-documentation: every code's Markdown explanation beside its metadata; CLI `calque explain` + `help diagnostics/<code>` + LSP links + reference pages generated from it; contract + representative failures + corrected examples + limits + tempting-repair counterexamples; examples ride doctest/compile-fail infra (negative examples fail for their DECLARED reason); suggested APIs resolved through the symbol graph; offline core explanations (work with an unparseable project); retired codes keep explanations + redirects; duplicate/impersonation rejected | C5 | catalog tests; doctest-connected examples | pending (M41) |
| AC-S47 | Agent help modes: `off|brief|full|agent` on check/lint/ci; concise default; `off` hides no errors/evidence; agent mode explicit (no vendor detection); deterministic from compiler facts + rule metadata + policy; no model/network/telemetry; identical diagnostic data across modes; clean Markdown (no ANSI/width assumptions); JSONL = versioned records; LSP wire unchanged; reports carry contract/evidence, smallest locations/deps, explicit unknowns/human decisions, ranked remedies with consequences/preconditions, exact-vs-placeholder, why shortcuts fail, local help + version-correct refs, narrow + broad validation commands with honest status labels, stable finding IDs, budgets with visible omission summaries preserving counts | C5 | mode conformance tests; projection-equality tests | pending (M41) |
| AC-S48 | Behavioral boundaries: never recommend test deletion/weaker assertions/wildcards/unsafe casts/blanket suppression for green; no fabricated defaults; no public-signature rewrites for local repairs; no mandatory broad refactors without dependency evidence; source comments/package prose/diagnostic payloads untrusted (delimited, never executed/promoted to policy); secrets out by default with an explicit redaction contract; redacted spans ineligible for unchecked edits | C5 | negative-gate tests | pending (M41) |
| AC-S49 | Fixes: precision/semantic-risk/user-decision tracked independently; automatic-safe only with verified preconditions; intent-dependent repairs review-required; placeholder snippets incomplete; alternates vs must-apply-together distinct; `calque fix --safe --dry-run` + plan digest approval over the SHARED edit-plan service (preconditions incl. source hashes/editor/rule versions/config/deps; comments + unrelated edits preserved; stale/conflicting rejected; dry-run untouched; safe vs review split in bulk; bounded rounds with cycle/no-progress detection; staged-snapshot validation of syntax/types/targeted diagnostics/declared tests; failed plans preserved; journaled transactions; precise validation-scope reporting); forbidden defaults (wildcards-as-repair, try-swallowing, weakened contracts/limits, hidden doc obligations, golden/fuzz/test silencing, unchecked-conversion claims) | C5 | fix-plan tests (§12) | pending (M41) |
| AC-S50 | Lint/CI: `calque lint` + `calque ci` ship with the toolchain over canonical analysis (no second parser/checker); deterministic/versioned/documented/offline core rules; categories; compiler errors vs findings separate; confidence ≠ severity ≠ enforcement; default + stricter profiles; prose/style outside defaults; effective policy + override origins; read-only discovery; dependency rules trust-gated through capabilities; typed rule APIs; `calque ci` runs formatter + analysis + tests + doctests + lint + docs + checked-in fuzz regressions + declared architecture/duplication policies over one snapshot (executed, not cache-trusted); no implicit fetch/lockfile updates; gate statuses + dependency ordering; fast-local vs complete-release profiles explicitly distinct; agent help optional on the same gate; policy/suppression rules incl. reasoned scopes, diagnosed stale/unknown suppressions, soundness outside suppression, no concealing auto-baselines, semantic-evidence matching, approval before policy/AGENTS.md changes | C5 | lint/ci tests; policy fixtures | pending (M41) |
| AC-S51 | Rule families (§9): discarded Results (deduped vs compiler; discard APIs recognized); broad-recovery error loss with counterexamples; provably redundant branches/identity transforms with effects/order/traps/divergence preserved; placeholder/vacuous reports only under explicit applicability (fixtures excluded); unnecessary wrappers; no duplicate reports; repeated-append/duplication claims only with structural justification + resolved-library suggestions; advisory clone detection with locations/similarity/literal differences; architecture boundaries with concrete paths + policy source; conservative reachability (never auto-delete); docs checks; optional evidence-specific prose heuristics; never author-shaming; counterexamples + local opt-outs | C5 | per-rule fixtures + mutation tests | pending (M41) |
| AC-S52 | Integration + verification: LSP projects identical records (codes/related/docs/tags/actions; snapshot + negotiation); CLI/editor fixes from identical plans; doctest failures map to original docs; value-difference reuse (tests/codecs); CTF offsets + CON/JSON spans/schema paths; nested cross-specimen/comptime identities; allocation paths from real analysis; package/installer failures in-model; foreign messages labeled; runtime vs trap vs native distinguishable; no duplicate terminal reports; §12 suites (schema/renderers/catalog/exit statuses; structured-vs-visual independence; projections preserve facts; offline help for every code; fuzzed decoding/formatting/mapping/plan validation; emergency reporter; incremental invalidation; convergence; unsaved buffers; measured latency/allocation; LSP path budgets); migration of Calque's own tools onto the system + compatibility path for existing Frankish diagnostics; `make calque-diagnostics`/`make calque-lint` gates; §13 demos 1–12 | C2–C6 | gate targets; migration notes; demo records | pending (M38) |

### STEERING AMENDMENT 5 (structured concurrency + modern CPUs — D-105)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-S53 | CONTRACTS FROZEN BEFORE SURFACE HARDENING (ownership/lifetime, transfer/share, cancellation, memory model, target-layout — D-105): scopes own children to completion incl. cleanup; dropped handles never detach; no unowned background tasks in safe code (explicit application root + shutdown protocol for services); handles/scope values cannot escape; single execution + single result consumption; compiler-known transfer vs share vs local-only properties derived from representation + capabilities; exclusive transfer leaves no residual owner; mutable-box capture across tasks rejected without explicit sync (including same-OS-thread sharing); local-only tasks under an explicit non-migrating executor; unsafe boundary explicit only; diagnostics explain value/operation/conflict and never default to unsafe annotations | C1/C2 | SPEC/source docs + conformance tests as they land (D-105) | pending (M38) |
| AC-S54 | Async surface: `async fn`/async closures/`await`/structured scopes as specified; cold operations (argument evaluation preserved at creation; started only by await/explicit scoped spawn; registration complete before publication); cold vs running-handle vs completed-result distinguished in types + diagnostics; discarded cold operations diagnosed; `try await` respects typed Result rules with explicit cross-error adaptation; suspension requirements tracked through higher-order/generic code; synchronous callers get an explicit root bridge (never a hidden block-on); a single-thread executor exists for tests/constrained targets and is never described as multicore | C2 | surface tests + executable specification before signature freeze (the amendment's §3) | pending (M38) |
| AC-S55 | Scope semantics: join-all on normal exit; early failure/cancellation requests child cancellation + drains before releasing shared resources; fail-fast sibling cancellation under the default policy with explicit collect-results; task failure vs a successful result containing a domain error frozen in API types + conformance; observed non-cancellation failures retained (not first-error-only) with task/spawn-site provenance in stable order (no schedule-independent-discovery claims); fallible scope/await result types before public adoption; cancellation never conflated with recovery; native memory corruption never surfaces as a recoverable task error | C2/C4 | scope/error conformance suite | pending (M38) |
| AC-S56 | Cancellation/deadlines/cleanup: tree propagation; nested deadlines + scoped cancellation tokens; monotonic runtime time vs virtual test time; checkpoints at documented suspension points + explicit cooperative checks for long CPU loops (compiler-inserted policy documented, no arbitrary-preemption claims); exactly one terminal outcome (pre-start/during-suspension/concurrent-with-completion behaviors defined); broad `catch` cannot silently consume cancellation; required cleanup on cancellation and error; child-resource vs drained-parent cleanup separated; bounded shielding for suspending cleanup; cleanup failures reported without losing the initiator; I/O buffers/handles retained until OS/adapter completion or safe abandonment; no arbitrary thread killing; non-cooperative foreign work keeps the scope alive or uses process isolation; request latency reported separately from shutdown latency | C2/C4 | cancellation/cleanup suite (§19) | pending (M38) |
| AC-S57 | Synchronization/communication: bounded typed channels with backpressure/closure/cancellation (per-sender + concurrent-send ordering defined); async-safe waits without blocked cooperative workers; mutexes/scoped guards/semaphores/events/atomic values; suspension under an OS mutex guard rejected (async-aware locks distinguished); references cannot escape protected access; atomic widths + orders per target with SC default, explicit weaker orders, invalid combinations rejected, verified LLVM mapping, honest lock-free reporting, no volatile-as-sync; `select`/race with declared fairness + loser ownership; commit-point semantics for send/receive/I/O cancellation; ownership of unsent values preserved; no silently lost received messages; live tasks/locks/channels/handles excluded from CON/CTF | C2/C4 | sync/comm conformance + litmus tests | pending (M38) |
| AC-S58 | Executors/I/O: bounded native worker pools with real simultaneous OS-thread execution; cooperative vs blocking work separated; scoped dedicated threads (entry/join/failure/cleanup defined); configurable worker counts within host limits; parallelism budgets shared across nested operations (no per-call pools, no parent-child exhaustion deadlock); bounded pending work with admission/backpressure; locality policy chosen from contention measurements (local-only affinity preserved under every policy); task-local context inheritance + restore; completion-publication/consumption synchronization; registration/wakeup/duplicate-wake/saturation/shutdown-with-pending-I/O tested; awaitable timers + capability-based sockets on supported hosts; async file ops with explicit implementation details; a maintained I/O substrate behind a generic interface (never one blocked thread per socket wait); DNS/blocking calls through a bounded adapter; readiness vs completion distinguished; partial read/write/interruption/EOS preserved; late-completion-safe cancellation cleanup; local network fixture; unsupported capabilities diagnosed | C4/C5 | executor/I-O suites + local fixture | pending (M40) |
| AC-S59 | SIMD/data/pipeline: a DISTINCT SIMD type family (fixed lanes, comparisons, masks, selections, shuffles, loads/stores/reductions; lane order + numeric behavior independent of physical width; masked tails never OOB/poison; unaligned valid + explicitly checked aligned ops; gather/scatter bounds + duplicate destinations defined; scalarization on ISA-lacking targets; scalable vectors behind an explicit capability; no optional-ISA baseline requirement); scalar per-lane reference semantics in the interpreter/comptime; strict numerics by default with explicit verified policies for reassociation/contraction/approximations (no global fast-math; honest miss reports); empty/short/partial/misaligned/width-boundary cases; real SIMD instructions verified on x86-64 + AArch64; data paths (contiguous arrays/slices/builders/SoA; slice + async-buffer lifetimes; sustained frozen low-level capabilities with their own stage criteria) and a measured pipeline (dev/release profiles; pass-run verification; arithmetic/control/memory/vector/LLVM ops; no opaque-call culture; specialization/inlining/scalar-replacement/DCE/loop simplification; vectorization verified; fusion/tiling/interchange/unrolling evaluated not mandated; remarks tied to source; opt-in LTO/PGO with reproducible identity; stable partitions + fast analysis paths for local edits; artifact keys include pipeline/target/features/profiles/ABI) | C4 | SIMD-vs-scalar suites; instruction dumps; pipeline tests | pending (M40) |
| AC-S60 | Collections/targets/gates: bounded parallel map/chunk (ordered output by default; execution order documented separately; grain-sized partitioning; sequential fallback under the same contract; scope/cancellation/budget inheritance; unsafe captures + overlapping partitions rejected; observed-error/cancellation behavior separate from ordering; no silent parallelization — traps/divergence counted; ordered vs tree reductions separated with checked-overflow semantics or an explicit recorded policy; no unspecified-tree FP-equivalence claims); alignment/cache/prefetch (checked alignment preserved through containers/closures/frames/FFI; stride alignment; target-aware cache-isolation wrappers, never a universal 64-byte law; per-worker accumulation preferred where measured; padded state + physical layout outside CON/CTF; invalid packed-atomic/misaligned-SIMD diagnosed; prefetch = address-safe hint, harmful-allowed, benchmarked vs baselines); target dispatch (baseline default, explicit native, target-resolved features, multiversioning with validated dispatch + fallback, baseline-conforming startup/dispatch, explicit ISA restrictions, equivalent attributes implemented through the real backend — no Clang-attribute reliance, per-input variant caches, capability reports incl. threads/IO/atomics/SIMD, honest rejection, WASI path preserved); gates `make calque-concurrency`/`calque-cpu` carrying the §19 suite + §20 evidence (two-worker proof, 1-vs-N, cost decomposition, memory/tail/cancel/drain, many suspended I/O tasks, zero-escape fail-fast, ordered map + SIMD kernel with IR/instructions/equivalence, prefetch experiment, hardware records, repeated trials, compile-latency/size visibility); delivery order 1–7; bootstrap/self-host/existing goldens preserved | C4–C6 | gate targets; hardware records; demo records; preserved gates | pending (M40) |

### STEERING AMENDMENT 6 (vector/array/SIMD syntax — D-108) + 7 (Silat Style — D-109)

| ID | Requirement | Stage | Evidence | Status |
|---|---|---|---|---|
| AC-S61 | Three spellings: `[T]` = persistent `Vector(T)` (formatter/examples prefer it), `[N]T` fixed inline array, `vec[N]T` SIMD with N logical lanes independent of register width; `[a, b, c]` literals with context-directed construction (no persistent intermediate before a typed `[N]T`/`vec[N]T`; empty literals need context or an annotation; exactly-N for fixed/SIMD; heterogeneous elements diagnose; trailing commas/multiline/comments/source-mapped element errors); checked indexing; half-open slices with `0 <= start <= end <= length` (constant violations rejected at compile time); spreads evaluate once in written order; `++` concat with checked length arithmetic (never elementwise, no implicit collection→SIMD); indexed assignment = replacement + rebinding of a mutable place (aliases preserved, `const` rejected, once each in documented order, not atomic, reuse only under ownership analysis); eager ordered comprehensions `[for b in s [if p] => e]` (source once, predicate per element, result only for accepted, ranges without intermediate vectors, one builder where valid, no hidden tasks/fast-math); patterns (`[]`, exact, one rest `.._`/named (immutable persistent capture) at start/middle/end, composed with the existing pattern/exhaustiveness machinery, concrete witnesses, guards not unconditional, ignored rests unallocated) | C2 | conformance tests through the compiler (the amendment's §8 evidence) | pending (M38) |
| AC-S62 | SIMD sugar: lane-wise operators + ONCE-evaluated scalar broadcast (no implicit narrowing/promotion; mismatches diagnose at source); lane masks for comparisons (`simd.any`/`simd.all` explicit; no mask→bool); `assert.equal` structural; `simd.select(mask, then, else)` (broadcast-compatible args, eager, never trap-preventing claims); checked lane access + read-only `x/y/z/w` + read-only 2–4 lane swizzles (compile-time, no intermediate; no assignable/mixed-family swizzles; shuffles via one typed op); prior checked-arithmetic/FP policies (stable failing-lane diagnostics; no global fast-math from syntax; masked tails; scalarization under the same contract; values flow through args/results/structs/specialization; real instructions verified where supported); contextual SIMD literals allocate no persistent collections | C4 | SIMD-vs-scalar suites; instruction dumps (D-108 §8) | pending (M40) |
| AC-S63 | Vector-sugar tooling/cost: canonical formatter + LSP completion/hover showing the three spellings distinctly; diagnostics point at the incompatible element/lane/index/broadcast operand; source origins through comprehension/update expansion; builder/intermediate internals stay out of user-facing primary locations; slice-retention/concat-cost/builder docs with cost-analysis evidence for allocation claims; missed vectorization explained (never unsafe rewrites); CLI Markdown for all syntax/cost docs; examples/docs migrate AWAY from wrapper-heavy construction (with the b0 bootstrap mapping recorded: `Vector(T)`/`vector.new()` is the seed's contract, migrated as C2's surface compiles) | C2/C4/C5 | formatter/LSP/doc tests; migration records | pending (M38) |
| AC-S64 | Silat Style adopted (docs/calque/SILAT_STYLE.md, pinned): "Write the intent. Own the effects. Prove the result."; bootstrap limitations are never the language's personality (explicit owners + exit conditions per exception); naming (PascalCase types, snake_case terms, one term per concept, no responsibility-free layers or rename-only wrappers); honest state shapes (variants over rendered names/sentinels; explicit error nodes with origins REJECTED before execution; no dummy successes; real cursors/admitted mutable cells over collection-as-cell; no forced functional reconstruction); language use (literals/patterns/comprehensions/typed propagation; builders/loops when their contract matters; no mechanical-call source); errors (`try`/match/catch/assert/diagnostic boundary; never swallow for green; never wildcard-for-exhaustiveness); ownership (scopes/awaits/cancellation/parallelism-explicitness; no unchecked-capture repairs; no per-library pools); cost (observable choices stated; evidence for claims; correctness ≠ speed); docs/tests (contracts in source docs, invariants in comments, no preambles; failure boundaries + regression inputs; no overclaiming); non-rules respected (no recursion/closure/allocation bans, no quotas/length limits, no mandatory wrappers, no author classification) | C2 (lint profile at C5) | rule fixtures with preferred example + counterexample + applicability (D-109; enforcement via D-104's lint) | pending (M38) |
| AC-S65 | Silat migration evidence: variant-based token handling; shared structured State failures; explicit Expr.Bad recovery nodes rejected before execution; presentation-only rendering; real cursor state instead of scalar cells; equivalent behavior through byte-exact positive and negative gates | C1/C2 | D-109's lexer/parser migration; D-112 retired the parser cursor cells; D-140 retires Emitter ast/file/ret cells and mechanical accessors; `make calque-selfhost` and `make calque-test` preserve the current compiler and corpus; remaining owning-module/private-helper migration and MatchCtx cells are explicit in STATE.md | pending (M38; recorded migrations implemented, remaining section 6 work not waived) |
| AC-S66 | Silat tool integration (existing services only — no second linter/CI): `calque fmt --check`, `calque lint --profile silat` (+ `--help-mode agent --diagnostic-format markdown`), `calque help style --format markdown`, `calque ci`; deterministic high-signal rules as the house default; advisory rules independently configurable; diagnostic IDs/explanations/edit plans shared with LSP/CLI (D-099/D-101/D-104); reasoned suppressions; missing prerequisite ≠ waived requirement | C5 | lint profile fixtures; CLI Markdown docs | pending (M41) |
| AC-S67 | The amendment's §12 acceptance evidence (both amendments): `[T]`/`[N]T`/`vec[N]T` demonstrated without constructor boilerplate; a comprehension + nested vector/struct pattern through the ACTUAL compiler; scalar-broadcast SIMD arithmetic without splat boilerplate; immutable element updates preserving aliases; contextual SIMD literals not allocating; arithmetic order/mismatch/masks/swizzles/scalar-fallback verified; before/after source for a real compiler function (the Silat phase-1 commit); frozen historical sources exempt; self-host fixed point + specimen matrix preserved; style examples published as executable source + CLI Markdown; regressed idioms rejected by rule tests | C2–C6 | the amendment demos; migration commits; preserved gates | pending (M38) |

---

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
| `calque-bootstrap` | M37+ | ordered seed/snapshot fold, current-source recompilation, artifact0/1/2 hashes and byte-exact fixed point; network-free after setup (D-126/D-141) |
| `calque-selfhost` | M37 (interp legs) / M40 (native + fixed point) | chain verification + comparisons |
| `calque-comptime` | M39 | comptime + cross-specimen conformance |
| `calque-dx` | M41 | formatter/LSP/REPL/incremental verification |
| `calque-bench` | M40 | reproducible measurements |
| `calque-demos` | M42 | demo verification + artifact production |
| `calque-docs` | M41 | documentation graph: site/Markdown export/CLI-help conformance (D-099) |
| `calque-packages` | M40 | fetch/verify + installer-plan conformance over signed local fixture repos (D-101) |
| `calque-fuzz` | M40 | quick deterministic corpus/wiring tier; budgeted campaigns stay explicit (D-102) |
| `calque-lsp` | M41 | language server: library/wire/end-to-end tests + real-subprocess and client smoke checks (D-103) |
| `calque-diagnostics` / `calque-lint` | M38+ | diagnostic record/renderer/catalog conformance; the lint + `calque ci` quality gate (D-104) |
| `calque-concurrency` / `calque-cpu` | M38+ | scope/await/orchestration conformance + multicore evidence; SIMD/pipeline/dispatch/target gates (D-105) |
| `calque fmt --check` / `calque lint --profile silat` | M41 | formatter check + the Silat house-style profile on the EXISTING lint (D-109; no second linter) |
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
