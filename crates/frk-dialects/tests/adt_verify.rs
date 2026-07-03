//! Verifiers for the frk semantic verifier itself (K1 second half; law
//! L1: landed with the pass). Every rule fires at least once; malformed
//! encodings must produce findings, never panics.

use melior::Context;
use melior::ir::Module;
use melior::ir::operation::OperationLike;

fn adt_context() -> Context {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    context
}

/// Parses (must succeed and MLIR-verify — these programs are all
/// IRDL-legal) then runs the frk semantic verifier.
fn frk_verify(context: &Context, source: &str) -> Result<(), String> {
    let module = Module::parse(context, source).expect("test program must parse");
    assert!(
        module.as_operation().verify(),
        "test programs must be IRDL-legal; this one failed MLIR verification"
    );
    frk_dialects::verify(context, &module).map_err(|errors| errors.to_string())
}

fn expect_finding(context: &Context, source: &str, needle: &str) {
    let message = frk_verify(context, source).expect_err("must produce a finding");
    assert!(
        message.contains(needle),
        "finding should mention {needle:?}, got:\n{message}"
    );
}

const OPTION_I64: &str = "!frk_adt.sum<[[], [i64]]>";

#[test]
fn well_typed_program_passes_the_semantic_verifier() {
    let context = adt_context();
    frk_verify(
        &context,
        &format!(
            r#"func.func @main(%x: i64) -> i64 {{
                %some = "frk_adt.make_sum"(%x) {{variant = 1 : i64}} : (i64) -> {OPTION_I64}
                %tag = "frk_adt.tag_of"(%some) : ({OPTION_I64}) -> i64
                %val = "frk_adt.extract"(%some) {{variant = 1 : i64, field = 0 : i64}} : ({OPTION_I64}) -> i64
                %pair = "frk_adt.make_product"(%tag, %val) : (i64, i64) -> !frk_adt.product<[i64, i64]>
                %fst = "frk_adt.get"(%pair) {{field = 0 : i64}} : (!frk_adt.product<[i64, i64]>) -> i64
                return %fst : i64
            }}"#
        ),
    )
    .expect("well-typed program must pass");
}

#[test]
fn upstream_only_modules_pass_vacuously() {
    let context = adt_context();
    frk_verify(
        &context,
        r#"func.func @main() -> i64 {
            %a = arith.constant 1 : i64
            return %a : i64
        }"#,
    )
    .expect("no frk ops, nothing to find");
}

#[test]
fn make_sum_variant_out_of_range() {
    let context = adt_context();
    expect_finding(
        &context,
        &format!(
            r#"func.func @main(%x: i64) -> {OPTION_I64} {{
                %s = "frk_adt.make_sum"(%x) {{variant = 2 : i64}} : (i64) -> {OPTION_I64}
                return %s : {OPTION_I64}
            }}"#
        ),
        "variant 2 out of range",
    );
}

#[test]
fn make_sum_arity_must_match_the_variant() {
    let context = adt_context();
    // Variant 0 of Option has no fields; passing one operand is a lie.
    expect_finding(
        &context,
        &format!(
            r#"func.func @main(%x: i64) -> {OPTION_I64} {{
                %s = "frk_adt.make_sum"(%x) {{variant = 0 : i64}} : (i64) -> {OPTION_I64}
                return %s : {OPTION_I64}
            }}"#
        ),
        "1 operand(s) for 0 field(s)",
    );
}

#[test]
fn make_sum_operand_types_must_match_the_variant() {
    let context = adt_context();
    expect_finding(
        &context,
        &format!(
            r#"func.func @main(%x: i1) -> {OPTION_I64} {{
                %s = "frk_adt.make_sum"(%x) {{variant = 1 : i64}} : (i1) -> {OPTION_I64}
                return %s : {OPTION_I64}
            }}"#
        ),
        "operand 0 has type i1",
    );
}

