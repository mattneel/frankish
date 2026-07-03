//! K3 verifiers for the frk.adt lowering (law L1: landed with the
//! pass). Structural properties checked here; *semantic* equivalence is
//! enforced corpus-wide by the differential law the moment the adt
//! goldens run under both interp and jit.

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
    manager.add_pass(frk_dialects::lower_adt_pass());
    manager.run(&mut module).map_err(|_| ())?;
    assert!(module.as_operation().verify(), "lowered module must verify");
    Ok(module.as_operation().to_string())
}

const OPTION_I64: &str = "!frk_adt.sum<[[], [i64]]>";

#[test]
fn lowering_eliminates_frk_ops_and_types() {
    let context = adt_context();
    let lowered = lower(
        &context,
        &format!(
            r#"func.func @main() -> i64 {{
                %x = arith.constant 41 : i64
                %s = "frk_adt.make_sum"(%x) {{variant = 1 : i64}} : (i64) -> {OPTION_I64}
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
    let sum = "!frk_adt.sum<[[i1]]>";
    let lowered = lower(
        &context,
        &format!(
            r#"func.func @main() -> i64 {{
                %b = arith.constant true
                %s = "frk_adt.make_sum"(%b) {{variant = 0 : i64}} : (i1) -> {sum}
                %v = "frk_adt.extract"(%s) {{variant = 0 : i64, field = 0 : i64}} : ({sum}) -> i1
                %one = arith.constant 1 : i64
                %zero = arith.constant 0 : i64
                %r = arith.select %v, %one, %zero : i64
                return %r : i64
            }}"#
        ),
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
                %some = "frk_adt.make_sum"(%x) {{variant = 1 : i64}} : (i64) -> {OPTION_I64}
                %r = func.call @through(%some) : ({OPTION_I64}) -> i64
                return %r : i64
            }}"#
        ),
    )
    .expect("lowering must succeed");
    assert!(!lowered.contains("frk_adt"), "{lowered}");
}

#[test]
fn nested_adt_fields_fail_the_pass_loudly() {
    let context = adt_context();
    let nested = "!frk_adt.sum<[[!frk_adt.product<[i64]>]]>";
    let result = lower(
        &context,
        &format!(
            r#"func.func @main(%p: !frk_adt.product<[i64]>) -> {nested} {{
                %s = "frk_adt.make_sum"(%p) {{variant = 0 : i64}} : (!frk_adt.product<[i64]>) -> {nested}
                return %s : {nested}
            }}"#
        ),
    );
    assert!(result.is_err(), "nested adt fields are fenced in v0 (D-032)");
}
