//! The M37 reference-emission tests (D-092; law L1 — these land with
//! the runtime rows they exercise). Hand-written kernel IR, run
//! through the full host path: marker check → compose → parse →
//! verify (including the D-062 rt-declaration projection) → interp.
//!
//! The seed and calq0 must reproduce this BEHAVIOR; byte-identical IR
//! is not required (BOOTSTRAP §4 compares like with like under the
//! fixed-point rules — the seed and the compiler are different
//! implementations).

use frk_interp::Value;

fn run(artifact: &str) -> Result<(Vec<Value>, String), String> {
    calque::check_artifact_marker(artifact)?;
    let context = frk_core::context();
    frk_dialects::register(&context).map_err(|error| format!("register: {error}"))?;
    let text = calque::compose_unit(artifact);
    let module = melior::ir::Module::parse(&context, &text)
        .ok_or_else(|| "artifact text failed to parse".to_string())?;
    frk_dialects::verify(&context, &module).map_err(|error| format!("verify: {error:?}"))?;
    let mut interp =
        frk_interp::Interp::new(&module).map_err(|error| format!("interp: {error}"))?;
    frk_dialects::register_eval(&mut interp);
    frk_harness::runner::register_protocol_builtins(&mut interp);
    let values = interp
        .eval_function("main", &[])
        .map_err(|error| error.to_string())?;
    Ok((values, interp.take_output()))
}

#[test]
fn fib_reference_runs_end_to_end() {
    let (values, output) =
        run(include_str!("assets/fib_reference.mlir")).expect("fib reference must run");
    assert_eq!(
        values[0].as_signed().expect("i64 result"),
        55,
        "fib(10) = 55 (hand-derived from SPEC semantics)"
    );
    assert_eq!(output, "", "the reference prints nothing");
}

#[test]
fn print_is_byte_exact_with_no_implicit_newline() {
    let (values, output) =
        run(include_str!("assets/print_probe.mlir")).expect("print probe must run");
    assert_eq!(values[0].as_signed().expect("i64 result"), 0);
    assert_eq!(
        output, "abc",
        "io.print writes exactly the bytes given (SPEC §6.6)"
    );
}

#[test]
fn trap_is_deterministic_and_carries_the_composed_message() {
    let error = run(include_str!("assets/trap_probe.mlir"))
        .expect_err("the trap probe must trap under the interp");
    assert!(
        error.contains("calque trap: demo.calq:3:9: error: CQ0001: integer overflow (i64 add)"),
        "{error}"
    );
    assert!(error.contains("D-088"), "{error}");
}

#[test]
fn artifact_marker_is_enforced() {
    let bad = "// not-a-calque-artifact\nfunc.func @main() -> i64 { %z = arith.constant 0 : i64\n return %z : i64 }";
    let error = run(bad).expect_err("a missing marker must be refused");
    assert!(error.contains("must begin with"), "{error}");
}
