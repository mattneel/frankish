//! Parameter protocol fences (D-081.2) — non-differential witnesses:
//! the (p v) setter spelling hits the arity trap (chibi silently SETS
//! per SRFI-39, so the shape is corpus-poison), and the converter arg
//! to make-parameter is refused at compile time until its recorded
//! admission tests come due.

use frk_harness::runner::{InterpRunner, Runner};

fn run_scheme(name: &str, source: &str) -> Result<String, String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = root.join(format!("target/{name}-fixture"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("case.scm"), source).unwrap();
    std::fs::write(dir.join("expected.out"), "unreachable\n").unwrap();
    let cases = frk_harness::case::discover(&dir).unwrap();
    InterpRunner.run(&cases[0]).map_err(|e| e.to_string())
}

#[test]
fn parameter_setter_spelling_hits_the_arity_trap() {
    let error = run_scheme(
        "scm-param-arity",
        "(define p (make-parameter 1))\n(display (p 5))\n",
    )
    .expect_err("(p v) must hit the D-081 arity trap");
    assert!(error.contains("parameter protocol arity"), "{error}");
    assert!(error.contains("D-081"), "{error}");
}

#[test]
fn make_parameter_converter_is_refused_at_compile_time() {
    let error = run_scheme(
        "scm-param-conv",
        "(define p (make-parameter 1 (lambda (x) x)))\n(display (p))\n",
    )
    .expect_err("the converter form must be refused");
    assert!(error.contains("converter"), "{error}");
    assert!(error.contains("D-081"), "{error}");
}
