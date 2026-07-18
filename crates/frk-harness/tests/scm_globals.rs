//! Use-before-define of a top-level value (D-081.1): DETERMINISTIC
//! nil on both twins — the globals array is nil-filled at main entry
//! (D-077's "fill REQUIRED" precedent), so an unwritten slot reads as
//! '() everywhere instead of splitting interp (Float-zero error) from
//! native (zeroed words). Chibi ERRORS here ("undefined variable",
//! exit 70), which auto-excludes the shape from the corpus — this
//! unit test is the fence's witness (never differential).

use frk_harness::runner::{InterpRunner, JitRunner, Runner};

#[test]
fn use_before_define_reads_deterministic_nil_on_both_twins() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = root.join("target/scm-ubd-fixture");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("case.scm"),
        "(define (f) z)\n(display (f)) (newline)\n(define z 7)\n(display (f)) (newline)\n",
    )
    .unwrap();
    std::fs::write(dir.join("expected.out"), "()\n7\n").unwrap();

    let cases = frk_harness::case::discover(&dir).unwrap();
    let expected = "()\n7\n";
    let interp = InterpRunner.run(&cases[0]).expect("interp runs");
    assert_eq!(interp, expected, "interp");
    let jit = JitRunner { strategy: frk_dialects::Strategy::Arena }
        .run(&cases[0])
        .expect("jit runs");
    assert_eq!(jit, expected, "jit-arena");
    let jit_rc = JitRunner { strategy: frk_dialects::Strategy::Rc }
        .run(&cases[0])
        .expect("jit-rc runs");
    assert_eq!(jit_rc, expected, "jit-rc");
}
