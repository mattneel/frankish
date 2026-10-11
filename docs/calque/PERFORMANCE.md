# calque — Performance and Incrementality Plan

Status: plan, opened at M37 (C1) so budgets and targets are recorded
BEFORE the optimization milestone (goal §15.8). **No measured Calque
numbers exist yet; nothing in this file is a performance claim.**
Rulings: D-089 (content-addressed discipline), D-093 (the incremental
computation engine — the architecture this plan measures).

## 1. The architecture being measured (D-093)

A demand-driven QUERY ENGINE with dependency tracking and
unchanged-result detection, not merely a cache:

> An edit invalidates the computations that depend on it. If their
> observable results remain unchanged, invalidation stops there.

Three cooperating components: a resident live query database
(dependency edges + revisions; validation separate from value
retrieval), a persistent query/action index (input-derived keys,
validated lazily after restart), and a content-addressed artifact
store (immutable bytes under content hashes; action key ≠ output
hash). The tight warm path — update source representation → validate
demanded queries → recompute only invalid ones → stop at unchanged
results → emit/link only where required — is what the benchmarks
below must demonstrate. Backend work is measured separately from
frontend work (stable native-code partitions, D-093).

## 2. Measurement procedure (goal §15.6, restated)

1. Correct baseline implementations first; performance changes never
   outrun correctness.
2. Record hardware, toolchain, target, build mode, runtime strategy.
3. Measure compiler startup separately from program execution.
4. Measure cold compilation separately from warm incremental work.
5. Measure allocation counts and peak memory alongside elapsed time.
6. Repeated trials; report variability, not single runs.
7. Compare generated code against an equivalent direct implementation
   for representative cases.
8. Record budgets BEFORE the optimization milestone (this file).
9. Preserve benchmark inputs and measurement scripts in-repo.
10. Investigate statistically meaningful regressions before release.

## 3. Environment record (template, filled per run)

| Field | Value |
|---|---|
| Hardware | (per run) |
| Toolchain | pins from `versions.env`; compiler identity hash per D-093 |
| Target / build mode | (per run) |
| Runtime strategy | `rc` (the managed profile) |
| Process state | warm (resident) vs cold (fresh process) — ALWAYS reported separately |

## 4. Iteration profiles

| Workflow | Primary objective |
|---|---|
| Editor analysis | Update diagnostics without native code generation |
| Interactive execution | Reuse frontend results; demand only required executable work |
| Development executable | Reuse stable native partitions + necessary linkage |
| Release executable | Broader optimization when runtime benefit justifies build cost |

## 5. Benchmark matrix (work counts, not just milliseconds)

Each row needs elapsed time AND structural counters, warm and cold.

| Edit | Required evidence |
|---|---|
| No change | No repeated semantic analysis or compile-time evaluation |
| Comment-only edit | Updated source mapping; no unnecessary semantic invalidation |
| Private body edit | Limited recomputation outside actual semantic/optimization consumers |
| Compile-time implementation edit, same value | **Invalidation stops at the unchanged frozen result** (red-green) |
| Compile-time value change | Only genuine consumers receive invalidation |
| Declared input-file change | Correct transitive invalidation + action-index replacement |
| Compiler or evaluator change | No reuse of incompatible persistent results (toolchain-identity namespace) |

## 6. Counters (D-093)

Executed queries; deserialized bytes (validation must not force
deserialization of unchanged values); interpreter steps (comptime);
regenerated object partitions; allocations and peak memory; retained
live-set sizes across edits in a resident session.

## 7. Planned targets (to be MEASURED, never claimed early)

From D-089/D-093, for selected workflows (small local edits, warm
process, fine-grained hits): warm body-local change in the tens of
milliseconds (single-digit possible with a resident process); medium
change (a few functions, some comptime fallout) in the 50–200 ms
class; cold or large invalidation ≈ a full compile of the affected
subgraph; the inception demo's second run: frontend/eval reuse with
the frozen value's consumers untouched. These are plausible targets,
not forecasts; the C4 report replaces them with measurements — and
the README/demos may then quote measured numbers with their
environment record.

## 8. Claim discipline

No invented numbers, no decorative terminal summaries: every
allocation/partition claim in a demo clip needs its own executable
test (goal §15.7, playbook §20). The strongest public claim is the
red-green demonstration (AC-20.9): "I changed the program that runs
during compilation. Its answer stayed the same. The application did
not rebuild." — shown via `calque build --explain-incremental` with
dependency classes reported separately (frozen value vs diagnostic
provenance).

## 9. Reproducibility

Benchmark scripts, inputs, and the full transcripts live in-repo
(`make calque-bench`, landing at C4); the short clips show real
timings from those scripts, with cuts disclosed.
