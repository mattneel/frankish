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
fn seed_compiles_the_vector_surface() {
    // Vector(i64): growth across 4 -> 8 -> 16 with 10 pushes, get/set
    // bounds, len. Hand-derived: get(3) = 6; after set(0, 42) get(0)
    // = 42; len = 10.
    let files = vec![
        stdlib_str(),
        unit(
            "vec.calq",
            None,
            "pub fn main() i64 {\n    var v: Vector(i64) = vector.new();\n    var i: i64 = 0;\n    while i < 10 {\n        vector.push(v, i + i);\n        i = i + 1;\n    }\n    io.print(str.fromI64(vector.get(v, 3)));\n    io.print(\"\\n\");\n    vector.set(v, 0, 42);\n    io.print(str.fromI64(vector.get(v, 0)));\n    io.print(\"\\n\");\n    io.print(str.fromI64(vector.len(v)));\n    io.print(\"\\n\");\n    0\n}\n",
        ),
    ];
    let artifact = seed_translate(&files).expect("the seed translates the vector probe");
    let outcome = run_artifact(&artifact).expect("the artifact runs");
    assert_eq!(outcome.output, "6\n42\n10\n");
}

#[test]
fn seed_compiles_str_and_named_vector_slots() {
    // bstr slots (growth past the initial 4) and erased-reference slots
    // (declared struct values). Hand-derived: get(4) = "epsilon";
    // len = 5; 5 + 7 = 12.
    let files = vec![
        stdlib_str(),
        unit(
            "vec2.calq",
            None,
            "pub const Pair = struct {\n    left: i64,\n    right: i64,\n};\n\npub fn main() i64 {\n    var names: Vector(Str) = vector.new();\n    vector.push(names, \"alpha\");\n    vector.push(names, \"beta\");\n    vector.push(names, \"gamma\");\n    vector.push(names, \"delta\");\n    vector.push(names, \"epsilon\");\n    io.print(vector.get(names, 4));\n    io.print(\"\\n\");\n    io.print(str.fromI64(vector.len(names)));\n    io.print(\"\\n\");\n    var pairs: Vector(Pair) = vector.new();\n    vector.push(pairs, %Pair{left: 2, right: 3});\n    vector.push(pairs, %Pair{left: 5, right: 7});\n    const p: Pair = vector.get(pairs, 1);\n    io.print(str.fromI64(p.left + p.right));\n    io.print(\"\\n\");\n    0\n}\n",
        ),
    ];
    let artifact = seed_translate(&files).expect("the seed translates the slot probe");
    let outcome = run_artifact(&artifact).expect("the artifact runs");
    assert_eq!(outcome.output, "epsilon\n5\n12\n");
}

#[test]
fn seed_vector_out_of_bounds_traps() {
    let files = vec![
        stdlib_str(),
        unit(
            "vec_oob.calq",
        None,
        "pub fn main() i64 {\n    var v: Vector(i64) = vector.new();\n    vector.push(v, 1);\n    io.print(str.fromI64(vector.get(v, 5)));\n    0\n}\n",
    )];
    let artifact = seed_translate(&files).expect("the seed translates the oob probe");
    let error = run_artifact(&artifact).expect_err("the oob get must trap");
    assert!(
        error.contains("vector.get: index out of bounds"),
        "trap message must name the operation: {error}"
    );
}

#[test]
fn seed_compiles_checked_multiplication() {
    // Hand-derived: 6*7=42; (0-5)*3=-15; 0*max=0; max*1=max.
    let files = vec![
        stdlib_str(),
        unit(
            "mul.calq",
            None,
            "pub fn main() i64 {\n    const a: i64 = 6;\n    const b: i64 = 7;\n    io.print(str.fromI64(a * b));\n    io.print(\"\\n\");\n    const neg: i64 = 0 - 5;\n    io.print(str.fromI64(neg * 3));\n    io.print(\"\\n\");\n    io.print(str.fromI64(0 * 9223372036854775807));\n    io.print(\"\\n\");\n    const m: i64 = 9223372036854775807;\n    io.print(str.fromI64(m * 1));\n    io.print(\"\\n\");\n    0\n}\n",
        ),
    ];
    let artifact = seed_translate(&files).expect("the seed translates the mul probe");
    let outcome = run_artifact(&artifact).expect("the artifact runs");
    assert_eq!(outcome.output, "42\n-15\n0\n9223372036854775807\n");
}

