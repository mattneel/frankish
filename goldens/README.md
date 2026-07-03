# goldens — the execution corpus

Law: L2 (byte-exact after canonicalization; blessing requires a commit
message justification), L3 (every runner must agree on every case).
Format ruled in D-027; comparison contract in docs/canon.md.

## Layout

    goldens/<suite>/<case>/
      case.mlir       the program (this suite: upstream dialects only)
      expected.out    blessed canonical output (committed)
      output.actual   written on mismatch for diffing (gitignored)

A directory is a case iff it contains `case.mlir`. Suites are just
directories; `upstream/` holds programs over func/arith/scf/cf used to
prove the harness and, from M2, the interpreter.

## Case directives

Optional `// frk-case: key=value` comment lines anywhere in case.mlir:

    // frk-case: entry=main      entry function symbol   (default: main)
    // frk-case: result=i64      return rendering        (default: i64; v0's only type)

Unknown keys and unsupported values are errors — a typo'd directive must
never silently become a default.

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
