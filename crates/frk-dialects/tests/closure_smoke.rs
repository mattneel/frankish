//! K1 smoke for frk.closure (law L1: landed with the dialect
//! definition). IRDL shape properties and the deep semantic contract
//! (callee existence + signature = captures ++ params -> results;
//! apply arity/typing) — positive and negative.

use melior::Context;
use melior::ir::Module;
use melior::ir::operation::OperationLike;

fn closure_context() -> Context {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    context
}

const FN_I64: &str = "!frk_closure.fn<[i64], [i64]>";
const P_EMPTY: &str = "!frk_adt.product<[]>";
const P_I64: &str = "!frk_adt.product<[i64]>";
const P_FN: &str = "!frk_adt.product<[!frk_closure.fn<[i64], [i64]>]>";

/// The church shape: capture a closure, return a closure upward.
/// Packed surface (D-036): envs and arg lists are frk_adt products.
fn church_source() -> String {
    format!(
        r#"func.func @inc(%n: i64) -> i64 {{
            %one = arith.constant 1 : i64
            %r = arith.addi %n, %one : i64
            return %r : i64
        }}
        func.func @two_inner(%f: {FN_I64}, %x: i64) -> i64 {{
            %e0 = "frk_adt.product_new"() : () -> {P_EMPTY}
            %a1 = "frk_adt.product_snoc"(%e0, %x) : ({P_EMPTY}, i64) -> {P_I64}
            %fx = "frk_closure.apply"(%f, %a1) : ({FN_I64}, {P_I64}) -> i64
            %a2 = "frk_adt.product_snoc"(%e0, %fx) : ({P_EMPTY}, i64) -> {P_I64}
            %ffx = "frk_closure.apply"(%f, %a2) : ({FN_I64}, {P_I64}) -> i64
            return %ffx : i64
        }}
        func.func @two_outer(%f: {FN_I64}) -> {FN_I64} {{
            %e = "frk_adt.product_new"() : () -> {P_EMPTY}
            %env = "frk_adt.product_snoc"(%e, %f) : ({P_EMPTY}, {FN_I64}) -> {P_FN}
            %two = "frk_closure.make"(%env) {{callee = @two_inner}} : ({P_FN}) -> {FN_I64}
            return %two : {FN_I64}
        }}
        func.func @main() -> i64 {{
            %c40 = arith.constant 40 : i64
            %e = "frk_adt.product_new"() : () -> {P_EMPTY}
            %inc = "frk_closure.make"(%e) {{callee = @inc}} : ({P_EMPTY}) -> {FN_I64}
            %two = func.call @two_outer(%inc) : ({FN_I64}) -> {FN_I64}
            %args = "frk_adt.product_snoc"(%e, %c40) : ({P_EMPTY}, i64) -> {P_I64}
            %r = "frk_closure.apply"(%two, %args) : ({FN_I64}, {P_I64}) -> i64
            return %r : i64
        }}"#
    )
}

fn frk_verify(context: &Context, source: &str) -> Result<(), String> {
    let module = Module::parse(context, source).expect("source must parse");
    assert!(module.as_operation().verify(), "must pass MLIR verification");
    frk_dialects::verify(context, &module).map_err(|errors| errors.to_string())
}

fn expect_finding(context: &Context, source: &str, needle: &str) {
    let message = frk_verify(context, source).expect_err("must produce a finding");
    assert!(
        message.contains(needle),
        "finding should mention {needle:?}, got:\n{message}"
    );
}

#[test]
fn church_shape_parses_verifies_and_frk_verifies() {
    let context = closure_context();
    frk_verify(&context, &church_source()).expect("church shape must be clean");
}

#[test]
fn closure_types_round_trip() {
    let context = closure_context();
    for spelling in [
        "!frk_closure.fn<[i64], [i64]>",
        "!frk_closure.fn<[], [i64]>",
        "!frk_closure.fn<[!frk_closure.fn<[i64], [i64]>, i64], [i64]>",
    ] {
        let parsed = melior::ir::Type::parse(&context, spelling).expect("type must parse");
        assert_eq!(parsed.to_string(), spelling);
    }
}

