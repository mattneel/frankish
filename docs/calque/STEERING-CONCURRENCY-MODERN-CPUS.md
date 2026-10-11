# Steering amendment: Structured concurrency and modern CPU execution

## 1. Authority and timing

Structured concurrency, async execution, and real multicore parallelism are core Calque requirements. SIMD and data layout are part of the same execution design. They are not optional packages or promises about later LLVM optimization.

This amendment extends the active goal and all earlier amendments. The priority order remains developer experience, runtime performance, then compile/iteration speed. Correctness and safety remain requirements.

### Immediate procedure

- Read the current constitution, state, decisions, specification, and active amendments.
- Inspect the actual interpreter, memory strategies, control runtime, ABI registry, and native pipeline.
- Store this amendment as `docs/calque/STEERING-CONCURRENCY-MODERN-CPUS.md` or an equivalent permanent record.
- Add its integrity pin and executable acceptance rows without changing historical goal documents.
- Authorize the structured local scheduler and host integration required by this amendment.
- Supersede conflicting exclusions through explicit decisions.
- Keep a distributed actor runtime and a BEAM clone outside this task.
- Establish concurrency ownership and target-layout rules before the current frontend and runtime contracts harden.
- Land executable core concurrency in the early language workstream, not only the final tooling milestone.
- Preserve current bootstrap progress and unrelated user changes.
- Preserve historical compiler snapshots through explicit bootstrap transitions.
- Continue through implementation and verification rather than stopping at another architecture document.

## 2. One concurrency model with explicit execution choices

Tasks define work and ownership. Scopes define lifetimes. Executors place work on threads. SIMD operates across data lanes within a computation.

| Surface | Contract |
|---|---|
| `async fn` and `await` | Suspend a task without an implicit blocked worker. |
| Task scope | Own children until they finish, including required cleanup. |
| Scoped async spawn | Start a child concurrently on an admitted executor. |
| Scoped parallel work | Execute CPU work through a bounded pool of real OS threads. |
| Scoped blocking work | Isolate blocking calls from cooperative async workers. |
| Scoped dedicated thread | Provide explicit OS-thread identity or affinity when required. |
| Parallel collections | Bound work and define result order separately from execution order. |
| SIMD operations | Preserve explicit lane, mask, arithmetic, and memory semantics. |

### Architectural requirements

- Keep synchronous pure functions usable without an async runtime.
- Make async execution available without a third-party runtime selection ceremony.
- Supply an explicit root execution context through application entry points and host APIs.
- Avoid implicit nested runtimes or process-global executor ownership.
- Separate task ownership from executor placement in the public contract.
- Make library operations inherit the caller's scope, limits, and capabilities.
- Permit scoped local tasks for values that cannot cross threads.
- Require transfer-safe captures and results for work that crosses a thread boundary.
- Preserve a single-thread executor for tests, constrained targets, and deterministic exploration.
- Do not describe that executor as multicore parallelism.

## 3. Async surface and task values

The following example defines the intended shape. Exact library signatures and failure types require an executable specification before their freeze.

```calque
pub async fn answer(value: i64) i64 {
    await task.yield();
    value
}

pub async fn sumPair() i64 {
    await task.scope(async fn (scope) {
        const left = scope.spawn(answer(20));
        const right = scope.spawn(answer(22));

        (await left) + (await right)
    })
}
```

### Language requirements

- Implement async functions, async closures, await expressions, and structured task scopes.
- Define an async call as a cold operation that does not execute its body immediately.
- Preserve ordinary argument evaluation when the cold operation is created.
- Start a cold operation through await or explicit scoped spawn.
- Make scope registration complete before a spawned child can publish completion.
- Distinguish cold operations, running child handles, and completed results in types and diagnostics.
- Enforce single execution and single result consumption unless an explicit shared-result abstraction permits otherwise.
- Diagnose discarded cold operations that appear unintentionally unused.
- Prevent task handles and scope-dependent values from escaping their valid lifetime.
- Infer ordinary lifetime obligations without mandatory lifetime annotations throughout application code.
- Keep `try await` compatible with the existing typed `Result` rules.
- Preserve explicit adaptation between unrelated error types.
- Track suspension and concurrency requirements through higher-order functions and generic specialization.
- Give synchronous callers an explicit root bridge instead of a hidden block-on operation.

