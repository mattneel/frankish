# STATE — frankish live handoff

Updated: 2026-07-02 (M0+M1+M2 session)
Phase: M2 complete (tag m2-done); M3 not started.
Tree: green — `make test` passes; clean-clone scripts/ci.sh verified
(exit 0); `make diff`: 8 cases, interp vs jit, 0 divergent.

## Next action
M3 frk.adt, step 0 DONE (registration ruled: D-030 two-tier; evidence
in crates/frk-dialects/tests/registration.rs). Next: build the Tier-B
native shim skeleton — a small C++ ODS library (cmake, driven from
make setup/build via the existing MLIR_PREFIX; cmake+ninja+headers all
ship with llvm-22-dev/libmlir-22-dev on apt and with brew llvm@22)
exposing a mlirDialectHandle that frk_core::context() registers.
Define frk_adt there: match (region arms) + frk_adt.yield (a real
Terminator-trait op) + make/tag/extract (these three are IRDL-shaped,
but keep the dialect whole in one place). Smoke first (L1): parse +
verify a match, positive and negative. Then the §3 ladder: K2 Eval
impls (frk-interp trait is waiting), K3 decision-tree lowering
(Maranget, D-025, own goldens), exhaustiveness via
rustc_pattern_analysis behind a trait boundary.
Exit: K1–K7 checked; 3-way goldens green.

## In flight
Nothing.

## For the human
- Review ⚑ D-005 (host stack ruling) in docs/DECISIONS.md — made on your
  behalf. Evidence through M2 supports it: melior 0.27.2 builds, JITs,
  walks IR generically (interpreter), runs pass pipelines, and prints IR
  against LLVM/MLIR 22.1.8 with no binding gap encountered.

## Milestone log
m0-done — Shipped: SPEC §12 workspace skeleton (7 crates + sandbox/);
versions.env as the single pin point (RUST_TOOLCHAIN=1.96.0,
LLVM_MAJOR=22, LLVM_VERSION_TESTED=22.1.8, MELIOR_VERSION=0.27.2) with
scripts/check-pins.sh asserting every mirror stays in sync; make
setup|build|test|ci over POSIX scripts (L6); frk-core context() with
eager dialect loading (LANDSCAPE segfault warning); smoke verifier
jit_add_i64 — builder-API add(i64,i64) → convert-to-llvm →
ExecutionEngine, asserts 40+2=42; clean-clone make ci green. Learned:
apt.llvm.org hosts need libmlir-22-dev AND libpolly-22-dev
(llvm-config --libs names Polly; tblgen-rs fails to link without it —
the cairo-native footgun; setup.sh doctor checks both). melior 0.27
API notes: invoke_packed requires llvm.emit_c_interface on the func;
ExecutionEngine::new takes 5 args (enable_pic new); verify() lives on
the OperationLike trait; conversion passes are macro-generated,
create_to_llvm() is the one-shot lowering. Cheats awaiting promotion:
none — no dialect work happened; frnksh is an honest placeholder that
prints its pre-M8 status.

m1-done — Shipped: docs/canon.md v0 + frk_harness::canon (the single
§7.4 filter; CRLF/CR→LF, one trailing LF, i64 rendering, float policy
pinned); golden engine (goldens/<suite>/<case>/ layout, strict
`// frk-case:` directives, check writes gitignored output.actual,
bless reports changed/unchanged, zero-case corpus is an error);
6-case upstream corpus (arith/scf.for-i64/scf.if/cf.cond_br/func.call)
with expected outputs computed by hand before the runner ran, syntax
validated via mlir-opt 22; differential scaffold (diff_corpus, BTreeMap
matrix, L3 escalation text in the report) live on the corpus via
default_runners(); stage dumps (00-parsed + NN-<pass> per shared
pipeline table, out dir recreated whole, never goldened — docs/
stages.md); frnksh test/bless/diff/emit + make bless|diff. Ledger:
D-027 (custom golden runner, format, entry protocol, pipeline),
D-028 (single-pass-manager stage dumps). Learned: scf.for takes i64
induction directly (no index detour needed); the pipeline
scf-to-cf → convert-to-llvm → reconcile-unrealized-casts covers all
six cases; melior pass constructors are macro-named from C API symbols
(strip Conversion/Convert/Pass, snake-case). Cheats awaiting
promotion: none. Known wart (accepted for v0): `emit --stages` default
out dir uses the source file stem, so corpus files (all named
case.mlir) collide on out/stages/case — pass --out for those.

