//! Substrate verifier for D-030: frankish kernel dialects register
//! through IRDL runtime loading (melior `load_irdl_dialects`), which must
//! give us everything K1 demands from *registration*: parseable ops and
//! parametric types, print/parse round-trips, builder-API construction,
//! and — decisive — IRDL-generated verifiers that actually reject bad IR.
//!
//! This test IS the evidence the D-030 ruling cites. If an MLIR bump ever
//! breaks one of these properties, the ruling's revisit condition fires
//! here first.

use melior::ir::operation::{OperationBuilder, OperationLike};
use melior::ir::{Block, BlockLike, Location, Module, Region, RegionLike, Type};
use melior::utility::load_irdl_dialects;

/// A miniature stand-in for a kernel dialect: one parametric type, one op
/// whose result type must agree with its operand type (a type variable —
/// the same shape frk.adt's make/extract constraints will take).
const SPIKE_DIALECT: &str = r#"
irdl.dialect @frk_spike {
  irdl.type @box {
    %0 = irdl.any
    irdl.parameters(elem: %0)
  }
  irdl.operation @wrap {
    %0 = irdl.any
    %1 = irdl.parametric @frk_spike::@box<%0>
    irdl.operands(value: %0)
    irdl.results(box: %1)
  }
}
"#;

fn context_with_spike_dialect() -> melior::Context {
    let context = frk_core::context();
    let definitions =
        Module::parse(&context, SPIKE_DIALECT).expect("IRDL definition must parse");
    assert!(
        load_irdl_dialects(&definitions),
        "IRDL loading must succeed"
    );
    context
}

#[test]
fn irdl_dialect_loads_and_ops_parse_and_verify() {
    let context = context_with_spike_dialect();
    let module = Module::parse(
        &context,
        r#"func.func @main(%arg0: i64) -> !frk_spike.box<i64> {
            %b = "frk_spike.wrap"(%arg0) : (i64) -> !frk_spike.box<i64>
            return %b : !frk_spike.box<i64>
        }"#,
    )
    .expect("well-typed use of the dynamic dialect must parse");
    assert!(module.as_operation().verify());
}

#[test]
fn irdl_generated_verifier_rejects_type_constraint_violations() {
    let context = context_with_spike_dialect();
    // box<i32> result from an i64 operand violates the shared type
    // variable; mlir-opt rejects this at parse time ("expected 'i64' but
    // got 'i32'") and so must the embedded parser.
    let module = Module::parse(
        &context,
        r#"func.func @main(%arg0: i64) -> !frk_spike.box<i32> {
            %b = "frk_spike.wrap"(%arg0) : (i64) -> !frk_spike.box<i32>
            return %b : !frk_spike.box<i32>
        }"#,
    );
    assert!(
        module.is_none() || !module.unwrap().as_operation().verify(),
        "type-variable violation must not verify"
    );
}

#[test]
fn irdl_generated_verifier_rejects_arity_violations() {
    let context = context_with_spike_dialect();
    let module = Module::parse(
        &context,
        r#"func.func @main(%arg0: i64) -> !frk_spike.box<i64> {
            %b = "frk_spike.wrap"(%arg0, %arg0) : (i64, i64) -> !frk_spike.box<i64>
            return %b : !frk_spike.box<i64>
        }"#,
    );
    assert!(
        module.is_none() || !module.unwrap().as_operation().verify(),
        "arity violation must not verify"
    );
}

#[test]
fn dynamic_types_parse_and_print_round_trip() {
    let context = context_with_spike_dialect();
    let boxed = Type::parse(&context, "!frk_spike.box<i64>")
        .expect("dynamic type must parse standalone");
    assert_eq!(boxed.to_string(), "!frk_spike.box<i64>");
}

#[test]
fn dynamic_ops_are_constructible_with_the_builder_api() {
    // Lowering passes and frontends build ops programmatically; the
    // dynamic dialect must be reachable that way too, not only via the
    // parser.
    let context = context_with_spike_dialect();
    let location = Location::unknown(&context);

    let i64_type = Type::parse(&context, "i64").unwrap();
    let box_type = Type::parse(&context, "!frk_spike.box<i64>").unwrap();

    let block = Block::new(&[(i64_type, location)]);
    let wrap = OperationBuilder::new("frk_spike.wrap", location)
        .add_operands(&[block.argument(0).unwrap().into()])
        .add_results(&[box_type])
        .build()
        .expect("builder construction must succeed");
    let wrap = block.append_operation(wrap);
    assert!(wrap.verify(), "built op must pass the IRDL verifier");

    // And the whole thing prints as the dynamic dialect.
    let region = Region::new();
    region.append_block(block);
    assert!(wrap.to_string().contains("frk_spike.wrap"));
}
