//! K3 verifiers for the frk.contract lowering (D-072; law L1).
//! Structural: a PROVEN narrow vanishes inside the lowering pass (the
//! promotion runs at lower_kernel entry); a DEMOTED narrow becomes a
//! straight-line frk_rt_contract_check call carrying its blame bytes.
//! Semantic equivalence is the corpus's job under the differential law.

use melior::Context;
use melior::ir::Module;
use melior::ir::operation::OperationLike;
use melior::pass::PassManager;

fn contract_context() -> Context {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    context
}

fn lower(context: &Context, source: &str) -> String {
    let mut module = Module::parse(context, source).expect("test source must parse");
    assert!(module.as_operation().verify(), "input must verify");
    frk_dialects::verify(context, &module).expect("input must pass the frk verifier");

    let manager = PassManager::new(context);
    manager.add_pass(frk_dialects::lower_kernel_pass(frk_dialects::Strategy::Arena));
    manager.run(&mut module).expect("lowering must succeed");
    assert!(module.as_operation().verify(), "lowered module must verify");
    module.as_operation().to_string()
}

const SHAPE: &str = "!frk_adt.sum<[[f64], [f64]]>";

#[test]
fn proven_narrow_lowers_to_nothing() {
    let context = contract_context();
    let lowered = lower(
        &context,
        &format!(
            r#"func.func @f(%s: {SHAPE}) -> f64 {{
                %tag = "frk_adt.tag_of"(%s) : ({SHAPE}) -> i64
                %zero = arith.constant 0 : i64
                %hit = arith.cmpi eq, %tag, %zero : i64
                cf.cond_br %hit, ^then, ^else
            ^then:
                %n = "frk_contract.narrow"(%s) {{variant = 0 : i64, blame = "proven at case.ts:1:1"}} : ({SHAPE}) -> {SHAPE}
                %r = "frk_adt.extract"(%n) {{variant = 0 : i64, field = 0 : i64}} : ({SHAPE}) -> f64
                return %r : f64
            ^else:
                %z = arith.constant 0.0 : f64
                return %z : f64
            }}"#
        ),
    );
    assert!(
        !lowered.contains("frk_contract"),
        "no contract op may survive lowering:\n{lowered}"
    );
    assert!(
        !lowered.contains("frk_rt_contract_check"),
        "a proven narrow must not emit a runtime check:\n{lowered}"
    );
}

#[test]
fn demoted_narrow_lowers_to_a_blame_carrying_check() {
    let context = contract_context();
    let lowered = lower(
        &context,
        &format!(
            r#"func.func @f(%s: {SHAPE}) -> f64 {{
                %n = "frk_contract.narrow"(%s) {{variant = 1 : i64, blame = "demoted at case.ts:9:5"}} : ({SHAPE}) -> {SHAPE}
                %r = "frk_adt.extract"(%n) {{variant = 1 : i64, field = 0 : i64}} : ({SHAPE}) -> f64
                return %r : f64
            }}"#
        ),
    );
    assert!(
        !lowered.contains("frk_contract"),
        "no contract op may survive lowering:\n{lowered}"
    );
    assert!(
        lowered.contains("frk_rt_contract_check"),
        "a demoted narrow must call the runtime check:\n{lowered}"
    );
    // Blame bytes live in a module global ("demoted at case.ts:9:5"
    // starts with 'd' = 100, 'e' = 101, 'm' = 109).
    assert!(
        lowered.contains("__frk_blame_"),
        "blame global missing:\n{lowered}"
    );
    assert!(
        lowered.contains("100, 101, 109"),
        "blame bytes missing from the global:\n{lowered}"
    );
}
