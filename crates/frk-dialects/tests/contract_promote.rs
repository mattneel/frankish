//! Verifiers for the D-072 promotion pass (law L1): forward
//! must-dataflow over cf edges deletes every narrow it can PROVE from
//! `tag_of` tests; everything else survives to runtime. Each test
//! hand-writes a CFG shape and asserts which fate its narrows meet.

use melior::ir::Module;
use melior::ir::operation::OperationLike;

const SHAPE: &str = "!frk_adt.sum<[[f64], [f64]]>";
const TRI: &str = "!frk_adt.sum<[[f64], [f64], [f64]]>";

/// Parses, verifies, promotes; returns (promoted, surviving).
fn promote(source: &str) -> (usize, usize) {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    let module = Module::parse(&context, source).expect("test source must parse");
    assert!(module.as_operation().verify(), "must pass MLIR verification");
    frk_dialects::verify(&context, &module).expect("must pass frk semantic verification");
    let counts = frk_dialects::contract::promote_narrows(module.as_operation())
        .expect("promotion");
    assert!(
        module.as_operation().verify(),
        "module must still verify after promotion:\n{}",
        module.as_operation()
    );
    counts
}

#[test]
fn eq_true_edge_proves_the_narrow() {
    // if (tag(s) == 0) { narrow-to-0 }  — the direct discriminant test.
    let (promoted, surviving) = promote(&format!(
        r#"func.func @f(%s: {SHAPE}) -> f64 {{
            %tag = "frk_adt.tag_of"(%s) : ({SHAPE}) -> i64
            %zero = arith.constant 0 : i64
            %hit = arith.cmpi eq, %tag, %zero : i64
            cf.cond_br %hit, ^then, ^else
        ^then:
            %n = "frk_contract.narrow"(%s) {{variant = 0 : i64, blame = "t"}} : ({SHAPE}) -> {SHAPE}
            %r = "frk_adt.extract"(%n) {{variant = 0 : i64, field = 0 : i64}} : ({SHAPE}) -> f64
            return %r : f64
        ^else:
            %z = arith.constant 0.0 : f64
            return %z : f64
        }}"#
    ));
    assert_eq!((promoted, surviving), (1, 0));
}

#[test]
fn else_edge_implies_the_other_variant_of_two() {
    // Two variants: the false edge of (tag == 0) proves variant 1 —
    // the mask-subtraction rule (D-072).
    let (promoted, surviving) = promote(&format!(
        r#"func.func @f(%s: {SHAPE}) -> f64 {{
            %tag = "frk_adt.tag_of"(%s) : ({SHAPE}) -> i64
            %zero = arith.constant 0 : i64
            %hit = arith.cmpi eq, %tag, %zero : i64
            cf.cond_br %hit, ^then, ^else
        ^then:
            %z = arith.constant 0.0 : f64
            return %z : f64
        ^else:
            %n = "frk_contract.narrow"(%s) {{variant = 1 : i64, blame = "e"}} : ({SHAPE}) -> {SHAPE}
            %r = "frk_adt.extract"(%n) {{variant = 1 : i64, field = 0 : i64}} : ({SHAPE}) -> f64
            return %r : f64
        }}"#
    ));
    assert_eq!((promoted, surviving), (1, 0));
}

#[test]
fn chained_subtraction_proves_the_last_of_three() {
    // else-if chain over three variants: two false edges leave {2}.
    let (promoted, surviving) = promote(&format!(
        r#"func.func @f(%s: {TRI}) -> f64 {{
            %tag = "frk_adt.tag_of"(%s) : ({TRI}) -> i64
            %zero = arith.constant 0 : i64
            %h0 = arith.cmpi eq, %tag, %zero : i64
            cf.cond_br %h0, ^v0, ^t1
        ^v0:
            %a = "frk_contract.narrow"(%s) {{variant = 0 : i64, blame = "a"}} : ({TRI}) -> {TRI}
            %ra = "frk_adt.extract"(%a) {{variant = 0 : i64, field = 0 : i64}} : ({TRI}) -> f64
            return %ra : f64
        ^t1:
            %tag1 = "frk_adt.tag_of"(%s) : ({TRI}) -> i64
            %one = arith.constant 1 : i64
            %h1 = arith.cmpi eq, %tag1, %one : i64
            cf.cond_br %h1, ^v1, ^v2
        ^v1:
            %b = "frk_contract.narrow"(%s) {{variant = 1 : i64, blame = "b"}} : ({TRI}) -> {TRI}
            %rb = "frk_adt.extract"(%b) {{variant = 1 : i64, field = 0 : i64}} : ({TRI}) -> f64
            return %rb : f64
        ^v2:
            %c = "frk_contract.narrow"(%s) {{variant = 2 : i64, blame = "c"}} : ({TRI}) -> {TRI}
            %rc = "frk_adt.extract"(%c) {{variant = 2 : i64, field = 0 : i64}} : ({TRI}) -> f64
            return %rc : f64
        }}"#
    ));
    assert_eq!((promoted, surviving), (3, 0));
}

