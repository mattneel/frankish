//! K1/K2 verifiers for frk.dyn v0 (D-051; law L1). K3 is scheduled
//! with the femto_lua implementation milestone — these prove the
//! contract that milestone builds against.

use frk_interp::Interp;
use melior::Context;
use melior::ir::Module;
use melior::ir::operation::OperationLike;

fn dyn_context() -> Context {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    context
}

#[test]
fn tag_space_is_closed_and_mandatory() {
    let context = dyn_context();
    // Out-of-range tag.
    let module = Module::parse(
        &context,
        r#"func.func @main(%x: f64) -> !frk_dyn.dyn {
            %d = "frk_dyn.wrap"(%x) {tag = 9 : i64} : (f64) -> !frk_dyn.dyn
            return %d : !frk_dyn.dyn
        }"#,
    )
    .unwrap();
    let message = frk_dialects::verify(&context, &module)
        .expect_err("tag 9 is outside the closed space")
        .to_string();
    assert!(message.contains("closed v0 space"), "{message}");

    // Missing tag.
    let module = Module::parse(
        &context,
        r#"func.func @main(%x: f64) -> !frk_dyn.dyn {
            %d = "frk_dyn.wrap"(%x) : (f64) -> !frk_dyn.dyn
            return %d : !frk_dyn.dyn
        }"#,
    )
    .unwrap();
    assert!(frk_dialects::verify(&context, &module).is_err());
}

#[test]
fn wrap_unwrap_roundtrips_and_mismatch_traps_with_location() {
    let context = dyn_context();
    let source = r#"func.func @roundtrip() -> f64 {
        %x = arith.constant 42.5 : f64
        %d = "frk_dyn.wrap"(%x) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
        %t = "frk_dyn.tag_of"(%d) : (!frk_dyn.dyn) -> i64
        %y = "frk_dyn.unwrap"(%d) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
        return %y : f64
    }
    func.func @mismatch() -> i1 {
        %x = arith.constant 1.0 : f64
        %d = "frk_dyn.wrap"(%x) {tag = 2 : i64} : (!frk_dyn.dyn) -> !frk_dyn.dyn loc("lua_case.lua":7:3)
        %b = "frk_dyn.unwrap"(%d) {tag = 1 : i64} : (!frk_dyn.dyn) -> i1 loc("lua_case.lua":8:3)
        return %b : i1
    }"#;
    // The wrap in @mismatch has a deliberately wrong operand type
    // spelling above; fix: wrap takes f64.
    let source = source.replace(
        r#"(!frk_dyn.dyn) -> !frk_dyn.dyn loc("lua_case.lua":7:3)"#,
        r#"(f64) -> !frk_dyn.dyn loc("lua_case.lua":7:3)"#,
    );
    let module = Module::parse(&context, &source).expect("parse");
    assert!(module.as_operation().verify());
    frk_dialects::verify(&context, &module).unwrap();

    let mut interp = Interp::new(&module).unwrap();
    frk_dialects::register_eval(&mut interp);

    let values = interp.eval_function("roundtrip", &[]).unwrap();
    assert_eq!(values[0].as_float().unwrap(), 42.5);

    let error = interp
        .eval_function("mismatch", &[])
        .expect_err("wrong-tag unwrap must trap (D-051)")
        .to_string();
    assert!(error.contains("dyn tag mismatch: expected 1, got 2"), "{error}");
    assert!(
        error.contains("lua_case.lua"),
        "the trap carries its threaded location (§6.5 discipline): {error}"
    );
}