## 4. Scope exit, errors, and resource ownership

A scope cannot finish while its children can still access scope-owned resources. Dropping a handle does not detach its task. Required child failure handling does not depend on the caller remembering every await.

### Scope contract

- Join all children on normal scope exit.
- On early failure or cancellation, request child cancellation and drain all children before releasing shared scope resources.
- Prohibit unowned background tasks in ordinary safe code.
- Represent long-lived services beneath an explicit application root with an explicit shutdown protocol.
- Preserve resources captured by children until their last permitted access ends.
- Make implicit scope-exit waits visible in source documentation and compiler analysis.
- Define fail-fast behavior for child task failures independently from await order.
- Request sibling cancellation when the scope observes a terminal child failure under its default policy.
- Provide explicit collect-results operations when expected child errors must remain ordinary data.
- Separate task failure from a successful task whose result contains a domain error value.
- Freeze that distinction in API types and conformance tests.
- Retain observed non-cancellation failures rather than reporting only the first convenient error.
- Identify the initiating failure and related failures through task and spawn-site provenance.
- Present observed failures in stable order without claiming schedule-independent failure discovery.
- Define fallible scope and await result types before public API adoption.
- Keep cancellation distinct from ordinary domain-error recovery.
- Never convert native memory corruption into a recoverable task error.

## 5. Cancellation, deadlines, and cleanup

Cancellation is a cooperative request. A deadline requests cancellation at a time boundary. Neither mechanism proves that arbitrary native work stopped at that instant.

### Required behavior

- Propagate cancellation through the owned task tree.
- Support nested deadlines and explicit cancellation tokens with scoped authority.
- Use monotonic time for runtime deadlines and virtual time for deterministic tests.
- Include cancellation checks at documented suspension and scheduler checkpoints.
- Provide explicit cooperative checks for long CPU loops.
- Specify compiler-inserted checkpoint policy without claiming arbitrary preemption.
- Define cancellation behavior before start, during suspension, and concurrently with completion.
- Preserve exactly one terminal task outcome.
- Prevent ordinary broad `catch` code from silently consuming structured cancellation.
- Run required cleanup on cancellation and ordinary error propagation.
- Separate child-resource cleanup from parent-resource cleanup that requires drained children.
- Provide explicit bounded cancellation shielding for cleanup that must suspend.
- Report cleanup failures without losing the initiating failure.
- Never free an in-flight I/O buffer because its waiter received cancellation.
- Retain buffers and handles until the OS or adapter establishes completion or safe abandonment.
- Do not kill an arbitrary native thread or abandon its borrowed scope state.
- If foreign work cannot cancel cooperatively, keep the scope alive until that work returns.
- Use process isolation when the application requires a hard deadline for untrusted native work.
- Report cancellation-request latency separately from full scope-shutdown latency.

## 6. Thread-safe values without a language full of locks

Immutability helps application reasoning. It does not automatically make reference counts, runtime caches, or captured mutable boxes thread-safe.

### Type and memory contract

- Define distinct compiler-known properties for transfer between threads and safe shared access.
- Derive those properties recursively from representation and capabilities, not nominal type names alone.
- Make immutable structs and persistent collections shareable only when their runtime representation supports it safely.
- Permit exclusive ownership transfer of admitted mutable values.
- Prevent the original owner from continued access after such a transfer.
- Reject ordinary mutable-box capture across concurrent tasks without explicit synchronization or exclusive transfer.
- Account for concurrent mutation even when tasks share one OS thread.
- Reject resource and capability transfers that violate thread affinity or ownership.
- Keep local-only tasks available with an explicit non-migrating executor contract.
- Require an explicit unsafe boundary for unverified foreign transfer guarantees.
- Explain the captured value, rejected operation, and actual conflict in source diagnostics.
- Do not propose unsafe annotations as the default quick fix.
- Require explicit partition proofs or trusted library contracts for disjoint mutable parallel slices.
- Preserve automatic lifetimes and the established immutable update semantics.