#[test]
fn seed_checked_multiplication_traps_on_overflow() {
    // max * 2 overflows (CQ0003); the message must carry the code.
    let files = vec![
        stdlib_str(),
        unit(
            "mul_ovf.calq",
            None,
            "pub fn main() i64 {\n    const m: i64 = 9223372036854775807;\n    io.print(str.fromI64(m * 2));\n    0\n}\n",
        ),
    ];
    let artifact = seed_translate(&files).expect("the seed translates the overflow probe");
    let error = run_artifact(&artifact).expect_err("max * 2 must trap");
    assert!(error.contains("CQ0003"), "trap must name CQ0003: {error}");

    // min * -1 overflows too, WITHOUT ever executing divsi(min, -1):
    // the select path routes it to the quotient 0 and the check fires.
    let files = vec![
        stdlib_str(),
        unit(
            "mul_min.calq",
            None,
            "pub fn main() i64 {\n    const mn: i64 = 0 - 9223372036854775807 - 1;\n    const m1: i64 = 0 - 1;\n    io.print(str.fromI64(mn * m1));\n    0\n}\n",
        ),
    ];
    let artifact = seed_translate(&files).expect("the seed translates the min probe");
    let error = run_artifact(&artifact).expect_err("min * -1 must trap");
    assert!(error.contains("CQ0003"), "trap must name CQ0003: {error}");
}

#[test]
fn calq0_lexes_b0_source() {
    // calq0 slice 1: the lexer translates and runs; the dump is
    // hand-derived from the seed's lexical rules (positions counted
    // by hand, comments consume no columns on the line).
    let files = vec![
        stdlib_str(),
        unit(
            "tools/calque/compiler/lex.calq",
            None,
            include_str!("../../../tools/calque/compiler/lex.calq"),
        ),
        unit(
            "tools/calque/compiler/lexdump.calq",
            None,
            include_str!("../../../tools/calque/compiler/lexdump.calq"),
        ),
    ];
    let artifact = seed_translate(&files).expect("the seed translates calq0's lexer");
    let source = "const x: i64 = 42 + 7; // note\nfn main() i64 { return x; }";
    let outcome = run_artifact_with_input(&artifact, &[unit("probe.calq", None, source)])
        .expect("the artifact runs");
    let expected = "\
kw const 1:1\n\
name x 1:7\n\
punct : 1:8\n\
kw i64 1:10\n\
punct = 1:14\n\
int 42 1:16\n\
punct + 1:19\n\
int 7 1:21\n\
punct ; 1:22\n\
kw fn 2:1\n\
name main 2:4\n\
punct ( 2:8\n\
punct ) 2:9\n\
kw i64 2:11\n\
punct { 2:15\n\
kw return 2:17\n\
name x 2:24\n\
punct ; 2:25\n\
punct } 2:27\n\
eof  2:28\n\
count 20\n";
    assert_eq!(outcome.output, expected);
    assert_eq!(outcome.values[0].as_signed().expect("i64 result"), 0);
}

#[test]
fn calq0_lexes_strings_and_comments() {
    let files = vec![
        stdlib_str(),
        unit(
            "tools/calque/compiler/lex.calq",
            None,
            include_str!("../../../tools/calque/compiler/lex.calq"),
        ),
        unit(
            "tools/calque/compiler/lexdump.calq",
            None,
            include_str!("../../../tools/calque/compiler/lexdump.calq"),
        ),
    ];
    let artifact = seed_translate(&files).expect("the seed translates calq0's lexer");
    let source = "/* c1 */ const s: Str = \"a\\tb\";";
    let outcome = run_artifact_with_input(&artifact, &[unit("probe.calq", None, source)])
        .expect("the artifact runs");
    let expected = "\
kw const 1:10\n\
name s 1:16\n\
punct : 1:17\n\
name Str 1:19\n\
punct = 1:23\n\
str a\tb 1:25\n\
punct ; 1:31\n\
eof  1:32\n\
count 8\n";
    assert_eq!(outcome.output, expected);
}

