//! The seed's end-to-end tests (M37, D-086): b0 Calque source →
//! seed (a femto_lua program) → kernel-IR artifact → parse → verify →
//! interp, with expected outputs hand-derived from SPEC semantics
//! (L2's no-blessing rule; there is no upstream oracle, MANIFEST
//! "Conformance").

use calque::{run_artifact, run_artifact_with_input, seed_translate, seed_translate_jit, UnitFile};

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

// Translation cache (D-089's discipline applied to the dev loop): the
// seed is a PURE function of (its own source, the unit texts), and
// translating the 1.5k-line parser under the reference interpreter
// costs minutes - so each set's artifact is cached under
// target/calque-artifacts/, keyed by a hash OF ALL INPUTS. A cache hit
// is re-validated by the key (any input or seed change = new key), and
// the cache only ever holds artifacts the REAL pipeline produced.
// Cold misses translate through the JIT path (native, seconds); the
// ignored verify_seed_translation_runners test asserts the JIT and
// reference-interpreter artifacts are byte-identical (L3).
fn translate_cached(tag: &str, files: &[UnitFile]) -> String {
    use std::hash::{Hash, Hasher};
    let seed_src = include_str!("../../../tools/calque/bootstrap/seed.lua");
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    seed_src.hash(&mut hasher);
    for f in files {
        f.name.hash(&mut hasher);
        f.ns.hash(&mut hasher);
        f.text.hash(&mut hasher);
    }
    let key = format!("calque-artifacts/{tag}-{:016x}.mlir", hasher.finish());
    let path = std::path::Path::new("../../target").join(&key);
    if let Ok(cached) = std::fs::read_to_string(&path) {
        return cached;
    }
    // The seed runs on frk_interp, whose frames ride the HOST stack
    // (the repo's documented ceiling: libtest's 2 MiB default is too
    // skinny for the 1.5k-line parser) - translate on the standard
    // STACK_SIZE thread, like the harness runner does.
    let files: Vec<UnitFile> = files.to_vec();
    let artifact = std::thread::Builder::new()
        .stack_size(frk_interp::STACK_SIZE)
        .spawn(move || {
            calque::seed_translate_jit(&files).expect("the seed translates the set (jit)")
        })
        .expect("spawn")
        .join()
        .expect("join");
    let _ = std::fs::create_dir_all(path.parent().unwrap());
    let _ = std::fs::write(&path, &artifact);
    artifact
}

fn compile_artifact() -> &'static str {
    static A: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    A.get_or_init(|| {
        let files = vec![
            stdlib_str(),
            unit(
                "tools/calque/compiler/lex.calq",
                None,
                include_str!("../../../tools/calque/compiler/lex.calq"),
            ),
            unit(
                "tools/calque/compiler/report.calq",
                None,
                include_str!("../../../tools/calque/compiler/report.calq"),
            ),
            unit(
                "tools/calque/compiler/parse.calq",
                None,
                include_str!("../../../tools/calque/compiler/parse.calq"),
            ),
            unit(
                "tools/calque/compiler/emit.calq",
                None,
                include_str!("../../../tools/calque/compiler/emit.calq"),
            ),
            unit(
                "tools/calque/compiler/compile.calq",
                None,
                include_str!("../../../tools/calque/compiler/compile.calq"),
            ),
        ];
        translate_cached("compile", &files)
    })
}

