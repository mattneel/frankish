# calque — Bootstrap and Self-Hosting Contract

Status: v0.1, M36 (C0). Implements the goal's §5 shape; decisions in
D-086 (seed choice, artifact, comparison) and D-088 (emission rules).
Amendments via DECISIONS entries.

The required chain (goal §5, verbatim shape):

    Auditable seed + bootstrap-compatible compiler source in Calque
        -> initial compiler kernel artifact
        -> Frankish interprets the compiler
        -> compiler translates Calque source, including compiler source
        -> Frankish compiles that artifact to native code
        -> native Calque compiler rebuilds the same compiler

## §1 The seed

**Choice (D-086): the seed is a Lua 5.1 program — a translator from
b0 (SPEC §12) to kernel-IR text — executed on the existing femto_lua
specimen.** Rationale: the goal prefers an existing specimen when its
admitted subset supports the work, and femto_lua v0.4 does: tables
(symbol tables, ASTs), closures, recursion with the tail-call law,
byte strings with `sub`/`rep`/`concat`/`len`/interning, `print`,
`while`/numeric-for. The seed therefore runs through machinery that
is already pinned, differentially verified against lua5.1, and green
on every runner and triple.

Priced alternatives (recorded, not chosen):
- **Rust seed** — materially more code, and it would put
  Calque-shaped translation logic in the host language the goal
  explicitly wants kept out of the production path; kept as the
  documented fallback if a concrete blocker appears.
- **Scheme seed (r7rs_core)** — capable (i64 fixnums, proper tail
  calls, substring/append) but weaker data structures (no
  string-ref in v0.4; alists/hand tables for the seed's needs).
- **Hand-written initial kernel artifact** — the smallest *file*
  count, but thousands of lines of hand-maintained MLIR that must be
  re-derived on every b0 change; the seed-translator shape is what
  the goal describes and what stays auditable.

### §1.1 Seed input channel
The driver assembles a Lua source text and runs it through the
existing pipeline (`frk_front::lua::compile_lua` → verify →
`frk_interp`; any runner may execute it):

    <seed.lua source>
    local __input = { [1] = "<unit file 1 bytes>", ..., [N] = "<...>" }
    __calque_seed(__input, N)

- Literals are emitted by the driver with the femto_lua escape set
  (`\n \t \r \\ \" \'` and `\ddd` for other ASCII bytes).
- The channel is **ASCII-only**: bytes ≥ 0x80 in a unit file are a
  loud driver error naming file+offset (the femto_lua literal fence,
  D-056; SPEC F18). b0 sources are ASCII by construction; the
  compiler's own input channel (§3.3) is unrestricted.

### §1.2 Seed output
The seed writes the artifact text to stdout via `print` (one line
per call; the driver captures the interp output buffer / process
stdout). Stdout carries the artifact and nothing else. Seed failures
are PROCESS failures: the seed aborts (a Lua runtime error) and the
driver reports them as seed failures — never as compiler
diagnostics (which are the artifact-side protocol, §2.2).

### §1.3 Seed scope and freeze
The seed translates b0 exactly, syntax-directed, with the emission
rules of D-088 (uniform closure convention; recref erasure; checked
arithmetic sequences; tag-compare match lowering; vector/intrinsic
mapping; string escaping into MLIR attributes). The seed does NOT
type-check beyond b0's explicit annotations' well-formedness
(explicit-typed language in, explicit IR out).

**The seed FROZEN at the M37 initial self-host gate**: no evolution
afterwards; its source, its unit corpus, and the exact b0 snapshot
of the compiler it translates are preserved under
`tools/calque/bootstrap/` (+ `snapshots/b0/`). Later language growth
never requires the seed to parse new syntax (goal §5).

## §2 The artifact

An **artifact** is one kernel-IR module in MLIR textual form
(LLVM/MLIR 22 + the frk kernel dialects), defining:

- `main() -> i64` — the entry, carrying `llvm.emit_c_interface`
  (the repo's entry protocol v0; the compiler returns 0 on success
  and a deterministic nonzero code for rejected input). The manifest
  may select another entry symbol via the standard `entry=` protocol.
- Semantic content: the compiled unit. Locations carry
  `file:line:col` (file = unit file index; the driver maps indices
  to paths for rendering).
- The artifact MUST begin with the version marker line
  `// calque-artifact v0` (a comment in the module text; the driver
  refuses any other version; bumps are D-entries that preserve
  existing producers per goal §6).

### §2.1 Composition (driver)
The runnable module for an interpreted or compiled run is composed
as TEXT, then parsed and verified as one module:

    <calque intrinsics.mlir>        // shipped seed module: __calque_* declarations
    <artifact text>                  // the compiler's output
    <input injection>                // driver-generated: the unit under compilation

Component roles follow the D-062 authoring-surface precedent and
M17's seed-module pattern, with Calque as the fourth frontend:

- `intrinsics.mlir` — Calque's primitive surface (SPEC §6.6/§6.8,
  `vector.*`, `io.*`) as kernel-IR functions/declarations; carries
  its own `frk_rt_*` declarations, checked against the ABI registry.