#[test]
fn irdl_rejects_wrong_attribute_and_operand_kinds() {
    let context = closure_context();
    // Integer callee: rejected at the IRDL layer.
    let bad_callee = format!(
        r#"func.func @main() -> {FN_I64} {{
            %c = "frk_closure.make"() {{callee = 7 : i64}} : () -> {FN_I64}
            return %c : {FN_I64}
        }}"#
    );
    let module = Module::parse(&context, &bad_callee);
    assert!(module.is_none() || !module.unwrap().as_operation().verify());

    // Applying a plain integer: rejected at the IRDL layer.
    let bad_apply = r#"func.func @main(%x: i64, %p: !frk_adt.product<[]>) -> i64 {
        %r = "frk_closure.apply"(%x, %p) : (i64, !frk_adt.product<[]>) -> i64
        return %r : i64
    }"#;
    let module = Module::parse(&context, bad_apply);
    assert!(module.is_none() || !module.unwrap().as_operation().verify());
}

#[test]
fn make_requires_an_existing_callee() {
    let context = closure_context();
    expect_finding(
        &context,
        &format!(
            r#"func.func @main() -> {FN_I64} {{
                %e = "frk_adt.product_new"() : () -> {P_EMPTY}
                %c = "frk_closure.make"(%e) {{callee = @ghost}} : ({P_EMPTY}) -> {FN_I64}
                return %c : {FN_I64}
            }}"#
        ),
        "@ghost is not a func.func",
    );
}

#[test]
fn make_checks_the_captures_plus_params_convention() {
    let context = closure_context();
    // @wrong takes only i64 — a capture of i1 plus param i64 needs (i1, i64).
    expect_finding(
        &context,
        &format!(
            r#"func.func @wrong(%x: i64) -> i64 {{
                return %x : i64
            }}
            func.func @main(%b: i1) -> {FN_I64} {{
                %e = "frk_adt.product_new"() : () -> {P_EMPTY}
                %env = "frk_adt.product_snoc"(%e, %b) : ({P_EMPTY}, i1) -> !frk_adt.product<[i1]>
                %c = "frk_closure.make"(%env) {{callee = @wrong}} : (!frk_adt.product<[i1]>) -> {FN_I64}
                return %c : {FN_I64}
            }}"#
        ),
        "takes 1 input(s); 1 capture(s) + 1 param(s) = 2 expected",
    );
    // Capture type must equal the callee's leading input.
    expect_finding(
        &context,
        &format!(
            r#"func.func @callee(%c: i64, %x: i64) -> i64 {{
                return %x : i64
            }}
            func.func @main(%b: i1) -> {FN_I64} {{
                %e = "frk_adt.product_new"() : () -> {P_EMPTY}
                %env = "frk_adt.product_snoc"(%e, %b) : ({P_EMPTY}, i1) -> !frk_adt.product<[i1]>
                %c = "frk_closure.make"(%env) {{callee = @callee}} : (!frk_adt.product<[i1]>) -> {FN_I64}
                return %c : {FN_I64}
            }}"#
        ),
        "capture 0 has type i1",
    );
}

#[test]
fn apply_checks_the_arg_pack_and_result_against_the_closure_type() {
    let context = closure_context();
    let preamble = format!(
        r#"func.func @id(%x: i64) -> i64 {{
            return %x : i64
        }}
        func.func @main() -> i64 {{
            %e = "frk_adt.product_new"() : () -> {P_EMPTY}
            %c = "frk_closure.make"(%e) {{callee = @id}} : ({P_EMPTY}) -> {FN_I64}"#
    );
    // Empty arg pack for a 1-param closure.
    expect_finding(
        &context,
        &format!(
            r#"{preamble}
            %r = "frk_closure.apply"(%c, %e) : ({FN_I64}, {P_EMPTY}) -> i64
            return %r : i64
        }}"#
        ),
        "arg pack has 0 field(s), the closure takes 1",
    );
    // Wrong arg type in the pack.
    expect_finding(
        &context,
        &format!(
            r#"{preamble}
            %b = arith.constant true
            %pb = "frk_adt.product_snoc"(%e, %b) : ({P_EMPTY}, i1) -> !frk_adt.product<[i1]>
            %r = "frk_closure.apply"(%c, %pb) : ({FN_I64}, !frk_adt.product<[i1]>) -> i64
            return %r : i64
        }}"#
        ),
        "arg 0 has type i1",
    );
    // Wrong result type.
    expect_finding(
        &context,
        &format!(
            r#"{preamble}
            %x = arith.constant 1 : i64
            %px = "frk_adt.product_snoc"(%e, %x) : ({P_EMPTY}, i64) -> {P_I64}
            %r = "frk_closure.apply"(%c, %px) : ({FN_I64}, {P_I64}) -> i1
            %z = arith.constant 0 : i64
            return %z : i64
        }}"#
        ),
        "apply result has type i1",
    );
}
