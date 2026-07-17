//! K3 verifiers for the D-075 itab lowering (law L1): the SAME
//! dispatch program the K2 tests run through the dictionary executes
//! natively through a real itab — stack table, method addresses,
//! indirect calls — and returns the same answer. Structural check:
//! nothing frk survives the pipeline.

use melior::ExecutionEngine;
use melior::ir::Module;
use melior::ir::operation::OperationLike;
use melior::pass::{self, PassManager};

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
        func.func @main() -> i64 attributes {{llvm.emit_c_interface}} {{
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
fn the_itab_dispatches_natively() {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    let mut module = Module::parse(&context, &program()).expect("parses");
    assert!(module.as_operation().verify(), "input verifies");
    frk_dialects::verify(&context, &module).expect("frk verification");

    let manager = PassManager::new(&context);
    manager.add_pass(frk_dialects::lower_kernel_pass(frk_dialects::Strategy::Arena));
    manager.add_pass(pass::conversion::create_scf_to_control_flow());
    manager.add_pass(pass::conversion::create_to_llvm());
    manager.add_pass(pass::conversion::create_reconcile_unrealized_casts());
    manager.run(&mut module).expect("pipeline");
    let lowered = module.as_operation().to_string();
    // Runtime symbols (frk_rt_*) are the legitimate survivors; no
    // kernel TYPE or OP may remain.
    assert!(!lowered.contains("!frk_"), "no kernel type survives:\n{lowered}");
    assert!(!lowered.contains("frk_dyn."), "no dyn op survives:\n{lowered}");

    let engine = ExecutionEngine::new(&module, 2, &[], false, false);
    unsafe {
        engine.register_symbol(
            "frk_rt_arena_alloc",
            frk_rt::frk_rt_arena_alloc as *mut (),
        );
    }
    let mut result: i64 = 0;
    unsafe {
        engine
            .invoke_packed("main", &mut [&mut result as *mut i64 as *mut ()])
            .expect("jit run");
    }
    // A: 10 + 5 = 15; B: 3 * 5 = 15 — through two different itabs.
    assert_eq!(result, 30);
}
