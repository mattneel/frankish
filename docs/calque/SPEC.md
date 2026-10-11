# calque — Language Specification

Status: v0.1, frozen 2026-10-11 (M36, C0 charter; authority: `GOAL.md`
+ D-085). Amendments require a DECISIONS.md entry and a §12 freeze-log
row. Language version v0.1 = the bootstrap subset **b0** (defined
here; every rule marked [b0] is frozen; rules marked [roadmap] are
intent, frozen by the stage that lands them).

Reading guide: C1 (M37) implementers need §2–§6, §10, §11 and §12.
C2 (M38) adds §7–§9 items marked C2. C3 (M39) owns §8.

---

## §1 Framing

Calque is a strict, expression-oriented, Zig-like functional language
on Frankish/MLIR. Priority order (goal §1): developer experience >
runtime performance > compile/iteration speed > everything else.
Correctness and safety are requirements, not positions in that order.

Two implementation laws:

- **Determinism.** A conforming compiler MUST be deterministic:
  identical inputs and configuration produce byte-identical
  artifacts and byte-identical diagnostics. Nondeterminism is a
  compiler bug, not a diagnostic-tolerated variance (BOOTSTRAP §4).
- **Strictness and order.** Evaluation is strict and left-to-right
  everywhere (§6.2). `and`/`or` short-circuit; nothing else does.

## §2 Lexical structure [b0]

Source encoding: UTF-8. The bootstrap pipeline (seed, driver) accepts
ASCII source only and refuses bytes ≥ 0x80 loudly (the seed runs on
the femto_lua specimen, whose literals are ASCII-fenced, D-056);
full UTF-8 source is a C2 item for the compiler path.

- Whitespace: space, tab, CR, LF. CRLF accepted (treated as LF).
- Comments: `// ...` to end of line; `/* ... */` block, NON-nesting.
  Unclosed block comment = error.
- Identifiers: `[A-Za-z_][A-Za-z0-9_]*`.
- Keywords: `and bool break const continue else false fn i64 if
  match or pub return struct true union var while` — reserved; no
  other use.
- Reserved module names (may not be bound by user code):
  `str io vector` (b0); the C2 set will extend this list and
  re-freeze §12.
- Reserved prelude type names (may not be bound by user code):
  `Str Vector Option Result` (`i64`/`bool` are keywords); the C2
  set extends this list with the new prelude types.
- Integer literals: `[0-9]+`, decimal, must fit in i64; no sign in
  the literal (unary `-` applies). No underscores/hex in b0.
- Bool literals: `true`, `false`.
- String literals: `"..."` on one line, escapes `\n \t \r \\ \" \'
  \0` and `\xNN` (two hex digits; b0 requires value ≤ 0x7F, else a
  loud error at the seed/driver boundary). Raw bytes ≥ 0x80 inside
  literals are refused by the bootstrap pipeline; [roadmap] the
  compiler path admits full UTF-8 text. [C2, per the playbook]
  multiline `"""..."""` literals.
- Punctuation/operators: `( ) { } [ ]`, `, ; : .`, `= == != < <= >
  >= + - * / % !`, `|>`, `=>`, `_` (wildcard; also a digit separator
  from C2). [C2 forms, per the demo playbook (D-090): `@` attributes,
  `..` rest/spread, `|` or-patterns, brackets for fixed arrays/index
  syntax.]

## §3 Types [b0]

Primitive: `i64`, `bool`, `Str`. Unit: `()`. Shorthands:
`?T` = `Option(T)`; `E!T` = `Result(T, E)` (an error-union type)
[D-090; usable from the stage that lands the corresponding library
surface, syntactically frozen now].
Generic (builtin, non-user): `Vector(T)`, `Option(T)`,
`Result(T, E)` — T/E are types.
Function types: `fn(T1, T2) R` (return last).
User types: bound by top-level `const Name = <type expression>;`:

    const User  = struct { name: Str, score: i64, active: bool, };
    const Token = union(enum) { Plus, Number: i64, Ident: Str, };

