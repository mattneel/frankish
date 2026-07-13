# Appendix A: A Tour of the Ledger

The complete decision ledger, `docs/DECISIONS.md`, in numeric order.
Sixty rulings span `D-001`–`D-061`. **`D-005` was never issued**, and
**`D-030` was struck by the human and superseded by `D-031`** (the
two-tier registration scheme replaced by IRDL-runtime-loading-only). The
ledger is append-only: entries are never edited in place; a decision that
changes is *superseded* by a later entry that says so.

For the method behind the ledger — the L4 protocol, entry anatomy, and
worked examples — see [The Decision Ledger](../method/ledger.md).

| # | Area | Ruling |
|---|---|---|
| D-001 | `name` | Project is **frankish** — lingua franca nod; survives-as- loanwords etymology is the thesis. |
| D-002 | `cli` | Binary is **frnksh**; bare invocation = the REPL ("the frankish shell"); `frankish` ships as alias symlink. |
| D-003 | `format` | Typed-AST interchange is named **loanword**. |
| D-004 | `docs` | Thin constitution (AGENTS.md) + per-specimen MANIFESTs, not a monolith. |
| D-006 | `dialects` | v1 users compose framework-owned dialects only; user- defined dialects deferred to v2 via IRDL runtime loading. |
| D-007 | `contract` | Every kernel dialect ships K1–K7 (SPEC §3); verifier and goldens land first (law L1). |
| D-008 | `semantics` | The derived interpreter is reference semantics; JIT/AOT must byte-match on goldens; specimen upstreams are third oracles (law L3). |
| D-009 | `specimens` | Order: ml_core → TS-0 (demo/loanword forcing) → femto_lua → r7rs_core; c_oracle rig early and parallel, as oracle not frontend. |
| D-010 | `specimens` | Subsets are named, versioned, frozen against a pinned upstream; admission rule = a feature enters only carrying a new idiom; fence lists are law (L5). |
| D-011 | `ctl` | Default error lowering is result-passing; unwinding is a Tier-2 opt-in strategy of the same ops. |
| D-012 | `ctl` | Effects/handlers lower via evidence passing (Koka-style); the Rocq handler calculus is the semantic anchor and source of verifier obligations. |
| D-013 | `ts` | `number` is f64, specimen-faithful; i32/i64 annotations are a named profile extension (a frankish dialect of TS), not the specimen. |
| D-014 | `profiles` | Sealed-world (closed unions, final classes → devirt) is a profile switch, default off. |
| D-015 | `dyn×contract` | Gradual boundary casts are contract ops with blame payloads — gradual typing = dyn × contract, no fourth mechanism. |
| D-016 | `wasm` | wasm32-wasi via the normal LLVM path is the supported wasm target (Tier 1, linear-memory rt); WasmGC deferred. |
| D-017 | `portability` | Portability is a CI grid (specimen × triple), executed via qemu-user + wasmtime; s390x is the big-endian canary. |
| D-018 | `toolchain` | Cross linking via bundled `zig cc` driver; clang+sysroots documented as fallback. |
| D-019 | `frontends` | Borrowed specimens ride tree-sitter/upstream parsers as scaffolding; native readers (pratt/sexp/enforest/phrase) are reserved for original languages. |
| D-020 | `types` | Trait/typeclass solving is dictionary-passing only in v1; declarative type-system genericity is out of scope. |
| D-021 | `scope` | Lazy evaluation is a v1 non-goal. |
| D-022 | `scope` | LSP/editor tooling is a v1 non-goal; pipeline stays pure and coarse-grained so incrementality is addable. |
| D-023 | `agents` | Agent-portability laws (L6–L7): AGENTS.md canonical with CLAUDE.md symlink; all workflows via make; STATE.md handoff mandatory; no vendor feature i… |
| D-024 | `loanword` | Canonical encoding v0 = sorted-key canonical JSON, UTF-8, SHA-256 content id; CBOR revisited at freeze (M9) with measurements. |
| D-025 | `adt` | Pattern-match compilation is Maranget decision trees; niche/ tag-packing is a separate, separately-goldened pass. |
| D-026 | `dyn` | Structural interface dispatch uses Go-style itabs (cached interface/type pairs); inline caches deferred. |
| D-027 | `harness` | Golden runner is custom, not insta: corpus at goldens/&lt;suite&gt;/&lt;case&gt;/ (case.mlir + expected.out + gitignored *.actual), directives as `// frk-case:… |
| D-028 | `harness` | Stage dumps v0 = one single-pass PassManager per pipeline entry, snapshots in MLIR default textual form, out dir recreated whole, dumps never golde… |
| D-029 | `interp` | The derived interpreter is total and deterministic: MLIR-level UB (div by zero, signed-div overflow, non-positive scf.for step) traps; call depth c… |
| D-030 | `dialects` | Kernel dialect registration is two-tier. |
| D-031 | `dialects` | **Supersedes D-030 (struck by the human, 2026-07-02).** Kernel dialects register via IRDL runtime loading ONLY; there is no C++ ODS shim anywhere i… |
| D-032 | `adt` | K3 v0 lowering is an external MLIR pass (melior create_external) in the shared pipeline table — "lower-frk-adt", stage 01 in every dump. |
| D-033 | `harness` | Golden cases may declare runner applicability (`// frk-case: runners=a,b`; default all) per SPEC §7.2 "all applicable runners" — for op sets ahead… |
| D-034 | `adt` | Decision-tree pass v0 (D-025 executed): pure matrix→tree compilation in frk-dialects (adt_dtree) — pattern language = variant / product / int-liter… |
| D-035 | `closure` | v0 strategy rulings, made ahead of code. |
| D-036 | `dialects` | **No variadic operand/result groups in kernel dialects** — hardening D-031 with a newly proven ceiling: LLVM-22 IRDL constraint variables bind once… |
| D-037 | `dialects` | The kernel lowering is ONE pass ("lower-frk-kernel", superseding D-032's per-dialect packaging; representation and fences unchanged): adt products… |
| D-038 | `ml_core` | ⚑ M5 frontend rulings; items (1),(2),(6) touch the ratified manifest's surface and deserve human review. |
| D-039 | `front` | Green-tree decision (SPEC §15, due M5, decided at M6 with evidence): DEFERRED with a named trigger. |
| D-040 | `specimens` | M6 retrospective fires D-009's revisit: the order is CONFIRMED. |
| D-041 | `mem` | ⚑ frk.mem v0 surface + strategy knob, designed to retire four ledgered debts as one design (D-032 boxed reps, D-035 arena discipline and by-ref cap… |
| D-042 | `grid` | The AOT/cross protocol (M7 second half). |
| D-043 | `repl` | The shell's semantics (M8; SPEC §9 as amended by D-044.4). |
| D-044 | `human-review` | First ⚑-queue adjudication (2026-07-03), with dispositions from the human, recorded verbatim in effect: (1) D-041 RATIFIED — v0 rc without liveness… |
| D-045 | `repl` | Amendment to D-043's revisit clause (human directive, 2026-07-03): revisit ADDITIONALLY when the shell can observe effects (IO or cross-line identi… |
| D-046 | `loanword` | loanword v1 FROZEN (M9; SPEC §6.3, D-024 executed). |
| D-047 | `ts0` | TS-0 slice conventions (M9). number = f64 (D-013 faithful; the FIRST float in the kernel — it enters through the admission rule as the idiom ml_cor… |
| D-048 | `front` | D-039's hard M9 trigger fires and resolves: the green tree is NOT adopted. |
| D-049 | `ts0/mem/str` | Strings + arrays for TS-0 manifest completion (M9 second half). |
| D-050 | `human-review` | Second review integration (2026-07-03, arrived mid-implementation of the strings/arrays session): (1) noImplicitReturns: true joins the producer op… |
| D-051 | `dyn` | The tagging fork (due at M10 per SPEC §4.5): frk.dyn v0 uses FAT VALUES — a two-slot {tag: i64, payload: i64} pair, riding the exact machinery clos… |
| D-052 | `femto_lua` | MANIFEST ratified (M10 exit item) + the Lua string ruling. |
| D-053 | `gc` | The M10 GC gate is decided: rc + cycle collection (Bacon–Rajan trial deletion) over the shipped rc strategy; MMTk stays the Tier-2 slot. |
| D-054 | `m11` | The human picked the recommended track (2026-07-03, "Do it"): femto_lua implementation INTERLEAVED with the GC ladder, named M11. |
| D-055 | `gc/canon` | Third review integration (2026-07-03): the M10 rulings endorsed; two directives executed. |
| D-056 | `bstr/dyn` | The femto_lua kernel prerequisites (M11 bar 3 design; executes D-052's deferred representation choice). |
| D-057 | `m12/gc` | The human picked the GC ladder ("Do it", second time). |
| D-058 | `m13/lua` | femto_lua v0.2 ("Continue" ⇒ queue order, L4). |
| D-059 | `m14/ctl` | Tail calls as law, first rung ("Keep going" ⇒ queue order; r7rs is queue-top but its OWN stub gates ratification on the ctl effects design, and SPE… |
| D-060 | `m15/ctl` | The Rocq-anchor delegation, resolved by the human: "Why do I need to provide the calculus? You can do it just fine." §4.4's anchor is satisfied IN-… |
| D-061 | `m15/ctl` | The frk.ctl v0 native lowering, settled after a 3-designer+judge panel (transcript in the session; judge chose PASS-OVER-LLVM). |

## How to read the ledger

- **Append-only.** No entry is edited after landing; supersession is
  explicit (`D-031` opens by citing that it supersedes `D-030`).
- **One ruling, its rationale, its revisit condition.** Many entries end
  with `Revisit: …` — the named trigger under which the decision should be
  reopened (`never`, a milestone, or a concrete event). An unruled,
  blocking fork is decided on the spot and logged; the project never
  stalls on adjudication (L4).
- **The `⚑` marker** flags entries whose surface touches a ratified
  manifest or otherwise deserves human review; those are batched to the
  human at milestone boundaries and their dispositions recorded verbatim
  (see the `human-review` entries `D-044`, `D-050`).
