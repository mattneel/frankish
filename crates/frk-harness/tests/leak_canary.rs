//! The leak canary (D-041's ratification rider paid off; D-053 step
//! 1): under the rc strategy, block-locally dying allocations are
//! RELEASED at runtime — measured through the counters that were
//! installed for exactly this day. Counters are process-cumulative,
//! so the test reads deltas around one JIT run.

use frk_harness::case::SourceKind;
use melior::ExecutionEngine;

#[test]
fn dying_boxes_release_at_runtime_under_rc() {
    let context = frk_core::context();
    frk_dialects::register(&context).unwrap();
    // Three allocations, all dying block-locally.
    let source = r#"func.func @main() -> i64 attributes {llvm.emit_c_interface} {
        %x = arith.constant 14 : i64
        %a = "frk_mem.box_new"(%x) : (i64) -> !frk_mem.box<i64>
        %b = "frk_mem.box_new"(%x) : (i64) -> !frk_mem.box<i64>
        %c = "frk_mem.box_new"(%x) : (i64) -> !frk_mem.box<i64>
        %va = "frk_mem.box_get"(%a) : (!frk_mem.box<i64>) -> i64
        %vb = "frk_mem.box_get"(%b) : (!frk_mem.box<i64>) -> i64
        %vc = "frk_mem.box_get"(%c) : (!frk_mem.box<i64>) -> i64
        %s1 = arith.addi %va, %vb : i64
        %s2 = arith.addi %s1, %vc : i64
        return %s2 : i64
    }"#;
    let mut module = melior::ir::Module::parse(&context, source).unwrap();
    frk_dialects::verify(&context, &module).unwrap();
    frk_harness::pipeline::lower_to_llvm(&context, &mut module, frk_dialects::Strategy::Rc)
        .unwrap();

    let engine = ExecutionEngine::new(&module, 2, &[], false, false);
    unsafe {
        engine.register_symbol("frk_rt_rc_alloc", frk_rt::frk_rt_rc_alloc as *mut ());
        engine.register_symbol("frk_rt_rc_retain", frk_rt::frk_rt_rc_retain as *mut ());
        engine.register_symbol("frk_rt_rc_release", frk_rt::frk_rt_rc_release as *mut ());
    }

    let allocs_before = frk_rt::frk_rt_alloc_count();
    let releases_before = frk_rt::frk_rt_rc_release_count();
    let mut result: i64 = 0;
    unsafe {
        engine
            .invoke_packed("main", &mut [&mut result as *mut i64 as *mut ()])
            .unwrap();
    }
    assert_eq!(result, 42);
    let allocs = frk_rt::frk_rt_alloc_count() - allocs_before;
    let releases = frk_rt::frk_rt_rc_release_count() - releases_before;
    assert_eq!(allocs, 3, "three boxes allocated");
    assert_eq!(releases, 3, "all three die block-locally and release");

    // Silence the unused-import lint honestly: SourceKind is the
    // harness surface this test deliberately bypasses (no golden can
    // read process-cumulative counters).
    let _ = SourceKind::Mlir;
}