### Runtime audit

- Audit all managed objects, reference counters, cycle collectors, intern tables, arrays, and allocation metadata before cross-thread use.
- Audit pending control state, coroutine channels, runtime globals, and JIT symbol adapters.
- Put task-specific state in task execution context rather than thread-local storage alone.
- Preserve that state correctly when a migratable task resumes on another worker.
- Prevent one test, nested interpreter, or compiler request from consuming another task's pending state.
- Establish safe publication and reclamation rules for every shared object representation.
- Specify cycle-collection coordination for graphs that cross workers.
- Do not treat atomic retain/release as a complete concurrent collector design.
- Preserve the retain/trace/layout symmetry in both runtime implementations.
- Retain arena owners across child work or reject the escape before execution.
- Keep thread-local fast paths only where confinement is established.

## 7. Synchronization and communication

The safe surface favors immutable messages, explicit transfer, and scoped access. Low-level synchronization remains available for systems code under a documented memory model.

### Required primitives

- Provide bounded channels with backpressure, closure, cancellation, and typed messages.
- Define per-sender ordering and concurrent-send ordering explicitly.
- Provide async-safe waits without a blocked cooperative worker.
- Provide mutexes, scoped lock guards, semaphores, events, and atomic values.
- Reject suspension with an ordinary OS mutex guard held.
- Separate an explicitly async-aware lock from an OS mutex.
- Reject references that escape protected access after a lock guard ends.
- Define atomic widths and supported memory orders per target.
- Use sequentially consistent atomics as the simple default.
- Require explicit selection of weaker orders and reject invalid order combinations.
- Map language memory-order guarantees to verified LLVM operations.
- Never use volatile as a substitute for synchronization.
- Document whether an atomic operation is lock-free on the selected target.
- Provide honest fallback or rejection when the requested atomic width lacks support.
- Exclude live tasks, mutexes, channels, and OS handles from portable CON/CTF serialization.

### Selection operations

- Provide structured wait-all and race/select operations with declared fairness and loser ownership.
- On a completed race, cancel and drain owned losers before the race scope exits.
- Define send, receive, and I/O cancellation at their commit points.
- Preserve ownership of unsent values after a canceled send.
- Do not silently lose a received message because selection and cancellation overlapped.

## 8. Executors, real threads, and async I/O

### Scheduler requirements

- Implement bounded native worker pools with actual simultaneous OS-thread execution.
- Keep cooperative async tasks separate from explicitly blocking work.
- Provide scoped dedicated threads for affinity-sensitive or foreign APIs.
- Define thread-entry, join, failure, and cleanup behavior.
- Make worker counts configurable and account for supported host CPU limits.
- Share parallelism budgets across nested operations rather than creating a new pool per call.
- Prevent parent-child waits from exhausting the pool and deadlocking runnable children.
- Bound pending work and define admission or backpressure behavior.
- Measure contention before choosing global queues, per-worker queues, or work stealing.
- Preserve local-only task affinity under every scheduling policy.
- Define task-local context inheritance and restore it before user code resumes.
- Preserve synchronization between completion publication and result consumption.
- Test registration/wakeup races, duplicate wakes, queue saturation, and runtime shutdown.

### I/O requirements

- Provide awaitable timers and capability-based socket operations on supported development hosts.
- Provide async-facing file operations with explicit native or bounded-offload implementation details.
- Reuse a suitable maintained I/O substrate behind a generic host interface when it meets the contracts.
- Do not require a custom kernel-specific reactor before the first release.
- Do not implement every socket wait as one permanently blocked OS thread.
- Isolate DNS and other blocking library calls through a bounded adapter.
- Distinguish readiness from completed I/O.
- Preserve partial-read, partial-write, interruption, and end-of-stream semantics.
- Make cancellation cleanup safe under late completion and reused descriptor identities.
- Provide a local network fixture without external-service dependencies.
- Diagnose unsupported target capabilities instead of pretending that every host offers threads and sockets.

## 9. Frankish interpreter and native transformation

