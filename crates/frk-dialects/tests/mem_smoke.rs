//! K1/K2 verifiers for frk.mem (law L1). Shape negatives at both
//! verifier layers, eval semantics for the shared-cell model, and the
//! strategy-specific lowering shapes (arena vs rc symbols, the retain
//! and its transfer elision).

use frk_dialects::Strategy;
use frk_interp::Interp;
use melior::Context;
use melior::ir::Module;
use melior::ir::operation::OperationLike;
use melior::pass::PassManager;

fn mem_context() -> Context {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    context
}

fn frk_verify(context: &Context, source: &str) -> Result<(), String> {
    let module = Module::parse(context, source).expect("test source must parse");
    assert!(module.as_operation().verify(), "must pass MLIR verification");
    frk_dialects::verify(context, &module).map_err(|e| e.to_string())
}

#[test]
fn box_type_equations_are_enforced() {
    let context = mem_context();
    // Wrong payload type into box<i64>.
    let message = frk_verify(
        &context,
        r#"func.func @main(%b: i1) -> !frk_mem.box<i64> {
            %x = "frk_mem.box_new"(%b) : (i1) -> !frk_mem.box<i64>
            return %x : !frk_mem.box<i64>
        }"#,
    )
    .expect_err("must reject");
    assert!(message.contains("box_new stores a i1"), "{message}");

    // Wrong get result.
    let message = frk_verify(
        &context,
        r#"func.func @main(%b: !frk_mem.box<i64>) -> i1 {
            %v = "frk_mem.box_get"(%b) : (!frk_mem.box<i64>) -> i1
            return %v : i1
        }"#,
    )
    .expect_err("must reject");
    assert!(message.contains("box_get yields i1"), "{message}");

    // Wrong set payload.
    let message = frk_verify(
        &context,
        r#"func.func @main(%b: !frk_mem.box<i64>, %v: i1) -> i64 {
            "frk_mem.box_set"(%b, %v) : (!frk_mem.box<i64>, i1) -> ()
            %z = arith.constant 0 : i64
            return %z : i64
        }"#,
    )
    .expect_err("must reject");
    assert!(message.contains("box_set stores a i1"), "{message}");

    // IRDL layer: box ops over non-box operands.
    let module = Module::parse(
        &context,
        r#"func.func @main(%x: i64) -> i64 {
            %v = "frk_mem.box_get"(%x) : (i64) -> i64
            return %v : i64
        }"#,
    );
    assert!(module.is_none() || !module.unwrap().as_operation().verify());
}

#[test]
fn boxes_are_shared_cells_in_the_reference_semantics() {
    let context = mem_context();
    // Two aliases of one box observe each other's writes.
    let source = r#"func.func @main() -> i64 {
        %a = arith.constant 40 : i64
        %b = "frk_mem.box_new"(%a) : (i64) -> !frk_mem.box<i64>
        %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
        %p = "frk_adt.product_snoc"(%e, %b) : (!frk_adt.product<[]>, !frk_mem.box<i64>) -> !frk_adt.product<[!frk_mem.box<i64>]>
        %alias = "frk_adt.get"(%p) {field = 0 : i64} : (!frk_adt.product<[!frk_mem.box<i64>]>) -> !frk_mem.box<i64>
        %two = arith.constant 42 : i64
        "frk_mem.box_set"(%alias, %two) : (!frk_mem.box<i64>, i64) -> ()
        %r = "frk_mem.box_get"(%b) : (!frk_mem.box<i64>) -> i64
        return %r : i64
    }"#;
    let module = Module::parse(&context, source).unwrap();
    frk_dialects::verify(&context, &module).unwrap();
    let mut interp = Interp::new(&module).unwrap();
    frk_dialects::register_eval(&mut interp);
    let values = interp.eval_function("main", &[]).unwrap();
    assert_eq!(values[0].as_signed().unwrap(), 42);
}

fn lower(context: &Context, source: &str, strategy: Strategy) -> String {
    let mut module = Module::parse(context, source).expect("parse");
    let manager = PassManager::new(context);
    manager.add_pass(frk_dialects::lower_kernel_pass(strategy));
    manager.run(&mut module).expect("lowering");
    module.as_operation().to_string()
}

