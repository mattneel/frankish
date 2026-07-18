//! Plain raise trap paths (D-081.4) — non-differential by design:
//! chibi models handler-returned as a secondary CATCHABLE exception
//! (exit 0 under a guard!) and prints uncaught raises to stderr with
//! exit 70; we trap deterministically on both twins. These interp
//! witnesses pin the messages; the C twin's channel is proven by
//! scm_trap.rs's AOT subprocess test.

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
fn handler_returning_from_plain_raise_traps() {
    let error = run_scheme(
        "scm-raise-hret",
        "(with-exception-handler (lambda (e) 42)\n  (lambda () (raise 7) (display \"after\")))\n",
    )
    .expect_err("a handler returning from a plain raise must trap");
    assert!(error.contains("exception handler returned (raise)"), "{error}");
    assert!(error.contains("D-081"), "{error}");
}

#[test]
fn uncaught_plain_raise_hits_the_unhandled_effect_trap() {
    let error = run_scheme("scm-raise-uncaught", "(display 1) (newline) (raise 7)\n")
        .expect_err("an uncaught raise must trap");
    assert!(error.contains("unhandled effect"), "{error}");
}