// Visible cache warming: translates every calq0 set ONCE (content-
// keyed; a run after this is seconds). Run with:
//   cargo test -p calque --test seed -- --ignored --nocapture warm
// Prints progress per set; safe to re-run (fresh sets only).
/// The L3 differential for the seed pipeline itself (ignored: the
/// reference interpreter takes minutes per set): translate every calq0
/// set BOTH ways and assert byte-equality. Run at milestones:
///   cargo test -p calque --test seed -- --ignored --nocapture verify
#[test]
#[ignore]
fn verify_seed_translation_runners() {
    let sets: Vec<(&str, Vec<UnitFile>)> = vec![
        ("lexdump", vec![
            stdlib_str(),
            unit("tools/calque/compiler/lex.calq", None, include_str!("../../../tools/calque/compiler/lex.calq")),
            unit("tools/calque/compiler/report.calq", None, include_str!("../../../tools/calque/compiler/report.calq")),
            unit("tools/calque/compiler/lexdump.calq", None, include_str!("../../../tools/calque/compiler/lexdump.calq")),
        ]),
        ("compile", vec![
            stdlib_str(),
            unit("tools/calque/compiler/lex.calq", None, include_str!("../../../tools/calque/compiler/lex.calq")),
            unit("tools/calque/compiler/report.calq", None, include_str!("../../../tools/calque/compiler/report.calq")),
            unit("tools/calque/compiler/parse.calq", None, include_str!("../../../tools/calque/compiler/parse.calq")),
            unit("tools/calque/compiler/emit.calq", None, include_str!("../../../tools/calque/compiler/emit.calq")),
            unit("tools/calque/compiler/compile.calq", None, include_str!("../../../tools/calque/compiler/compile.calq")),
        ]),
    ];
    for (tag, files) in sets {
        println!("[verify] {tag}: jit leg...");
        let f2 = files.clone();
        let jit = std::thread::Builder::new()
            .stack_size(frk_interp::STACK_SIZE)
            .spawn(move || seed_translate_jit(&f2).expect("jit translates"))
            .expect("spawn")
            .join()
            .expect("join");
        println!("[verify] {tag}: interp leg...");
        let interp = std::thread::Builder::new()
            .stack_size(frk_interp::STACK_SIZE)
            .spawn(move || seed_translate(&files).expect("interp translates"))
            .expect("spawn")
            .join()
            .expect("join");
        assert_eq!(interp, jit, "[{tag}] interp and jit artifacts differ (L3)");
        println!("[{tag}] interp == jit: {} bytes", jit.len());
    }
}

// Scaling probe (ignored): translate synthetic programs of 1x/2x/4x size
// under the reference interpreter and print the times - linear vs
// superlinear decides whether the seed's table-length idioms or the
// interpreter need the next optimization pass.
#[test]
#[ignore]
fn bench_translation_scaling() {
    use std::time::Instant;
    for reps in [1usize, 2, 4] {
        let mut prog = String::from("pub const P = struct { a: i64, b: i64 };\n\npub fn main() i64 {\n    const p = %P{a: 1, b: 2};\n    var acc: i64 = 0;\n");
        for i in 0..reps * 8 {
            prog.push_str(&format!("    acc = acc + {i}; // filler line\n"));
        }
        prog.push_str("    acc\n}\n");
        let t0 = Instant::now();
        let files = vec![stdlib_str(), unit("bench.calq", None, &prog)];
        let _ = seed_translate(&files).expect("translates");
        println!("[bench] {} statements ({} bytes) in {:?}", reps * 8 + 5, prog.len(), t0.elapsed());
    }
}

// Four INDEPENDENT ignored tests: libtest runs them in parallel, so
// the cold cost is ~max(set), not the sum.
#[test]
#[ignore]
fn warm_lexdump() {
    use std::time::Instant;
    let t0 = Instant::now();
    println!("[warm] lexdump set...");
    let _ = lexer_dump_artifact();
    println!("[warm] lexdump cached in {:?}", t0.elapsed());
}

#[test]
#[ignore]
fn warm_exprdump() {
    use std::time::Instant;
    let t0 = Instant::now();
    println!("[warm] exprdump set...");
    let _ = expr_dump_artifact();
    println!("[warm] exprdump cached in {:?}", t0.elapsed());
}

#[test]
#[ignore]
fn warm_parsedump() {
    use std::time::Instant;
    let t0 = Instant::now();
    println!("[warm] parsedump set...");
    let _ = prog_dump_artifact();
    println!("[warm] parsedump cached in {:?}", t0.elapsed());
}

#[test]
#[ignore]
fn warm_compile() {
    use std::time::Instant;
    let t0 = Instant::now();
    println!("[warm] compile set...");
    let _ = compile_artifact();
    println!("[warm] compile cached in {:?}", t0.elapsed());
}

// The calq0 driver sets are expensive to translate (the 1.5k-line
// parser under the reference interpreter), and libtest runs tests in
// parallel - so each DRIVER SET is translated ONCE and shared
// (std::sync::OnceLock): tests only run the artifact.
// On a differential mismatch, report the FIRST differing byte with
// context (ASCII everywhere, so byte indexing is safe).
fn diff_hint(a: &str, b: &str) -> String {
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    let n = ab.len().min(bb.len());
    let mut i = 0;
    while i < n && ab[i] == bb[i] {
        i += 1;
    }
    let lo = i.saturating_sub(40);
    let hi_a = (i + 80).min(a.len());
    let hi_b = (i + 80).min(b.len());
    format!(
        "first difference at byte {i} (got len {}, expected len {})\n  got: {:?}\n  exp: {:?}",
        a.len(),
        b.len(),
        &a[lo..hi_a],
        &b[lo..hi_b]
    )
}

