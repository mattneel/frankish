//! D-098: `frk_bstr.lit` admits arbitrary 8-bit bytes, and the
//! lowering carries them verbatim into the interned global (byte-based
//! end to end — the D-056 v0.1 ASCII fence is lifted for the Calque
//! compiler's input channel, BOOTSTRAP §2.1). Structural checks here;
//! the interp-path byte check is the calque channel test's job.

use melior::Context;
use melior::ir::Module;
use melior::ir::operation::OperationLike;
use melior::pass::PassManager;

fn kernel_context() -> Context {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    context
}

#[test]
fn non_ascii_bstr_literals_verify_and_lower_byte_exact() {
    for strategy in [frk_dialects::Strategy::Arena, frk_dialects::Strategy::Rc] {
        let context = kernel_context();
        // "caf\u{e9}" = 63 61 66 C3 A9 — five BYTES, not four.
        let source = r#"func.func @main() -> i64 {
            %s = "frk_bstr.lit"() {text = "caf\C3\A9"} : () -> !frk_bstr.str
            %n = "frk_bstr.len"(%s) : (!frk_bstr.str) -> i64
            return %n : i64
        }"#;
        let mut module = Module::parse(&context, source).expect("must parse");
        assert!(module.as_operation().verify(), "must verify as MLIR");
        frk_dialects::verify(&context, &module)
            .expect("D-098: non-ASCII bstr bytes must pass the verifier");
        let manager = PassManager::new(&context);
        manager.add_pass(frk_dialects::lower_kernel_pass(strategy));
        manager.run(&mut module).expect("lowering must succeed");
        assert!(module.as_operation().verify(), "lowered must verify");
        let text = module.as_operation().to_string();
        // MLIR prints i8 values SIGNED: 0xC3 0xA9 as -61, -87 (or, on
        // unsigned printers, 195, 169).
        assert!(
            text.contains("-61, -87") || text.contains("195, 169"),
            "the raw bytes must survive the lowering ({strategy:?}):\n{text}"
        );
        assert!(
            text.contains("constant 5 : i64"),
            "len must count BYTES (5), not chars ({strategy:?}):\n{text}"
        );
        assert!(
            !text.contains("frk_bstr.lit"),
            "no kernel bstr ops may remain ({strategy:?}):\n{text}"
        );
        assert!(
            text.contains("frk_rt_bstr_intern"),
            "the literal must intern through the byte-based rt call ({strategy:?})"
        );
    }
}
