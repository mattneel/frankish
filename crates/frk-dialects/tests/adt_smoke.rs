//! K1 smoke for frk.adt (packed surface, D-036; law L1). IRDL-enforceable
//! shape properties — positive round-trip (including MIXED-TYPE fields,
//! the case that forced D-036), targeted negatives, type round-trip,
//! builder construction. Semantic invariants beyond IRDL live in the frk
//! verification pass (tests/adt_verify.rs).

use melior::Context;
use melior::ir::attribute::{StringAttribute, TypeAttribute};
use melior::ir::operation::{OperationBuilder, OperationLike};
use melior::ir::r#type::FunctionType;
use melior::ir::{Block, BlockLike, Location, Module, Region, RegionLike, Type};

fn adt_context() -> Context {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("kernel dialect registration must succeed");
    context
}

/// Mixed-type fields end to end — inexpressible under the old variadic
/// surface (IRDL unifies variadic elements; D-036), fine when packed.
const WELL_TYPED: &str = r#"
func.func @main(%x: i64, %b: i1) -> i64 {
  %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %p1 = "frk_adt.product_snoc"(%e, %x) : (!frk_adt.product<[]>, i64) -> !frk_adt.product<[i64]>
  %p2 = "frk_adt.product_snoc"(%p1, %b) : (!frk_adt.product<[i64]>, i1) -> !frk_adt.product<[i64, i1]>
  %s = "frk_adt.make_sum"(%p2) {variant = 0 : i64} : (!frk_adt.product<[i64, i1]>) -> !frk_adt.sum<[[i64, i1]]>
  %tag = "frk_adt.tag_of"(%s) : (!frk_adt.sum<[[i64, i1]]>) -> i64
  %val = "frk_adt.extract"(%s) {variant = 0 : i64, field = 0 : i64} : (!frk_adt.sum<[[i64, i1]]>) -> i64
  %fst = "frk_adt.get"(%p2) {field = 0 : i64} : (!frk_adt.product<[i64, i1]>) -> i64
  %sum = arith.addi %tag, %val : i64
  %out = arith.addi %sum, %fst : i64
  return %out : i64
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
fn well_typed_mixed_field_program_parses_and_verifies() {
    let context = adt_context();
    let module = Module::parse(&context, WELL_TYPED).expect("well-typed program must parse");
    assert!(module.as_operation().verify());
}

#[test]
fn tag_of_result_must_be_i64() {
    let context = adt_context();
    rejects(
        &context,
        r#"func.func @main(%s: !frk_adt.sum<[[i64]]>) -> i32 {
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
        r#"func.func @main(%p: !frk_adt.product<[i64]>) -> i64 {
            %t = "frk_adt.tag_of"(%p) : (!frk_adt.product<[i64]>) -> i64
            return %t : i64
        }"#,
        "tag_of over a product (base types are distinct)",
    );
}

#[test]
fn make_sum_requires_the_variant_attribute_and_a_product_payload() {
    let context = adt_context();
    rejects(
        &context,
        r#"func.func @main(%p: !frk_adt.product<[i64]>) -> !frk_adt.sum<[[i64]]> {
            %s = "frk_adt.make_sum"(%p) : (!frk_adt.product<[i64]>) -> !frk_adt.sum<[[i64]]>
            return %s : !frk_adt.sum<[[i64]]>
        }"#,
        "make_sum without {variant}",
    );
    rejects(
        &context,
        r#"func.func @main(%x: i64) -> !frk_adt.sum<[[i64]]> {
            %s = "frk_adt.make_sum"(%x) {variant = 0 : i64} : (i64) -> !frk_adt.sum<[[i64]]>
            return %s : !frk_adt.sum<[[i64]]>
        }"#,
        "make_sum over a bare integer payload",
    );
}

#[test]
fn adt_types_round_trip_through_print_and_parse() {
    let context = adt_context();
    for spelling in [
        "!frk_adt.sum<[[], [i64]]>",
        "!frk_adt.sum<[[i64, i1], [i64]]>",
        "!frk_adt.product<[i64, i64]>",
        "!frk_adt.product<[]>",
    ] {
        let parsed = Type::parse(&context, spelling).expect("type must parse");
        assert_eq!(parsed.to_string(), spelling);
    }
}

#[test]
fn the_packed_chain_is_constructible_with_the_builder_api() {
    let context = adt_context();
    let location = Location::unknown(&context);

    let empty = Type::parse(&context, "!frk_adt.product<[]>").unwrap();

    let block = Block::new(&[]);
    let new = block.append_operation(
        OperationBuilder::new("frk_adt.product_new", location)
            .add_results(&[empty])
            .build()
            .expect("builder must construct product_new"),
    );
    assert!(new.verify(), "built product_new must pass the IRDL verifier");

    block.append_operation(
        OperationBuilder::new("func.return", location)
            .add_operands(&[new.result(0).unwrap().into()])
            .build()
            .unwrap(),
    );
    let region = Region::new();
    region.append_block(block);
    let function = melior::dialect::func::func(
        &context,
        StringAttribute::new(&context, "build_smoke"),
        TypeAttribute::new(FunctionType::new(&context, &[], &[empty]).into()),
        region,
        &[],
        location,
    );
    let module = Module::new(location);
    module.body().append_operation(function);
    assert!(module.as_operation().verify());
    assert!(
        module
            .as_operation()
            .to_string()
            .contains("frk_adt.product_new"),
        "dialect ops must print under their own namespace"
    );
}
