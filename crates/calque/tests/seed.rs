//! The seed's end-to-end tests (M37, D-086): b0 Calque source →
//! seed (a femto_lua program) → kernel-IR artifact → parse → verify →
//! interp, with expected outputs hand-derived from SPEC semantics
//! (L2's no-blessing rule; there is no upstream oracle, MANIFEST
//! "Conformance").

use calque::{run_artifact, seed_translate, UnitFile};

fn unit(name: &str, ns: Option<&str>, text: &str) -> UnitFile {
    UnitFile {
        name: name.to_string(),
        ns: ns.map(str::to_string),
        text: text.to_string(),
    }
}

fn stdlib_str() -> UnitFile {
    unit(
        "tools/calque/std/str.calq",
        Some("str"),
        include_str!("../../../tools/calque/std/str.calq"),
    )
}

#[test]
fn seed_compiles_fib_to_the_expected_bytes() {
    let files = vec![
        stdlib_str(),
        unit(
            "specimens/calque/examples/fib.calq",
            None,
            include_str!("../../../specimens/calque/examples/fib.calq"),
        ),
    ];
    let artifact = seed_translate(&files).expect("the seed translates fib.calq");
    assert!(
        artifact.starts_with("// calque-artifact v0"),
        "the seed must emit the version marker first"
    );
    let outcome = run_artifact(&artifact).expect("the artifact runs");
    assert_eq!(outcome.output, "55\n", "fib.calq prints 55 (hand-derived)");
    assert_eq!(outcome.values[0].as_signed().expect("i64 result"), 0);
}

#[test]
fn seed_compiles_the_exact_from_i64_vectors() {
    let files = vec![
        stdlib_str(),
        unit(
            "crates/calque/tests/assets/from_i64_probe.calq",
            None,
            include_str!("assets/from_i64_probe.calq"),
        ),
    ];
    let artifact = seed_translate(&files).expect("the seed translates the probe");
    let outcome = run_artifact(&artifact).expect("the artifact runs");
    assert_eq!(
        outcome.output, "0 7 1234 -42 9223372036854775807 -9223372036854775808\n",
        "every i64 renders exactly (SPEC §6.8; hand-derived vectors)"
    );
}

#[test]
fn seed_refuses_non_ascii_input_loudly() {
    let files = vec![unit("bad.calq", None, "pub fn main() i64 { 0 } // héllo")];
    let error = seed_translate(&files).expect_err("bytes >= 0x80 are refused (SPEC F18)");
    assert!(error.contains("non-ASCII byte"), "{error}");
}

#[test]
fn seed_reports_source_mapped_errors() {
    // A feature outside seed v0 must fail with the marker protocol
    // (file:line:col), never mis-translate.
    let files = vec![unit(
        "bad.calq",
        None,
        "pub fn main() i64 {\n    const x: i64 = 1 * 2;\n    x\n}",
    )];
    let error = seed_translate(&files).expect_err("`*` is outside seed v0");
    assert!(error.contains("__calque_seed_error:"), "{error}");
    assert!(error.contains("bad.calq:2:22"), "{error}");
    assert!(error.contains("mul-overflow slice pending"), "{error}");
}