fn lexer_dump_artifact() -> &'static str {
    static A: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    A.get_or_init(|| {
        let files = vec![
            stdlib_str(),
            unit(
                "tools/calque/compiler/lex.calq",
                None,
                include_str!("../../../tools/calque/compiler/lex.calq"),
            ),
            unit(
                "tools/calque/compiler/report.calq",
                None,
                include_str!("../../../tools/calque/compiler/report.calq"),
            ),
            unit(
                "tools/calque/compiler/lexdump.calq",
                None,
                include_str!("../../../tools/calque/compiler/lexdump.calq"),
            ),
        ];
        translate_cached("lexdump", &files)
    })
}

fn expr_dump_artifact() -> &'static str {
    static A: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    A.get_or_init(|| {
        let files = vec![
            stdlib_str(),
            unit(
                "tools/calque/compiler/lex.calq",
                None,
                include_str!("../../../tools/calque/compiler/lex.calq"),
            ),
            unit(
                "tools/calque/compiler/report.calq",
                None,
                include_str!("../../../tools/calque/compiler/report.calq"),
            ),
            unit(
                "tools/calque/compiler/parse.calq",
                None,
                include_str!("../../../tools/calque/compiler/parse.calq"),
            ),
            unit(
                "tools/calque/compiler/exprdump.calq",
                None,
                include_str!("../../../tools/calque/compiler/exprdump.calq"),
            ),
        ];
        translate_cached("exprdump", &files)
    })
}