- program/artifact text — user/compiler code only; it must not
  redeclare intrinsics symbols (an error, checked by the driver).
- input injection — for compiling a unit with the interpreted
  compiler, the driver appends:

      __calque_input_count() -> i64
      __calque_input(i: i64) -> Str     // 0-based; bstr literal

  generated as kernel IR with proper MLIR string escapes (arbitrary
  bytes allowed here). This is the compiler's "stdin"; it is the
  ONLY difference between compiling a unit and compiling the
  compiler itself.

### §2.2 Streams and exit codes
- stdout (`io.print`) = the artifact text of the unit under
  compilation; the driver captures it byte-for-byte.
- stderr (`io.eprint`) = diagnostics in the SPEC §11 wire format,
  one per line; the driver captures and canonicalizes them.
- Exit code: 0 = success (artifact valid); 1 = input rejected
  (diagnostics are the record); other codes are reserved and MUST
  NOT be emitted by a conforming compiler.
- Diagnostics emitted by the CLI for the USER's consumption are a
  rendering (C5); conformance compares the wire format only.

## §3 Running the compiler; the chain steps

M37 scope is steps 1–4 and 7–8 below (interpreted legs); M40 owns the
native legs (5–6) and the clean-input fixed point.

1. **Seed → artifact0.** Driver assembles the b0 snapshot unit
   (seed input, §1.1) and runs the seed; output = artifact0.
2. **Interpret.** artifact0 + intrinsics + input-injection for the
   unit U → parse, verify, `frk_interp` runs `main` → artifact_U
   (or diagnostics). This is "Frankish interprets the compiler".
3. **Compile the compiler.** U = the current canonical compiler
   source (its b0 snapshot at M37; grown sources later) → artifact1.
4. **Check.** artifact1 is a valid artifact (parses, verifies) and
   behaves as a compiler on the corpus (its outputs match the
   spec-derived expectations; diagnostics compare canonically).
5. **Native build (M40).** Lower artifact1 through the standard
   pipeline (strategy = Rc for the compiler; Arena may be used for
   side experiments) and produce the native compiler binary.
6. **Native recompile (M40).** artifact2 = native-compiler(compiler
   source). Compare artifact1 vs artifact2 (§4). Rebuild once more
   → artifact3; artifact2 == artifact3 is the fixed point.
7. **Corpus through both compilers (M37+).** Run every conformance
   case through the interpreted and native compilers; compare
   canonical artifacts AND canonical diagnostics (goal §5.8–5.9).
8. **Behavioral seed check (M37+).** For every b0 unit in the seed
   corpus, run seed(ctx→artifact) and calq0(ctx→artifact) as
   compilers of a small program and compare their OUTPUTS. The seed
   and the compiler need not emit identical IR (different
   implementations); they must agree on behavior.

