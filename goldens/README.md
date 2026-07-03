# goldens — the execution corpus

Law: L2 (byte-exact after canonicalization; blessing requires a commit
message justification), L3 (every runner must agree on every case).
Format ruled in D-027; comparison contract in docs/canon.md.

## Layout

    goldens/<suite>/<case>/
      case.mlir       the program (this suite: upstream dialects only)
      expected.out    blessed canonical output (committed)
      output.actual   written on mismatch for diffing (gitignored)

A directory is a case iff it contains `case.mlir` or `case.ml`.
Suites are just directories; `upstream/` holds MLIR programs over
func/arith/scf/cf; `adt/` and `closure/` exercise the kernel dialects;
`mem/` exercises the
allocation surface under every strategy runner; `ml_core/` holds specimen programs — plain OCaml files that must (a)
define `let main () = <int expr>` and (b) run verbatim under the
`ocaml` oracle, which appends `print_int (main ())`. Directives in .ml
files spell `(* frk-case: ... *)`. `ts0/` holds TypeScript TS-0
programs (`case.ts`): compiled through the loanword producer
(node + tsc), entry is void, the OUTPUT is the console.log stream,
and node itself is the oracle; number printing obeys canon §6's
fence. `repl/` holds scripted shell transcripts (`transcript.in`). Keep integer results under 2^62
(OCaml's 63-bit ints; the divergence rule lives in the ml_core
MANIFEST).

## Entry protocol note (AOT)

The AOT/grid runners rename the entry function to `frk_entry` before
lowering (the C shim owns `main`), so entry functions must be
externally-invoked-only — nothing else in the module may call the
entry symbol (D-042).

## Case directives

Optional `// frk-case: key=value` comment lines anywhere in case.mlir:

    // frk-case: entry=main      entry function symbol   (default: main)
    // frk-case: result=i64      return rendering        (default: i64; v0's only type)
    // frk-case: runners=a,b     applicable runners      (default: all — SPEC §7.2)

Unknown keys and unsupported values are errors — a typo'd directive must
never silently become a default. `runners=` exists for op sets that are
ahead of some execution path (adt before its lowering); skips are
reported per case, a corpus where everything skips a runner is an error,
and a case no registered runner can execute is red in `make diff`.

## UB is inadmissible

Cases must be free of MLIR-level undefined behavior — division by zero,
signed-division overflow, non-positive scf.for steps, and kin. The
reference interpreter traps deterministically on these (D-029); native
paths do whatever LLVM does; nothing comparable comes out. Wrap-around
integer arithmetic (no overflow flags) is *defined* — modulo 2^n — and
fair game.

## Entry protocol (v0)

The entry function takes no arguments and returns one `i64`, rendered per
docs/canon.md §2. It must carry `attributes {llvm.emit_c_interface}` —
the JIT invokes through the generated `_mlir_ciface_` wrapper. Helper
functions don't need the attribute.

## Running and blessing

    make test    runs the corpus (inside cargo test) among everything else
    make bless   rewrites every expected.out from current output — L2:
                 the commit message must say *why* the bytes changed

Never bless to silence a diff you don't understand (AGENTS.md L2).
