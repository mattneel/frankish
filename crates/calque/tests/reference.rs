//! The M37 reference-emission tests (D-092; law L1 — these land with
//! the runtime rows they exercise). Hand-written kernel IR, run
//! through the full host path (marker check → compose → parse →
//! verify including the D-062 rt-declaration projection → interp).
//!
//! The seed and calq0 must reproduce this BEHAVIOR; byte-identical IR
//! is not required (BOOTSTRAP §4 compares like with like under the
//! fixed-point rules — the seed and the compiler are different
//! implementations).

use calque::run_artifact;

#[test]
fn fib_reference_runs_end_to_end() {
    let outcome = run_artifact(include_str!("assets/fib_reference.mlir"))
        .expect("fib reference must run");
    assert_eq!(
        outcome.values[0].as_signed().expect("i64 result"),
        55,
        "fib(10) = 55 (hand-derived from SPEC semantics)"
    );
    assert_eq!(outcome.output, "", "the reference prints nothing");
}

#[test]
fn print_is_byte_exact_with_no_implicit_newline() {
    let outcome =
        run_artifact(include_str!("assets/print_probe.mlir")).expect("print probe must run");
    assert_eq!(outcome.values[0].as_signed().expect("i64 result"), 0);
    assert_eq!(
        outcome.output, "abc",
        "io.print writes exactly the bytes given (SPEC §6.6)"
    );
}

#[test]
fn trap_is_deterministic_and_carries_the_composed_message() {
    let error = run_artifact(include_str!("assets/trap_probe.mlir"))
        .expect_err("the trap probe must trap under the interp");
    assert!(
        error.contains("calque trap: demo.calq:3:9: error: CQ0001: integer overflow (i64 add)"),
        "{error}"
    );
    assert!(error.contains("D-088"), "{error}");
}

#[test]
fn artifact_marker_is_enforced() {
    let bad = "// not-a-calque-artifact\nfunc.func @main() -> i64 { %z = arith.constant 0 : i64\n  return %z : i64 }";
    let error = run_artifact(bad).expect_err("a missing marker must be refused");
    assert!(error.contains("must begin with"), "{error}");
}