m2-done — Shipped: frk-interp — the K2 Eval trait + derived
interpreter over func/arith/scf/cf (value domain: two's-complement
ints ≤64 bits; per-call Frame keyed by MLIR value identity; multi-
block CFG execution; single-block structured regions; symbol-indexed
func.call). Semantics ruled in D-029: total & deterministic — UB
traps (div-by-zero, signed-div overflow, non-positive for-step), call
depth caps at 1024; corollary: corpus must be UB-free (goldens/README
rule added). Harness: InterpRunner on a STACK_SIZE thread;
default_runners()=[interp, jit]; reference_runner()=interp (D-008
assumed in full — blessing writes reference bytes). Corpus at 8 cases
(+fib_rec recursion, +add_wrap modulo-2^64 canary). L3 IS ENFORCED IN
CI: make test two-way-diffs every golden; make diff prints the
matrix (8 cases, 0 divergent). Learned: interpreted frames cost a few
KiB of host stack each — depth-ceiling programs need the STACK_SIZE
thread (libtest's 2 MiB default overflows); melior successor operand
splitting needs no segment attribute (destination arg counts
suffice); i1 sign-extends to -1 (tested, it will bite someone).
Cheats awaiting promotion: none — but note the interpreter recurses
on the host stack; if a specimen needs unbounded depth, that's a
rework flag, not a knob.

## Handoff template (copy for every session end)
    Session end: <date>
    Milestone/step: <where>
    Green? <yes/no — if no, why and where>
    Did: <bullets>
    Next: <single concrete action>
    Landmines: <anything the next agent must not step on>

## Session log

    Session end: 2026-07-02 (fourth entry this session)
    Milestone/step: M3 step 0 — dialect-registration ruling (D-030)
    Green? yes — make test green (registration spike adds 5 tests)
    Did:
    - spiked IRDL end to end via mlir-opt AND melior: definitions,
      dynamic parametric types, generated verifiers (arity + type
      variables + attribute kind), builder-path construction — all work
    - found the ceiling: LLVM 22 IRDL has no trait declarations —
      dynamic ops can't be terminators or carry successors; that blocks
      region-based match under pure IRDL
    - verified the C++ escape hatch is provisioned (headers +
      MLIRConfig.cmake + cmake/ninja on apt; brew ships same)
    - ruled D-030 (Tier A IRDL / Tier B C++ shim); pinned the ceiling
      in LANDSCAPE + a watch item to re-fold B into A
    Next: Tier-B shim skeleton (cmake lib + frk_adt ODS + handle
    registration in frk_core::context()), smoke-verified, then K2–K7
    Landmines:
    - `irdl.is i64` constrains attribute-EQUALS-type-i64; use
      `irdl.base "#builtin.integer"` for "an integer attribute"
    - irdl.parametric needs fully nested refs (@dialect::@type)
    - the registration spike test doubles as D-030's standing evidence;
      if an MLIR bump breaks it, revisit the ruling before patching it

    Session end: 2026-07-02 (third entry this session)
    Milestone/step: M2 complete, tagged m2-done
    Green? yes — make test green (incl. two-way diff on all 8 goldens);
    clean-clone scripts/ci.sh exit 0
    Did:
    - frk-interp: Eval trait, value domain, CFG/structured execution,
      upstream evaluators (arith/func/cf/scf), 20 verifiers
    - D-029 (trap semantics + depth ceiling); UB-free corpus rule
    - InterpRunner wired as reference; corpus → 8 cases; L3 live in CI
    Next: M3 frk.adt — but FIRST settle the custom-dialect registration
    mechanism (IRDL vs unregistered ops vs C++ shim) as a D-entry; it
    shapes everything K1 onward
    Landmines:
    - deep interpretation needs frk_interp::STACK_SIZE threads; never
      call interpret_entry on a default 2 MiB test thread for recursive
      programs
    - blessing now writes INTERPRETER bytes; if jit disagrees afterward
      that is an L3 first-rank finding, not a blessing mistake
    - i1 as_signed() is -1, not 1 — use as_bool()/as_unsigned() unless
      you really mean sign extension

    Session end: 2026-07-02 (second entry this session)
    Milestone/step: M1 complete, tagged m1-done
    Green? yes — make test green; clean-clone scripts/ci.sh exit 0
    Did:
    - canon contract + filter; golden engine + 6-case corpus; diff
      scaffold; stage dumps; frnksh subcommands; make bless|diff
    - D-027, D-028 appended; goldens/README.md + docs/canon.md +
      docs/stages.md written
    Next: M2 — Eval trait + interpreter over func/arith/scf/cf; append
    it to default_runners() and flip reference_runner() (D-008); exit =
    L3 enforced in CI
    Landmines:
    - run cargo via make (exports MLIR_SYS_220_PREFIX/TABLEGEN_220_PREFIX)
    - melior is alpha: build contexts via frk_core::context() only
    - never bless without an L2 justification line in the commit; the
      bless report prints changed/unchanged per case to keep you honest
    - goldens comparison happens ONLY through frk_harness::canon — do
      not add a second normalization anywhere

    Session end: 2026-07-02
    Milestone/step: M0 complete, tagged m0-done
    Green? yes — make test green; clean-clone make ci exit 0
    Did:
    - workspace skeleton per SPEC §12; versions.env + check-pins.sh
    - melior =0.27.2 pinned; frk-core context(); smoke jit_add_i64 green
    - installed libmlir-22-dev + libpolly-22-dev on this host; setup.sh
      doctor now checks for both so the next machine gets named hints
    - clean-clone make ci verified; README status refreshed; tagged; pushed
    Next: M1 harness v0 (SPEC §7): golden runner + bless + docs/canon.md
    v0 + stage dumps + differential scaffold
    Landmines:
    - run cargo via make (it exports MLIR_SYS_220_PREFIX /
      TABLEGEN_220_PREFIX); bare cargo without those exported will pick a
      wrong or absent LLVM
    - melior is alpha: build contexts via frk_core::context() (eager
      dialect loading) — unloaded-dialect access can segfault (LANDSCAPE)
