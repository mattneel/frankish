//! K2 verifiers for frk.adt (packed surface, D-036; law L1). Composition
//! mirrors the harness runner: register, parse, MLIR-verify, frk-verify,
//! interpret with the adt evaluators plugged in.

use frk_interp::{EvalError, Interp};
use melior::ir::Module;
use melior::ir::operation::OperationLike;

fn interpret_i64(source: &str) -> Result<i64, EvalError> {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    let module = Module::parse(&context, source).expect("test source must parse");
    assert!(module.as_operation().verify(), "must pass MLIR verification");
    frk_dialects::verify(&context, &module).expect("must pass frk semantic verification");

    let mut interp = Interp::new(&module)?;
    frk_dialects::register_eval(&mut interp);
    let values = interp.eval_function("main", &[])?;
    assert_eq!(values.len(), 1, "entry returned {values:?}");
    values[0].as_signed()
}

const OPTION_I64: &str = "!frk_adt.sum<[[], [i64]]>";
const P_EMPTY: &str = "!frk_adt.product<[]>";
const P_I64: &str = "!frk_adt.product<[i64]>";

#[test]
fn construct_tag_extract_round_trip() {
    let result = interpret_i64(&format!(
        r#"func.func @main() -> i64 {{
            %x = arith.constant 41 : i64
            %e = "frk_adt.product_new"() : () -> {P_EMPTY}
            %p = "frk_adt.product_snoc"(%e, %x) : ({P_EMPTY}, i64) -> {P_I64}
            %some = "frk_adt.make_sum"(%p) {{variant = 1 : i64}} : ({P_I64}) -> {OPTION_I64}
            %tag = "frk_adt.tag_of"(%some) : ({OPTION_I64}) -> i64
            %val = "frk_adt.extract"(%some) {{variant = 1 : i64, field = 0 : i64}} : ({OPTION_I64}) -> i64
            %sum = arith.addi %tag, %val : i64
            return %sum : i64
        }}"#
    ))
    .unwrap();
    assert_eq!(result, 42); // tag 1 + value 41
}

#[test]
fn products_build_by_snoc_and_project_mixed_types() {
    let result = interpret_i64(
        r#"func.func @main() -> i64 {
            %a = arith.constant 30 : i64
            %b = arith.constant true
            %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
            %p1 = "frk_adt.product_snoc"(%e, %a) : (!frk_adt.product<[]>, i64) -> !frk_adt.product<[i64]>
            %p2 = "frk_adt.product_snoc"(%p1, %b) : (!frk_adt.product<[i64]>, i1) -> !frk_adt.product<[i64, i1]>
            %x = "frk_adt.get"(%p2) {field = 0 : i64} : (!frk_adt.product<[i64, i1]>) -> i64
            %flag = "frk_adt.get"(%p2) {field = 1 : i64} : (!frk_adt.product<[i64, i1]>) -> i1
            %twelve = arith.constant 12 : i64
            %zero = arith.constant 0 : i64
            %bonus = arith.select %flag, %twelve, %zero : i64
            %sum = arith.addi %x, %bonus : i64
            return %sum : i64
        }"#,
    )
    .unwrap();
    assert_eq!(result, 42); // 30 + (true ? 12 : 0)
}

/// The de-regioned match shape (D-031) end to end on the packed surface.
#[test]
fn dispatch_rides_tag_of_plus_cf_switch() {
    let source = |variant: usize, payload: i64| {
        let make = if variant == 1 {
            format!(
                r#"%x = arith.constant {payload} : i64
                %e = "frk_adt.product_new"() : () -> {P_EMPTY}
                %p = "frk_adt.product_snoc"(%e, %x) : ({P_EMPTY}, i64) -> {P_I64}
                %s = "frk_adt.make_sum"(%p) {{variant = 1 : i64}} : ({P_I64}) -> {OPTION_I64}"#
            )
        } else {
            format!(
                r#"%e = "frk_adt.product_new"() : () -> {P_EMPTY}
                %s = "frk_adt.make_sum"(%e) {{variant = 0 : i64}} : ({P_EMPTY}) -> {OPTION_I64}"#
            )
        };
        format!(
            r#"func.func @main() -> i64 {{
                {make}
                %tag = "frk_adt.tag_of"(%s) : ({OPTION_I64}) -> i64
                cf.switch %tag : i64, [
                    default: ^unreachable,
                    0: ^none,
                    1: ^some
                ]
            ^none:
                %zero = arith.constant 0 : i64
                cf.br ^exit(%zero : i64)
            ^some:
                %v = "frk_adt.extract"(%s) {{variant = 1 : i64, field = 0 : i64}} : ({OPTION_I64}) -> i64
                %one = arith.constant 1 : i64
                %v1 = arith.addi %v, %one : i64
                cf.br ^exit(%v1 : i64)
            ^unreachable:
                %m1 = arith.constant -1 : i64
                cf.br ^exit(%m1 : i64)
            ^exit(%r: i64):
                return %r : i64
            }}"#
        )
    };
    assert_eq!(interpret_i64(&source(1, 41)).unwrap(), 42);
    assert_eq!(interpret_i64(&source(0, 0)).unwrap(), 0);
}

#[test]
fn wrong_variant_extraction_traps_deterministically() {
    let error = interpret_i64(&format!(
        r#"func.func @main() -> i64 {{
            %e = "frk_adt.product_new"() : () -> {P_EMPTY}
            %none = "frk_adt.make_sum"(%e) {{variant = 0 : i64}} : ({P_EMPTY}) -> {OPTION_I64}
            %v = "frk_adt.extract"(%none) {{variant = 1 : i64, field = 0 : i64}} : ({OPTION_I64}) -> i64
            return %v : i64
        }}"#
    ))
    .unwrap_err();
    assert!(matches!(error, EvalError::Trap(_)), "{error}");
}

#[test]
fn adt_values_flow_through_calls_and_block_args() {
    let result = interpret_i64(&format!(
        r#"func.func @unwrap_or_zero(%o: {OPTION_I64}) -> i64 {{
            %tag = "frk_adt.tag_of"(%o) : ({OPTION_I64}) -> i64
            %one = arith.constant 1 : i64
            %is_some = arith.cmpi eq, %tag, %one : i64
            cf.cond_br %is_some, ^some(%o : {OPTION_I64}), ^none
        ^some(%s: {OPTION_I64}):
            %v = "frk_adt.extract"(%s) {{variant = 1 : i64, field = 0 : i64}} : ({OPTION_I64}) -> i64
            return %v : i64
        ^none:
            %z = arith.constant 0 : i64
            return %z : i64
        }}
        func.func @main() -> i64 {{
            %x = arith.constant 42 : i64
            %e = "frk_adt.product_new"() : () -> {P_EMPTY}
            %p = "frk_adt.product_snoc"(%e, %x) : ({P_EMPTY}, i64) -> {P_I64}
            %some = "frk_adt.make_sum"(%p) {{variant = 1 : i64}} : ({P_I64}) -> {OPTION_I64}
            %r = func.call @unwrap_or_zero(%some) : ({OPTION_I64}) -> i64
            return %r : i64
        }}"#
    ))
    .unwrap();
    assert_eq!(result, 42);
}
