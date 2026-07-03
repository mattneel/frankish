//! The native half of the dyn trap contract (D-054): a wrong-tag
//! unwrap under AOT aborts the SUBPROCESS with the D-051 message.
//! (Interp semantics are verified in dyn_smoke; in-process JIT runs
//! are kept mismatch-free by corpus law.)

use frk_harness::runner::{AotRunner, Runner, Triple};

#[test]
fn aot_wrong_tag_unwrap_aborts_with_the_message() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = root.join("target/dyn-trap-fixture");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("case.mlir"),
        r#"func.func @main() -> i64 attributes {llvm.emit_c_interface} {
            %x = arith.constant 1.0 : f64
            %d = "frk_dyn.wrap"(%x) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
            %b = "frk_dyn.unwrap"(%d) {tag = 1 : i64} : (!frk_dyn.dyn) -> i1
            %r = arith.extui %b : i1 to i64
            return %r : i64
        }"#,
    )
    .unwrap();
    std::fs::write(dir.join("expected.out"), "unreachable\n").unwrap();

    let cases = frk_harness::case::discover(&dir).unwrap();
    let runner = AotRunner::new(Triple::X86_64Linux, frk_dialects::Strategy::Arena);
    let error = runner
        .run(&cases[0])
        .expect_err("wrong-tag unwrap must abort natively")
        .to_string();
    assert!(error.contains("dyn tag mismatch"), "{error}");
    assert!(error.contains("expected 1, got 2"), "{error}");
}