#[test]
fn calq0_reports_lexical_errors() {
    let files = vec![
        stdlib_str(),
        unit(
            "tools/calque/compiler/lex.calq",
            None,
            include_str!("../../../tools/calque/compiler/lex.calq"),
        ),
        unit(
            "tools/calque/compiler/lexdump.calq",
            None,
            include_str!("../../../tools/calque/compiler/lexdump.calq"),
        ),
    ];
    let artifact = seed_translate(&files).expect("the seed translates calq0's lexer");
    let source = "pub fn main() i64 { 0 } @";
    let outcome = run_artifact_with_input(&artifact, &[unit("probe.calq", None, source)])
        .expect("the artifact runs");
    assert!(
        outcome.output.starts_with("error 1:25 unexpected character @"),
        "{}",
        outcome.output
    );
    assert_eq!(outcome.values[0].as_signed().expect("i64 result"), 1);
}

#[test]
fn calq0_parses_expressions() {
    // Slice 2a: expressions. Precedence: 10 + (3 * 2); the qualified
    // callee keeps its module in the Ref. Hand-derived dump.
    let files = vec![
        stdlib_str(),
        unit(
            "tools/calque/compiler/lex.calq",
            None,
            include_str!("../../../tools/calque/compiler/lex.calq"),
        ),
        unit(
            "tools/calque/compiler/parse.calq",
            None,
            include_str!("../../../tools/calque/compiler/parse.calq"),
        ),
        unit(
            "tools/calque/compiler/parsedump.calq",
            None,
            include_str!("../../../tools/calque/compiler/parsedump.calq"),
        ),
    ];
    let artifact = seed_translate(&files).expect("the seed translates calq0's parser");
    let source = "fib(10) + str.fromI64(3) * 2";
    let outcome = run_artifact_with_input(&artifact, &[unit("probe.calq", None, source)])
        .expect("the artifact runs");
    assert_eq!(
        outcome.output,
        "(bin + (call (ref fib) (int 10)) (bin * (call (ref str.fromI64) (int 3)) (int 2)))\n"
    );
    assert_eq!(outcome.values[0].as_signed().expect("i64 result"), 0);
}

#[test]
fn calq0_reports_parse_errors() {
    let files = vec![
        stdlib_str(),
        unit(
            "tools/calque/compiler/lex.calq",
            None,
            include_str!("../../../tools/calque/compiler/lex.calq"),
        ),
        unit(
            "tools/calque/compiler/parse.calq",
            None,
            include_str!("../../../tools/calque/compiler/parse.calq"),
        ),
        unit(
            "tools/calque/compiler/parsedump.calq",
            None,
            include_str!("../../../tools/calque/compiler/parsedump.calq"),
        ),
    ];
    let artifact = seed_translate(&files).expect("the seed translates calq0's parser");

    // truncated expression: the EOF token carries the position
    let outcome = run_artifact_with_input(&artifact, &[unit("probe.calq", None, "1 + ")])
        .expect("the artifact runs");
    assert_eq!(outcome.output, "error 1:5 unexpected end of input\n");
    assert_eq!(outcome.values[0].as_signed().expect("i64 result"), 1);

    // the 2a fences are loud
    let outcome =
        run_artifact_with_input(&artifact, &[unit("probe.calq", None, "if x { y }")])
            .expect("the artifact runs");
    assert_eq!(
        outcome.output,
        "error 1:1 if-expressions land in slice 2b (statements first)\n"
    );
    assert_eq!(outcome.values[0].as_signed().expect("i64 result"), 1);

    // comparisons do not chain
    let outcome = run_artifact_with_input(&artifact, &[unit("probe.calq", None, "1 < 2 < 3")])
        .expect("the artifact runs");
    assert_eq!(outcome.output, "error 1:7 comparisons do not chain (SPEC 5.1)\n");
    assert_eq!(outcome.values[0].as_signed().expect("i64 result"), 1);
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
    // (file:line:col), never mis-translate. (`*` used to be the probe;
    // the mul slice landed, so `!` — still fenced — carries it now.)
    let files = vec![unit(
        "bad.calq",
        None,
        "pub fn main() i64 {\n    const x: bool = !true;\n    0\n}",
    )];
    let error = seed_translate(&files).expect_err("`!` is outside seed v0");
    assert!(error.contains("__calque_seed_error:"), "{error}");
    assert!(error.contains("bad.calq:2:"), "{error}");
    assert!(error.contains("`!` is not in seed v0"), "{error}");
}
