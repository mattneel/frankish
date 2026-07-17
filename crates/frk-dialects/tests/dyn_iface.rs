//! K1/K2 verifiers for structural interfaces (D-075; law L1): the
//! iface_make/iface_call pair dispatches through the dictionary
//! representation — two classes, one interface, the right method
//! bodies run with the right receivers.

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

// Two "classes": records with one i64 field, different method impls.
const REC: &str = "!frk_mem.box<!frk_adt.product<[i64]>>";
const P1: &str = "!frk_adt.product<[i64]>";
const ARGS1: &str = "!frk_adt.product<[i64]>";

fn program() -> String {
    format!(
        r#"
        func.func @A__get(%this: {REC}, %bump: i64) -> i64 {{
            %v = "frk_mem.field_get"(%this) {{field = 0 : i64}} : ({REC}) -> i64
            %r = arith.addi %v, %bump : i64
            return %r : i64
        }}
        func.func @B__get(%this: {REC}, %bump: i64) -> i64 {{
            %v = "frk_mem.field_get"(%this) {{field = 0 : i64}} : ({REC}) -> i64
            %r = arith.muli %v, %bump : i64
            return %r : i64
        }}
        func.func @via(%i: !frk_dyn.iface, %bump: i64) -> i64 {{
            %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
            %p = "frk_adt.product_snoc"(%e, %bump) : (!frk_adt.product<[]>, i64) -> {ARGS1}
            %r = "frk_dyn.iface_call"(%i, %p) {{method = 0 : i64}} : (!frk_dyn.iface, {ARGS1}) -> i64
            return %r : i64
        }}
        func.func @main() -> i64 {{
            %ten = arith.constant 10 : i64
            %three = arith.constant 3 : i64
            %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
            %pa = "frk_adt.product_snoc"(%e, %ten) : (!frk_adt.product<[]>, i64) -> {P1}
            %pb = "frk_adt.product_snoc"(%e, %three) : (!frk_adt.product<[]>, i64) -> {P1}
            %a = "frk_mem.box_new"(%pa) : ({P1}) -> {REC}
            %b = "frk_mem.box_new"(%pb) : ({P1}) -> {REC}
            %ia = "frk_dyn.iface_make"(%a) {{methods = [@A__get]}} : ({REC}) -> !frk_dyn.iface
            %ib = "frk_dyn.iface_make"(%b) {{methods = [@B__get]}} : ({REC}) -> !frk_dyn.iface
            %five = arith.constant 5 : i64
            %ra = func.call @via(%ia, %five) : (!frk_dyn.iface, i64) -> i64
            %rb = func.call @via(%ib, %five) : (!frk_dyn.iface, i64) -> i64
            %sum = arith.addi %ra, %rb : i64
            return %sum : i64
        }}"#
    )
}

#[test]
fn one_interface_dispatches_to_two_classes() {
    // A: 10 + 5 = 15; B: 3 * 5 = 15 — same call site, different impls.
    assert_eq!(interpret_i64(&program()).unwrap(), 30);
}

#[test]
fn dispatch_sees_receiver_mutation() {
    // The iface holds the OBJECT, not a copy: mutate after iface_make,
    // dispatch sees the new state.
    let result = interpret_i64(&format!(
        r#"
        func.func @A__get(%this: {REC}, %bump: i64) -> i64 {{
            %v = "frk_mem.field_get"(%this) {{field = 0 : i64}} : ({REC}) -> i64
            %r = arith.addi %v, %bump : i64
            return %r : i64
        }}
        func.func @main() -> i64 {{
            %ten = arith.constant 10 : i64
            %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
            %pa = "frk_adt.product_snoc"(%e, %ten) : (!frk_adt.product<[]>, i64) -> {P1}
            %a = "frk_mem.box_new"(%pa) : ({P1}) -> {REC}
            %i = "frk_dyn.iface_make"(%a) {{methods = [@A__get]}} : ({REC}) -> !frk_dyn.iface
            %forty = arith.constant 40 : i64
            "frk_mem.field_set"(%a, %forty) {{field = 0 : i64}} : ({REC}, i64) -> ()
            %two = arith.constant 2 : i64
            %ep = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
            %args = "frk_adt.product_snoc"(%ep, %two) : (!frk_adt.product<[]>, i64) -> {ARGS1}
            %r = "frk_dyn.iface_call"(%i, %args) {{method = 0 : i64}} : (!frk_dyn.iface, {ARGS1}) -> i64
            return %r : i64
        }}"#
    ))
    .unwrap();
    assert_eq!(result, 42); // 40 + 2 — the write through the box is seen
}

#[test]
fn verify_rejects_an_empty_method_list() {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    let source = format!(
        r#"func.func @main() -> i64 {{
            %ten = arith.constant 10 : i64
            %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
            %p = "frk_adt.product_snoc"(%e, %ten) : (!frk_adt.product<[]>, i64) -> {P1}
            %a = "frk_mem.box_new"(%p) : ({P1}) -> {REC}
            %i = "frk_dyn.iface_make"(%a) {{methods = []}} : ({REC}) -> !frk_dyn.iface
            %z = arith.constant 0 : i64
            return %z : i64
        }}"#
    );
    let module = Module::parse(&context, &source).expect("parses");
    let findings = frk_dialects::verify(&context, &module).unwrap_err();
    assert!(
        format!("{findings}").contains("at least one method"),
        "{findings}"
    );
}
