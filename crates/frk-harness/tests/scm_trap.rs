//! The scheme fence traps (D-081; law L1 — these witnesses land with
//! the registry row, before any frontend emits the symbol). Interp
//! side: all three messages through the registered builtin. Native
//! side: the C twin's abort message through an AOT subprocess (the
//! dyn_native_trap recipe); in-process JIT runs are kept trap-free by
//! corpus law, same as every other REAL-bound abort symbol.

use frk_harness::runner::{AotRunner, Runner, Triple};
use melior::ir::Module;

fn interp_trap_message(code: i64) -> String {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    let module = Module::parse(
        &context,
        &format!(
            r#"module {{
                func.func private @frk_rt_scm_trap(i64)
                func.func @main() -> i64 attributes {{llvm.emit_c_interface}} {{
                    %c = arith.constant {code} : i64
                    func.call @frk_rt_scm_trap(%c) : (i64) -> ()
                    %z = arith.constant 0 : i64
                    return %z : i64
                }}
            }}"#
        ),
    )
    .expect("module");
    let mut interp = frk_interp::Interp::new(&module).expect("interp");
    frk_harness::runner::register_protocol_builtins(&mut interp);
    match interp.eval_function("main", &[]) {
        Err(error) => error.to_string(),
        Ok(_) => panic!("frk_rt_scm_trap({code}) must trap in the interp"),
    }
}

#[test]
fn interp_traps_carry_the_three_d081_messages() {
    let handler_returned = interp_trap_message(1);
    assert!(
        handler_returned.contains("exception handler returned (raise)"),
        "{handler_returned}"
    );
    let continuable_reraise = interp_trap_message(2);
    assert!(
        continuable_reraise
            .contains("guard re-raise of a continuable condition is fenced"),
        "{continuable_reraise}"
    );
    assert!(continuable_reraise.contains("Tier-2"), "{continuable_reraise}");
    let arity = interp_trap_message(3);
    assert!(arity.contains("parameter protocol arity"), "{arity}");
    for message in [&handler_returned, &continuable_reraise, &arity] {
        assert!(message.contains("D-081"), "{message}");
    }
}

#[test]
fn aot_scm_trap_aborts_with_the_message() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = root.join("target/scm-trap-fixture");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("case.mlir"),
        r#"module {
            func.func private @frk_rt_scm_trap(i64)
            func.func @main() -> i64 attributes {llvm.emit_c_interface} {
                %c = arith.constant 1 : i64
                func.call @frk_rt_scm_trap(%c) : (i64) -> ()
                %z = arith.constant 0 : i64
                return %z : i64
            }
        }"#,
    )
    .unwrap();
    std::fs::write(dir.join("expected.out"), "unreachable\n").unwrap();

    let cases = frk_harness::case::discover(&dir).unwrap();
    let runner = AotRunner::new(Triple::X86_64Linux, frk_dialects::Strategy::Arena);
    let error = runner
        .run(&cases[0])
        .expect_err("frk_rt_scm_trap must abort natively")
        .to_string();
    assert!(error.contains("exception handler returned (raise)"), "{error}");
    assert!(error.contains("D-081"), "{error}");
}
