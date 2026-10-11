//! The seed's end-to-end tests (M37, D-086): b0 Calque source →
//! seed (a femto_lua program) → kernel-IR artifact → parse → verify →
//! interp, with expected outputs hand-derived from SPEC semantics
//! (L2's no-blessing rule; there is no upstream oracle, MANIFEST
//! "Conformance").

use calque::{run_artifact, run_artifact_with_input, seed_translate, UnitFile};

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
fn seed_compiles_records_unions_and_match() {
    let files = vec![
        stdlib_str(),
        unit(
            "crates/calque/tests/assets/adt_probe.calq",
            None,
            include_str!("assets/adt_probe.calq"),
        ),
    ];
    let artifact = seed_translate(&files).expect("the seed translates the ADT probe");
    let outcome = run_artifact(&artifact).expect("the artifact runs");
    assert_eq!(outcome.output, "p 3 p 10 circle 7 rect 1,2 nothing hit\n");
}

fn adt_negative(body: &str) -> String {
    let program = format!(
        "pub const Point = struct {{\n    x: i64,\n    y: i64,\n    label: Str = \"p\",\n}};\n\
         pub const Shape = union(enum) {{\n    Circle: i64,\n    Rect: Point,\n    Nothing,\n}};\n\
         pub fn main() i64 {{\n{body}\n    0\n}}\n"
    );
    let files = vec![stdlib_str(), unit("bad.calq", None, &program)];
    seed_translate(&files).expect_err("the seed must refuse this program")
}

#[test]
fn seed_refuses_bad_record_construction() {
    let msg = adt_negative("    const p = %Point{x: 1};");
    assert!(msg.contains("missing required field `y`"), "{msg}");
    let msg = adt_negative("    const p = %Point{x: 1, y: 2, z: 3};");
    assert!(msg.contains("unknown field `z`"), "{msg}");
    let msg = adt_negative("    const p = %Point{x: 1, y: 2, x: 3};");
    assert!(msg.contains("duplicate field `x`"), "{msg}");
}

#[test]
fn seed_refuses_bad_matches() {
    let msg = adt_negative(
        "    const s = Shape.Circle(1);\n    const r = match s {\n        Shape.Circle(n) => \"c\",\n        Shape.Rect(p) => \"r\",\n    };",
    );
    assert!(msg.contains("does not cover Shape.Nothing"), "{msg}");
    let msg = adt_negative(
        "    const s = Shape.Circle(1);\n    const r = match s {\n        Shape.Circle(n) => \"c\",\n        Shape.Circle(m) => \"d\",\n        _ => \"x\",\n    };",
    );
    assert!(msg.contains("redundant arm"), "{msg}");
    let msg = adt_negative("    const s = Shape.Blob;");
    assert!(msg.contains("unknown variant `Blob`"), "{msg}");
}

#[test]
fn seed_refuses_refining_final_arms() {
    let msg = adt_negative(
        "    const s = Shape.Circle(1);\n    const r = match s {\n        Shape.Rect(p) => \"r\",\n        Shape.Nothing => \"n\",\n        Shape.Circle(7) => \"seven\",\n    };",
    );
    assert!(msg.contains("add a following `_` arm"), "{msg}");
}

#[test]
fn seed_compiles_the_steering_struct_example() {
    let files = vec![
        stdlib_str(),
        unit(
            "specimens/calque/examples/user_badge.calq",
            None,
            include_str!("../../../specimens/calque/examples/user_badge.calq"),
        ),
    ];
    let artifact = seed_translate(&files).expect("the seed translates the example");
    let outcome = run_artifact(&artifact).expect("the artifact runs");
    assert_eq!(outcome.output, "Hello, Matt\nPerfect, Matt\n");
}

#[test]
fn seed_compiles_the_input_channel() {
    // The compiler's "stdin" (BOOTSTRAP 2.1): count + 0-based index,
    // UNRESTRICTED bytes (the seed's own channel is ASCII-only; this
    // one rides kernel-IR literals).
    let files = vec![unit(
        "channel.calq",
        None,
        "pub fn main() i64 {\n    const count: i64 = __calque_input_count();\n    const n: i64 = count - 1;\n    io.print(__calque_input(n));\n    0\n}\n",
    )];
    let artifact = seed_translate(&files).expect("the seed translates the channel probe");
    let units = vec![
        unit("alpha.calq", None, "alpha line\n"),
        unit("beta.calq", None, "beta caf\u{e9}\n"),
    ];
    let outcome =
        run_artifact_with_input(&artifact, &units).expect("the artifact runs with input");
    assert_eq!(outcome.output, "beta caf\u{e9}\n");
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
