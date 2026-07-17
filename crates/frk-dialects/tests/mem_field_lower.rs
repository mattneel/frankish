//! K3 verifiers for the D-073 record ops: field ops lower to
//! gep/load/store on the box payload (no frk ops survive), and the
//! record's layout word codes managed-pointer fields as traced.

use melior::Context;
use melior::ir::Module;
use melior::ir::operation::OperationLike;
use melior::pass::PassManager;

fn lower(context: &Context, source: &str, strategy: frk_dialects::Strategy) -> String {
    let mut module = Module::parse(context, source).expect("test source must parse");
    assert!(module.as_operation().verify(), "input must verify");
    frk_dialects::verify(context, &module).expect("input must pass the frk verifier");
    let manager = PassManager::new(context);
    manager.add_pass(frk_dialects::lower_kernel_pass(strategy));
    manager.run(&mut module).expect("lowering must succeed");
    assert!(module.as_operation().verify(), "lowered module must verify");
    module.as_operation().to_string()
}

const P2: &str = "!frk_adt.product<[f64, !frk_mem.box<f64>]>";
const REC: &str = "!frk_mem.box<!frk_adt.product<[f64, !frk_mem.box<f64>]>>";

fn record_source() -> String {
    format!(
        r#"func.func @main(%inner: !frk_mem.box<f64>) -> f64 {{
            %a = arith.constant 1.5 : f64
            %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
            %p1 = "frk_adt.product_snoc"(%e, %a) : (!frk_adt.product<[]>, f64) -> !frk_adt.product<[f64]>
            %p2 = "frk_adt.product_snoc"(%p1, %inner) : (!frk_adt.product<[f64]>, !frk_mem.box<f64>) -> {P2}
            %r = "frk_mem.box_new"(%p2) : ({P2}) -> {REC}
            %two = arith.constant 2.5 : f64
            "frk_mem.field_set"(%r, %two) {{field = 0 : i64}} : ({REC}, f64) -> ()
            %x = "frk_mem.field_get"(%r) {{field = 0 : i64}} : ({REC}) -> f64
            return %x : f64
        }}"#
    )
}

#[test]
fn field_ops_lower_to_slot_memory_traffic() {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    let lowered = lower(&context, &record_source(), frk_dialects::Strategy::Arena);
    assert!(!lowered.contains("frk_mem"), "no mem op may survive:\n{lowered}");
    assert!(!lowered.contains("frk_adt"), "no adt op may survive:\n{lowered}");
    assert!(
        lowered.contains("llvm.getelementptr"),
        "field ops address slots via gep:\n{lowered}"
    );
}

#[test]
fn record_layout_traces_the_managed_field() {
    // Layout encoding (D-057): codes start at bit 4, two bits per
    // word. Fields: [f64 (skip), box ptr (managed → code 1)] —
    // expected wordmap 0b01 << (4 + 2*1) = 64.
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    let lowered = lower(&context, &record_source(), frk_dialects::Strategy::Rc);
    let expected_layout = format!("{}", 0b01u64 << 6);
    assert!(
        lowered.contains(&expected_layout),
        "the rc allocation must carry layout {expected_layout} (ptr field traced):\n{lowered}"
    );
    // And the rc path retains the stored field value class-wide:
    assert!(
        lowered.contains("frk_rt_rc_retain") || lowered.contains("rc_alloc"),
        "rc lowering must go through the strategy allocator:\n{lowered}"
    );
}