The existing interpreter remains the semantic reference. The native scheduler supplies real parallel execution. A deterministic logical schedule is useful evidence, not a proof about every hardware interleaving.

### Shared implementation

- Extend the existing interpreter with explicit suspended tasks, scopes, wake events, and ownership state.
- Do not create a second production Calque evaluator.
- Give each new operation reference semantics and a verifier before native execution.
- Reuse existing control and memory primitives when their verified semantics suffice.
- Promote genuine reusable gaps through the kernel contract and ABI registry.
- Read the existing coroutine and control decisions before selecting a native representation.
- Evaluate LLVM coroutines, explicit frame transforms, and MLIR async operations against actual invariants and target support.
- Do not force `llvm.coro` merely because it exists.
- Do not treat `async.execute` or an await operation as proof of guaranteed parallel execution or scope ownership.
- Preserve GC-visible frame layouts, cancellation state, source origins, and exactly-once destruction.
- Prevent concurrent resumption of one task frame.
- Keep callback registration and frame lifetime safe across completion/cancellation races.
- Verify the interpreter's high-level contract independently from the native frame transform.
- Preserve the existing synchronous tail-call guarantee.
- Specify async tail-call behavior separately without promising zero frames for arbitrary suspension chains.
- Verify C ABI and runtime-twin parity for every admitted target and execution mode.

## 10. Deterministic tests and compile-time execution

### Test scheduler

- Add a virtual-clock scheduler to the shared test infrastructure.
- Record runnable-task choices, wake events, cancellation, and I/O fixture outcomes.
- Replay exact traces under a pinned scheduler contract.
- Explore bounded alternative schedules through the first-class fuzz infrastructure.
- Minimize a schedule while preserving the particular failure or differential mismatch.
- Compare synchronized semantic outputs across interpreter and native paths.
- Preserve causal events rather than normalizing away real order differences.
- Do not require byte-identical logs from intentionally unordered programs without a shared schedule contract.
- Use memory-model tests in addition to logical scheduler tests for weak atomic orders.
- Label bounded exploration as bounded rather than exhaustive proof.

### Compile-time policy

- Permit admitted pure structured computations under the existing interpreter with a deterministic scheduler.
- Deny OS threads, live sockets, wall clocks, and thread identity during default compile-time execution.
- Keep virtual test capabilities explicit and separate from production ambient authority.
- Treat host parallel evaluation as an internal optimization only when results and relevant failures remain equivalent.
- Do not cache schedule-dependent work under a false pure-function contract.
- Include target features, execution policy, limits, and semantic dependencies in cache validity.
- Reject live task handles and synchronization objects during value reification.
- Drain all compile-time child tasks before returning a frozen result or failure.
- Preserve nested compiler-request isolation and cancellation.

## 11. Modern CPUs require usable data representations

The default source remains ordinary functional Calque. The compiler needs enough representation information to produce efficient loops and native vector operations. A persistent `Vector(T)` and a hardware SIMD vector are different abstractions.

### Data-path requirements

- Keep nominal structs and tagged unions efficiently representable without mandatory dynamic boxing.
- Provide contiguous arrays, managed slices, and explicit builders for hot data paths.
- Preserve lifetimes for slices and asynchronous buffer views.
- Provide library-level structure-of-arrays storage without changing ordinary record semantics.
- Keep ordinary eager operations distinct from explicit sequence and parallel operations.
- Fuse admitted sequence operations before unnecessary heap intermediates become permanent.
- Preserve loop structure, effects, bounds, strides, and alias information through optimization.
- Emit alias, alignment, range, and purity metadata only when actual proofs or enforced contracts justify them.
- Never invent `noalias`, `inbounds`, `nsw`, or nonnull facts to persuade LLVM to optimize.
- Preserve layout information through specialization, captures, containers, and task frames.
- Keep parallel task scheduling out of pure scalar kernels that do not request it.

## 12. A measured MLIR/LLVM optimization pipeline

### Required compiler work

