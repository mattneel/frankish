# specimen: calque — ORIGINAL (no upstream); v0.1 = bootstrap subset b0, frozen at M36

## Identity & pin
**Calque** — a strict, Zig-like functional language on Frankish/MLIR.
ORIGINAL specimen: there is no upstream implementation and no upstream
oracle. The authority for the language is `docs/calque/SPEC.md`
(frozen decisions in its §12); the acceptance contract is
`docs/calque/GOAL.md` (the user's goal document, stored verbatim,
sha256 09c9ee8ddb7134fd222e580a908ff7d8e3ff7017a7ceb92a0433576659feff30).
Extension `.calq`; command `calque`. Language version v0.1 = the
bootstrap subset **b0** per SPEC §12; later versions amend via
D-entries and the SPEC freeze log.

## Role
Calque is the workbench's first **product-language specimen**: it
forces the middle layer in the direction of a usable language rather
than a pinned subset of an existing one. It carries (goal §2
authorization, D-085):
- a **self-hosted compiler** — the canonical compiler is written in
  Calque and runs under Frankish's interpreter before native
  compilation (bootstrap route: `docs/calque/BOOTSTRAP.md`);
- **comptime** — the SAME derived interpreter executes compile-time
  code; type construction, reflection, reification, capability and
  budget discipline are Calque's second forcing function (C3);
- **generics** — monomorphized, with comptime type parameters (C2/C3);
- **developer tools** — diagnostics, formatter, LSP, REPL off ONE
  canonical frontend (C5);
- **performance with a visible cost model** — specialization,
  tail-call guarantees, allocation contracts, measured evidence (C4).

Admission note (L5 relaxed for this specimen by D-085, per goal §2):
a feature may be admitted even when existing kernel operations
express it, because Calque's language contract requires it; but
**genuinely new reusable semantics still require promotion** into
Frankish under the K1–K7 contract before they are used as shortcuts.
Surface-intent source of record: the user's demo playbook
(`docs/calque-demo-playbook.md`; adopted by D-090) — it supplies the
C2+ spellings (dot construction, `union(enum)`, `try`/`catch`,
attributes, test blocks, the `calque.toml` build manifest) and the
C6 storyboard; SPEC §12's freeze log records what is frozen when.

## Stages
Each stage freezes at its milestone exit (goal §18; numbers from the
current ledger, D-085):

- **C1 (M37) — interpreted compiler in Calque.** Seed + host bridge;
  compiler core written in Calque; runs under `frk-interp`; compiles
  a small independent corpus; initial self-compilation gate.
- **C2 (M38) — useful functional language.** Pipelines/seq, matches,
  closures, Option/Result ergonomics, generics (monomorphized),
  recursive data, modules + stdlib; compiler grows through the
  recorded chain.
- **C3 (M39) — shared compile-time execution.** `comptime`, type
  construction/reflection/reification, capabilities, budgets,
  caching, nested compiler-library + cross-specimen evaluation.
- **C4 (M40) — native self-host and performance.** Native compiler
  from canonical source; reproducible fixed point; optimizations +
  cost controls; published measurements.
- **C5 (M41) — developer experience and portability.** Formatter,
  language server, REPL, incremental invalidation, native debug,
  target matrix.
- **C6 (M42) — release and demonstration package.** Demos A–E
  (asciinema + MP4), documentation audit, acceptance sweep.

## Scope grammar (b0 — frozen at M36; SPEC §12 is the precise text)
ASCII-source subset: `i64`, `bool`, `Str`, `Vector(T)`, `Option(T)`,
`Result(T, E)`; `const`/`var` locals and assignment; `fn` with
explicit types everywhere; closures; recursion (direct/mutual) with
the tail-call guarantee; records (nominal, `struct { ... }`);
tagged unions (nominal, single payload per variant); `match` with
variant/wildcard/binding patterns, exhaustiveness checked; `if`
(statement + expression), `while`, `break`, `continue`, blocks as
expressions; `+ - * / % == != < <= > >= and or !`; `|>` pipeline
sugar; `try`/`catch` error propagation on error unions (`?T`/`E!T`);
string literals with
ASCII escapes; `//` and `/* */` comments.
Multi-file compilation = host-ordered concatenation of one flat
module (the module system is C2). Recursive type groups are
permitted; representation erases in-group references through
`frk_mem.recref` (the D-074 recipe, generalized by D-088).

