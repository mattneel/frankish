//! K2/K3 verifiers for D-078 global cells (law L1): one shared cell
//! per sym across functions; zero-initialized; the native global slot
//! IS the box (addressof, no allocation).

use frk_interp::{EvalError, Interp};
use melior::ExecutionEngine;
use melior::ir::Module;
use melior::ir::operation::OperationLike;
use melior::pass::{self, PassManager};

const CELL: &str = "!frk_mem.box<f64>";

fn program() -> String {
    format!(
        r#"
        "frk_mem.global_decl"() {{sym = "counter", cell = f64}} : () -> ()
        func.func @bump(%by: f64) -> f64 {{
            %c = "frk_mem.global_get"() {{sym = "counter"}} : () -> {CELL}
            %v = "frk_mem.box_get"(%c) : ({CELL}) -> f64
            %n = arith.addf %v, %by : f64
            "frk_mem.box_set"(%c, %n) : ({CELL}, f64) -> ()
            return %n : f64
        }}
        func.func @main() -> i64 attributes {{llvm.emit_c_interface}} {{
            %one = arith.constant 1.0 : f64
            %x = func.call @bump(%one) : (f64) -> f64
            %two = arith.constant 2.0 : f64
            %y = func.call @bump(%two) : (f64) -> f64
            %c = "frk_mem.global_get"() {{sym = "counter"}} : () -> {CELL}
            %v = "frk_mem.box_get"(%c) : ({CELL}) -> f64
            %r = arith.fptosi %v : f64 to i64
            return %r : i64
        }}"#
    )
}

#[test]
fn one_cell_across_functions_starting_zero() -> Result<(), EvalError> {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    let module = Module::parse(&context, &program()).expect("parses");
    assert!(module.as_operation().verify());
    frk_dialects::verify(&context, &module).expect("frk verify");
    let mut interp = Interp::new(&module)?;
    frk_dialects::register_eval(&mut interp);
    let values = interp.eval_function("main", &[])?;
    assert_eq!(values[0].as_signed()?, 3); // 0 + 1 + 2, one shared cell
    Ok(())
}

#[test]
fn the_native_global_slot_is_the_box() {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    let mut module = Module::parse(&context, &program()).expect("parses");
    frk_dialects::verify(&context, &module).expect("frk verify");
    let manager = PassManager::new(&context);
    manager.add_pass(frk_dialects::lower_kernel_pass(frk_dialects::Strategy::Arena));
    manager.add_pass(pass::conversion::create_scf_to_control_flow());
    manager.add_pass(pass::conversion::create_to_llvm());
    manager.add_pass(pass::conversion::create_reconcile_unrealized_casts());
    manager.run(&mut module).expect("pipeline");
    let lowered = module.as_operation().to_string();
    assert!(
        lowered.contains("llvm.mlir.global internal @__frk_g_counter"),
        "the cell is a module global:\n{lowered}"
    );
    assert!(
        !lowered.contains("frk_rt_arena_alloc"),
        "no allocation — the global slot IS the box:\n{lowered}"
    );
    let engine = ExecutionEngine::new(&module, 2, &[], false, false);
    let mut result: i64 = 0;
    unsafe {
        engine
            .invoke_packed("main", &mut [&mut result as *mut i64 as *mut ()])
            .expect("jit run");
    }
    assert_eq!(result, 3);
}
