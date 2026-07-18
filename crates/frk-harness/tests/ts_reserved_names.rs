//! Synthesized-symbol namespace (M34; the D-082 landmine): user TS
//! top-level names flow into MLIR symbols verbatim, so names that
//! collide with synthesized functions (__frk_ctl_*, __exn_mark,
//! __try_body_N, the @main entry) are refused LOUDLY at the frontier
//! instead of dying in MLIR symbol redefinition mid-lowering.

use frk_harness::runner::{InterpRunner, Runner};

fn run_ts(name: &str, source: &str) -> Result<String, String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // TS cases need the repo root above them (the loanword producer).
    let dir = root.join("goldens").join(format!("zz_{name}"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("case.ts"), source).unwrap();
    std::fs::write(dir.join("expected.out"), "unreachable\n").unwrap();
    let cases = frk_harness::case::discover(&dir).unwrap();
    let result = InterpRunner.run(&cases[0]).map_err(|e| e.to_string());
    std::fs::remove_dir_all(&dir).unwrap();
    result
}

#[test]
fn double_underscore_names_are_refused() {
    let error = run_ts(
        "resv-skip",
        "function __frk_ctl_skip__(x: number): number {\n  return x + 1;\n}\ntry {\n  console.log(__frk_ctl_skip__(1));\n} finally {\n  console.log(\"fin\");\n}\n",
    )
    .expect_err("__-prefixed user names must be refused");
    assert!(error.contains("reserved for synthesized symbols"), "{error}");
    assert!(error.contains("D-082"), "{error}");
}

#[test]
fn user_main_is_refused() {
    let error = run_ts(
        "resv-main",
        "function main(): void {\n  console.log(1);\n}\nmain();\n",
    )
    .expect_err("a user function named main must be refused");
    assert!(error.contains("synthesized entry"), "{error}");
}
