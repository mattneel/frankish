//! K2 verifiers for frk.adt (law L1: landed with the Eval impls).
//! Composition mirrors the harness runner exactly: register the dialect,
//! parse, MLIR-verify, frk-verify, then interpret with the adt
//! evaluators plugged in.

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

#[test]
fn construct_tag_extract_round_trip() {
    let result = interpret_i64(&format!(
        r#"func.func @main() -> i64 {{
            %x = arith.constant 41 : i64
            %some = "frk_adt.make_sum"(%x) {{variant = 1 : i64}} : (i64) -> {OPTION_I64}
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
fn products_construct_and_project() {
    let result = interpret_i64(
        r#"func.func @main() -> i64 {
            %a = arith.constant 30 : i64
            %b = arith.constant 12 : i64
            %p = "frk_adt.make_product"(%a, %b) : (i64, i64) -> !frk_adt.product<[i64, i64]>
            %x = "frk_adt.get"(%p) {field = 0 : i64} : (!frk_adt.product<[i64, i64]>) -> i64
            %y = "frk_adt.get"(%p) {field = 1 : i64} : (!frk_adt.product<[i64, i64]>) -> i64
            %sum = arith.addi %x, %y : i64
            return %sum : i64
        }"#,
    )
    .unwrap();
    assert_eq!(result, 42);
}

#[test]
fn multi_field_variants_extract_by_index() {
    let sum = "!frk_adt.sum<[[i64, i64]]>";
    let result = interpret_i64(&format!(
        r#"func.func @main() -> i64 {{
            %a = arith.constant 40 : i64
            %b = arith.constant 2 : i64
            %s = "frk_adt.make_sum"(%a, %b) {{variant = 0 : i64}} : (i64, i64) -> {sum}
            %x = "frk_adt.extract"(%s) {{variant = 0 : i64, field = 0 : i64}} : ({sum}) -> i64
            %y = "frk_adt.extract"(%s) {{variant = 0 : i64, field = 1 : i64}} : ({sum}) -> i64
            %r = arith.addi %x, %y : i64
            return %r : i64
        }}"#
    ))
    .unwrap();
    assert_eq!(result, 42);
}

/// The de-regioned match shape (D-031) end to end: tag_of feeds
/// cf.switch, each arm extracts under its own tag guard. This is
/// exactly the IR the decision-tree pass will emit.
#[test]
fn dispatch_rides_tag_of_plus_cf_switch() {
    let source = |variant: usize, payload: i64| {
        let make = if variant == 1 {
            format!(
                r#"%x = arith.constant {payload} : i64
                %s = "frk_adt.make_sum"(%x) {{variant = 1 : i64}} : (i64) -> {OPTION_I64}"#
            )
        } else {
            format!(r#"%s = "frk_adt.make_sum"() {{variant = 0 : i64}} : () -> {OPTION_I64}"#)
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
    assert_eq!(interpret_i64(&source(1, 41)).unwrap(), 42); // Some(41) → 42
    assert_eq!(interpret_i64(&source(0, 0)).unwrap(), 0); // None → 0
}

#[test]
fn wrong_variant_extraction_traps_deterministically() {
    let error = interpret_i64(&format!(
        r#"func.func @main() -> i64 {{
            %none = "frk_adt.make_sum"() {{variant = 0 : i64}} : () -> {OPTION_I64}
            %v = "frk_adt.extract"(%none) {{variant = 1 : i64, field = 0 : i64}} : ({OPTION_I64}) -> i64
            return %v : i64
        }}"#
    ))
    .unwrap_err();
    assert!(matches!(error, EvalError::Trap(_)), "{error}");
}

#[test]
fn adt_values_flow_through_calls_and_block_args() {
    // Adt values crossing function boundaries and CFG edges exercises
    // the post-Copy clone paths in the interpreter core.
    let result = interpret_i64(&format!(
        r#"func.func @unwrap_or_zero(%o: {OPTION_I64}) -> i64 {{
            %tag = "frk_adt.tag_of"(%o) : ({OPTION_I64}) -> i64
            %one = arith.constant 1 : i64
            %is_some = arith.cmpi eq, %tag, %one : i64
            cf.cond_br %is_some, ^some({OPTION_I64_ARG}), ^none
        ^some(%s: {OPTION_I64}):
            %v = "frk_adt.extract"(%s) {{variant = 1 : i64, field = 0 : i64}} : ({OPTION_I64}) -> i64
            return %v : i64
        ^none:
            %z = arith.constant 0 : i64
            return %z : i64
        }}
        func.func @main() -> i64 {{
            %x = arith.constant 42 : i64
            %some = "frk_adt.make_sum"(%x) {{variant = 1 : i64}} : (i64) -> {OPTION_I64}
            %r = func.call @unwrap_or_zero(%some) : ({OPTION_I64}) -> i64
            return %r : i64
        }}"#,
        OPTION_I64_ARG = format!("%o : {OPTION_I64}")
    ))
    .unwrap();
    assert_eq!(result, 42);
}
