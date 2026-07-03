//! K3 verifiers for the frk.adt lowering (packed surface, D-036; law
//! L1). Structural checks here; semantic equivalence is the corpus's
//! job under the differential law.

use melior::Context;
use melior::ir::Module;
use melior::ir::operation::OperationLike;
use melior::pass::PassManager;

fn adt_context() -> Context {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    context
}

fn lower(context: &Context, source: &str) -> Result<String, ()> {
    let mut module = Module::parse(context, source).expect("test source must parse");
    assert!(module.as_operation().verify(), "input must verify");
    frk_dialects::verify(context, &module).expect("input must pass the frk verifier");

    let manager = PassManager::new(context);
    manager.add_pass(frk_dialects::lower_kernel_pass(frk_dialects::Strategy::Arena));
    manager.run(&mut module).map_err(|_| ())?;
    assert!(module.as_operation().verify(), "lowered module must verify");
    Ok(module.as_operation().to_string())
}

const OPTION_I64: &str = "!frk_adt.sum<[[], [i64]]>";
const P_EMPTY: &str = "!frk_adt.product<[]>";
const P_I64: &str = "!frk_adt.product<[i64]>";

#[test]
fn lowering_eliminates_frk_ops_and_types() {
    let context = adt_context();
    let lowered = lower(
        &context,
        &format!(
            r#"func.func @main() -> i64 {{
                %x = arith.constant 41 : i64
                %e = "frk_adt.product_new"() : () -> {P_EMPTY}
                %p = "frk_adt.product_snoc"(%e, %x) : ({P_EMPTY}, i64) -> {P_I64}
                %s = "frk_adt.make_sum"(%p) {{variant = 1 : i64}} : ({P_I64}) -> {OPTION_I64}
                %t = "frk_adt.tag_of"(%s) : ({OPTION_I64}) -> i64
                %v = "frk_adt.extract"(%s) {{variant = 1 : i64, field = 0 : i64}} : ({OPTION_I64}) -> i64
                %r = arith.addi %t, %v : i64
                return %r : i64
            }}"#
        ),
    )
    .expect("lowering must succeed");
    assert!(!lowered.contains("frk_adt"), "{lowered}");
    assert!(lowered.contains("llvm.insertvalue"), "{lowered}");
    assert!(lowered.contains("llvm.extractvalue"), "{lowered}");
    assert!(lowered.contains("llvm.struct<(i64, i64)>"), "{lowered}");
}

#[test]
fn narrow_fields_widen_into_slots_and_back() {
    let context = adt_context();
    let lowered = lower(
        &context,
        r#"func.func @main() -> i64 {
            %b = arith.constant true
            %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
            %p = "frk_adt.product_snoc"(%e, %b) : (!frk_adt.product<[]>, i1) -> !frk_adt.product<[i1]>
            %v = "frk_adt.get"(%p) {field = 0 : i64} : (!frk_adt.product<[i1]>) -> i1
            %one = arith.constant 1 : i64
            %zero = arith.constant 0 : i64
            %r = arith.select %v, %one, %zero : i64
            return %r : i64
        }"#,
    )
    .expect("lowering must succeed");
    assert!(lowered.contains("arith.extui"), "{lowered}");
    assert!(lowered.contains("arith.trunci"), "{lowered}");
}

#[test]
fn signatures_and_block_arguments_convert() {
    let context = adt_context();
    let lowered = lower(
        &context,
        &format!(
            r#"func.func @through(%o: {OPTION_I64}) -> i64 {{
                %one = arith.constant 1 : i64
                %tag = "frk_adt.tag_of"(%o) : ({OPTION_I64}) -> i64
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
                %r = func.call @through(%some) : ({OPTION_I64}) -> i64
                return %r : i64
            }}"#
        ),
    )
    .expect("lowering must succeed");
    assert!(!lowered.contains("frk_adt"), "{lowered}");
}

#[test]
fn nested_adt_fields_lower_through_word_slots() {
    // The D-032 nested-adt fence lifted with SlotKind::Words (M5): a
    // sum inside a product copies as verbatim i64 words.
    let context = adt_context();
    let lowered = lower(
        &context,
        &format!(
            r#"func.func @main(%s: {OPTION_I64}) -> i64 {{
                %e = "frk_adt.product_new"() : () -> {P_EMPTY}
                %p = "frk_adt.product_snoc"(%e, %s) : ({P_EMPTY}, {OPTION_I64}) -> !frk_adt.product<[{OPTION_I64}]>
                %back = "frk_adt.get"(%p) {{field = 0 : i64}} : (!frk_adt.product<[{OPTION_I64}]>) -> {OPTION_I64}
                %t = "frk_adt.tag_of"(%back) : ({OPTION_I64}) -> i64
                return %t : i64
            }}"#
        ),
    )
    .expect("nested adt fields must lower (Words slots)");
    assert!(!lowered.contains("frk_adt"), "{lowered}");
}

#[test]
fn f64_fields_lower_and_narrower_floats_still_fail() {
    // M9 lifted the f64 fence (SlotKind::F64, D-047): one slot,
    // bitcast in and out.
    let context = adt_context();
    let lowered = lower(
        &context,
        r#"func.func @main(%x: f64) -> f64 {
            %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
            %p = "frk_adt.product_snoc"(%e, %x) : (!frk_adt.product<[]>, f64) -> !frk_adt.product<[f64]>
            %back = "frk_adt.get"(%p) {field = 0 : i64} : (!frk_adt.product<[f64]>) -> f64
            return %back : f64
        }"#,
    )
    .expect("f64 fields lower via bitcast slots (M9)");
    assert!(!lowered.contains("frk_adt"), "{lowered}");
    assert!(lowered.contains("arith.bitcast"), "{lowered}");

    // f32 (and every other non-f64 float) remains outside the model.
    let result = lower(
        &context,
        r#"func.func @main(%x: f32) -> !frk_adt.product<[f32]> {
            %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
            %p = "frk_adt.product_snoc"(%e, %x) : (!frk_adt.product<[]>, f32) -> !frk_adt.product<[f32]>
            return %p : !frk_adt.product<[f32]>
        }"#,
    );
    assert!(result.is_err(), "f32 fields are outside the v0 slot model");
}
