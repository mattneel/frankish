//! Verifiers for the frk semantic verifier (packed surface, D-036; law
//! L1). Every rule fires at least once; malformed encodings must produce
//! findings, never panics.

use melior::Context;
use melior::ir::Module;
use melior::ir::operation::OperationLike;

fn adt_context() -> Context {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    context
}

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
const P_EMPTY: &str = "!frk_adt.product<[]>";
const P_I64: &str = "!frk_adt.product<[i64]>";

#[test]
fn well_typed_packed_program_passes_the_semantic_verifier() {
    let context = adt_context();
    frk_verify(
        &context,
        &format!(
            r#"func.func @main(%x: i64) -> i64 {{
                %e = "frk_adt.product_new"() : () -> {P_EMPTY}
                %p = "frk_adt.product_snoc"(%e, %x) : ({P_EMPTY}, i64) -> {P_I64}
                %some = "frk_adt.make_sum"(%p) {{variant = 1 : i64}} : ({P_I64}) -> {OPTION_I64}
                %tag = "frk_adt.tag_of"(%some) : ({OPTION_I64}) -> i64
                %val = "frk_adt.extract"(%some) {{variant = 1 : i64, field = 0 : i64}} : ({OPTION_I64}) -> i64
                %fst = "frk_adt.get"(%p) {{field = 0 : i64}} : ({P_I64}) -> i64
                %a = arith.addi %tag, %val : i64
                %b = arith.addi %a, %fst : i64
                return %b : i64
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
fn product_new_must_yield_an_empty_product() {
    let context = adt_context();
    expect_finding(
        &context,
        &format!(
            r#"func.func @main() -> {P_I64} {{
                %p = "frk_adt.product_new"() : () -> {P_I64}
                return %p : {P_I64}
            }}"#
        ),
        "must yield an empty product",
    );
}

#[test]
fn product_snoc_result_must_extend_the_operand() {
    let context = adt_context();
    // Wrong appended type in the result.
    expect_finding(
        &context,
        &format!(
            r#"func.func @main(%b: i1) -> !frk_adt.product<[i64]> {{
                %e = "frk_adt.product_new"() : () -> {P_EMPTY}
                %p = "frk_adt.product_snoc"(%e, %b) : ({P_EMPTY}, i1) -> !frk_adt.product<[i64]>
                return %p : !frk_adt.product<[i64]>
            }}"#
        ),
        "snoc appends a i1",
    );
    // Wrong prefix in the result.
    expect_finding(
        &context,
        &format!(
            r#"func.func @main(%x: i64, %b: i1) -> !frk_adt.product<[i1, i1]> {{
                %e = "frk_adt.product_new"() : () -> {P_EMPTY}
                %p = "frk_adt.product_snoc"(%e, %x) : ({P_EMPTY}, i64) -> {P_I64}
                %q = "frk_adt.product_snoc"(%p, %b) : ({P_I64}, i1) -> !frk_adt.product<[i1, i1]>
                return %q : !frk_adt.product<[i1, i1]>
            }}"#
        ),
        "result field 0 is i1",
    );
    // Wrong field count in the result.
    expect_finding(
        &context,
        &format!(
            r#"func.func @main(%x: i64) -> !frk_adt.product<[i64, i64]> {{
                %e = "frk_adt.product_new"() : () -> {P_EMPTY}
                %p = "frk_adt.product_snoc"(%e, %x) : ({P_EMPTY}, i64) -> !frk_adt.product<[i64, i64]>
                return %p : !frk_adt.product<[i64, i64]>
            }}"#
        ),
        "snoc result declares 2 field(s)",
    );
}