## Fences (b0 — law, L5)
No f64/float; no generics by the user (builtin Option/Result/Vector
only); no comptime; no modules/imports; no `defer`; no tuple types;
no literal/nested/or patterns or guards; no `while`-`else`; no
bit operations/shifts; no casts; no bytes buffer type; no operator
overloading; no method syntax; no floats in literals; no non-ASCII
bytes anywhere in b0 sources OR the seed input channel (the
femto_lua literal fence, D-056 — refused loudly by the driver, not
silently mis-lexed); no exception-like constructs beyond the
trapping safety behaviors in SPEC §6.
Fence lists are not TODO lists; each fence lifts by the stage that
prices it, recorded in SPEC §12 when it lifts.

## Conformance
No upstream oracle. Evidence layers (goal §16):
1. **Independently derived expectations** — every corpus
   `expected.out` is written from SPEC semantics by hand BEFORE the
   compiler runs (never blessed from output).
2. **Cross-path agreement** — interp = jit = jit-rc = AOT grid over
   the corpus (the L3 discipline, unchanged).
3. **Cross-compiler agreement** — interpreted vs native compiler
   produce canonical-equal artifacts for the same source; seed and
   compiler agree behaviorally on b0 programs (C1+).
4. **Subset transliteration oracles** (opportunistic) — a b0 program
   transliterated to ml_core/r7rs can be run under ocaml/chibi for
   additional independent bytes where the mapping is faithful.
5. **Metamorphic + property tests** (C2+), **mutant tests** on the
   artifact comparator (C1+), per goal §5/§16.
Corpus location: `goldens/calque/` (case.calq + expected.out), with
directives in `// frk-case:` form (SPEC-conformant comments).

## Oracles & canonicalization
No upstream binary. The artifact comparison contract for
compiler-parity evidence (fixed point, interpreted-vs-native) is
defined in `docs/calque/BOOTSTRAP.md` §4: parse + verify + MLIR
default print + canon byte discipline; compiler determinism is a
requirement, so the comparison excludes NO identifiers — any
nondeterminism is a compiler bug. Diagnostics compare in the
canonical line format of SPEC §11.

## Exit bars (per stage; M36 = this charter)
- **M36 (C0):** goal record stored; scope amendments ledgered
  (D-085+); b0 + examples frozen (SPEC §12); acceptance matrix
  current; `make calque-test` green; repo baseline recorded.
- **M37 (C1):** seed compiles b0 → kernel IR; compiler core in Calque
  runs under the interpreter; corpus ≥ N cases green across all
  runners (N set at C1 planning, ≥8); initial self-compilation gate
  (chain: seed → calq0 → calq0; plus corpus parity interpreted vs
  native); L1 verifiers landed with every piece.
- **M38 (C2):** language grows per goal §7–§9/§13 subset; compiler
  self-hosts the grown language through the chain; stdlib + modules
  live.
- **M39 (C3):** comptime service; type construction/reflection;
  reification; capabilities + budgets + cache; cross-specimen entry.
- **M40 (C4):** native compiler from canonical source; fixed point
  reproduced from clean inputs; cost reports + allocation contracts;
  measured results.
- **M41 (C5):** formatter/LSP/REPL/incremental/native-debug; target
  matrix per goal §17.
- **M42 (C6):** demos A–E recorded and verified; docs/README/book
  audited; §23 audit complete with evidence.

## Status
M36 (C0) in flight: charter artifacts under `docs/calque/`,
`specimens/calque/examples/`, acceptance matrix, and the make target
skeleton (`calque-test`). No compiler exists yet; nothing here is
claimed executable. See `docs/calque/ACCEPTANCE.md` for the current,
command-backed status of every goal requirement.
