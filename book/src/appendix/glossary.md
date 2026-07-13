# Appendix C: Glossary

Terms as *this repository* uses them. Where a term has a broader meaning
elsewhere, the definition here is frankish's.

**bless** — Rewrite golden expectations from the reference runner (`make
bless`). Law L2 forbids blessing a diff you don't understand; the commit
must say *why* the bytes changed.

**canary** — The s390x grid run (`make canary`), a **big-endian** check
(D-017). Every byte-layout assumption meets a big-endian target before it
can be believed portable.

**canon / canonicalization** — The byte-normalization rules
(`docs/canon.md`) applied before golden comparison: `LC_ALL=C`, number
formatting specified to the digit. "Byte-exact after canonicalization"
(L2) means byte-exact *through this filter*.

**dialect (kernel)** — An MLIR dialect in frankish's PL-idiom middle layer
(`frk_adt`, `frk_closure`, `frk_mem`, `frk_str`, `frk_bstr`, `frk_dyn`,
`frk_ctl`). Registered by IRDL runtime loading only (D-031), shipped whole
under the K1–K7 contract (D-007).

**differential law** — L3: the derived interpreter is the reference
semantics; every compiled path must agree byte-exactly; specimen upstreams
are third oracles; a disagreement halts the feature.

**dyn / fat value** — A `!frk_dyn.dyn`: a two-word `{tag: i64, payload:
i64}` pair (D-051), the runtime representation of a dynamically-typed
value. Tags: nil=0, bool=1, num=2, str=3, table=4, fun=5.

**extraction** — Promoting machinery a specimen built privately down into
the kernel, so later specimens inherit it (the decision-tree compiler,
M6). The milestone note's "extraction report" is the forcing thesis
working.

**fence** — A frozen edge of a specimen's subset (L5). Fence lists are
law, not TODO lists; crossing one requires a ledger entry.

**golden** — A test case directory (`goldens/<suite>/<case>/`) holding the
source (`case.mlir`/`.ml`/`.ts`/`.lua`/`.scm`), `expected.out`, and
optional directives. Simultaneously the verifier and the spec (L1).

**IRDL** — MLIR's dialect-definition dialect. frankish registers all
kernel dialects by loading IRDL at runtime (`melior`'s
`load_irdl_dialects`) — no C++/TableGen dialects anywhere (D-031).

**loanword** — The content-addressed typed-AST interchange (D-024/D-046):
canonical sorted-key JSON, SHA-256 content id, cryptographic refusal on
mismatch. The frontend boundary for elaboration-carrying languages
(TS-0).

**milestone** — A unit of scoped work (`m0`…`m15`), each planned against
exit criteria, verified under L1–L3, noted in `STATE.md`, tagged, pushed
(L8). Also the name of the project's development loop.

**musttail** — The LLVM `TailCallKind` frankish's native tail-call pass
sets on qualifying tail-shaped, identical-signature, direct calls (D-059),
guaranteeing a frame-replacing jump.

**oracle** — An independent authority a golden's output is checked
against: the reference interpreter, or a specimen's pinned upstream
(`ocaml`, `node`, `lua5.1`, `chibi-scheme`).

**pack (convention)** — femto_lua's uniform function signature
`fn<[arr<dyn>], [arr<dyn>]>` (D-058): one argument pack in, one values
pack out — flexible arity and multiple returns without variadics.

**prompt / abort** — The `frk_ctl` v0 escape-continuation ops (κ_frk):
`prompt` installs a fresh token and runs a body; `abort` unwinds to the
matching live prompt. Drop-clause continuations only in v0.

**promotion** — See *extraction*.

**runner** — A named way to execute a golden and produce raw output before
canonicalization: `interp`, `jit`, `jit-rc`, `ocaml`, `node`, `lua`,
`scheme`, `repl`, plus the AOT grid.

**specimen** — A real language frozen to a pinned subset, implemented to
force kernel dialects into existence and held against its upstream oracle
(L5). Four exist: ml_core, TS-0, femto_lua, r7rs_core.

**stage dump** — A per-pass snapshot of the IR (`frnksh emit --stages`),
in MLIR's default textual form (D-028). Dumps are never goldened; they are
a debugging lens.

**strategy (memory)** — A lowering *parameter*, not a language feature
(D-041): the same kernel IR lowers under **arena** (bump, no frees) or
**rc** (reference counting + cycle collection). `jit` vs `jit-rc` are the
two, held byte-identical.

**trampoline** — The reference interpreter's tail-call loop (D-059): a
tail-shaped call *replaces* the current frame instead of recursing, so the
depth cap counts non-tail entries only.

**twin (runtime)** — One of the two implementations of `frk-rt`: Rust
(canonical, for the in-process JIT) and C (`crates/frk-rt/c/frk_rt.c`,
cross-compiled per architecture for the AOT grid). Held behaviorally equal
by the differential law (D-042).

**verifier-first** — L1: no implementation lands without its verifier
landing in the same commit or earlier. The verifier is the spec; the
implementation is fungible.
