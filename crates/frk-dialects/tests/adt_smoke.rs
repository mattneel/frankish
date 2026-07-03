//! K1 smoke for frk.adt (M3 step 1; law L1: these verifiers land with
//! the dialect definition itself). IRDL-enforceable shape properties are
//! proven here — positive round-trip, targeted negatives, type
//! round-trip, builder construction. Semantic invariants beyond IRDL
//! (index ranges, extract result = field type) belong to the frk
//! verification pass, arriving as K1's second half.

use melior::Context;
use melior::ir::attribute::{IntegerAttribute, StringAttribute, TypeAttribute};
use melior::ir::operation::{OperationBuilder, OperationLike};
use melior::ir::r#type::{FunctionType, IntegerType};
use melior::ir::{Block, BlockLike, Identifier, Location, Module, Region, RegionLike, Type};

fn adt_context() -> Context {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("kernel dialect registration must succeed");
    context
}

/// Option<i64>-shaped sum plus a pair product, exercised end to end.
/// Note extract's variant=1/field=0: independently-valued attributes —
/// the regression surface for IRDL's value-unifying constraint vars.
const WELL_TYPED: &str = r#"
func.func @main(%x: i64) -> i64 {
  %some = "frk_adt.make_sum"(%x) {variant = 1 : i64} : (i64) -> !frk_adt.sum<[[], [i64]]>
  %none = "frk_adt.make_sum"() {variant = 0 : i64} : () -> !frk_adt.sum<[[], [i64]]>
  %tag = "frk_adt.tag_of"(%some) : (!frk_adt.sum<[[], [i64]]>) -> i64
  %val = "frk_adt.extract"(%some) {variant = 1 : i64, field = 0 : i64} : (!frk_adt.sum<[[], [i64]]>) -> i64
  %pair = "frk_adt.make_product"(%tag, %val) : (i64, i64) -> !frk_adt.product<[i64, i64]>
  %fst = "frk_adt.get"(%pair) {field = 0 : i64} : (!frk_adt.product<[i64, i64]>) -> i64
  %sum = arith.addi %fst, %val : i64
  return %sum : i64
}
"#;

fn rejects(context: &Context, source: &str, why: &str) {
    let module = Module::parse(context, source);
    assert!(
        module.is_none() || !module.unwrap().as_operation().verify(),
        "must reject: {why}"
    );
}

#[test]
fn registration_succeeds_on_a_frk_context() {
    adt_context();
}

#[test]
fn well_typed_adt_program_parses_and_verifies() {
    let context = adt_context();
    let module = Module::parse(&context, WELL_TYPED).expect("well-typed program must parse");
    assert!(module.as_operation().verify());
}

#[test]
fn tag_of_result_must_be_i64() {
    let context = adt_context();
    rejects(
        &context,
        r#"func.func @main(%x: i64) -> i32 {
            %s = "frk_adt.make_sum"(%x) {variant = 0 : i64} : (i64) -> !frk_adt.sum<[[i64]]>
            %t = "frk_adt.tag_of"(%s) : (!frk_adt.sum<[[i64]]>) -> i32
            return %t : i32
        }"#,
        "tag_of yielding i32",
    );
}

#[test]
fn sum_ops_reject_non_sum_operands() {
    let context = adt_context();
    rejects(
        &context,
        r#"func.func @main(%x: i64) -> i64 {
            %t = "frk_adt.tag_of"(%x) : (i64) -> i64
            return %t : i64
        }"#,
        "tag_of over a plain integer",
    );
    rejects(
        &context,
        r#"func.func @main(%x: i64) -> i64 {
            %p = "frk_adt.make_product"(%x) : (i64) -> !frk_adt.product<[i64]>
            %t = "frk_adt.tag_of"(%p) : (!frk_adt.product<[i64]>) -> i64
            return %t : i64
        }"#,
        "tag_of over a product (base types are distinct)",
    );
}

#[test]
fn make_sum_requires_the_variant_attribute() {
    let context = adt_context();
    rejects(
        &context,
        r#"func.func @main(%x: i64) -> i64 {
            %s = "frk_adt.make_sum"(%x) : (i64) -> !frk_adt.sum<[[i64]]>
            %t = "frk_adt.tag_of"(%s) : (!frk_adt.sum<[[i64]]>) -> i64
            return %t : i64
        }"#,
        "make_sum without {variant}",
    );
}

#[test]
fn adt_types_round_trip_through_print_and_parse() {
    let context = adt_context();
    for spelling in [
        "!frk_adt.sum<[[], [i64]]>",
        "!frk_adt.sum<[[i64, i1], [i64]]>",
        "!frk_adt.product<[i64, i64]>",
    ] {
        let parsed = Type::parse(&context, spelling).expect("type must parse");
        assert_eq!(parsed.to_string(), spelling);
    }
}

#[test]
fn make_sum_is_constructible_with_the_builder_api() {
    let context = adt_context();
    let location = Location::unknown(&context);

    let i64_type: Type = IntegerType::new(&context, 64).into();
    let sum_type = Type::parse(&context, "!frk_adt.sum<[[], [i64]]>").unwrap();

    let block = Block::new(&[(i64_type, location)]);
    let make = OperationBuilder::new("frk_adt.make_sum", location)
        .add_operands(&[block.argument(0).unwrap().into()])
        .add_attributes(&[(
            Identifier::new(&context, "variant"),
            IntegerAttribute::new(i64_type, 1).into(),
        )])
        .add_results(&[sum_type])
        .build()
        .expect("builder must construct make_sum");
    let make = block.append_operation(make);
    assert!(make.verify(), "built make_sum must pass the IRDL verifier");

    // Wrap it in a printable function so the whole construction path is
    // exercised the way emission will use it.
    block.append_operation(
        OperationBuilder::new("func.return", location)
            .add_operands(&[make.result(0).unwrap().into()])
            .build()
            .unwrap(),
    );
    let region = Region::new();
    region.append_block(block);
    let function = melior::dialect::func::func(
        &context,
        StringAttribute::new(&context, "build_smoke"),
        TypeAttribute::new(FunctionType::new(&context, &[i64_type], &[sum_type]).into()),
        region,
        &[],
        location,
    );
    let module = Module::new(location);
    module.body().append_operation(function);
    assert!(module.as_operation().verify());
    assert!(
        module.as_operation().to_string().contains("frk_adt.make_sum"),
        "dialect ops must print under their own namespace"
    );
}