#[test]
fn undominated_narrow_survives() {
    // No test anywhere: the imported fact cannot be re-derived —
    // it stays a runtime check (the demotion fate).
    let (promoted, surviving) = promote(&format!(
        r#"func.func @f(%s: {SHAPE}) -> f64 {{
            %n = "frk_contract.narrow"(%s) {{variant = 0 : i64, blame = "u"}} : ({SHAPE}) -> {SHAPE}
            %r = "frk_adt.extract"(%n) {{variant = 0 : i64, field = 0 : i64}} : ({SHAPE}) -> f64
            return %r : f64
        }}"#
    ));
    assert_eq!((promoted, surviving), (0, 1));
}

#[test]
fn wrong_variant_claim_survives_a_contradicting_test() {
    // Dominated by (tag == 0) but claiming variant 1: the mask {0} is
    // not a subset of {1} — the false fact reaches runtime and traps
    // there, never silently.
    let (promoted, surviving) = promote(&format!(
        r#"func.func @f(%s: {SHAPE}) -> f64 {{
            %tag = "frk_adt.tag_of"(%s) : ({SHAPE}) -> i64
            %zero = arith.constant 0 : i64
            %hit = arith.cmpi eq, %tag, %zero : i64
            cf.cond_br %hit, ^then, ^else
        ^then:
            %n = "frk_contract.narrow"(%s) {{variant = 1 : i64, blame = "w"}} : ({SHAPE}) -> {SHAPE}
            %r = "frk_adt.extract"(%n) {{variant = 1 : i64, field = 0 : i64}} : ({SHAPE}) -> f64
            return %r : f64
        ^else:
            %z = arith.constant 0.0 : f64
            return %z : f64
        }}"#
    ));
    assert_eq!((promoted, surviving), (0, 1));
}

#[test]
fn join_of_incompatible_paths_survives() {
    // Both arms reach the join; only one proves variant 0 — the union
    // at the merge is the full mask, so the narrow survives.
    let (promoted, surviving) = promote(&format!(
        r#"func.func @f(%s: {SHAPE}) -> f64 {{
            %tag = "frk_adt.tag_of"(%s) : ({SHAPE}) -> i64
            %zero = arith.constant 0 : i64
            %hit = arith.cmpi eq, %tag, %zero : i64
            cf.cond_br %hit, ^then, ^join
        ^then:
            cf.br ^join
        ^join:
            %n = "frk_contract.narrow"(%s) {{variant = 0 : i64, blame = "j"}} : ({SHAPE}) -> {SHAPE}
            %r = "frk_adt.extract"(%n) {{variant = 0 : i64, field = 0 : i64}} : ({SHAPE}) -> f64
            return %r : f64
        }}"#
    ));
    assert_eq!((promoted, surviving), (0, 1));
}

#[test]
fn facts_survive_a_loop_body() {
    // The test dominates a loop; the narrow inside the body stays
    // proven across the back edge (facts never invalidate — sums are
    // pure values).
    let (promoted, surviving) = promote(&format!(
        r#"func.func @f(%s: {SHAPE}, %k: i64) -> f64 {{
            %tag = "frk_adt.tag_of"(%s) : ({SHAPE}) -> i64
            %zero = arith.constant 0 : i64
            %hit = arith.cmpi eq, %tag, %zero : i64
            cf.cond_br %hit, ^head(%k : i64), ^exit
        ^head(%i: i64):
            %done = arith.cmpi eq, %i, %zero : i64
            cf.cond_br %done, ^exit, ^body(%i : i64)
        ^body(%j: i64):
            %n = "frk_contract.narrow"(%s) {{variant = 0 : i64, blame = "l"}} : ({SHAPE}) -> {SHAPE}
            %r = "frk_adt.extract"(%n) {{variant = 0 : i64, field = 0 : i64}} : ({SHAPE}) -> f64
            %one = arith.constant 1 : i64
            %next = arith.subi %j, %one : i64
            cf.br ^head(%next : i64)
        ^exit:
            %z = arith.constant 0.0 : f64
            return %z : f64
        }}"#
    ));
    assert_eq!((promoted, surviving), (1, 0));
}

#[test]
fn chained_narrows_collapse_to_the_root() {
    // narrow(narrow(s)): both prove from one test; the inner RAUW
    // re-roots the outer before erasure.
    let (promoted, surviving) = promote(&format!(
        r#"func.func @f(%s: {SHAPE}) -> f64 {{
            %tag = "frk_adt.tag_of"(%s) : ({SHAPE}) -> i64
            %zero = arith.constant 0 : i64
            %hit = arith.cmpi eq, %tag, %zero : i64
            cf.cond_br %hit, ^then, ^else
        ^then:
            %n1 = "frk_contract.narrow"(%s) {{variant = 0 : i64, blame = "1"}} : ({SHAPE}) -> {SHAPE}
            %n2 = "frk_contract.narrow"(%n1) {{variant = 0 : i64, blame = "2"}} : ({SHAPE}) -> {SHAPE}
            %r = "frk_adt.extract"(%n2) {{variant = 0 : i64, field = 0 : i64}} : ({SHAPE}) -> f64
            return %r : f64
        ^else:
            %z = arith.constant 0.0 : f64
            return %z : f64
        }}"#
    ));
    assert_eq!((promoted, surviving), (2, 0));
}