- Build explicit development and release pipelines for the pinned toolchain.
- Verify that configured passes actually run on Calque-generated code.
- Use relevant upstream arithmetic, control-flow, memory, vector, and LLVM operations.
- Use structured linear-algebra or affine operations when the source workload actually fits their semantics.
- Preserve high-level information until transformations that need it finish.
- Do not emit every operation into opaque runtime calls before optimization.
- Support specialization, direct calls, inlining, scalar replacement, dead-code removal, and loop simplification.
- Enable profitable loop and SLP vectorization with preserved source semantics.
- Evaluate loop fusion, tiling, interchange, and unrolling on representative admitted workloads.
- Do not mandate every transformation on every loop.
- Provide pass inspection and optimization remarks tied to original source.
- Add opt-in LTO and profile-guided release workflows with reproducible profile identity.
- Preserve stable native partitions and fast analysis paths for local edits.
- Do not run whole-program release optimization on every editor request.
- Include pipeline configuration, target, features, profiles, and runtime ABI in artifact keys.

## 13. First-class SIMD with scalar reference semantics

### Surface and execution

- Provide a distinct SIMD type family rather than overloading persistent `Vector(T)`.
- Support fixed-lane arithmetic, comparisons, masks, selections, shuffles, loads, stores, and reductions.
- Specify lane order and numeric behavior independently from physical register width.
- Support masked tails without out-of-bounds accesses or invalid speculative operations.
- Provide valid unaligned operations as well as explicitly checked aligned operations.
- Define safe gather/scatter bounds and duplicate-destination behavior before admission.
- Scalarize admitted operations when a target lacks the required native vector instruction.
- Provide scalar per-lane semantics in Frankish's interpreter and compile-time execution.
- Keep scalable-vector support behind an explicit target-aware capability rather than faking fixed hardware width.
- Do not require AVX-512, SVE, or another optional ISA for baseline applications.
- Verify actual SIMD instructions on representative x86-64 and AArch64 hosts.

### Semantic preservation

- Preserve checked integer failures, evaluation order, signed zero, NaNs, and specified floating-point results by default.
- Do not make global fast-math the price of vectorization.
- Require explicit numeric policies for reassociation, contraction, and approximate operations.
- Verify those policies independently from ordinary strict arithmetic.
- Handle empty inputs, short inputs, partial lanes, misalignment, and target-width boundaries.
- Prevent masked inactive lanes from introducing poison, traps, or invalid memory effects.
- Distinguish explicit vector operations from compiler transformations of ordered scalar code.
- Report a missed optimization honestly when strict semantics prevent the rewrite.

## 14. Parallel collections and reductions

### Required behavior

- Provide bounded parallel map and chunk operations over admitted collections.
- Preserve output index order by default while documenting concurrent execution order.
- Keep completion-order streams explicit and bounded.
- Partition work at useful grain sizes rather than scheduling every element as a task.
- Support sequential fallback for small workloads under the same public result contract.
- Propagate the caller's task scope, cancellation, and parallelism budget.
- Reject unsafe captures and overlapping mutable partitions.
- Specify observed-error and cancellation behavior separately from output ordering.
- Do not silently parallelize an ordinary ordered pipeline because its functions appear pure.
- Account for traps and divergence as well as explicit I/O effects.
- Provide separate ordered and explicitly tree-shaped reduction contracts.
- Do not claim scalar-equivalent floating-point results from an unspecified parallel reduction tree.
- Preserve checked integer overflow semantics or require an explicit alternate reduction policy.
- Record reduction policy and relevant shape parameters in reproducible computation keys.

## 15. Alignment, cache locality, and false sharing

### Layout contract

- Provide explicit checked alignment for types, allocations, arrays, and buffers.
- Require valid alignment values and verify allocator support.
- Preserve over-alignment through containers, closures, coroutine frames, and FFI adapters.
- Ensure array stride preserves the required alignment of every element.
- Provide a cache-isolation wrapper or equivalent target-aware padding facility for hot shared state.
- Distinguish ABI alignment from a cache-locality tuning choice.
- Do not hardcode 64 bytes as a universal cache-coherence law.
- Document the chosen target assumptions and fallback policy.
- Isolate frequently written scheduler counters and queue metadata where measurements justify it.
- Prefer per-worker accumulation and explicit merges where they remove unnecessary shared writes safely.
- Measure memory growth and locality costs before blanket padding.
- Keep physical padding, pointers, and incidental object layouts outside CON/CTF wire data.
- Do not serialize struct memory by raw byte copy as an alignment optimization.
- Diagnose invalid packed-field atomic access and misaligned SIMD promises.

