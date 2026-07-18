//! K3 verifier for D-077's boxed pairs: a wrap{6} of a box-of-
//! [dyn,dyn] with field_get on a WORDS field (the multi-slot record
//! path the D-073 fence originally refused — scheme's mutable pairs
//! were its second consumer).

#[test]
fn boxed_pair_wrap_lowers() {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    const PROD: &str = "!frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>";
    const BOX: &str = "!frk_mem.box<!frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>>";
    let source = format!(
        r#"func.func @main() -> i64 {{
            %z = arith.constant 0 : i64
            %n = "frk_dyn.wrap"(%z) {{tag = 0 : i64}} : (i64) -> !frk_dyn.dyn
            %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
            %p1 = "frk_adt.product_snoc"(%e, %n) : (!frk_adt.product<[]>, !frk_dyn.dyn) -> !frk_adt.product<[!frk_dyn.dyn]>
            %p2 = "frk_adt.product_snoc"(%p1, %n) : (!frk_adt.product<[!frk_dyn.dyn]>, !frk_dyn.dyn) -> {PROD}
            %cell = "frk_mem.box_new"(%p2) : ({PROD}) -> {BOX}
            %pair = "frk_dyn.wrap"(%cell) {{tag = 6 : i64}} : ({BOX}) -> !frk_dyn.dyn
            %cb = "frk_dyn.unwrap"(%pair) {{tag = 6 : i64}} : (!frk_dyn.dyn) -> {BOX}
            %car = "frk_mem.field_get"(%cb) {{field = 0 : i64}} : ({BOX}) -> !frk_dyn.dyn
            return %z : i64
        }}"#
    );
    let mut module = melior::ir::Module::parse(&context, &source).expect("parses");
    use melior::ir::operation::OperationLike;
    assert!(module.as_operation().verify());
    frk_dialects::verify(&context, &module).expect("frk verify");
    let manager = melior::pass::PassManager::new(&context);
    manager.add_pass(frk_dialects::lower_kernel_pass(frk_dialects::Strategy::Arena));
    if manager.run(&mut module).is_err() {
        panic!("lowering failed:\n{}", module.as_operation());
    }
}