#[test]
fn extract_variant_out_of_range() {
    let context = adt_context();
    expect_finding(
        &context,
        &format!(
            r#"func.func @main(%x: i64) -> i64 {{
                %s = "frk_adt.make_sum"(%x) {{variant = 1 : i64}} : (i64) -> {OPTION_I64}
                %v = "frk_adt.extract"(%s) {{variant = 5 : i64, field = 0 : i64}} : ({OPTION_I64}) -> i64
                return %v : i64
            }}"#
        ),
        "variant 5 out of range",
    );
}

#[test]
fn extract_field_out_of_range() {
    let context = adt_context();
    expect_finding(
        &context,
        &format!(
            r#"func.func @main(%x: i64) -> i64 {{
                %s = "frk_adt.make_sum"(%x) {{variant = 1 : i64}} : (i64) -> {OPTION_I64}
                %v = "frk_adt.extract"(%s) {{variant = 1 : i64, field = 3 : i64}} : ({OPTION_I64}) -> i64
                return %v : i64
            }}"#
        ),
        "field 3 out of range",
    );
}

#[test]
fn extract_result_type_must_equal_the_field_type() {
    let context = adt_context();
    // IRDL says the result is irdl.any — exactly the gap this pass closes.
    expect_finding(
        &context,
        &format!(
            r#"func.func @main(%x: i64) -> i1 {{
                %s = "frk_adt.make_sum"(%x) {{variant = 1 : i64}} : (i64) -> {OPTION_I64}
                %v = "frk_adt.extract"(%s) {{variant = 1 : i64, field = 0 : i64}} : ({OPTION_I64}) -> i1
                return %v : i1
            }}"#
        ),
        "extract result type i1",
    );
}

#[test]
fn product_rules_fire_for_get_and_make_product() {
    let context = adt_context();
    expect_finding(
        &context,
        r#"func.func @main(%x: i64) -> i64 {
            %p = "frk_adt.make_product"(%x) : (i64) -> !frk_adt.product<[i64, i64]>
            %f = "frk_adt.get"(%p) {field = 0 : i64} : (!frk_adt.product<[i64, i64]>) -> i64
            return %f : i64
        }"#,
        "1 operand(s) for 2 field(s)",
    );
    expect_finding(
        &context,
        r#"func.func @main(%x: i64) -> i64 {
            %p = "frk_adt.make_product"(%x) : (i64) -> !frk_adt.product<[i64]>
            %f = "frk_adt.get"(%p) {field = 4 : i64} : (!frk_adt.product<[i64]>) -> i64
            return %f : i64
        }"#,
        "field 4 out of range",
    );
    expect_finding(
        &context,
        r#"func.func @main(%x: i64) -> i1 {
            %p = "frk_adt.make_product"(%x) : (i64) -> !frk_adt.product<[i64]>
            %f = "frk_adt.get"(%p) {field = 0 : i64} : (!frk_adt.product<[i64]>) -> i1
            return %f : i1
        }"#,
        "get result type i1",
    );
}

#[test]
fn ops_inside_nested_regions_are_reached() {
    let context = adt_context();
    expect_finding(
        &context,
        &format!(
            r#"func.func @main(%x: i64, %c: i1) -> i64 {{
                %r = scf.if %c -> (i64) {{
                    %s = "frk_adt.make_sum"(%x) {{variant = 9 : i64}} : (i64) -> {OPTION_I64}
                    %v = "frk_adt.extract"(%s) {{variant = 1 : i64, field = 0 : i64}} : ({OPTION_I64}) -> i64
                    scf.yield %v : i64
                }} else {{
                    scf.yield %x : i64
                }}
                return %r : i64
            }}"#
        ),
        "variant 9 out of range",
    );
}

#[test]
fn garbage_type_parameters_are_findings_not_panics() {
    let context = adt_context();
    // IRDL constrains the sum parameter only to "any attribute" — a
    // non-array parameter parses fine and must surface as a finding.
    expect_finding(
        &context,
        r#"func.func @main(%x: i64) -> i64 {
            %s = "frk_adt.make_sum"(%x) {variant = 0 : i64} : (i64) -> !frk_adt.sum<5>
            %t = "frk_adt.tag_of"(%s) : (!frk_adt.sum<5>) -> i64
            return %t : i64
        }"#,
        "must be an array of variants",
    );
}
