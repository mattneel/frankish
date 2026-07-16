# Authoring Surfaces: Intrinsics and the Runtime ABI

Defining a language takes more than a grammar and a code generator. Two
pieces every language needs had, until M17, no authoring surface in
frankish — they existed as conventions buried in code:

- **Intrinsics** — the language's primitive operations above the kernel
  ops but below user code: print protocols, truthiness, coercions,
  stdlib seeds. These were "synthesized IR helpers" (D-056.2): melior
  builder code inside each emitter, unreviewable as IR, invisible to
  the harness as units.
- **The runtime ABI** — every `frk_rt_*` function was authored two to
  *four* times (Rust twin, C twin, interp builtin, JIT capture shim),
  with the signatures held equal by discipline alone. The M15
  `display_bool` bug — `u8` in the twins, `i64` at the call site,
  caught only by wasm's import checker at grid time — was the witnessed
  cost. Even *enumerating* the ABI required parsing two languages.

D-062 made both first-class, after an adversarial design panel. The two
surfaces share one philosophy: **author once, as data; derive or check
everything else.**

## Surface A — intrinsics modules

A language's intrinsics are now kernel IR in a `.mlir` file shipped with
its frontend (SPEC §6.6), embedded via `include_str!`. Compilation
parses the intrinsics file as the **seed module**, and the emitter
appends the program's functions into it:

```rust
let module = crate::intrinsics::seed_module(
    context, "scheme", include_str!("intrinsics.mlir"))?;
```

Because intrinsics are ordinary functions in ordinary kernel IR,
everything composes by construction: they pass MLIR and frankish
semantic verification like any module; the interpreter evaluates them
(K2) and the pipeline lowers them (K3) with zero new machinery; and
they are reviewable, diffable *text* — a reviewer reads the actual IR,
not builder code that constructs it.

Two migrations landed with the surface: scheme's display protocol
(fully — the builder code is deleted), and femto_lua's nine plain-dyn
protocol helpers. The panel contributed the **sequencing rule**: the
`_v` pack wrappers and iterator protocol stayed emitter-built at first,
because their signatures rode the closure calling convention that the
uniform-signature work (D-059's gap) was about to rewrite. The rule
paid off exactly as written: M18 rewrote those signatures once, in
builder code — and M20 (D-065) then completed the migration, moving
the wrappers, the iterator protocol, the string module, and the
metatable index helper into the intrinsics file and **deleting
`emit_helpers` entirely**. The lua emitter now builds zero helper IR:
the protocol library is ~440 lines of reviewable kernel IR that seeds
every compilation.

## Surface B — the runtime ABI registry

`crates/frk-abi` is one declarative table with a row per runtime
symbol: name, argument and return types in an eight-variant ABI
vocabulary, the owning lane (per-language runtime extensions are
first-class rows), how the JIT binds it (real function, capturing
shim, or not linked), and where the interpreter gets its semantics.
The vocabulary encodes the ABI laws as types — sizes are 64-bit on
every target, and `PtrPayload` (the opaque managed-payload pointer)
renders as `void *` in C and `*mut u8` in Rust, the one deliberate
asymmetric mapping.

Every consumer now derives from the registry or is compile-time
checked against it:

| Layer | Enforcement |
|---|---|
| Rust twin | build-script-generated typed fn-pointer assertions — a drifted signature is a compile error |
| C twin | a generated header (`frk_rt_abi.h`, checked in, drift-tested, `make abi` regenerates) included by `frk_rt.c` — the C compiler enforces the contract at every compile, on every grid triple |
| JIT capture shims | generated typed assertions per Capture row (the panel's strongest finding: shims are registered by type-erased pointer — the one layer the twins' checks didn't reach) |
| Kernel lowering | extern declarations *derived* from the registry (the hand-written type tables are deleted), with dedup against symbols an intrinsics file already declares |
| Every module | the semantic verifier projects each bodyless `frk_rt_*` declaration onto its registry row (class-level, with the `i1`/`i8` ↔ `u8` widening rule pinned) — covering frontends' hand declarations and intrinsics files uniformly |

Each layer carries an L1 witness. The sharpest is the tamper test: it
**replays the M15 bug on purpose** — compiles a definition whose
signature contradicts the registered contract — and asserts the C
compiler refuses it with `conflicting types`. The bug class did not get
harder to write; it became *impossible to compile*.

## What the surfaces caught on day one

The value showed up before the milestone closed. The generated header's
first compile against the C twin found **eleven functions of real
latent drift** (`void *` in C where Rust said `uint8_t *`) — harmless
today, but exactly the class that becomes a silent grid failure the day
a width changes. The registry's `NotLinked` column exposed three print
functions as dead exports referenced by nothing. And the verifier's
declaration check now guards a boundary nobody had been checking:
loanword's hand-written `frk_rt_*` declarations.

The deeper point is the same one the [differential
law](../method/differential.md) makes: frankish does not ask
contributors to be careful. It asks the registry to be right, and makes
everything else refuse to compile until it agrees.