## 16. Prefetch is a performance hint, not a correctness mechanism

### Requirements

- Expose an explicit prefetch API through normal source types and the shared interpreter interface.
- Define read/write intent and locality under a documented hint contract.
- Form addresses only through valid ownership, bounds, and overflow rules.
- Never introduce invalid pointer arithmetic to construct a speculative future address.
- Keep prefetch distinct from a load, synchronization operation, ownership transfer, or cancellation checkpoint.
- Make unsupported hints semantically harmless under the same address-evaluation rules.
- Preserve buffer lifetimes until all actual consumers finish.
- Compare automatic hardware prefetch, explicit hints, and no-hint baselines.
- Benchmark cold and warm data plus sequential and irregular access patterns.
- Keep hints optional when bandwidth, pollution, or instruction overhead makes them counterproductive.
- Do not assert that more prefetches always improve performance.

## 17. CPU targets, dispatch, and portability

### Build contract

```console
calque build --cpu baseline
calque build --cpu native
calque build --cpu baseline --multiversion
calque explain src/kernel.calq --optimizations --format markdown
calque help concurrency --format markdown
calque help simd --format markdown
```

These commands define the intended interface. Freeze exact flag names against the existing CLI before implementation.

### Target requirements

- Keep a documented portable baseline as the default distribution target.
- Make host-specific `native` compilation explicit.
- Resolve target features from the selected target rather than the build host during cross-compilation.
- Separate instruction-set requirements from microarchitecture tuning.
- Support optional multiversioned kernels with a valid baseline fallback.
- Validate CPU features and required OS register-state support before dispatch.
- Ensure startup and dispatch code itself obey the baseline feature set.
- Support explicit ISA restrictions for deployment and reproducible benchmarks.
- Verify feature selection and baseline fallback with controlled tests.
- Do not rely on Clang frontend attributes that the Calque pipeline never processes.
- Implement equivalent LLVM attributes, variants, and dispatch through actual backend integration.
- Keep function-variant caches separate by all relevant feature and ABI inputs.
- Expose thread, I/O, atomic-width, and SIMD capabilities in target reports.
- Preserve single-thread application support where host threads are unavailable.
- Reject an explicit OS-thread requirement on such a target instead of pretending to satisfy it sequentially.
- Preserve the current WASI path without assuming unimplemented threads or shared-memory support.

## 18. Toolchain adoption and diagnostics

### Required consumers

- Use structured scopes for compiler queries, LSP background work, tests, and fuzz worker supervision where those services need concurrency.
- Avoid one thread pool per library or nested compiler invocation.
- Preserve immutable query snapshots and synchronized publication of cached results.
- Keep native parallel compilation output deterministic under the declared artifact contract.
- Run async tests through the integrated test runner with explicit virtual-clock fixtures.
- Distinguish test-case concurrency from concurrency inside a tested application.
- Retain process isolation for memory-unsafe failures and noncooperative native code.
- Cancel obsolete LSP tasks without invalidating results still required by other requests.
- Expose task trees, spawn sites, awaited resources, and cancellation causes through optional diagnostic reports.
- Report unsafe captures, escaped scopes, lock-across-await errors, and unsupported CPU features through the shared diagnostic schema.
- Provide lint rules for known blocking calls on cooperative workers and abandoned cold operations.
- Never suggest detach, unchecked sharing, weaker atomics, or global fast-math merely to silence a failure.
- Integrate concurrency and optimization explanations with the docs graph and CLI Markdown help.
- Keep machine-readable traces optional, bounded, and free from automatic external uploads.
- Keep `@noalloc` claims transitive across runtime services and task-frame allocation.

## 19. Verification, race detection, and schedule fuzzing

### Correctness gates