#[test]
fn strategies_pick_their_runtime_symbols() {
    let context = mem_context();
    let source = r#"func.func @main() -> i64 {
        %x = arith.constant 1 : i64
        %b = "frk_mem.box_new"(%x) : (i64) -> !frk_mem.box<i64>
        %v = "frk_mem.box_get"(%b) : (!frk_mem.box<i64>) -> i64
        return %v : i64
    }"#;
    let arena = lower(&context, source, Strategy::Arena);
    assert!(arena.contains("frk_rt_arena_alloc"), "{arena}");
    assert!(!arena.contains("frk_rt_rc"), "{arena}");

    let rc = lower(&context, source, Strategy::Rc);
    assert!(rc.contains("frk_rt_rc_alloc"), "{rc}");
    assert!(!rc.contains("frk_rt_arena_alloc"), "{rc}");
}

#[test]
fn rc_retains_shared_stores_and_elides_transfers() {
    let context = mem_context();
    // %b is used twice (stored AND read back) → shared → retained.
    let shared = r#"func.func @main() -> i64 {
        %x = arith.constant 1 : i64
        %b = "frk_mem.box_new"(%x) : (i64) -> !frk_mem.box<i64>
        %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
        %p = "frk_adt.product_snoc"(%e, %b) : (!frk_adt.product<[]>, !frk_mem.box<i64>) -> !frk_adt.product<[!frk_mem.box<i64>]>
        %v = "frk_mem.box_get"(%b) : (!frk_mem.box<i64>) -> i64
        return %v : i64
    }"#;
    // Note: the declaration alone also contains the symbol name — the
    // CALL is the assertion.
    let lowered = lower(&context, shared, Strategy::Rc);
    assert!(lowered.contains("llvm.call @frk_rt_rc_retain"), "{lowered}");

    // %b's only use is the store → ownership transfer → elided.
    let transfer = r#"func.func @main() -> i64 {
        %x = arith.constant 1 : i64
        %b = "frk_mem.box_new"(%x) : (i64) -> !frk_mem.box<i64>
        %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
        %p = "frk_adt.product_snoc"(%e, %b) : (!frk_adt.product<[]>, !frk_mem.box<i64>) -> !frk_adt.product<[!frk_mem.box<i64>]>
        %z = arith.constant 0 : i64
        return %z : i64
    }"#;
    let lowered = lower(&context, transfer, Strategy::Rc);
    assert!(!lowered.contains("llvm.call @frk_rt_rc_retain"), "{lowered}");
}

#[test]
fn block_local_allocations_release_and_escaping_ones_leak() {
    // GC ladder step 1 (D-053/D-054): a box whose uses all sit in its
    // own block is released before the terminator; a box that escapes
    // (returned) is not — the documented conservative frontier.
    let context = mem_context();
    let local = r#"func.func @main() -> i64 {
        %x = arith.constant 41 : i64
        %b = "frk_mem.box_new"(%x) : (i64) -> !frk_mem.box<i64>
        %v = "frk_mem.box_get"(%b) : (!frk_mem.box<i64>) -> i64
        %one = arith.constant 1 : i64
        %r = arith.addi %v, %one : i64
        return %r : i64
    }"#;
    let lowered = lower(&context, local, Strategy::Rc);
    assert!(
        lowered.contains("llvm.call @frk_rt_rc_release"),
        "block-local death must release: {lowered}"
    );
    // Arena never releases.
    let arena = lower(&context, local, Strategy::Arena);
    assert!(!arena.contains("frk_rt_rc_release"), "{arena}");

    let escaping = r#"func.func @main() -> !frk_mem.box<i64> {
        %x = arith.constant 41 : i64
        %b = "frk_mem.box_new"(%x) : (i64) -> !frk_mem.box<i64>
        return %b : !frk_mem.box<i64>
    }"#;
    let lowered = lower(&context, escaping, Strategy::Rc);
    assert!(
        !lowered.contains("llvm.call @frk_rt_rc_release"),
        "escaping values leak conservatively: {lowered}"
    );
}