- `struct { field: T, ... }` — named fields, declaration order;
  duplicate field = error; trailing comma allowed; empty allowed.
- `union(enum) { Name, Name: T, ... }` — variants carry a NAMED
  payload TYPE (multi-field payloads use an inline `struct { ... }`);
  a bare `Name` is a unit variant; duplicate variant = error; ≥1
  variant required. (Spelling revised by D-090 per the user's demo
  playbook; the earlier `union { V(T) }` form is superseded.)
- Type expressions (`struct {...}`, `union(enum) {...}`, `Pair(Str)`)
  may appear wherever a type is expected, including variant payloads.
- Type references inside a declaration group may be recursive and
  mutually recursive (order-independent, flat top-level scope).
  Representation: in-group references erase through the runtime
  reference form (D-074's `recref` recipe, generalized by D-088);
  no infinite inline values.
- Nominality: a type written as a name is that declaration;
  builtin applications are structural on their arguments. Two
  identical `struct {...}` texts written twice are two DIFFERENT
  types [b0: no structural equivalence for user types].
- [roadmap C2] local inference, tuples, f32/f64, `Bytes`, `Map`,
  `Set`, user generics `Fn(comptime T: type) type`, inference at
  binding sites, optional/union field types.

## §4 Declarations [b0]

Top level (order-independent):

    pub const NAME: T = expr;         // value
    const NAME: T = expr;             // private value
    pub const NAME = struct { ... };  // type
    pub const NAME = union(enum) { ... };   // type
    pub fn name(p: T, ...) R { ... }  // function (R omitted = unit)
    fn name(...) ...                  // private

- `pub` exports; b0's single flat module makes both visible within
  the module; the distinction becomes real at C2 (modules).
- Top-level values are immutable (no top-level `var`).
- Functions: parameters and return types explicit (b0 has NO
  inference); parameters immutable; `main` is the conventional
  program entry (manifest may select another entry symbol).

Locals (inside blocks):

    const x: T = expr;   // immutable binding; annotation REQUIRED
    var   x: T = expr;   // mutable binding
    x = expr;            // assignment (var only; type must match)

- [b0] annotations are required on every binding and every
  parameter/return; [roadmap C2] context inference fills them.
- Scoping: lexical, block-scoped. A new binding may shadow an
  outer local or a top-level value/function. It may NOT shadow a
  reserved module name, a reserved prelude type name, or a
  top-level TYPE name (a record/union const — shadowing it would
  make construction syntax ambiguous). Same-scope redeclaration =
  error.
- `return expr;` early exit; `return;` for unit.

## §5 Expressions [b0]

Literals; names; module calls `str.concat(a, b)`; calls `f(a, b)`;
record construction `User { .name = "a", .score = 1, .active = true }`
and context-typed `.{ .name = "a", ... }`; union construction
`.Ident("a")` / bare `.Plus` (context-typed) or qualified
`Token.Ident("a")`; field access `user.name`; blocks `{ stmt; ...;
expr }` (value = last expression; a trailing `;` makes the block value
`()`); `if cond { } else if cond { } else { }` (PAREN-LESS conditions,
D-090; statement or expression; `else` required in expression
position); `while cond { }`; `break;` `continue;` (innermost loop);
`match e { .P => expr, ... }` (expression; pattern set in §5.2);
closures `fn (x: T, y: U) R { ... }` — b0 requires explicit types on
closure params/returns; context-inferred closure parameters
(`fn (user) { ... }`, the goal's example) arrive with C2 inference;
capture by binding (§6.4); pipeline `x |> f(a)` (sugar, §6.3); prefix
`try e` and `e catch |err| { ... }` / `e catch { ... }` (§6.5);
unary `!e`, `-e`; binary operators per §5.1.
Context-typed (dot) forms are legal wherever the EXPECTED type is
known (binding annotations, parameter/return types, match arms, field
positions); elsewhere the qualified spelling is required (b0
simplicity; C2's inference widens this).

### §5.1 Operators and precedence
Loosest to tightest; all binaries left-associative:

    0.  e catch ...     error-replacement handler (§6.5, D-090)
    1.  |>              pipeline
    2.  or
    3.  and
    4.  == != < <= > >= comparison (non-associative: a < b < c = error)
    5.  + -
    6.  * / %
    7.  unary ! -  and  try   (prefix)
    8.  postfix: call, field, dot-construction

`and`/`or` are short-circuiting with bool operands and bool result.
Comparisons are bool-valued. `+ - * / %` are i64-only in b0 (no
string/float arithmetic). No implicit conversions exist anywhere in
Calque.

### §5.2 Patterns [b0]
`_` wildcard; `name` binding; `.Variant` / `.Variant(pat)` (variant
patterns; sub-patterns in b0: `_` or `name` only). Match arms
separated by commas; a trailing comma allowed. Exhaustiveness: a
match over a union must cover every variant or carry a `_` arm;
violations are compile ERRORS with the missing variant named as a
concrete witness (the playbook's `error: match does not cover
Job.Cancelled` shape). Redundant arms are errors (stricter is
deterministic; D-038 precedent). The scrutinee is evaluated once;
arms are tried in source order (semantically irrelevant in b0
except for bindings). [C2, per the playbook (D-090): literal payload
patterns (`.Queued(0)`), struct patterns `.{ .attempt = 1, .. }`,
guards (`pat if cond`), or-patterns `pat | pat` (no
cross-alternative bindings, enforced), `..` rest in struct patterns;
a guarded arm does NOT count toward variant coverage —
the unguarded fallback is what establishes it; unreachable/
redundant-arm severity policy documented at that freeze.]

## §6 Semantics [b0]

### §6.1 Values, lifetimes, mutation
Every value has automatic lifetimes; no user-visible allocator or
borrow bookkeeping appears in ordinary code. Immutable values
(records, unions, `Str`, `Option`, `Result`, `i64`, `bool`) have
value semantics — aliasing is UNOBSERVABLE for them and the
implementation is free to share (the M31 lesson recorded in the
freeze log).

The ONE mutable-aliasing surface in b0 is `Vector(T)`: it is a
growable array with REFERENCE semantics — aliases observe `push`
and `set`. [C2 replaces this with immutable `Vector` (structural
sharing) + an explicit builder; b0 code migrates through the
bootstrap chain, snapshots preserved (goal §5).]

`var` locals are mutable bindings; assigning a value to `var`
rebinds that name. Mutation through immutable values (fields,
variants) does not exist in b0. [C2+] `defer` scoped cleanup.

### §6.2 Evaluation order
Strict, left-to-right, exactly once: call callee then arguments
left-to-right; record initializers in written order; binary operands
left then right; pipeline §6.3; `match` scrutinee once; `try`/`catch`
operand once; `if`/`while` conditions once before the arm/body.
Effects appear only through explicit capability functions (§6.6).

### §6.3 Pipelines
`x |> f(a, b)` ≡ `f(x, a, b)`; `x |> f` ≡ `f(x)`. x is evaluated
exactly once, before the arguments. Chains associate left:
`x |> f(a) |> g(b)` ≡ `g(f(x, a), b)`. The callee may be any
callable path (`str.concat`, a fn-typed value, a closure). A
pipeline stage that fails to type-check reports at the stage.

### §6.4 Functions, recursion, tail calls
First-class functions and closures. Captures are by BINDING: a `const`
captures the value; a `var` captures the BOX (mutations visible to
the closure and vice versa); parameters behave as const captures.
Recursion and mutual recursion are unrestricted (flat top-level
scope). THE TAIL-CALL GUARANTEE: proper tail calls — direct, mutual,
and through closures — are guaranteed by the language; they must not
grow the stack (this is a semantic guarantee, implemented by the
uniform closure convention + trampoline/musttail machinery already
proven in Femto/M14/D-063). NON-tail recursion is not constant-stack
and is documented as such: the interpreter caps non-tail depth
(D-029, 1024 frames, deterministic trap); native builds are bounded
by the host stack.

### §6.5 Errors: `try`, `catch`, Option, Result
`?T` = `Option(T)` = `{ Some(T), None }`; `E!T` = `Result(T, E)` =
`{ Ok(T), Err(E) }` (builtin unions; shorthands per D-090).
PREFIX `try e` on a `Result`/error-union expression in a function
whose return type is `E!U` with the SAME E: evaluates once; on
`.Err(x)` returns `.Err(x)` from the enclosing function; on `.Ok(v)`
yields v. `try` does NOT convert unrelated error types — explicit
adaptation uses `result.mapError`. `catch` supplies a REPLACEMENT
value: `e catch |err| { ... }` (the handler receives the error and
its block value replaces the whole expression) or `e catch { ... }`
(no binding; the block may `return`/`break` outright — the
playbook's `parsePort(...) catch { return .Err(...) }` shape).
Recovery is otherwise ordinary `match` on the variants, and Option
combines through `option.*` combinators (`option.map`,
`option.unwrapOr` — no Option-`try`). Traps (§6.7) are NOT `Result`
errors: they abort deterministically and are not catchable in b0.
[C2+: `for` loops, `defer`, fixed arrays, tuples.]

### §6.6 Effects and capabilities
[b0] Purity: [roadmap C3 enforces this; b0 documents it and the
corpus follows it] a function is pure unless its signature names a
capability. The b0 capability surface: `io.print(s: Str)` (artifact/
stdout stream) and `io.eprint(s: Str)` (diagnostics/stderr stream);
both write the string's bytes EXACTLY as given — no implicit
newline, no formatting, no color. Line discipline belongs to the
program (the compiler emits its own `"\n"`).
These are the ONLY effectful primitives in b0; everything else is
pure. Capabilities are explicit parameters where a function needs
them [C3+], never ambient authority (`io.*` in b0 is the
bootstrap's explicit exemption, recorded in the freeze log and
replaced by capability parameters at C3 when the comptime sandbox
must deny it].

### §6.7 Safety semantics
Deterministic traps (abort with a canonical diagnostic + location;
NOT catchable in b0): i64 `+ - *` overflow; `/` by zero; `%` by
zero; `i64.minValue / -1` (and `%`); `vector.get`/`vector.set`
out-of-bounds; `str.sub` out-of-bounds. `str.len` is BYTES.
Overflow-checked arithmetic is preserved in ordinary builds
(goal §10); explicit wrapping operations arrive at C2 (`i64.wrapAdd`
etc., wrapping two's-complement, no UB). Division truncates toward
zero; `%` takes the dividend's sign.

### §6.8 Strings
`Str` is an immutable byte string with UTF-8 text convention. In b0
only: `str.len(s) i64` (bytes), `str.concat(a, b) Str`,
`str.eq(a, b) bool`, `str.sub(s, start, len) Str` (0-based, traps
when out of range), and `str.fromI64(v) Str` (exact for all i64;
implemented in Calque, not as a runtime primitive). String literals
are `Str` values. [C2+: chars/iteration, comparisons, `Bytes`
distinct buffer type, identifier helpers.]

### §6.9 Integers
i64 only. Literals `[0-9]+` must fit. No implicit widening/narrowing
(C2+ explicit casts). [C2+: u8..u64/i8..i32/f32/f64, checked
conversions, shifts/bit ops, wrapping ops.]

## §7 Modules, stdlib, packages

[b0] One flat compilation unit: the driver passes an ordered list of
source files; they form ONE module; duplicate top-level names are
errors; `pub` is accepted (semantics identical to private within the
unit). Stdlib in b0 = the intrinsic set of §6.6/§6.8 + `vector.*` +
`str.fromI64` (Calque source), all compiled with the unit.

[C2] Explicit imports with public boundaries, import-cycle detection,
path-independent resolution, local package dependencies + lockable
manifest. [C5] reproducible builds, clean-install/offline workflows.
[Never in this goal: an online package registry.]

### §7.3 Incrementality and the cache contract [design intent — D-089]

Incrementality is architectural, not a late optimization: the compiler
and the comptime evaluation service are PURE functions of their inputs,
and their outputs are cached under fine-grained content-addressed keys
(declaration/function/small-module grain; keys include source content,
target, capabilities, resource policy, and compiler identity). The warm
path for a small local edit is hash → cache lookup → dependency-graph
reconstruction → minimal emission/linking; failed or partial work never
enters the cache; every verification-path use re-validates cached
artifacts (BOOTSTRAP §4). Language-visible consequence: declarations
carry STABLE IDENTITIES across recompiles (compile-order and
whole-program-blob independence are requirements of the module design,
verified from C2 on) so that fine-grained invalidation is sound. The
quantitative targets live in D-089 and are measured at C4/C5, never
claimed before measurement.

## §8 comptime (C3 — roadmap; design intent frozen here)

`comptime` expressions and blocks run in Frankish's existing derived
interpreter (goal §11) — the same per-op semantics as runtime code,
never a second evaluator. [C3] Freezes: comptime syntax; which
expressions are required-comptime; type values (`type`) as
compile-time values; `comptime T: type` parameters; reflection over
fields/variants/signatures returning STRUCTURED type descriptors;
reification of comptime results into runtime constants (deterministic
encoding, cycles/sharing policy, refusal of live resources);
capabilities/budgets/cancellation/caching. The service shape and the
`frk.stage` question are D-087/D-088 scope.

## §9 Memory and runtime contract

Application profile: the existing RC strategy (D-041/D-057 ladder:
sized releases, layout descriptors, cycle collection) with automatic
lifetimes; strategy stays a lowering parameter. [C4] escape analysis
+ uniqueness reuse where evidence supports them; no-heap-allocation
contracts (`noalloc` on supported functions, reachable-call closure,
source-level call path on rejection); allocation cost reporting.
[Never: mandatory borrow annotations in ordinary code.]

## §10 Targets

Applications: the existing grid (Tier-0 triples incl. wasm32-wasi and
the s390x canary) per D-016/D-017/D-042 — behavior is
target-independent except where §12 says otherwise. The COMPILER is a
hosted program (Rust host + interpreter today); it is not required to
be freestanding. Compile-time arithmetic is defined by the TARGET's
semantics (goal §11: "target arithmetic independent from the build
host"), verified against target runtime results on 32- and 64-bit
targets.

## §11 Diagnostics

Structured before rendered. Canonical wire format (one diagnostic per
line, LF-terminated, ASCII):

    <path>:<line>:<col>: <severity>: <code>: <message>

`line`/`col` are 1-based (col = byte offset in line); severity ∈
{error, warning, note}; code = stable `CQnnnn` identifier; message is
single-line with no trailing period. Compilers MUST emit this format
on the diagnostics stream (`io.eprint` for the compiler artifact);
the CLI's rendered form (colors, source snippets, multi-span notes)
arrives at C5 and is NOT compared. Diagnostics are part of
conformance (goal §16: "Compare diagnostics and emitted canonical
artifacts").

## §12 Freeze log (v0.1)

Every entry is frozen at M36/C0 unless marked otherwise; lifts/edits
require a D-entry and a row here. "Frozen-by" records the decision.

| # | Decision | Frozen-by |
|---|---|---|
| F1 | name Calque, `.calq`, command `calque` | GOAL §3, D-085 |
| F2 | expression-oriented bodies; strict; left-to-right (§6.2) | GOAL §7, D-086 |
| F3 | `const`/`var` surface; explicit local mutation; no implicit shared mutation | GOAL §7, D-086 |
| F4 | fn syntax `fn name(params) R`; explicit types everywhere in b0 | D-086 |
| F5 | nominal records/unions via top-level consts; `struct { f: T }` / `union(enum) { V, V: T }`; construction `Type { .f = e }` / context-typed `.{ .f = e }` / `.Variant` | D-086; revised by D-090 |
| F6 | `match` with exhaustiveness+redundancy as errors; b0 pattern set | D-086 |
| F7 | operator set + precedence table (§5.1); `try` prefix, `catch` handler; postfix `?` RETIRED | D-086; revised by D-090 |
| F8 | pipeline desugar exact (§6.3) | GOAL §8, D-086 |
| F9 | propagation `try e` + replacement `e catch` (§6.5); identical-E rule in b0; `?T`/`E!T` shorthands | D-086; revised by D-090 |
| F10 | tail-call guarantee: direct, mutual, closure (§6.4) | GOAL §7, D-086 |
| F11 | safety: checked `+ - *`, trap on div/rem zero & BOUNDS (§6.7) | GOAL §10, D-086 |
| F12 | `Str` = immutable byte string; b0 str surface (§6.8) | D-086 |
| F13 | `Vector(T)` reference semantics in b0; immutable at C2 (§6.1) | D-086 |
| F14 | recursive types allowed; D-074 recref erasure generalized | D-088 |
| F15 | reserved modules `str io vector`; no shadowing them | D-086 |
| F16 | diagnostics wire format (§11) | GOAL §14, D-086 |
| F17 | compiler determinism is a hard requirement (§1) | GOAL §5, D-086 |
| F18 | b0 = ASCII source/seed channel; loud refusal above 0x7F | D-086 |
| F19 | b0 single flat unit (concatenated files); imports at C2 (§7) | D-086 |
| F20 | no implicit conversions anywhere | D-086 |
| F21 | `comptime` reuses the derived interpreter (§8) | GOAL §11, D-087 |
| F22 | RC profile + strategy-as-parameter stands (§9) | GOAL §10, D-085 |
| F23 | `io.print`/`io.eprint` are b0's only effects; capability params at C3 | D-086 |
| F24 | non-tail recursion is not constant-stack; documented (§6.4) | GOAL §7, D-086 |
| F25 | C2-visible syntax (inference, generics, floats, tuples, modules) NOT frozen yet; their freeze rows are due at M38 planning | D-086 |
| F26 | incrementality is content-addressed at declaration grain; compiler + comptime eval are pure functions; cache keys = complete semantic inputs | D-089 |
| F27 | if/while conditions paren-less; `for (x) \| v` at C2; dot-shorthand constructors/patterns | D-090 |
| F28 | the user's demo playbook (docs/calque-demo-playbook.md) is the surface-intent + C6 storyboard source; its console blocks are release targets | D-090 |
| F29 | `calque.toml` build manifest + named `[build.inputs]`, enforced at the service boundary | D-090 (C3/C5) |
| F30 | CLI surfaces: check/build/run/test/explain (+ `--explain-comptime`, `--runtime-artifact`, `--function`) | D-090 (C1/C5) |
| F31 | `compiler.compile`, `frankish.eval`, `frankish.evalSpecimen` are the host-service spellings | D-090 (C3) |
| F32 | `@noalloc` spelling; allocation claims require executable tests, never terminal decoration | D-090 (C4) |
| F33 | multiline `"""` strings, `_` numeric separators, `@intCast`, `usize`, `[N]T`, tuples, `[...]` literals, `..spread`, `test` blocks | D-090 (C2) |

Rows cite the entry that froze them; revisions cite the revising
entry (D-090 revised F5/F7/F9 after the user's demo playbook).

## §13 Version plan

v0.1 = b0 (this file). v0.2 = C2 language (inference, generics,
floats, tuples, modules, immutable collections, defer, seq). v0.3 =
C3 (comptime, types-as-values, reflection). v1.0 = C6 release
(complete §23 evidence). The canonical compiler source always
targets the newest frozen version; bootstrap snapshots keep older
versions compilable by the frozen seed (BOOTSTRAP §5). The user's
demo playbook (docs/calque-demo-playbook.md) fixes the C6 narrative
order and the surface-intent; its console blocks are release
targets, never captured-run substitutes (D-090).