#[test]
fn make_sum_payload_must_match_the_variant_shape() {
    let context = adt_context();
    // Variant out of range.
    expect_finding(
        &context,
        &format!(
            r#"func.func @main() -> {OPTION_I64} {{
                %e = "frk_adt.product_new"() : () -> {P_EMPTY}
                %s = "frk_adt.make_sum"(%e) {{variant = 2 : i64}} : ({P_EMPTY}) -> {OPTION_I64}
                return %s : {OPTION_I64}
            }}"#
        ),
        "variant 2 out of range",
    );
    // Payload arity mismatch: variant 0 of Option has no fields.
    expect_finding(
        &context,
        &format!(
            r#"func.func @main(%x: i64) -> {OPTION_I64} {{
                %e = "frk_adt.product_new"() : () -> {P_EMPTY}
                %p = "frk_adt.product_snoc"(%e, %x) : ({P_EMPTY}, i64) -> {P_I64}
                %s = "frk_adt.make_sum"(%p) {{variant = 0 : i64}} : ({P_I64}) -> {OPTION_I64}
                return %s : {OPTION_I64}
            }}"#
        ),
        "payload has 1 field(s), variant 0 needs 0",
    );
    // Payload field type mismatch.
    expect_finding(
        &context,
        &format!(
            r#"func.func @main(%b: i1) -> {OPTION_I64} {{
                %e = "frk_adt.product_new"() : () -> {P_EMPTY}
                %p = "frk_adt.product_snoc"(%e, %b) : ({P_EMPTY}, i1) -> !frk_adt.product<[i1]>
                %s = "frk_adt.make_sum"(%p) {{variant = 1 : i64}} : (!frk_adt.product<[i1]>) -> {OPTION_I64}
                return %s : {OPTION_I64}
            }}"#
        ),
        "payload field 0 is i1",
    );
}

#[test]
fn extract_rules_still_fire() {
    let context = adt_context();
    let mk_some = format!(
        r#"%e = "frk_adt.product_new"() : () -> {P_EMPTY}
        %p = "frk_adt.product_snoc"(%e, %x) : ({P_EMPTY}, i64) -> {P_I64}
        %s = "frk_adt.make_sum"(%p) {{variant = 1 : i64}} : ({P_I64}) -> {OPTION_I64}"#
    );
    expect_finding(
        &context,
        &format!(
            r#"func.func @main(%x: i64) -> i64 {{
                {mk_some}
                %v = "frk_adt.extract"(%s) {{variant = 5 : i64, field = 0 : i64}} : ({OPTION_I64}) -> i64
                return %v : i64
            }}"#
        ),
        "variant 5 out of range",
    );
    expect_finding(
        &context,
        &format!(
            r#"func.func @main(%x: i64) -> i64 {{
                {mk_some}
                %v = "frk_adt.extract"(%s) {{variant = 1 : i64, field = 3 : i64}} : ({OPTION_I64}) -> i64
                return %v : i64
            }}"#
        ),
        "field 3 out of range",
    );
    expect_finding(
        &context,
        &format!(
            r#"func.func @main(%x: i64) -> i1 {{
                {mk_some}
                %v = "frk_adt.extract"(%s) {{variant = 1 : i64, field = 0 : i64}} : ({OPTION_I64}) -> i1
                return %v : i1
            }}"#
        ),
        "extract result type i1",
    );
}

#[test]
fn get_rules_still_fire() {
    let context = adt_context();
    expect_finding(
        &context,
        &format!(
            r#"func.func @main(%p: {P_I64}) -> i64 {{
                %f = "frk_adt.get"(%p) {{field = 4 : i64}} : ({P_I64}) -> i64
                return %f : i64
            }}"#
        ),
        "field 4 out of range",
    );
    expect_finding(
        &context,
        &format!(
            r#"func.func @main(%p: {P_I64}) -> i1 {{
                %f = "frk_adt.get"(%p) {{field = 0 : i64}} : ({P_I64}) -> i1
                return %f : i1
            }}"#
        ),
        "get result type i1",
    );
}

#[test]
fn ops_inside_nested_regions_are_reached() {
    let context = adt_context();
    expect_finding(
        &context,
        &format!(
            r#"func.func @main(%c: i1) -> i64 {{
                %r = scf.if %c -> (i64) {{
                    %e = "frk_adt.product_new"() : () -> {P_EMPTY}
                    %s = "frk_adt.make_sum"(%e) {{variant = 9 : i64}} : ({P_EMPTY}) -> {OPTION_I64}
                    %t = "frk_adt.tag_of"(%s) : ({OPTION_I64}) -> i64
                    scf.yield %t : i64
                }} else {{
                    %z = arith.constant 0 : i64
                    scf.yield %z : i64
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
    expect_finding(
        &context,
        r#"func.func @main(%s: !frk_adt.sum<5>) -> i64 {
            %v = "frk_adt.extract"(%s) {variant = 0 : i64, field = 0 : i64} : (!frk_adt.sum<5>) -> i64
            return %v : i64
        }"#,
        "must be an array of variants",
    );
}