- Test scope exit through success, failure, early return, cancellation, and cleanup failure.
- Test simultaneous completion/cancellation and cancellation before task registration finishes.
- Test exactly-once result delivery, frame destruction, and owned-resource release.
- Test nested pools, bounded channels, worker saturation, and shutdown with pending I/O.
- Test local-task affinity and migratable task-local context across workers.
- Test shared RC, cycle collection, publication, and deallocation under native stress.
- Test disjoint mutable partitions and rejection of overlapping ones.
- Test atomic publication and weak-memory litmus cases on applicable targets.
- Integrate ThreadSanitizer or an equivalent race detector on supported hosts.
- Prove instrumentation in generated Calque code and relevant runtime components with deliberate race canaries.
- Do not treat sanitizer linking alone as instrumentation.
- Use separate supported sanitizer build profiles when static linking or combined sanitizers prevent a valid run.
- Keep unsupported instrumentation modes visibly unavailable rather than passed.
- Fuzz scheduler event sequences and minimize actionable replay traces.
- Compare SIMD and scalar results across boundaries, numeric edge cases, and admitted target features.
- Verify aligned-allocation promises with addresses and layout tests.
- Preserve canonical CTF bytes across target layouts and optimized paths.
- Verify that all existing specimen goldens retain their admitted behavior.

## 20. Performance evidence and release sequence

The acceptance contract includes actual multicore work and actual SIMD code. Timing alone does not establish either fact. Conversely, emitted instructions alone do not establish a useful speedup.

### Measurements and demonstrations

- Prove overlapping CPU execution on multiple native threads with a bounded synchronization fixture.
- Benchmark one worker and multiple workers on substantial CPU work.
- Separate task creation, wakeup, context transfer, queue contention, and useful work costs.
- Measure memory use, tail latency, cancellation latency, and complete drain time.
- Demonstrate many suspended local-I/O tasks without one OS thread per socket wait.
- Demonstrate fail-fast cancellation with complete cleanup and zero escaped tasks.
- Demonstrate ordered parallel map plus a SIMD kernel with a scalar reference.
- Show actual lowered IR, native instructions, allocations, and output equivalence.
- Compare packed and cache-isolated hot state while retaining realistic memory-footprint measurements.
- Show at least one measured prefetch experiment without presupposing its outcome.
- Validate baseline and feature-specific kernels on actual supported hardware.
- Separate correctness emulation from native performance claims.
- Record hardware topology, CPU features, OS, runtime policy, toolchain, and build configuration.
- Use repeated trials and report per-workload variability rather than a universal speedup.
- Keep compile latency and binary-size regressions visible alongside runtime gains.

### Delivery order

1. Freeze task lifetime, transfer safety, cancellation, memory-model, and target-layout contracts now.
2. Land a minimal interpreter scope/await implementation and native two-worker proof with the early language core.
3. Complete runtime-state isolation, shared-memory safety, cleanup, channels, and supported I/O adapters.
4. Land scalar-defined SIMD operations, explicit alignment, and target-feature configuration.
5. Complete bounded parallel collections, optimized native pipelines, dispatch, and cache-locality work.
6. Adopt the services in compiler tools and complete replay, sanitizer, and performance evidence.
7. Verify bootstrap, self-hosting, existing specimens, and the complete target capability matrix.

### Completion requirements

- Do not put ownership and concurrency semantics entirely behind the final editor-tool milestone.
- Do not mark a single-thread simulation as completion of native threading.
- Do not mark LLVM availability as completion of SIMD or CPU optimization.
- Publish memory, cancellation, scheduling, target, and numeric contracts in source documentation.
- Add `make calque-concurrency` and `make calque-cpu` or equivalent discoverable gates.
- Extend the existing fuzz, benchmark, sanitizer, documentation, and acceptance infrastructure.
- Preserve every earlier mandatory Calque requirement.
- Update `STATE.md` with executable evidence and the next action before handoff.
- Keep this amendment open until its required behaviors and evidence exist in the current tree.
- Keep the overall goal active while any mandatory work remains unfinished.

**Structured lifetimes above. Real threads underneath. Data layouts and machine code that earn their performance claims.**
