//! Witnesses for the registered-ABI declaration check (M17, D-062;
//! law L1 — the refusal must be proven). The frankish semantic
//! verifier projects every bodyless `frk_rt_*` func.func declaration
//! onto the frk-abi registry; these tests prove a correct declaration
//! passes, a drifted one is refused, and an unregistered symbol is
//! refused.

use melior::ir::Module;
use melior::ir::operation::OperationLike;

fn verify(source: &str) -> Result<(), String> {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    let module = Module::parse(&context, source).expect("test source must parse");
    assert!(module.as_operation().verify(), "must pass MLIR verification");
    frk_dialects::verify(&context, &module).map_err(|errors| errors.to_string())
}

#[test]
fn registered_declaration_passes() {
    verify(
        r#"func.func private @frk_rt_scm_display_bool(i64)
        func.func @main() -> i64 {
            %z = arith.constant 0 : i64
            return %z : i64
        }"#,
    )
    .unwrap();
}

#[test]
fn widened_bool_flag_declaration_passes() {
    // The i1↔u8 widening rule: loanword declares PRINT_BOOL as i1;
    // the registry row is U8. Class-compatible.
    verify(
        r#"func.func private @frk_rt_print_bool(i1)
        func.func @main() -> i64 {
            %z = arith.constant 0 : i64
            return %z : i64
        }"#,
    )
    .unwrap();
}

#[test]
fn drifted_declaration_is_refused() {
    // The M15 bug, at the declaration layer: f64 where the registry
    // says i64.
    let error = verify(
        r#"func.func private @frk_rt_scm_display_bool(f64)
        func.func @main() -> i64 {
            %z = arith.constant 0 : i64
            return %z : i64
        }"#,
    )
    .unwrap_err();
    assert!(error.contains("the registry says"), "{error}");
}

#[test]
fn wrong_arity_is_refused() {
    let error = verify(
        r#"func.func private @frk_rt_scm_newline(i64)
        func.func @main() -> i64 {
            %z = arith.constant 0 : i64
            return %z : i64
        }"#,
    )
    .unwrap_err();
    assert!(error.contains("argument"), "{error}");
}

#[test]
fn unregistered_symbol_is_refused() {
    let error = verify(
        r#"func.func private @frk_rt_totally_invented(i64)
        func.func @main() -> i64 {
            %z = arith.constant 0 : i64
            return %z : i64
        }"#,
    )
    .unwrap_err();
    assert!(error.contains("not in the frk-abi registry"), "{error}");
}