The commands (targets land with their stages; `make calque-test`
exists now):
`make calque-bootstrap` (step 1 artifact0 rebuild, reproducible
offline), `make calque-selfhost` (steps 2–7 comparisons),
`make calque-test` (corpus + charter checks), `make calque-comptime`
(C3), `make calque-dx` (C5), `make calque-bench` (C4),
`make calque-demos` (C6), aggregate `make calque-release` (C6).

## §4 Fixed-point comparison rules (goal §5 evidence)

**Canonical artifact form.** Parse the artifact text (host), verify
(MLIR + frk semantic verifier), print with MLIR's default printer,
then apply the canon byte discipline (LF normalization, single
trailing LF). Compare bytes. The comparison inputs are: the version
marker, all semantic attributes, target/config information, entry
signatures, source locations, and symbol names.

**Exclusions: none beyond the printer's own conventions** (SSA value
names, attribute ordering — normalized by parse+print). Compiler
output must be DETERMINISTIC (SPEC F17): any nondeterminism is a
compiler bug, and the comparator is designed to surface it rather
than excuse it. Caching (D-089) may store artifacts under
content-addressed keys, but every verification-path use RE-VALIDATES
(parse+verify or hash) before comparison — the comparator never
trusts a cache entry it cannot validate, and a stale artifact is a
hard failure, not a fallback.

**Comparator verification (required, C1+).**
- *Intentional semantic mutations*: a test corpus of mutated
  artifacts (changed integer constant; swapped variant tag; wrong
  arity at one op; a dropped location) MUST fail comparison or
  verification — land these as unit tests before depending on the
  comparator.
- *A compiler-source edit that changes generated output*: a test
  edits a constant/behavior in the compiler source, recompiles, and
  asserts the artifact bytes changed (no stale-artifact acceptance).
- *A new program not in any bootstrap fixture*: compiled through
  both compilers; outputs compared; run.
- *Stale/hardcoded artifacts are REJECTED*: the driver always
  regenerates from sources; no checked-in artifact may stand in for
  compilation in the verification path (goal §5).

**Artifacts of record.** `make calque-bootstrap`/`calque-selfhost`
write to `out/calque/` (gitignored): artifact texts, their
canonical forms, stage hashes (sha256 of canonical bytes), and the
comparison report. The M40 evidence report cites these hashes.

## §5 Snapshots and chain discipline

- `tools/calque/bootstrap/` — the seed (Lua source), its frozen
  unit corpus, the driver recipe, and `snapshots/<version>/` =
  bootstrap-compatible compiler source snapshots, each with a
  README recording: the b0 cutoff it satisfies, the seed invocation,
  and the hop it serves.
- Frozen historical snapshots are clearly separated from the
  canonical compiler (`tools/calque/compiler/`): snapshots are
  inputs to the chain, never the product.
- Each hop in the chain is recorded (source → source, or
  source → artifact → source) with the language features that forced
  it. The chain stays short: a snapshot exists only when the current
  compiler genuinely outgrows the previous seed/compiler's subset.
- Reproduction is network-free after toolchain setup: all inputs
  (seed, snapshots, intrinsics) live in the repo.

## §6 Trust boundaries (reported explicitly; goal §5)

Trusted beyond the Calque sources themselves:

1. the pinned toolchain and LLVM/MLIR 22 via melior;
2. the Rust host (driver): file I/O, text assembly, parse/verify,
   comparison, CLI — NOT Calque semantics (no Calque parsing or
   typing logic exists on the host side, per goal §4's prohibition);
3. the femto_lua frontend that compiles the seed, and lua5.1 as its
   oracle;
4. `frk-interp` + the kernel dialect evaluators (reference
   semantics), the kernel lowerings, and `frk-rt` (both twins) —
   the components the goal explicitly keeps as Frankish's job;
5. the seed itself (b0 translation), bounded by its size, its
   corpus, and the behavioral cross-check of §3.8.

Self-compilation is reported as bootstrap evidence — agreement
between execution paths — NOT as proof of semantic correctness
(goal §5); correctness evidence is the corpus, the oracles, and the
mutant tests.
