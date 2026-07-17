//! K2 verifiers for frk.contract (D-072; law L1). The narrow op is a
//! checked cast: identity on success, deterministic blame trap on
//! refutation. The interpreter executes EVERY check — it is the
//! reference the promotion pass is diffed against.

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

const SHAPE: &str = "!frk_adt.sum<[[f64], [f64]]>"; // circle(r) | square(s)
const P_EMPTY: &str = "!frk_adt.product<[]>";
const P_F64: &str = "!frk_adt.product<[f64]>";

fn make_variant(variant: usize, value: f64) -> String {
    format!(
        r#"%x = arith.constant {value:?} : f64
            %e = "frk_adt.product_new"() : () -> {P_EMPTY}
            %p = "frk_adt.product_snoc"(%e, %x) : ({P_EMPTY}, f64) -> {P_F64}
            %s = "frk_adt.make_sum"(%p) {{variant = {variant} : i64}} : ({P_F64}) -> {SHAPE}"#
    )
}

#[test]
fn narrow_is_identity_when_the_fact_holds() {
    let build = make_variant(1, 6.0);
    let result = interpret_i64(&format!(
        r#"func.func @main() -> i64 {{
            {build}
            %n = "frk_contract.narrow"(%s) {{variant = 1 : i64, blame = "cast to 'square' at case.ts:3:9"}} : ({SHAPE}) -> {SHAPE}
            %side = "frk_adt.extract"(%n) {{variant = 1 : i64, field = 0 : i64}} : ({SHAPE}) -> f64
            %i = arith.fptosi %side : f64 to i64
            return %i : i64
        }}"#
    ))
    .unwrap();
    assert_eq!(result, 6);
}

#[test]
fn refuted_narrow_traps_with_blame() {
    let build = make_variant(0, 2.0);
    let error = interpret_i64(&format!(
        r#"func.func @main() -> i64 {{
            {build}
            %n = "frk_contract.narrow"(%s) {{variant = 1 : i64, blame = "cast to 'square' at case.ts:7:15"}} : ({SHAPE}) -> {SHAPE}
            %side = "frk_adt.extract"(%n) {{variant = 1 : i64, field = 0 : i64}} : ({SHAPE}) -> f64
            %i = arith.fptosi %side : f64 to i64
            return %i : i64
        }}"#
    ))
    .unwrap_err();
    let message = format!("{error:?}");
    assert!(
        message.contains("narrowing refuted"),
        "unexpected trap: {message}"
    );
    assert!(
        message.contains("expected variant 1, got 0"),
        "trap must name both tags: {message}"
    );
    assert!(
        message.contains("cast to 'square' at case.ts:7:15"),
        "trap must carry the blame span: {message}"
    );
}

#[test]
fn semantic_verify_rejects_type_changing_narrow() {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    let other = "!frk_adt.sum<[[f64]]>";
    let build = make_variant(0, 1.0);
    let source = format!(
        r#"func.func @main() -> i64 {{
            {build}
            %n = "frk_contract.narrow"(%s) {{variant = 0 : i64, blame = "x"}} : ({SHAPE}) -> {other}
            %z = arith.constant 0 : i64
            return %z : i64
        }}"#
    );
    let module = Module::parse(&context, &source).expect("parses");
    let findings = frk_dialects::verify(&context, &module).unwrap_err();
    assert!(
        format!("{findings}").contains("identity-on-success"),
        "expected the identity rule: {findings}"
    );
}

#[test]
fn semantic_verify_rejects_out_of_range_variant() {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    let build = make_variant(0, 1.0);
    let source = format!(
        r#"func.func @main() -> i64 {{
            {build}
            %n = "frk_contract.narrow"(%s) {{variant = 9 : i64, blame = "x"}} : ({SHAPE}) -> {SHAPE}
            %z = arith.constant 0 : i64
            return %z : i64
        }}"#
    );
    let module = Module::parse(&context, &source).expect("parses");
    let findings = frk_dialects::verify(&context, &module).unwrap_err();
    assert!(
        format!("{findings}").contains("out of range"),
        "expected the range rule: {findings}"
    );
}