fn prog_dump_artifact() -> &'static str {
    static A: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    A.get_or_init(|| {
        let files = vec![
            stdlib_str(),
            unit(
                "tools/calque/compiler/lex.calq",
                None,
                include_str!("../../../tools/calque/compiler/lex.calq"),
            ),
            unit(
                "tools/calque/compiler/report.calq",
                None,
                include_str!("../../../tools/calque/compiler/report.calq"),
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
        translate_cached("parsedump", &files)
    })
}

#[test]
fn calq0_lexes_b0_source() {
    // calq0 slice 1: the lexer translates and runs; the dump is
    // hand-derived from the seed's lexical rules (positions counted
    // by hand, comments consume no columns on the line).
    let artifact = lexer_dump_artifact();
    let source = "const x: i64 = 42 + 7; // note\nfn main() i64 { return x; }";
    let outcome = run_artifact_with_input(artifact, &[unit("probe.calq", None, source)])
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
    let artifact = lexer_dump_artifact();
    let source = "/* c1 */ const s: Str = \"a\\tb\";";
    let outcome = run_artifact_with_input(artifact, &[unit("probe.calq", None, source)])
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
    let artifact = lexer_dump_artifact();
    let source = "pub fn main() i64 { 0 } @";
    let outcome = run_artifact_with_input(artifact, &[unit("probe.calq", None, source)])
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
    let artifact = expr_dump_artifact();
    let source = "fib(10) + str.fromI64(3) * 2";
    let outcome = run_artifact_with_input(artifact, &[unit("probe.calq", None, source)])
        .expect("the artifact runs");
    assert_eq!(
        outcome.output,
        "(bin + (call (ref fib) (int 10)) (bin * (call (ref str.fromI64) (int 3)) (int 2)))\n"
    );
    assert_eq!(outcome.values[0].as_signed().expect("i64 result"), 0);
}

#[test]
fn calq0_reports_parse_errors() {
    let artifact = expr_dump_artifact();

    // truncated expression: the EOF token carries the position
    let outcome = run_artifact_with_input(artifact, &[unit("probe.calq", None, "1 + ")])
        .expect("the artifact runs");
    assert_eq!(outcome.output, "error 1:5 unexpected end of input\n");
    assert_eq!(outcome.values[0].as_signed().expect("i64 result"), 1);

    // slice 2b landed: if-EXPRESSIONS now parse (this probe was the 2a
    // fence; it now proves the fence lifted)
    let outcome =
        run_artifact_with_input(artifact, &[unit("probe.calq", None, "if x { y }")])
            .expect("the artifact runs");
    assert_eq!(outcome.output, "(if (ref x) (block (ref y)))\n");
    assert_eq!(outcome.values[0].as_signed().expect("i64 result"), 0);

    // trailing tokens after a complete expression still report
    let outcome =
        run_artifact_with_input(artifact, &[unit("probe.calq", None, "1 + 2 3")])
            .expect("the artifact runs");
    assert_eq!(outcome.output, "error 1:7 trailing tokens after the expression\n");
    assert_eq!(outcome.values[0].as_signed().expect("i64 result"), 1);

    // comparisons do not chain
    let outcome = run_artifact_with_input(artifact, &[unit("probe.calq", None, "1 < 2 < 3")])
        .expect("the artifact runs");
    assert_eq!(outcome.output, "error 1:7 comparisons do not chain (SPEC 5.1)\n");
    assert_eq!(outcome.values[0].as_signed().expect("i64 result"), 1);
}

#[test]
fn seed_compiles_destructuring_shorthand() {
    // D-107: `const {a, b} = e` / `var {a, b: q} = e` — plain and
    // rename entries, mixable. Hand-derived: 3+4=7; 3*4=12; 3+10=13.
    let files = vec![
        stdlib_str(),
        unit(
            "destr.calq",
            None,
            "pub const P2 = struct {\n    a: i64,\n    b: i64,\n};\n\npub fn sum(p: P2) i64 {\n    const {a, b} = p;\n    return a + b;\n}\n\npub fn main() i64 {\n    const p = %P2{a: 3, b: 4};\n    io.print(str.fromI64(sum(p)));\n    io.print(\"\\n\");\n    const {a: x, b: y} = p;\n    io.print(str.fromI64(x * y));\n    io.print(\"\\n\");\n    var {a, b: q} = p;\n    q = 10;\n    io.print(str.fromI64(a + q));\n    io.print(\"\\n\");\n    0\n}\n",
        ),
    ];
    let artifact = seed_translate(&files).expect("the seed translates the destructuring probe");
    let outcome = run_artifact(&artifact).expect("the artifact runs");
    assert_eq!(outcome.output, "7\n12\n13\n");
}

#[test]
fn calq0_parses_programs() {
    // Slice 2b: full b0 programs. One translation, three probes; every
    // dump is hand-derived from the d_* rules (strings carry length +
    // bytes; absent optionals vanish).
    let artifact = prog_dump_artifact();

    let p1 = "pub const P2 = struct { a: i64, b: i64 = 1 };\npub fn f(x: i64) i64 { const y = x; y }\n";
    let o1 = run_artifact_with_input(artifact, &[unit("p1.calq", None, p1)]).expect("runs");
    assert_eq!(
        o1.output,
        "(ast (struct P2 (a (ty i64)) (b (ty i64) = (int 1))) (fn f ((x (ty i64))) (ty i64) (block (const y (ref x)) (ref y))))\n"
    );

    let p2 = "pub const Pair = struct { a: i64, b: i64 };\npub const Op = union(enum) { Add: Pair, Neg };\n\npub fn g(o: Op) i64 {\n    const r = match o {\n        Op.Add(q) => q.a,\n        Op.Neg => 0,\n    };\n    r\n}\n";
    let o2 = run_artifact_with_input(artifact, &[unit("p2.calq", None, p2)]).expect("runs");
    assert_eq!(
        o2.output,
        "(ast (struct Pair (a (ty i64)) (b (ty i64))) (union Op (Add (ty Pair)) (Neg)) (fn g ((o (ty Op))) (ty i64) (block (const r (match (ref o) (arm (pat variant Op.Add (pat bind q)) (ref q.a)) (arm (pat variant Op.Neg) (int 0)))) (ref r))))\n"
    );

    let p3 = "pub const P2 = struct { a: i64, b: i64 };\n\npub fn h(p: P2, n: i64) P2 {\n    const {a, b: q} = p;\n    var i: i64 = 0;\n    while i < n { i = i + 1; }\n    if n < a { p } else { %P2{a: n, b: q} }\n}\n";
    let o3 = run_artifact_with_input(artifact, &[unit("p3.calq", None, p3)]).expect("runs");
    assert_eq!(
        o3.output,
        "(ast (struct P2 (a (ty i64)) (b (ty i64))) (fn h ((p (ty P2)) (n (ty i64))) (ty P2) (block (destr const (a b: q) (ref p)) (var i (ty i64) (int 0)) (while (bin < (ref i) (ref n)) (block (assign i (bin + (ref i) (int 1))))) (if (bin < (ref n) (ref a)) (block (ref p)) (block (make P2 (a = (ref n)) (b = (ref q))))))))\n"
    );
}

#[test]
fn calq0_reports_program_errors() {
    let artifact = prog_dump_artifact();

    // missing `;` before `}` (the `}` token carries the position)
    let e1 = run_artifact_with_input(
        &artifact,
        &[unit("e1.calq", None, "pub fn f() i64 { const x: i64 = 1 }")],
    )
    .expect("runs");
    assert_eq!(e1.output, "error 1:35 expected `;`\n");
    assert_eq!(e1.values[0].as_signed().expect("i64"), 1);

    // top-level junk names the offender
    let e2 = run_artifact_with_input(artifact, &[unit("e2.calq", None, "x")]).expect("runs");
    assert_eq!(
        e2.output,
        "error 1:1 top-level items are fn declarations and struct/union type declarations (got `x`)\n"
    );
    assert_eq!(e2.values[0].as_signed().expect("i64"), 1);
}

#[test]
fn calq0_emits_the_seed_shape() {
    // Slice 3a: calq0 compiles a trivial module and its artifact text
    // is BYTE-IDENTICAL to the seed's emission for the same program
    // (the differential, from day one) - and the emitted artifact then
    // actually RUNS.
    let artifact = compile_artifact();

    let seed_zero = "// calque-artifact v0\nfunc.func @__calque_main(%env: !frk_closure.envref) -> i64 {\n  %t1 = arith.constant 0 : i64\n  func.return %t1 : i64\n}\nfunc.func @main() -> i64 attributes {llvm.emit_c_interface} {\n  %env = \"frk_adt.product_new\"() : () -> !frk_adt.product<[]>\n  %cl = \"frk_closure.make\"(%env) {callee = @__calque_main} : (!frk_adt.product<[]>) -> !frk_closure.fn<[], [i64]>\n  %args = \"frk_adt.product_new\"() : () -> !frk_adt.product<[]>\n  %r = \"frk_closure.apply\"(%cl, %args) : (!frk_closure.fn<[], [i64]>, !frk_adt.product<[]>) -> i64\n  return %r : i64\n}\n";
    let o = run_artifact_with_input(artifact, &[unit("p.calq", None, "pub fn main() i64 { 0 }")])
        .expect("the compiler runs");
    assert_eq!(o.output, seed_zero, "calq0's emission must equal the seed's");
    // And the emitted artifact executes under the reference interpreter.
    let emitted = run_artifact(&o.output).expect("the emitted artifact runs");
    assert_eq!(emitted.values[0].as_signed().expect("i64"), 0);

    let o2 = run_artifact_with_input(artifact, &[unit("p.calq", None, "pub fn main() i64 { 42 }")])
        .expect("the compiler runs");
    assert!(
        o2.output.contains("%t1 = arith.constant 42 : i64"),
        "42 must flow into the constant: {}",
        o2.output
    );
    let emitted2 = run_artifact(&o2.output).expect("the emitted artifact runs");
    assert_eq!(emitted2.values[0].as_signed().expect("i64"), 42);
}

#[test]
fn calq0_matches_the_seed_on_statements_and_arithmetic() {
    // Slice 3b-alpha: the differential computed LIVE - the seed's
    // emission for the same unit is the expected artifact, and calq0's
    // compiled compiler must produce it byte-for-byte; the emitted
    // artifact then RUNS.
    let p1 = "pub fn main() i64 {\n    var acc: i64 = 0;\n    var i: i64 = 0;\n    while i < 5 {\n        acc = acc + i * 2;\n        i = i + 1;\n    }\n    acc\n}\n";
    let expected = seed_translate(&[unit("probe.calq", None, p1)]).expect("the seed emits P1");
    let got = run_artifact_with_input(compile_artifact(), &[unit("probe.calq", None, p1)])
        .expect("calq0 compiles P1");
    assert_eq!(
        got.output,
        expected,
        "calq0's emission must equal the seed's (P1): {}",
        diff_hint(&got.output, &expected)
    );
    let emitted = run_artifact(&got.output).expect("the emitted artifact runs");
    assert_eq!(emitted.values[0].as_signed().expect("i64"), 20);

    // fib: recursion, comparisons, subtraction, the tail-if special
    // case, and the namespaced stdlib call (str.fromI64) - both sides
    // take the stdlib unit, exactly like the seed corpus does.
    let fib = include_str!("../../../specimens/calque/examples/fib.calq");
    let fib_units = |src: &str| {
        vec![
            stdlib_str(),
            unit("specimens/calque/examples/fib.calq", None, src),
        ]
    };
    let expected_fib = seed_translate(&fib_units(fib)).expect("the seed emits fib");
    let got_fib = run_artifact_with_input(compile_artifact(), &fib_units(fib))
        .expect("calq0 compiles fib");
    assert_eq!(
        got_fib.output,
        expected_fib,
        "calq0's emission must equal the seed's (fib): {}",
        diff_hint(&got_fib.output, &expected_fib)
    );
    let emitted_fib = run_artifact(&got_fib.output).expect("the emitted fib runs");
    assert_eq!(emitted_fib.values[0].as_signed().expect("i64"), 0);
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
