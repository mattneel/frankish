//! K2/K1 verifiers for the D-073 record ops (law L1): field-granular
//! mutation on a box of a product — shared-cell semantics (aliases
//! observe writes), typed field projection, verify rejections.

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

const P2: &str = "!frk_adt.product<[i64, i64]>";
const REC: &str = "!frk_mem.box<!frk_adt.product<[i64, i64]>>";

fn build_record(a: i64, b: i64) -> String {
    format!(
        r#"%a = arith.constant {a} : i64
            %b = arith.constant {b} : i64
            %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
            %p1 = "frk_adt.product_snoc"(%e, %a) : (!frk_adt.product<[]>, i64) -> !frk_adt.product<[i64]>
            %p2 = "frk_adt.product_snoc"(%p1, %b) : (!frk_adt.product<[i64]>, i64) -> {P2}
            %r = "frk_mem.box_new"(%p2) : ({P2}) -> {REC}"#
    )
}

#[test]
fn field_get_projects_and_field_set_mutates_in_place() {
    let build = build_record(10, 20);
    let result = interpret_i64(&format!(
        r#"func.func @main() -> i64 {{
            {build}
            %seven = arith.constant 7 : i64
            "frk_mem.field_set"(%r, %seven) {{field = 0 : i64}} : ({REC}, i64) -> ()
            %x = "frk_mem.field_get"(%r) {{field = 0 : i64}} : ({REC}) -> i64
            %y = "frk_mem.field_get"(%r) {{field = 1 : i64}} : ({REC}) -> i64
            %sum = arith.addi %x, %y : i64
            return %sum : i64
        }}"#
    ))
    .unwrap();
    assert_eq!(result, 27); // 7 + 20 — the write landed, field 1 kept
}

#[test]
fn aliases_observe_field_writes() {
    // Pass the record through a call; the callee's write is visible
    // through the caller's reference — identity lives in the box.
    let build = build_record(1, 2);
    let result = interpret_i64(&format!(
        r#"func.func @bump(%r: {REC}) {{
            %v = "frk_mem.field_get"(%r) {{field = 1 : i64}} : ({REC}) -> i64
            %one = arith.constant 40 : i64
            %n = arith.addi %v, %one : i64
            "frk_mem.field_set"(%r, %n) {{field = 1 : i64}} : ({REC}, i64) -> ()
            return
        }}
        func.func @main() -> i64 {{
            {build}
            func.call @bump(%r) : ({REC}) -> ()
            %y = "frk_mem.field_get"(%r) {{field = 1 : i64}} : ({REC}) -> i64
            return %y : i64
        }}"#
    ))
    .unwrap();
    assert_eq!(result, 42);
}

#[test]
fn recursive_records_tie_and_read_back_through_the_knot() {
    // D-074: two records reference each other through type-erased
    // recrefs; the cycle ties with field_set and reads back with
    // rec_cast — identity survives erasure.
    const NP: &str = "!frk_adt.product<[i64, !frk_mem.recref]>";
    const NODE: &str = "!frk_mem.box<!frk_adt.product<[i64, !frk_mem.recref]>>";
    let result = interpret_i64(&format!(
        r#"func.func @main() -> i64 {{
            %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
            %a1 = arith.constant 30 : i64
            %b1 = arith.constant 12 : i64
            %pa0 = "frk_adt.product_snoc"(%e, %a1) : (!frk_adt.product<[]>, i64) -> !frk_adt.product<[i64]>
            %pb0 = "frk_adt.product_snoc"(%e, %b1) : (!frk_adt.product<[]>, i64) -> !frk_adt.product<[i64]>
            // Seed each next-field with a self-reference, then tie.
            %seeda = "frk_mem.box_new"(%pa0) : (!frk_adt.product<[i64]>) -> !frk_mem.box<!frk_adt.product<[i64]>>
            %ra0 = "frk_mem.rec_ref"(%seeda) : (!frk_mem.box<!frk_adt.product<[i64]>>) -> !frk_mem.recref
            %pa = "frk_adt.product_snoc"(%pa0, %ra0) : (!frk_adt.product<[i64]>, !frk_mem.recref) -> {NP}
            %pb = "frk_adt.product_snoc"(%pb0, %ra0) : (!frk_adt.product<[i64]>, !frk_mem.recref) -> {NP}
            %a = "frk_mem.box_new"(%pa) : ({NP}) -> {NODE}
            %b = "frk_mem.box_new"(%pb) : ({NP}) -> {NODE}
            %rb = "frk_mem.rec_ref"(%b) : ({NODE}) -> !frk_mem.recref
            %ra = "frk_mem.rec_ref"(%a) : ({NODE}) -> !frk_mem.recref
            "frk_mem.field_set"(%a, %rb) {{field = 1 : i64}} : ({NODE}, !frk_mem.recref) -> ()
            "frk_mem.field_set"(%b, %ra) {{field = 1 : i64}} : ({NODE}, !frk_mem.recref) -> ()
            // a.next.val + a.next.next.val = 12 + 30 = 42.
            %n1r = "frk_mem.field_get"(%a) {{field = 1 : i64}} : ({NODE}) -> !frk_mem.recref
            %n1 = "frk_mem.rec_cast"(%n1r) : (!frk_mem.recref) -> {NODE}
            %v1 = "frk_mem.field_get"(%n1) {{field = 0 : i64}} : ({NODE}) -> i64
            %n2r = "frk_mem.field_get"(%n1) {{field = 1 : i64}} : ({NODE}) -> !frk_mem.recref
            %n2 = "frk_mem.rec_cast"(%n2r) : (!frk_mem.recref) -> {NODE}
            %v2 = "frk_mem.field_get"(%n2) {{field = 0 : i64}} : ({NODE}) -> i64
            %sum = arith.addi %v1, %v2 : i64
            return %sum : i64
        }}"#
    ))
    .unwrap();
    assert_eq!(result, 42);
}

#[test]
fn verify_rejects_wrong_field_type_and_range() {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    let build = build_record(0, 0);
    for (op_text, expected) in [
        (
            format!(r#"%x = "frk_mem.field_get"(%r) {{field = 0 : i64}} : ({REC}) -> f64"#),
            "field_get yields",
        ),
        (
            format!(r#"%x = "frk_mem.field_get"(%r) {{field = 5 : i64}} : ({REC}) -> i64"#),
            "out of range",
        ),
    ] {
        let source = format!(
            r#"func.func @main() -> i64 {{
                {build}
                {op_text}
                %z = arith.constant 0 : i64
                return %z : i64
            }}"#
        );
        let module = Module::parse(&context, &source).expect("parses");
        let findings = frk_dialects::verify(&context, &module).unwrap_err();
        assert!(
            format!("{findings}").contains(expected),
            "wanted {expected:?} in: {findings}"
        );
    }
}

#[test]
fn verify_rejects_field_ops_on_non_product_boxes() {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    let source = r#"func.func @main() -> i64 {
        %v = arith.constant 3 : i64
        %r = "frk_mem.box_new"(%v) : (i64) -> !frk_mem.box<i64>
        %x = "frk_mem.field_get"(%r) {field = 0 : i64} : (!frk_mem.box<i64>) -> i64
        return %x : i64
    }"#;
    let module = Module::parse(&context, source).expect("parses");
    let findings = frk_dialects::verify(&context, &module).unwrap_err();
    assert!(
        format!("{findings}").contains("box of a product"),
        "{findings}"
    );
}
