//! The seed's end-to-end tests (M37, D-086): b0 Calque source →
//! seed (a femto_lua program) → kernel-IR artifact → parse → verify →
//! interp, with expected outputs hand-derived from SPEC semantics
//! (L2's no-blessing rule; there is no upstream oracle, MANIFEST
//! "Conformance").

use calque::{run_artifact, run_artifact_with_input_cached, run_test_blocks, seed_translate, seed_translate_cached, seed_translate_jit, UnitFile};


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
    let artifact = seed_translate_cached(&files).expect("the seed translates fib.calq");
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
    let artifact = seed_translate_cached(&files).expect("the seed translates the probe");
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
    let artifact = seed_translate_cached(&files).expect("the seed translates the ADT probe");
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
    let artifact = seed_translate_cached(&files).expect("the seed translates the example");
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
    let artifact = seed_translate_cached(&files).expect("the seed translates the channel probe");
    let units = vec![
        unit("alpha.calq", None, "alpha line\n"),
        unit("beta.calq", None, "beta caf\u{e9}\n"),
    ];
    let outcome =
        run_artifact_with_input_cached(&artifact, &units).expect("the artifact runs with input");
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
    let artifact = seed_translate_cached(&files).expect("the seed translates the vector probe");
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
    let artifact = seed_translate_cached(&files).expect("the seed translates the slot probe");
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
    let artifact = seed_translate_cached(&files).expect("the seed translates the oob probe");
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
    let artifact = seed_translate_cached(&files).expect("the seed translates the mul probe");
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
    let artifact = seed_translate_cached(&files).expect("the seed translates the overflow probe");
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
    let artifact = seed_translate_cached(&files).expect("the seed translates the min probe");
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

fn compiler_files() -> Vec<UnitFile> {
    vec![
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
    ]
}

/// "calq0" for the tests: since D-124 the frozen seed cannot build
/// the current sources (they use post-freeze surface), so the
/// canonical object is the CHAIN's product (D-123/D-126): the seed
/// builds the oldest snapshot, each hop builds the next, and the
/// newest snapshot's compiler builds the current set.
fn compile_artifact() -> &'static str {
    static A: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    A.get_or_init(|| {
        calque::canonical_compiler()
            .expect("the chain builds the current compiler")
            .to_string()
    })
}

// calq0 compiling a set through the JIT (D-110's fast path: the same
// pipeline, seconds instead of minutes). L3: byte-equal to the
// interpreter - asserted by verify_calq0_runners_agree (ignored, run
// at gates) and by the corpus differentials' interpreter goldens.
fn compile_via_calq0_jit(calq0: &str, files: &[UnitFile]) -> String {
    let text = calque::compose_unit_with_input(calq0, files);
    let context = frk_core::context();
    frk_dialects::register(&context).expect("register");
    let (capture, _code) = frk_harness::runner::jit_run_text(
        &context,
        &text,
        "main",
        frk_dialects::Strategy::Arena,
    )
    .expect("calq0 compiles the set (jit)");
    capture
}

#[test]
#[ignore = "minutes: the interpreter leg of the L3 runner-equality check"]
fn verify_calq0_runners_agree() {
    let seed_built = compile_artifact();
    let files = compiler_files();
    let via_interp = {
        let calq0_owned = seed_built.to_string();
        let files_owned = files.clone();
        std::thread::Builder::new()
            .stack_size(frk_interp::STACK_SIZE)
            .spawn(move || {
                calque::run_artifact_with_input_cached(&calq0_owned, &files_owned)
                    .expect("calq0 (interp)")
                    .output
            })
            .expect("spawn")
            .join()
            .expect("join")
    };
    let via_jit = compile_via_calq0_jit(seed_built, &files);
    assert_eq!(
        via_jit,
        via_interp,
        "the JIT and interpreter legs must agree byte-exactly (L3): {}",
        diff_hint(&via_jit, &via_interp)
    );
}

// calq0 compiling a set - the compiler-as-a-program path, cached like
// translate_cached (the key covers the seed, calq0's own artifact, and
// every input unit, so no change can hide behind a stale entry).
fn compile_via_calq0_cached(tag: &str, calq0: &str, files: &[UnitFile]) -> String {
    use std::hash::{Hash, Hasher};
    let seed_src = include_str!("../../../tools/calque/bootstrap/seed.lua");
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    seed_src.hash(&mut hasher);
    calq0.hash(&mut hasher);
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
    let calq0_owned = calq0.to_string();
    let files: Vec<UnitFile> = files.to_vec();
    let artifact = std::thread::Builder::new()
        .stack_size(frk_interp::STACK_SIZE)
        .spawn(move || compile_via_calq0_jit(&calq0_owned, &files))
        .expect("spawn")
        .join()
        .expect("join");
    let _ = std::fs::create_dir_all(path.parent().unwrap());
    let _ = std::fs::write(&path, &artifact);
    artifact
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
        let _ = seed_translate_cached(&files).expect("translates");
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
    let outcome = run_artifact_with_input_cached(artifact, &[unit("probe.calq", None, source)])
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
    let outcome = run_artifact_with_input_cached(artifact, &[unit("probe.calq", None, source)])
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
    let outcome = run_artifact_with_input_cached(artifact, &[unit("probe.calq", None, source)])
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
    let outcome = run_artifact_with_input_cached(artifact, &[unit("probe.calq", None, source)])
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
    let outcome = run_artifact_with_input_cached(artifact, &[unit("probe.calq", None, "1 + ")])
        .expect("the artifact runs");
    assert_eq!(outcome.output, "error 1:5 unexpected end of input\n");
    assert_eq!(outcome.values[0].as_signed().expect("i64 result"), 1);

    // slice 2b landed: if-EXPRESSIONS now parse (this probe was the 2a
    // fence; it now proves the fence lifted)
    let outcome =
        run_artifact_with_input_cached(artifact, &[unit("probe.calq", None, "if x { y }")])
            .expect("the artifact runs");
    assert_eq!(outcome.output, "(if (ref x) (block (ref y)))\n");
    assert_eq!(outcome.values[0].as_signed().expect("i64 result"), 0);

    // trailing tokens after a complete expression still report
    let outcome =
        run_artifact_with_input_cached(artifact, &[unit("probe.calq", None, "1 + 2 3")])
            .expect("the artifact runs");
    assert_eq!(outcome.output, "error 1:7 trailing tokens after the expression\n");
    assert_eq!(outcome.values[0].as_signed().expect("i64 result"), 1);

    // comparisons do not chain
    let outcome = run_artifact_with_input_cached(artifact, &[unit("probe.calq", None, "1 < 2 < 3")])
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
    let artifact = seed_translate_cached(&files).expect("the seed translates the destructuring probe");
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
    let o1 = run_artifact_with_input_cached(artifact, &[unit("p1.calq", None, p1)]).expect("runs");
    assert_eq!(
        o1.output,
        "(ast (struct P2 (a (ty i64)) (b (ty i64) = (int 1))) (fn f ((x (ty i64))) (ty i64) (block (const y (ref x)) (ref y))))\n"
    );

    let p2 = "pub const Pair = struct { a: i64, b: i64 };\npub const Op = union(enum) { Add: Pair, Neg };\n\npub fn g(o: Op) i64 {\n    const r = match o {\n        Op.Add(q) => q.a,\n        Op.Neg => 0,\n    };\n    r\n}\n";
    let o2 = run_artifact_with_input_cached(artifact, &[unit("p2.calq", None, p2)]).expect("runs");
    assert_eq!(
        o2.output,
        "(ast (struct Pair (a (ty i64)) (b (ty i64))) (union Op (Add (ty Pair)) (Neg)) (fn g ((o (ty Op))) (ty i64) (block (const r (match (ref o) (arm (pat variant Op.Add (pat bind q)) (ref q.a)) (arm (pat variant Op.Neg) (int 0)))) (ref r))))\n"
    );

    let p3 = "pub const P2 = struct { a: i64, b: i64 };\n\npub fn h(p: P2, n: i64) P2 {\n    const {a, b: q} = p;\n    var i: i64 = 0;\n    while i < n { i = i + 1; }\n    if n < a { p } else { %P2{a: n, b: q} }\n}\n";
    let o3 = run_artifact_with_input_cached(artifact, &[unit("p3.calq", None, p3)]).expect("runs");
    assert_eq!(
        o3.output,
        "(ast (struct P2 (a (ty i64)) (b (ty i64))) (fn h ((p (ty P2)) (n (ty i64))) (ty P2) (block (destr const (a b: q) (ref p)) (var i (ty i64) (int 0)) (while (bin < (ref i) (ref n)) (block (assign i (bin + (ref i) (int 1))))) (if (bin < (ref n) (ref a)) (block (ref p)) (block (make P2 (a = (ref n)) (b = (ref q))))))))\n"
    );
}

#[test]
fn calq0_reports_program_errors() {
    let artifact = prog_dump_artifact();

    // missing `;` before `}` (the `}` token carries the position)
    let e1 = run_artifact_with_input_cached(
        &artifact,
        &[unit("e1.calq", None, "pub fn f() i64 { const x: i64 = 1 }")],
    )
    .expect("runs");
    assert_eq!(e1.output, "error 1:35 expected `;`\n");
    assert_eq!(e1.values[0].as_signed().expect("i64"), 1);

    // top-level junk is refused at its position, naming the offender
    let e2 = run_artifact_with_input_cached(artifact, &[unit("e2.calq", None, "x")]).expect("runs");
    assert!(e2.output.starts_with("error 1:1 "), "{}", e2.output);
    assert!(e2.output.contains("top-level items are"), "{}", e2.output);
    assert!(e2.output.contains("`x`"), "{}", e2.output);
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
    let o = run_artifact_with_input_cached(artifact, &[unit("p.calq", None, "pub fn main() i64 { 0 }")])
        .expect("the compiler runs");
    assert_eq!(o.output, seed_zero, "calq0's emission must equal the seed's");
    // And the emitted artifact executes under the reference interpreter.
    let emitted = run_artifact(&o.output).expect("the emitted artifact runs");
    assert_eq!(emitted.values[0].as_signed().expect("i64"), 0);

    let o2 = run_artifact_with_input_cached(artifact, &[unit("p.calq", None, "pub fn main() i64 { 42 }")])
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
    let expected = seed_translate_cached(&[unit("probe.calq", None, p1)]).expect("the seed emits P1");
    let got = run_artifact_with_input_cached(compile_artifact(), &[unit("probe.calq", None, p1)])
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
    let expected_fib = seed_translate_cached(&fib_units(fib)).expect("the seed emits fib");
    let got_fib = run_artifact_with_input_cached(compile_artifact(), &fib_units(fib))
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
fn calq0_matches_the_seed_on_records_and_vectors() {
    // Slice 3b-beta-1: records (defaults, immutable update), the
    // destructuring statement, field access, and the vector prelude -
    // all live-differential against the seed, then executed.
    let artifact = compile_artifact();

    let head = "pub const P2 = struct { a: i64, b: i64 = 7 };\n";
    let probes = [
        ("A make+default+field", "pub fn main() i64 {\n    const p = %P2{a: 3};\n    const c: i64 = p.b;\n    c\n}\n", 7),
        ("B update", "pub fn main() i64 {\n    const p = %P2{a: 3};\n    const q = %P2{p | b: 9};\n    const c: i64 = q.b;\n    c\n}\n", 9),
        ("C destr", "pub fn main() i64 {\n    const p = %P2{a: 3};\n    const {a, b} = p;\n    a + b\n}\n", 10),
    ];
    for (pname, body, want) in probes {
        let src = format!("{head}{body}");
        let expected_p = seed_translate_cached(&[unit("probe.calq", None, &src)]).expect("the seed emits the probe");
        let got_p = run_artifact_with_input_cached(artifact, &[unit("probe.calq", None, &src)])
            .expect("calq0 compiles the probe");
        assert_eq!(
            got_p.output,
            expected_p,
            "calq0's emission must equal the seed's ({pname}): {}",
            diff_hint(&got_p.output, &expected_p)
        );
        let emitted_p = run_artifact(&got_p.output).expect("the emitted probe runs");
        assert_eq!(emitted_p.values[0].as_signed().expect("i64"), want, "{pname}");
    }
    let r1 = "pub const P2 = struct { a: i64, b: i64 = 7 };\n\npub fn main() i64 {\n    const p = %P2{a: 3};\n    const q = %P2{p | b: 9};\n    const {a, b} = q;\n    a + b\n}\n";
    let expected = seed_translate_cached(&[unit("probe.calq", None, r1)]).expect("the seed emits R1");
    let got = run_artifact_with_input_cached(artifact, &[unit("probe.calq", None, r1)])
        .expect("calq0 compiles R1");
    assert_eq!(
        got.output,
        expected,
        "calq0's emission must equal the seed's (R1): {}",
        diff_hint(&got.output, &expected)
    );
    let emitted = run_artifact(&got.output).expect("the emitted R1 runs");
    assert_eq!(emitted.values[0].as_signed().expect("i64"), 12);

    let v1 = "pub fn main() i64 {\n    var v: Vector(i64) = vector.new();\n    vector.push(v, 5);\n    vector.push(v, 6);\n    vector.get(v, 1)\n}\n";
    let expected_v = seed_translate_cached(&[unit("probe.calq", None, v1)]).expect("the seed emits V1");
    let got_v = run_artifact_with_input_cached(artifact, &[unit("probe.calq", None, v1)])
        .expect("calq0 compiles V1");
    assert_eq!(
        got_v.output,
        expected_v,
        "calq0's emission must equal the seed's (V1): {}",
        diff_hint(&got_v.output, &expected_v)
    );
    let emitted_v = run_artifact(&got_v.output).expect("the emitted V1 runs");
    assert_eq!(emitted_v.values[0].as_signed().expect("i64"), 6);
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

#[test]
fn calq0_matches_the_seed_on_match() {
    // Slice 3b-beta-2: the match emission - tag chains with failing
    // payload sub-conditions (the split path), struct record patterns,
    // builtin literals and pins, Str subjects, and unit-valued joins.
    // The first probe is the shape that found a REFERENCE BUG in the
    // seed's emit_arm (dangling false target when a chain arm's
    // payload pattern can fail); the fixed seed is the oracle.
    let artifact = compile_artifact();

    let m1 = "pub const P2 = struct { a: i64, b: i64 };\npub const Op2 = union(enum) { Add: P2, Neg };\npub fn classify(o: Op2) i64 {\n    const r = match o {\n        Op2.Add(%P2{a: 1, b: 2}) => 10,\n        Op2.Neg => 30,\n        _ => 40,\n    };\n    r\n}\npub fn main() i64 {\n    const hit = Op2.Add(%P2{a: 1, b: 2});\n    const mis = Op2.Add(%P2{a: 9, b: 9});\n    const neg = Op2.Neg;\n    classify(hit) * 10000 + classify(mis) * 100 + classify(neg)\n}\n";
    let expected_m1 = seed_translate_cached(&[unit("m.calq", None, m1)]).expect("the seed emits M1");
    let got_m1 = run_artifact_with_input_cached(artifact, &[unit("m.calq", None, m1)])
        .expect("calq0 compiles M1");
    assert_eq!(
        got_m1.output,
        expected_m1,
        "calq0's emission must equal the seed's (M1 union chain): {}",
        diff_hint(&got_m1.output, &expected_m1)
    );
    let emitted_m1 = run_artifact(&got_m1.output).expect("the emitted M1 runs");
    assert_eq!(
        emitted_m1.values[0].as_signed().expect("i64"),
        10 * 10000 + 40 * 100 + 30
    );

    let m2 = "pub const P2 = struct { a: i64, b: i64 };\npub fn pick(p: P2) i64 {\n    const r = match p {\n        %P2{a: 1, b: _} => 10,\n        _ => 20,\n    };\n    r\n}\npub fn main() i64 {\n    const p = %P2{a: 1, b: 5};\n    const q = %P2{a: 3, b: 5};\n    pick(p) * 100 + pick(q)\n}\n";
    let expected_m2 = seed_translate_cached(&[unit("m.calq", None, m2)]).expect("the seed emits M2");
    let got_m2 = run_artifact_with_input_cached(artifact, &[unit("m.calq", None, m2)])
        .expect("calq0 compiles M2");
    assert_eq!(
        got_m2.output,
        expected_m2,
        "calq0's emission must equal the seed's (M2 struct patterns): {}",
        diff_hint(&got_m2.output, &expected_m2)
    );
    let emitted_m2 = run_artifact(&got_m2.output).expect("the emitted M2 runs");
    assert_eq!(emitted_m2.values[0].as_signed().expect("i64"), 10 * 100 + 20);

    let m3 = "pub fn tag(n: i64) i64 {\n    const k: i64 = 3;\n    const r = match n {\n        1 => 100,\n        2 => 200,\n        ^k => 300,\n        _ => 0,\n    };\n    r\n}\npub fn main() i64 {\n    tag(2) * 100000 + tag(3) * 100 + tag(9)\n}\n";
    let expected_m3 = seed_translate_cached(&[unit("m.calq", None, m3)]).expect("the seed emits M3");
    let got_m3 = run_artifact_with_input_cached(artifact, &[unit("m.calq", None, m3)])
        .expect("calq0 compiles M3");
    assert_eq!(
        got_m3.output,
        expected_m3,
        "calq0's emission must equal the seed's (M3 builtin literals+pins): {}",
        diff_hint(&got_m3.output, &expected_m3)
    );
    let emitted_m3 = run_artifact(&got_m3.output).expect("the emitted M3 runs");
    assert_eq!(
        emitted_m3.values[0].as_signed().expect("i64"),
        200 * 100000 + 300 * 100 + 0
    );

    let m4 = "pub fn grade(g: Str) i64 {\n    const r = match g {\n        \"a\" => 1,\n        \"b\" => 2,\n        other => 3,\n    };\n    r\n}\npub fn main() i64 {\n    grade(\"a\") * 1000 + grade(\"b\") * 10 + grade(\"z\")\n}\n";
    let expected_m4 = seed_translate_cached(&[unit("m.calq", None, m4)]).expect("the seed emits M4");
    let got_m4 = run_artifact_with_input_cached(artifact, &[unit("m.calq", None, m4)])
        .expect("calq0 compiles M4");
    assert_eq!(
        got_m4.output,
        expected_m4,
        "calq0's emission must equal the seed's (M4 Str subjects): {}",
        diff_hint(&got_m4.output, &expected_m4)
    );
    let emitted_m4 = run_artifact(&got_m4.output).expect("the emitted M4 runs");
    assert_eq!(
        emitted_m4.values[0].as_signed().expect("i64"),
        1 * 1000 + 2 * 10 + 3
    );

    let m5 = "pub fn say(n: i64) i64 {\n    var acc: i64 = 0;\n    match n {\n        1 => { acc = 1; },\n        _ => { acc = 2; },\n    };\n    acc\n}\npub fn main() i64 {\n    say(1) * 100 + say(0)\n}\n";
    let expected_m5 = seed_translate_cached(&[unit("m.calq", None, m5)]).expect("the seed emits M5");
    let got_m5 = run_artifact_with_input_cached(artifact, &[unit("m.calq", None, m5)])
        .expect("calq0 compiles M5");
    assert_eq!(
        got_m5.output,
        expected_m5,
        "calq0's emission must equal the seed's (M5 unit join): {}",
        diff_hint(&got_m5.output, &expected_m5)
    );
    let emitted_m5 = run_artifact(&got_m5.output).expect("the emitted M5 runs");
    assert_eq!(emitted_m5.values[0].as_signed().expect("i64"), 1 * 100 + 2);
}

#[test]
fn calq0_reports_match_errors() {
    // Slice 3c: the match rules' error paths, asserted by hand. The
    // texts mirror the seed's, with REAL spans (the match keyword's
    // position; the pattern-level 0:0 debt is paid).
    let artifact = compile_artifact();

    let e1 = "pub const S = union(enum) { A: i64, B };\npub fn f(s: S) i64 {\n    const r = match s {\n        S.A(x) => 1,\n        S.A(y) => 2,\n        S.B => 0,\n    };\n    r\n}\npub fn main() i64 { 0 }\n";
    let o1 = run_artifact_with_input_cached(artifact, &[unit("e.calq", None, e1)]).expect("runs");
    assert_eq!(o1.output, "error 3:15 redundant arm: `A` is already covered\n");
    assert_eq!(o1.values[0].as_signed().expect("i64"), 1);

    let e2 = "pub const S = union(enum) { A: i64, B };\npub fn f(s: S) i64 {\n    const r = match s {\n        S.A(x) => 1,\n    };\n    r\n}\npub fn main() i64 { 0 }\n";
    let o2 = run_artifact_with_input_cached(artifact, &[unit("e.calq", None, e2)]).expect("runs");
    assert_eq!(o2.output, "error 3:15 match does not cover S.B\n");
    assert_eq!(o2.values[0].as_signed().expect("i64"), 1);

    let e3 = "pub const P = struct { a: i64 };\npub fn f(p: P) i64 {\n    const r = match p {\n        %P{a: 1} => 1,\n    };\n    r\n}\npub fn main() i64 { 0 }\n";
    let o3 = run_artifact_with_input_cached(artifact, &[unit("e.calq", None, e3)]).expect("runs");
    assert_eq!(
        o3.output,
        "error 3:15 a struct match needs a final catch-all arm (`_`, a binding, or an all-binding record pattern) (seed v0)\n"
    );
    assert_eq!(o3.values[0].as_signed().expect("i64"), 1);

    let e4 = "pub fn f(n: i64) i64 {\n    const r = match n {\n        1 => 1,\n    };\n    r\n}\npub fn main() i64 { 0 }\n";
    let o4 = run_artifact_with_input_cached(artifact, &[unit("e.calq", None, e4)]).expect("runs");
    assert_eq!(o4.output, "error 2:15 a builtin match needs a final `_` or binding arm (seed v0)\n");
    assert_eq!(o4.values[0].as_signed().expect("i64"), 1);

    let e5 = "pub const S = union(enum) { A: i64, B: i64 };\npub fn f(s: S) i64 {\n    const r = match s {\n        S.A(1) => 1,\n        S.B(2) => 0,\n    };\n    r\n}\npub fn main() i64 { 0 }\n";
    let o5 = run_artifact_with_input_cached(artifact, &[unit("e.calq", None, e5)]).expect("runs");
    assert_eq!(
        o5.output,
        "error 3:15 this payload pattern can fail; add a following `_` arm (seed v0)\n"
    );
    assert_eq!(o5.values[0].as_signed().expect("i64"), 1);

    let e6 = "pub const S = union(enum) { A: i64, B };\npub const T = union(enum) { C: i64 };\npub fn f(s: S) i64 {\n    const r = match s {\n        T.C(x) => 1,\n        _ => 0,\n    };\n    r\n}\npub fn main() i64 { 0 }\n";
    let o6 = run_artifact_with_input_cached(artifact, &[unit("e.calq", None, e6)]).expect("runs");
    assert_eq!(o6.output, "error 4:15 pattern `T` does not match scrutinee type `S`\n");
    assert_eq!(o6.values[0].as_signed().expect("i64"), 1);
}

#[test]
fn seed_mut_params_and_field_assignment() {
    // D-112: mut parameters + field assignment. Caller-visible state
    // updates through the borrow, independent emitter instances, and
    // the exclusivity rules (immutable bindings and duplicate mut
    // positions are errors).
    let files = vec![
        stdlib_str(),
        unit(
            "mut.calq",
            None,
            "pub const Emitter = struct {
    count: i64 = 0,
    total: i64 = 0,
    label: Str = \"\",
};
pub fn bump(mut e: Emitter, n: i64) i64 {
    e.count = e.count + 1;
    e.total = e.total + n;
    e.count
}
pub fn main() i64 {
    var a = %Emitter{};
    var b = %Emitter{};
    const c1: i64 = bump(a, 5);
    const c2: i64 = bump(a, 7);
    const c3: i64 = bump(b, 1);
    io.print(str.fromI64(a.total));
    io.print(\"\\n\");
    io.print(str.fromI64(b.total));
    io.print(\"\\n\");
    io.print(str.fromI64(a.count));
    io.print(\"\\n\");
    io.print(str.fromI64(c1 + c2 + c3));
    io.print(\"\\n\");
    0
}",
        ),
    ];
    let artifact = seed_translate_cached(&files).expect("the seed emits the mut probe");
    let outcome = run_artifact(&artifact).expect("the mut probe runs");
    assert_eq!(outcome.output, "12\n1\n2\n4\n");

    // an immutable binding cannot be a mut argument.
    let bad1 = vec![
        stdlib_str(),
        unit(
            "mutbad1.calq",
            None,
            "pub const E = struct { n: i64 = 0 };\npub fn bump(mut e: E) i64 {\n    e.n = e.n + 1;\n    e.n\n}\npub fn main() i64 {\n    const x = %E{};\n    bump(x)\n}\n",
        ),
    ];
    let err1 = seed_translate(&bad1).expect_err("a const cannot be a mut argument");
    assert!(
        err1.contains("is not mutable") && err1.contains("`x`"),
        "the exclusive-mutation rule must name the binding: {err1}"
    );

    // one binding cannot occupy two mut positions in a call.
    let bad2 = vec![
        stdlib_str(),
        unit(
            "mutbad2.calq",
            None,
            "pub const E = struct { n: i64 = 0 };\npub fn two(mut x: E, mut y: E) i64 {\n    x.n = x.n + 1;\n    y.n = y.n + 1;\n    x.n + y.n\n}\npub fn main() i64 {\n    var a = %E{};\n    two(a, a)\n}\n",
        ),
    ];
    let err2 = seed_translate(&bad2).expect_err("one binding cannot be two mut positions");
    assert!(
        err2.contains("two mut positions"),
        "the exclusivity rule must fire: {err2}"
    );
}

#[test]
fn seed_collection_literals_and_inference() {
    // D-112's constructor-scope surface: `[T]` types, annotation-typed
    // literals `[]` / `[a, b]` (named slots wrap like vector.push),
    // collection-typed struct defaults, and `var x = e` inference.
    let files = vec![
        stdlib_str(),
        unit(
            "lit.calq",
            None,
            "pub const P2 = struct { a: i64, b: i64 };
pub const B = struct { label: Str = \"\", lines: [Str] = [] };
pub fn main() i64 {
    var v: [i64] = [1, 2, 3];
    const names: [Str] = [\"a\", \"b\"];
    var empty: [i64] = [];
    var inferred = 42;
    const ps: [P2] = [%P2{a: 1, b: 2}];
    const b = %B{};
    io.print(str.fromI64(vector.len(v)));
    io.print(\"\\n\");
    io.print(str.fromI64(vector.len(names)));
    io.print(\"\\n\");
    io.print(str.fromI64(vector.len(empty)));
    io.print(\"\\n\");
    io.print(str.fromI64(inferred));
    io.print(\"\\n\");
    io.print(str.fromI64(vector.len(ps)));
    io.print(\"\\n\");
    io.print(str.fromI64(vector.len(b.lines)));
    io.print(\"\\n\");
    0
}",
        ),
    ];
    let artifact = seed_translate_cached(&files).expect("the seed emits the literal probe");
    let outcome = run_artifact(&artifact).expect("the literal probe runs");
    assert_eq!(outcome.output, "3\n2\n0\n42\n1\n0\n");
}

#[test]
fn calq0_matches_the_seed_on_the_new_surface() {
    // D-112's surface through the canonical compiler: [T] types,
    // collection literals (i64/Str/named slots), struct field defaults,
    // var inference, mut parameters, and field assignment - calq0's
    // emission must equal the seed's byte-for-byte, then run.
    let artifact = compile_artifact();
    let lit = "pub const P2 = struct { a: i64, b: i64 };
pub const B = struct { label: Str = \"\", lines: [Str] = [] };
pub fn main() i64 {
    var v: [i64] = [1, 2, 3];
    const names: [Str] = [\"a\", \"b\"];
    var empty: [i64] = [];
    var inferred = 42;
    const ps: [P2] = [%P2{a: 1, b: 2}];
    const b = %B{};
    io.print(str.fromI64(vector.len(v)));
    io.print(\"\\n\");
    io.print(str.fromI64(vector.len(names)));
    io.print(\"\\n\");
    io.print(str.fromI64(vector.len(empty)));
    io.print(\"\\n\");
    io.print(str.fromI64(inferred));
    io.print(\"\\n\");
    io.print(str.fromI64(vector.len(ps)));
    io.print(\"\\n\");
    io.print(str.fromI64(vector.len(b.lines)));
    io.print(\"\\n\");
    0
}";
    let mts = "pub const Emitter = struct {
    count: i64 = 0,
    total: i64 = 0,
    label: Str = \"\",
};
pub fn bump(mut e: Emitter, n: i64) i64 {
    e.count = e.count + 1;
    e.total = e.total + n;
    e.count
}
pub fn main() i64 {
    var a = %Emitter{};
    var b = %Emitter{};
    const c1: i64 = bump(a, 5);
    const c2: i64 = bump(a, 7);
    const c3: i64 = bump(b, 1);
    io.print(str.fromI64(a.total));
    io.print(\"\\n\");
    io.print(str.fromI64(b.total));
    io.print(\"\\n\");
    io.print(str.fromI64(a.count));
    io.print(\"\\n\");
    io.print(str.fromI64(c1 + c2 + c3));
    io.print(\"\\n\");
    0
}";
    for (name, src, want) in [
        ("lit.calq", lit, "3\n2\n0\n42\n1\n0\n"),
        ("mut.calq", mts, "12\n1\n2\n4\n"),
    ] {
        let files = vec![stdlib_str(), unit(name, None, src)];
        let expected = seed_translate_cached(&files).expect("the seed emits the probe");
        let got = run_artifact_with_input_cached(artifact, &files).expect("calq0 compiles the probe");
        assert_eq!(
            got.output,
            expected,
            "calq0's emission must equal the seed's ({name}): {}",
            diff_hint(&got.output, &expected)
        );
        let emitted = run_artifact(&got.output).expect("the emitted probe runs");
        assert_eq!(emitted.output, *want, "{name}");
    }
}

#[test]
fn seed_shortcircuit_and_pipelines() {
    // SPEC 5.1/6.3: `and`/`or` short-circuit on bool operands (the
    // right side runs only on the deciding edge); `x |> f(a, b)` is
    // `f(x, a, b)`, x first and exactly once; chains associate left.
    let files = vec![
        stdlib_str(),
        unit(
            "sc.calq",
            None,
            "pub fn noisy(x: i64) bool {
    io.print(str.fromI64(x));
    io.print(\"\\n\");
    return x > 0;
}
pub fn add(a: i64, b: i64) i64 {
    a + b
}
pub fn main() i64 {
    if false and noisy(1) { io.print(\"bad\\n\"); } else { io.print(\"ok-a\\n\"); }
    if true or noisy(2) { io.print(\"ok-b\\n\"); } else { io.print(\"bad\\n\"); }
    if true and noisy(3) { io.print(\"ran\\n\"); } else { io.print(\"bad\\n\"); }
    io.print(str.fromI64(5 |> add(3)));
    io.print(\"\\n\");
    io.print(str.fromI64(1 |> add(2) |> add(10)));
    io.print(\"\\n\");
    0
}",
        ),
    ];
    let artifact = seed_translate_cached(&files).expect("the seed emits the probe");
    let outcome = run_artifact(&artifact).expect("the probe runs");
    assert_eq!(outcome.output, "ok-a\nok-b\n3\nran\n8\n13\n");
}

#[test]
fn calq0_matches_the_seed_on_shortcircuit_and_pipelines() {
    // SPEC 5.1/6.3 through the canonical compiler: short-circuiting
    // and/or (the deciding value rides the false/true edge) and the
    // pipeline desugar, byte-equal to the seed, then executed.
    let artifact = compile_artifact();
    let src = "pub fn noisy(x: i64) bool {
    io.print(str.fromI64(x));
    io.print(\"\\n\");
    return x > 0;
}
pub fn add(a: i64, b: i64) i64 {
    a + b
}
pub fn main() i64 {
    if false and noisy(1) { io.print(\"bad\\n\"); } else { io.print(\"ok-a\\n\"); }
    if true or noisy(2) { io.print(\"ok-b\\n\"); } else { io.print(\"bad\\n\"); }
    if true and noisy(3) { io.print(\"ran\\n\"); } else { io.print(\"bad\\n\"); }
    io.print(str.fromI64(5 |> add(3)));
    io.print(\"\\n\");
    io.print(str.fromI64(1 |> add(2) |> add(10)));
    io.print(\"\\n\");
    0
}";
    let files = vec![stdlib_str(), unit("sc.calq", None, src)];
    let expected = seed_translate_cached(&files).expect("the seed emits the probe");
    let got = run_artifact_with_input_cached(artifact, &files).expect("calq0 compiles the probe");
    assert_eq!(
        got.output,
        expected,
        "calq0's emission must equal the seed's (sc): {}",
        diff_hint(&got.output, &expected)
    );
    let emitted = run_artifact(&got.output).expect("the emitted probe runs");
    assert_eq!(emitted.output, "ok-a\nok-b\n3\nran\n8\n13\n");
}

#[test]
fn seed_closures_capture_and_apply() {
    // SPEC 6.4 / D-092: `fn (x: T) R { ... }` literals lift to
    // envref-first functions; the capture env rides a product (consts
    // by value, vars by box); calls through values are apply(args).
    let files = vec![
        stdlib_str(),
        unit(
            "cl.calq",
            None,
            "pub fn apply1(f: fn (i64) i64, x: i64) i64 {
    f(x)
}
pub fn main() i64 {
    const base: i64 = 10;
    const addBase: fn (i64) i64 = fn (x: i64) i64 { x + base };
    io.print(str.fromI64(apply1(addBase, 5)));
    io.print(\"\\n\");
    const noCap: fn (i64) i64 = fn (x: i64) i64 { x * 2 };
    io.print(str.fromI64(noCap(21)));
    io.print(\"\\n\");
    0
}
",
        ),
    ];
    let artifact = seed_translate_cached(&files).expect("the seed emits the closure probe");
    let outcome = run_artifact(&artifact).expect("the closure probe runs");
    assert_eq!(outcome.output, "15\n42\n");

    // A captured VAR shares the box (D-092): the closure's mutation is
    // visible to the caller.
    let var_probe = vec![
        stdlib_str(),
        unit(
            "clv.calq",
            None,
            "pub fn main() i64 {\n    var n: i64 = 5;\n    const bump: fn () i64 = fn () i64 { n = n + 1; n };\n    const a: i64 = bump();\n    const b: i64 = bump();\n    io.print(str.fromI64(a + b + n));\n    io.print(\"\\n\");\n    0\n}\n",
        ),
    ];
    let va = seed_translate_cached(&var_probe).expect("the seed emits the var-capture probe");
    let vo = run_artifact(&va).expect("the var-capture probe runs");
    assert_eq!(vo.output, "20\n"); // bump: n=6 ret 6; again: n=7 ret 7; a+b+n = 6+7+7
}

#[test]
fn calq0_matches_the_seed_on_closures() {
    // SPEC 6.4 through the canonical compiler: fn types, the lambda
    // lift with capture envs (const by value, var by box), and apply -
    // byte-equal to the seed, then executed.
    let artifact = compile_artifact();
    let cl = "pub fn apply1(f: fn (i64) i64, x: i64) i64 {
    f(x)
}
pub fn main() i64 {
    const base: i64 = 10;
    const addBase: fn (i64) i64 = fn (x: i64) i64 { x + base };
    io.print(str.fromI64(apply1(addBase, 5)));
    io.print(\"\\n\");
    const noCap: fn (i64) i64 = fn (x: i64) i64 { x * 2 };
    io.print(str.fromI64(noCap(21)));
    io.print(\"\\n\");
    0
}
";
    let files = vec![stdlib_str(), unit("cl.calq", None, cl)];
    let expected = seed_translate_cached(&files).expect("the seed emits the closure probe");
    let got = run_artifact_with_input_cached(artifact, &files).expect("calq0 compiles the closure probe");
    assert_eq!(
        got.output,
        expected,
        "calq0's emission must equal the seed's (closures): {}",
        diff_hint(&got.output, &expected)
    );
    let emitted = run_artifact(&got.output).expect("the emitted probe runs");
    assert_eq!(emitted.output, "15\n42\n");
}

#[test]
fn seed_dot_variant_constructors() {
    // SPEC 6.5's `.Err(...)` shape (D-090's type-elided dot forms): the
    // target union comes from the expected-type context (here, the
    // enclosing function's return type).
    let files = vec![
        stdlib_str(),
        unit(
            "dv.calq",
            None,
            "pub const R = union(enum) { Ok: i64, Err: Str };
pub fn f(x: i64) R {
    if x > 0 { return .Ok(x); }
    return .Err(\"neg\");
}
pub fn main() i64 {
    const a = f(5);
    const b = f(0 - 1);
    const r = match a {
        R.Ok(v) => v,
        R.Err(e) => 0,
    };
    const r2 = match b {
        R.Ok(v) => v,
        R.Err(e) => 7,
    };
    io.print(str.fromI64(r * 10 + r2));
    io.print(\"\\n\");
    0
}",
        ),
    ];
    let artifact = seed_translate_cached(&files).expect("the seed emits the dot probe");
    let outcome = run_artifact(&artifact).expect("the dot probe runs");
    assert_eq!(outcome.output, "57\n");
}

#[test]
fn calq0_matches_the_seed_on_dot_variants() {
    // The `.Variant` synthesis through the canonical compiler:
    // byte-equal to the seed, then executed.
    let artifact = compile_artifact();
    let src = "pub const R = union(enum) { Ok: i64, Err: Str };
pub fn f(x: i64) R {
    if x > 0 { return .Ok(x); }
    return .Err(\"neg\");
}
pub fn main() i64 {
    const a = f(5);
    const b = f(0 - 1);
    const r = match a {
        R.Ok(v) => v,
        R.Err(e) => 0,
    };
    const r2 = match b {
        R.Ok(v) => v,
        R.Err(e) => 7,
    };
    io.print(str.fromI64(r * 10 + r2));
    io.print(\"\\n\");
    0
}";
    let files = vec![stdlib_str(), unit("dv.calq", None, src)];
    let expected = seed_translate_cached(&files).expect("the seed emits the dot probe");
    let got = run_artifact_with_input_cached(artifact, &files).expect("calq0 compiles the dot probe");
    assert_eq!(
        got.output,
        expected,
        "calq0's emission must equal the seed's (dot variants): {}",
        diff_hint(&got.output, &expected)
    );
    let emitted = run_artifact(&got.output).expect("the emitted probe runs");
    assert_eq!(emitted.output, "57\n");
}

#[test]
fn seed_try_propagates_the_same_error() {
    // SPEC 6.5: `try e` on an Ok/Err union; on Err the enclosing
    // function returns Err(x) (the same E), on Ok the payload flows.
    let files = vec![
        stdlib_str(),
        unit(
            "tr.calq",
            None,
            "pub const Res = union(enum) { Ok: i64, Err: i64 };
pub fn parse1(x: i64) Res {
    if x > 0 { return .Ok(x); }
    return .Err(7);
}
pub fn twice(x: i64) Res {
    const v = try parse1(x);
    return .Ok(v * 2);
}
pub fn main() i64 {
    const a = twice(5);
    const b = twice(0);
    const r = match a {
        Res.Ok(v) => v,
        Res.Err(e) => 0,
    };
    const r2 = match b {
        Res.Ok(v) => v,
        Res.Err(e) => e,
    };
    io.print(str.fromI64(r * 100 + r2));
    io.print(\"\\n\");
    0
}",
        ),
    ];
    let artifact = seed_translate_cached(&files).expect("the seed emits the try probe");
    let outcome = run_artifact(&artifact).expect("the try probe runs");
    assert_eq!(outcome.output, "1007\n");
}

#[test]
fn calq0_matches_the_seed_on_try() {
    // SPEC 6.5's `try` through the canonical compiler: byte-equal to
    // the seed, then executed (Ok flows, Err returns from the fn).
    let artifact = compile_artifact();
    let src = "pub const Res = union(enum) { Ok: i64, Err: i64 };
pub fn parse1(x: i64) Res {
    if x > 0 { return .Ok(x); }
    return .Err(7);
}
pub fn twice(x: i64) Res {
    const v = try parse1(x);
    return .Ok(v * 2);
}
pub fn main() i64 {
    const a = twice(5);
    const b = twice(0);
    const r = match a {
        Res.Ok(v) => v,
        Res.Err(e) => 0,
    };
    const r2 = match b {
        Res.Ok(v) => v,
        Res.Err(e) => e,
    };
    io.print(str.fromI64(r * 100 + r2));
    io.print(\"\\n\");
    0
}";
    let files = vec![stdlib_str(), unit("tr.calq", None, src)];
    let expected = seed_translate_cached(&files).expect("the seed emits the try probe");
    let got = run_artifact_with_input_cached(artifact, &files).expect("calq0 compiles the try probe");
    assert_eq!(
        got.output,
        expected,
        "calq0's emission must equal the seed's (try): {}",
        diff_hint(&got.output, &expected)
    );
    let emitted = run_artifact(&got.output).expect("the emitted probe runs");
    assert_eq!(emitted.output, "1007\n");
}

#[test]
fn seed_dot_patterns_in_matches() {
    // `.Variant` patterns (D-090's type-elided dot forms): the
    // scrutinee's union supplies the type for the recovery match.
    let files = vec![
        stdlib_str(),
        unit(
            "dp.calq",
            None,
            "pub const Res = union(enum) { Ok: i64, Err: i64 };
pub fn parse1(x: i64) Res {
    if x > 0 { return .Ok(x); }
    return .Err(7);
}
pub fn twice(x: i64) Res {
    const v = try parse1(x);
    return .Ok(v * 2);
}
pub fn score(r: Res) i64 {
    const s = match r {
        .Ok(v) => v,
        .Err(e) => e,
    };
    s
}
pub fn main() i64 {
    const a = twice(5);
    const b = twice(0);
    io.print(str.fromI64(score(a) * 100 + score(b)));
    io.print(\"\\n\");
    0
}",
        ),
    ];
    let artifact = seed_translate_cached(&files).expect("the seed emits the dot-pattern probe");
    let outcome = run_artifact(&artifact).expect("the dot-pattern probe runs");
    assert_eq!(outcome.output, "1007\n");
}

#[test]
fn calq0_matches_the_seed_on_dot_patterns() {
    // `.Variant` patterns through the canonical compiler (the recovery
    // match form of SPEC 6.5): byte-equal, then executed.
    let artifact = compile_artifact();
    let src = "pub const Res = union(enum) { Ok: i64, Err: i64 };
pub fn parse1(x: i64) Res {
    if x > 0 { return .Ok(x); }
    return .Err(7);
}
pub fn twice(x: i64) Res {
    const v = try parse1(x);
    return .Ok(v * 2);
}
pub fn score(r: Res) i64 {
    const s = match r {
        .Ok(v) => v,
        .Err(e) => e,
    };
    s
}
pub fn main() i64 {
    const a = twice(5);
    const b = twice(0);
    io.print(str.fromI64(score(a) * 100 + score(b)));
    io.print(\"\\n\");
    0
}";
    let files = vec![stdlib_str(), unit("dp.calq", None, src)];
    let expected = seed_translate_cached(&files).expect("the seed emits the dot-pattern probe");
    let got = run_artifact_with_input_cached(artifact, &files).expect("calq0 compiles the dot-pattern probe");
    assert_eq!(
        got.output,
        expected,
        "calq0's emission must equal the seed's (dot patterns): {}",
        diff_hint(&got.output, &expected)
    );
    let emitted = run_artifact(&got.output).expect("the emitted probe runs");
    assert_eq!(emitted.output, "1007\n");
}

#[test]
fn seed_option_result_shorthands() {
    // SPEC 6.5's surface end to end: `?i64` / `Er!i64` structural
    // error unions, `.Some`/`.None`/`.Ok`/`.Err(.TooSmall)` dot
    // constructors, dot patterns in the recovery matches, and `try`
    // propagating through a Result shorthand.
    let files = vec![
        stdlib_str(),
        unit(
            "sh.calq",
            None,
            "pub const Er = union(enum) { TooSmall };
pub fn opt(x: i64) ?i64 {
    if x > 0 { return .Some(x); }
    return .None;
}
pub fn or0(v: ?i64) i64 {
    const s = match v {
        .Some(x) => x,
        .None => 0,
    };
    s
}
pub fn dbl(x: i64) Er!i64 {
    if x > 0 { return .Ok(x * 2); }
    return .Err(.TooSmall);
}
pub fn use(x: i64) Er!i64 {
    const v = try dbl(x);
    return .Ok(v + 1);
}
pub fn main() i64 {
    const a = or0(opt(5));
    const b = or0(opt(0));
    const u1 = use(5);
    const r1 = match u1 {
        .Ok(v) => v,
        .Err(e) => 0,
    };
    const u2 = use(0);
    const r2 = match u2 {
        .Ok(v) => v,
        .Err(e) => 9,
    };
    io.print(str.fromI64(a * 1000 + b * 100 + r1 + r2));
    io.print(\"\\n\");
    0
}",
        ),
    ];
    let artifact = seed_translate_cached(&files).expect("the seed emits the shorthand probe");
    let outcome = run_artifact(&artifact).expect("the shorthand probe runs");
    assert_eq!(outcome.output, "5020\n");
}

#[test]
fn calq0_matches_the_seed_on_option_result_shorthands() {
    // The differential for `?T` / `E!T`: calq0 (compiled by the seed)
    // compiles the shorthand probe - its whole pipeline (parse wrapper,
    // synthetic union decls, typed dot constructors, try, dot
    // patterns) byte-equal to the seed's own artifact.
    let files = vec![
        stdlib_str(),
        unit(
            "sh.calq",
            None,
            "pub const Er = union(enum) { TooSmall };
pub fn opt(x: i64) ?i64 {
    if x > 0 { return .Some(x); }
    return .None;
}
pub fn or0(v: ?i64) i64 {
    const s = match v {
        .Some(x) => x,
        .None => 0,
    };
    s
}
pub fn dbl(x: i64) Er!i64 {
    if x > 0 { return .Ok(x * 2); }
    return .Err(.TooSmall);
}
pub fn use(x: i64) Er!i64 {
    const v = try dbl(x);
    return .Ok(v + 1);
}
pub fn main() i64 {
    const a = or0(opt(5));
    const b = or0(opt(0));
    const u1 = use(5);
    const r1 = match u1 {
        .Ok(v) => v,
        .Err(e) => 0,
    };
    const u2 = use(0);
    const r2 = match u2 {
        .Ok(v) => v,
        .Err(e) => 9,
    };
    io.print(str.fromI64(a * 1000 + b * 100 + r1 + r2));
    io.print(\"\\n\");
    0
}",
        ),
    ];
    let artifact = compile_artifact();
    let expected = seed_translate_cached(&files).expect("the seed emits the shorthand probe");
    let got = run_artifact_with_input_cached(artifact, &files).expect("calq0 compiles the shorthand probe");
    assert_eq!(
        got.output,
        expected,
        "calq0's emission must equal the seed's (option/result shorthands): {}",
        diff_hint(&got.output, &expected)
    );
    let emitted = run_artifact(&got.output).expect("the emitted probe runs");
    assert_eq!(emitted.output, "5020\n");
}

#[test]
fn seed_catch_recovery() {
    // SPEC 6.5's `catch` (precedence level 0): the Ok path unwraps,
    // the Err path runs the replacement block - both the diverging
    // playbook shape (`catch { return .Err(...) }`) and the binding
    // form (`catch |e| { ... }`).
    let files = vec![
        stdlib_str(),
        unit(
            "ca.calq",
            None,
            "pub const Er = union(enum) { TooSmall };
pub fn chk(x: i64) Er!i64 {
    if x > 0 { return .Ok(x); }
    return .Err(.TooSmall);
}
pub fn f(x: i64) Er!i64 {
    const v = chk(x) catch { return .Err(.TooSmall); };
    return .Ok(v + 100);
}
pub fn g(x: i64) i64 {
    const v = chk(x) catch |e| { 7 };
    v
}
pub fn main() i64 {
    const fa = f(5);
    const a = match fa { .Ok(v) => v, .Err(e) => 0 };
    const fb = f(0);
    const b = match fb { .Ok(v) => v, .Err(e) => 1 };
    const c = g(5);
    const d = g(0);
    io.print(str.fromI64(a * 1000 + b * 100 + c * 10 + d));
    io.print(\"\\n\");
    0
}",
        ),
    ];
    let artifact = seed_translate_cached(&files).expect("the seed emits the catch probe");
    let outcome = run_artifact(&artifact).expect("the catch probe runs");
    assert_eq!(outcome.output, "105157\n");
}

#[test]
fn calq0_matches_the_seed_on_catch() {
    // The differential for `catch` (SPEC 6.5, level 0): calq0's parse
    // (the loosest-precedence clause), the Err-path block with its
    // binding, and the join - byte-equal to the seed's artifact.
    let files = vec![
        stdlib_str(),
        unit(
            "ca.calq",
            None,
            "pub const Er = union(enum) { TooSmall };
pub fn chk(x: i64) Er!i64 {
    if x > 0 { return .Ok(x); }
    return .Err(.TooSmall);
}
pub fn f(x: i64) Er!i64 {
    const v = chk(x) catch { return .Err(.TooSmall); };
    return .Ok(v + 100);
}
pub fn g(x: i64) i64 {
    const v = chk(x) catch |e| { 7 };
    v
}
pub fn main() i64 {
    const fa = f(5);
    const a = match fa { .Ok(v) => v, .Err(e) => 0 };
    const fb = f(0);
    const b = match fb { .Ok(v) => v, .Err(e) => 1 };
    const c = g(5);
    const d = g(0);
    io.print(str.fromI64(a * 1000 + b * 100 + c * 10 + d));
    io.print(\"\\n\");
    0
}",
        ),
    ];
    let artifact = compile_artifact();
    let expected = seed_translate_cached(&files).expect("the seed emits the catch probe");
    let got = run_artifact_with_input_cached(artifact, &files).expect("calq0 compiles the catch probe");
    assert_eq!(
        got.output,
        expected,
        "calq0's emission must equal the seed's (catch): {}",
        diff_hint(&got.output, &expected)
    );
    let emitted = run_artifact(&got.output).expect("the emitted probe runs");
    assert_eq!(emitted.output, "105157\n");
}

#[test]
fn seed_monomorphized_generics() {
    // D-113: `comptime T: type` parameters, explicit instantiation -
    // one specialized function per type tuple, memoized (identity at
    // i64 AND Str; pick at i64).
    let files = vec![
        stdlib_str(),
        unit(
            "ge.calq",
            None,
            "pub fn identity(comptime T: type, value: T) T {
    value
}
pub fn pick(comptime T: type, a: T, b: T, take_first: bool) T {
    if take_first { return a; }
    b
}
pub fn main() i64 {
    const a = identity(i64, 41);
    const s = identity(Str, \"abc\");
    const c = pick(i64, 1, 2, false);
    io.print(str.fromI64(a + 1));
    io.print(\" \");
    io.print(s);
    io.print(\" \");
    io.print(str.fromI64(c));
    io.print(\"\\n\");
    0
}",
        ),
    ];
    let artifact = seed_translate_cached(&files).expect("the seed emits the generics probe");
    let outcome = run_artifact(&artifact).expect("the generics probe runs");
    assert_eq!(outcome.output, "42 abc 2\n");
}

#[test]
fn calq0_matches_the_seed_on_generics() {
    // The D-113 differential: calq0's monomorphization (comptime
    // params, explicit instantiation, the tvars funnels, the instance
    // queue) - byte-equal to the seed's artifact, then executed.
    let files = vec![
        stdlib_str(),
        unit(
            "ge.calq",
            None,
            "pub fn identity(comptime T: type, value: T) T {
    value
}
pub fn pick(comptime T: type, a: T, b: T, take_first: bool) T {
    if take_first { return a; }
    b
}
pub fn main() i64 {
    const a = identity(i64, 41);
    const s = identity(Str, \"abc\");
    const c = pick(i64, 1, 2, false);
    io.print(str.fromI64(a + 1));
    io.print(\" \");
    io.print(s);
    io.print(\" \");
    io.print(str.fromI64(c));
    io.print(\"\\n\");
    0
}",
        ),
    ];
    let artifact = compile_artifact();
    let expected = seed_translate_cached(&files).expect("the seed emits the generics probe");
    let got = run_artifact_with_input_cached(artifact, &files).expect("calq0 compiles the generics probe");
    assert_eq!(
        got.output,
        expected,
        "calq0's emission must equal the seed's (generics): {}",
        diff_hint(&got.output, &expected)
    );
    let emitted = run_artifact(&got.output).expect("the emitted probe runs");
    assert_eq!(emitted.output, "42 abc 2\n");
}

#[test]
fn seed_inferred_closure_params() {
    // D-086's inferred closure parameters: a bare or partial closure
    // literal at a fn-typed position takes its signature from the
    // expected type; a closure with NO expected type fails loudly.
    let files = vec![
        stdlib_str(),
        unit(
            "inf.calq",
            None,
            "pub fn twice(comptime T: type, f: fn (T) T, x: T) T {
    return f(f(x));
}
pub fn main() i64 {
    const inc = fn (n: i64) i64 { return n + 1; };
    const a = twice(i64, inc, 5);
    const b = twice(i64, fn (n) { n * 3 }, 2);
    const c = twice(i64, fn (n: i64) { return n - 1; }, 10);
    io.print(str.fromI64(a * 100 + b * 10 + c));
    io.print(\"\\n\");
    0
}",
        ),
    ];
    let artifact = seed_translate_cached(&files).expect("the seed emits the inference probe");
    let outcome = run_artifact(&artifact).expect("the inference probe runs");
    // twice applies f twice: 7, 18, 8 -> 888.
    assert_eq!(outcome.output, "888\n");
}

#[test]
fn seed_seq_stdlib() {
    // AC-8.3: the `seq` pull-sequence stdlib in Calque (iter/filter/
    // map/fold/collect/take), driven by D-113 inference - the frozen
    // active_score.calq surface with bare closure parameters.
    let files = vec![
        stdlib_str(),
        unit("tools/calque/std/seq.calq", Some("seq"), "// seq - explicit pull sequences (AC-8.3; the playbook's `seq.*`
// surface). A sequence is a `fn () ?T` closure: `iter` builds one
// over a Vector, the adapters wrap an upstream sequence, and `fold`/
// `collect` consume it. Element types come from the D-113 comptime
// parameters; each distinct type tuple instantiates its own chain.
// Eager collection operations stay on `vector.*`.

pub fn iter(comptime T: type, v: Vector(T)) fn () ?T {
    var i: i64 = 0;
    return fn () ?T {
        if i < vector.len(v) {
            const x: T = vector.get(v, i);
            i = i + 1;
            return .Some(x);
        }
        return .None;
    };
}

pub fn filter(comptime T: type, up: fn () ?T, pred: fn (T) bool) fn () ?T {
    return fn () ?T {
        var go: bool = true;
        while go {
            const x: ?T = up();
            match x {
                .Some(v) => {
                    if pred(v) {
                        return .Some(v);
                    }
                },
                .None => {
                    go = false;
                },
            };
        }
        return .None;
    };
}

pub fn map(comptime T: type, comptime U: type, up: fn () ?T, f: fn (T) U) fn () ?U {
    return fn () ?U {
        const x: ?T = up();
        return match x {
            .Some(v) => {
                const y: U = f(v);
                const r: ?U = .Some(y);
                r
            },
            .None => {
                const r2: ?U = .None;
                r2
            },
        };
    };
}

pub fn fold(comptime T: type, comptime A: type, up: fn () ?T, init: A, f: fn (A, T) A) A {
    var acc: A = init;
    var go: bool = true;
    while go {
        const x: ?T = up();
        match x {
            .Some(v) => {
                acc = f(acc, v);
            },
            .None => {
                go = false;
            },
        };
    }
    acc
}

pub fn collect(comptime T: type, up: fn () ?T) Vector(T) {
    var out: Vector(T) = vector.new();
    var go: bool = true;
    while go {
        const x: ?T = up();
        match x {
            .Some(v) => {
                vector.push(out, v);
            },
            .None => {
                go = false;
            },
        };
    }
    return out;
}

pub fn take(comptime T: type, up: fn () ?T, n: i64) fn () ?T {
    var left: i64 = n;
    return fn () ?T {
        if left <= 0 {
            return .None;
        }
        left = left - 1;
        return up();
    };
}

pub fn flat_map(comptime T: type, comptime U: type, up: fn () ?T, f: fn (T) fn () ?U) fn () ?U {
    return fn () ?U {
        var have: bool = false;
        var inner: fn () ?U = f;
        var go: bool = true;
        var result: ?U = .None;
        while go {
            if have {
                const x2: ?U = inner();
                match x2 {
                    .Some(v2) => {
                        result = .Some(v2);
                        go = false;
                    },
                    .None => {
                        have = false;
                    },
                };
            } else {
                const x: ?T = up();
                match x {
                    .Some(v) => {
                        inner = f(v);
                        have = true;
                    },
                    .None => {
                        go = false;
                    },
                };
            }
        }
        result
    };
}
"),
        unit("sq.calq", None, "const User = struct {
    name: Str,
    score: i64,
    active: bool,
};

pub fn activeScore(users: Vector(User)) i64 {
    users
        |> seq.iter()
        |> seq.filter(fn (user) { user.active })
        |> seq.map(fn (user) { user.score })
        |> seq.fold(0, fn (total, score) { total + score })
}

pub fn main() i64 {
    var us: Vector(User) = vector.new();
    vector.push(us, %User{name: \"a\", score: 10, active: true});
    vector.push(us, %User{name: \"b\", score: 5, active: false});
    vector.push(us, %User{name: \"c\", score: 7, active: true});
    const total = activeScore(us);
    const evens = us
        |> seq.iter()
        |> seq.filter(fn (u) { u.active })
        |> seq.collect();
    io.print(str.fromI64(total));
    io.print(\" \");
    io.print(str.fromI64(vector.len(evens)));
    io.print(\" \");
    io.print(str.fromI64(us |> seq.iter() |> seq.take(2) |> seq.fold(0, fn (n, _u) { n + 1 })));
    io.print(\"\\n\");
    0
}"),
    ];
    let artifact = seed_translate_cached(&files).expect("the seed emits the seq probe");
    let outcome = run_artifact(&artifact).expect("the seq probe runs");
    assert_eq!(outcome.output, "17 2 2\n");
}

#[test]
fn calq0_matches_the_seed_on_seq() {
    // The AC-8.3 differential: calq0's inference (tyarg scan, arg
    // probes, lambda tail probes), inferred closure params, and the
    // namespaced generic dispatch - byte-equal to the seed's
    // artifact, then executed.
    let files = vec![
        stdlib_str(),
        unit("tools/calque/std/seq.calq", Some("seq"), "// seq - explicit pull sequences (AC-8.3; the playbook's `seq.*`
// surface). A sequence is a `fn () ?T` closure: `iter` builds one
// over a Vector, the adapters wrap an upstream sequence, and `fold`/
// `collect` consume it. Element types come from the D-113 comptime
// parameters; each distinct type tuple instantiates its own chain.
// Eager collection operations stay on `vector.*`.

pub fn iter(comptime T: type, v: Vector(T)) fn () ?T {
    var i: i64 = 0;
    return fn () ?T {
        if i < vector.len(v) {
            const x: T = vector.get(v, i);
            i = i + 1;
            return .Some(x);
        }
        return .None;
    };
}

pub fn filter(comptime T: type, up: fn () ?T, pred: fn (T) bool) fn () ?T {
    return fn () ?T {
        var go: bool = true;
        while go {
            const x: ?T = up();
            match x {
                .Some(v) => {
                    if pred(v) {
                        return .Some(v);
                    }
                },
                .None => {
                    go = false;
                },
            };
        }
        return .None;
    };
}

pub fn map(comptime T: type, comptime U: type, up: fn () ?T, f: fn (T) U) fn () ?U {
    return fn () ?U {
        const x: ?T = up();
        return match x {
            .Some(v) => {
                const y: U = f(v);
                const r: ?U = .Some(y);
                r
            },
            .None => {
                const r2: ?U = .None;
                r2
            },
        };
    };
}

pub fn fold(comptime T: type, comptime A: type, up: fn () ?T, init: A, f: fn (A, T) A) A {
    var acc: A = init;
    var go: bool = true;
    while go {
        const x: ?T = up();
        match x {
            .Some(v) => {
                acc = f(acc, v);
            },
            .None => {
                go = false;
            },
        };
    }
    acc
}

pub fn collect(comptime T: type, up: fn () ?T) Vector(T) {
    var out: Vector(T) = vector.new();
    var go: bool = true;
    while go {
        const x: ?T = up();
        match x {
            .Some(v) => {
                vector.push(out, v);
            },
            .None => {
                go = false;
            },
        };
    }
    return out;
}

pub fn take(comptime T: type, up: fn () ?T, n: i64) fn () ?T {
    var left: i64 = n;
    return fn () ?T {
        if left <= 0 {
            return .None;
        }
        left = left - 1;
        return up();
    };
}

pub fn flat_map(comptime T: type, comptime U: type, up: fn () ?T, f: fn (T) fn () ?U) fn () ?U {
    return fn () ?U {
        var have: bool = false;
        var inner: fn () ?U = f;
        var go: bool = true;
        var result: ?U = .None;
        while go {
            if have {
                const x2: ?U = inner();
                match x2 {
                    .Some(v2) => {
                        result = .Some(v2);
                        go = false;
                    },
                    .None => {
                        have = false;
                    },
                };
            } else {
                const x: ?T = up();
                match x {
                    .Some(v) => {
                        inner = f(v);
                        have = true;
                    },
                    .None => {
                        go = false;
                    },
                };
            }
        }
        result
    };
}
"),
        unit("sq.calq", None, "const User = struct {
    name: Str,
    score: i64,
    active: bool,
};

pub fn activeScore(users: Vector(User)) i64 {
    users
        |> seq.iter()
        |> seq.filter(fn (user) { user.active })
        |> seq.map(fn (user) { user.score })
        |> seq.fold(0, fn (total, score) { total + score })
}

pub fn main() i64 {
    var us: Vector(User) = vector.new();
    vector.push(us, %User{name: \"a\", score: 10, active: true});
    vector.push(us, %User{name: \"b\", score: 5, active: false});
    vector.push(us, %User{name: \"c\", score: 7, active: true});
    const total = activeScore(us);
    const evens = us
        |> seq.iter()
        |> seq.filter(fn (u) { u.active })
        |> seq.collect();
    io.print(str.fromI64(total));
    io.print(\" \");
    io.print(str.fromI64(vector.len(evens)));
    io.print(\" \");
    io.print(str.fromI64(us |> seq.iter() |> seq.take(2) |> seq.fold(0, fn (n, _u) { n + 1 })));
    io.print(\"\\n\");
    0
}"),
    ];
    let artifact = compile_artifact();
    let expected = seed_translate_cached(&files).expect("the seed emits the seq probe");
    let got = run_artifact_with_input_cached(artifact, &files).expect("calq0 compiles the seq probe");
    assert_eq!(
        got.output,
        expected,
        "calq0's emission must equal the seed's (seq): {}",
        diff_hint(&got.output, &expected)
    );
    let emitted = run_artifact(&got.output).expect("the emitted seq probe runs");
    assert_eq!(emitted.output, "17 2 2\n");
}

#[test]
fn seed_tuples() {
    // D-114's tuple surface: `(T1, T2)` types, `(a, b)` literals,
    // `t.0`/`t.1` access, structural products end to end.
    let files = vec![
        stdlib_str(),
        unit(
            "tp.calq",
            None,
            "pub fn swap(p: (i64, Str)) (Str, i64) {\n    (p.1, p.0)\n}\npub fn tdestr(p: (i64, Str)) i64 {\n    const (n, s) = p;\n    return n + str.len(s);\n}\npub fn tdestr2() i64 {\n    var (a, b) = (3, 4);\n    a = a + b;\n    a\n}\npub fn main() i64 {\n    const t: (i64, Str) = (7, \"ab\");\n    const s = swap(t);\n    io.print(s.0);\n    io.print(\" \");\n    io.print(str.fromI64(s.1));\n    io.print(\"\\n\");\n    io.print(str.fromI64(tdestr((5, \"xy\"))));\n    io.print(\" \");\n    io.print(str.fromI64(tdestr2()));\n    io.print(\"\\n\");\n    0\n}\n",
        ),
    ];
    let artifact = seed_translate_cached(&files).expect("the seed emits the tuple probe");
    let outcome = run_artifact(&artifact).expect("the tuple probe runs");
    assert_eq!(outcome.output, "ab 7\n7 7\n");
}

#[test]
fn calq0_matches_the_seed_on_tuples() {
    // D-114's differential: calq0's tuple types, literals, `t.0`
    // access, and destructuring - byte-equal to the seed, executed.
    let files = vec![
        stdlib_str(),
        unit(
            "tp.calq",
            None,
            "pub fn swap(p: (i64, Str)) (Str, i64) {\n    (p.1, p.0)\n}\npub fn tdestr(p: (i64, Str)) i64 {\n    const (n, s) = p;\n    return n + str.len(s);\n}\npub fn tdestr2() i64 {\n    var (a, b) = (3, 4);\n    a = a + b;\n    a\n}\npub fn main() i64 {\n    const t: (i64, Str) = (7, \"ab\");\n    const s = swap(t);\n    io.print(s.0);\n    io.print(\" \");\n    io.print(str.fromI64(s.1));\n    io.print(\"\\n\");\n    io.print(str.fromI64(tdestr((5, \"xy\"))));\n    io.print(\" \");\n    io.print(str.fromI64(tdestr2()));\n    io.print(\"\\n\");\n    0\n}\n",
        ),
    ];
    let artifact = compile_artifact();
    let expected = seed_translate_cached(&files).expect("the seed emits the tuple probe");
    let got = run_artifact_with_input_cached(artifact, &files).expect("calq0 compiles the tuple probe");
    assert_eq!(
        got.output,
        expected,
        "calq0's emission must equal the seed's (tuples): {}",
        diff_hint(&got.output, &expected)
    );
    let emitted = run_artifact(&got.output).expect("the emitted tuple probe runs");
    assert_eq!(emitted.output, "ab 7\n7 7\n");
}

#[test]
fn seed_literal_forms() {
    // D-115: `_` digit separators (stripped) and verbatim `"""`
    // multiline strings.
    let files = vec![
        stdlib_str(),
        unit("lit.calq", None, LIT_SRC),
    ];
    let artifact = seed_translate_cached(&files).expect("the seed emits the literal probe");
    let outcome = run_artifact(&artifact).expect("the literal probe runs");
    assert_eq!(outcome.output, "1000|\nline one\nline two\n|26\n");
}

const LIT_SRC: &str = "pub fn main() i64 {\n    const big = 1_000_000;\n    const doc = \"\"\"\nline one\nline two\n\"\"\";\n    io.print(str.fromI64(big / 1_000));\n    io.print(\"|\");\n    io.print(doc);\n    io.print(\"|\");\n    io.print(str.fromI64(2_5 + 1));\n    io.print(\"\\n\");\n    0\n}\n";

#[test]
fn calq0_matches_the_seed_on_literal_forms() {
    // D-115's differential: `_` separators and `"""` multiline
    // strings - byte-equal to the seed, executed.
    let files = vec![
        stdlib_str(),
        unit("lit.calq", None, LIT_SRC),
    ];
    let artifact = compile_artifact();
    let expected = seed_translate_cached(&files).expect("the seed emits the literal probe");
    let got = run_artifact_with_input_cached(artifact, &files).expect("calq0 compiles the literal probe");
    assert_eq!(
        got.output,
        expected,
        "calq0's emission must equal the seed's (literal forms): {}",
        diff_hint(&got.output, &expected)
    );
    let emitted = run_artifact(&got.output).expect("the emitted literal probe runs");
    assert_eq!(emitted.output, "1000|\nline one\nline two\n|26\n");
}

#[test]
fn seed_test_blocks() {
    // F33: `test "name" { ... }` blocks compile to __calque_test_N
    // entries; assert.equal passes or traps (the runner marks the
    // block failed on a trap).
    let files = vec![stdlib_str(), unit("tb.calq", None, "pub fn twice(x: i64) i64 {\n    x * 2\n}\n\ntest \"twice doubles\" {\n    assert.equal(twice(21), 42);\n}\n\ntest \"strings agree\" {\n    assert.equal(\"ab\", \"a\");\n}\n\npub fn main() i64 {\n    io.print(str.fromI64(twice(2)));\n    io.print(\"\\n\");\n    0\n}\n")];
    let artifact = seed_translate_cached(&files).expect("the seed emits the test blocks");
    let results = run_test_blocks(&artifact).expect("the test blocks run");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].1, true, "the doubling assert passes");
    assert_eq!(results[1].1, false, "the string mismatch traps");
    let outcome = run_artifact(&artifact).expect("main runs");
    assert_eq!(outcome.output, "4\n");
}

#[test]
fn calq0_matches_the_seed_on_test_blocks() {
    let files = vec![stdlib_str(), unit("tb.calq", None, TB_SRC)];
    let artifact = compile_artifact();
    let expected = seed_translate_cached(&files).expect("the seed emits the test blocks");
    let got = run_artifact_with_input_cached(artifact, &files).expect("calq0 compiles the test blocks");
    assert_eq!(
        got.output,
        expected,
        "calq0's emission must equal the seed's (test blocks): {}",
        diff_hint(&got.output, &expected)
    );
    let results = run_test_blocks(&got.output).expect("the blocks run under calq0's artifact");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].1, true);
    assert_eq!(results[1].1, false);
}

const TB_SRC: &str = "pub fn twice(x: i64) i64 {\n    x * 2\n}\n\ntest \"twice doubles\" {\n    assert.equal(twice(21), 42);\n}\n\ntest \"strings agree\" {\n    assert.equal(\"ab\", \"a\");\n}\n\npub fn main() i64 {\n    io.print(str.fromI64(twice(2)));\n    io.print(\"\\n\");\n    0\n}\n";

#[test]
fn seed_fixed_arrays() {
    // The vector steering: `[N]T` fixed-length inline arrays -
    // literals with exactly N elements, `v[i]` reads.
    let files = vec![stdlib_str(), unit("ar.calq", None, ARR_SRC)];
    let artifact = seed_translate_cached(&files).expect("the seed emits the array probe");
    let outcome = run_artifact(&artifact).expect("the array probe runs");
    assert_eq!(outcome.output, "15|6\n");
}

const ARR_SRC: &str = "pub fn sum3(xs: [3]i64) i64 {\n    xs[0] + xs[1] + xs[2]\n}\npub fn main() i64 {\n    const xs: [3]i64 = [4, 5, 6];\n    io.print(str.fromI64(sum3(xs)));\n    io.print(\"|\");\n    io.print(str.fromI64(xs[2]));\n    io.print(\"\\n\");\n    0\n}\n";

#[test]
fn calq0_matches_the_seed_on_fixed_arrays() {
    // The vector steering's `[N]T` differential: byte-equal to the
    // seed, executed.
    let files = vec![stdlib_str(), unit("ar.calq", None, ARR_SRC)];
    let artifact = compile_artifact();
    let expected = seed_translate_cached(&files).expect("the seed emits the array probe");
    let got = run_artifact_with_input_cached(artifact, &files).expect("calq0 compiles the array probe");
    assert_eq!(
        got.output,
        expected,
        "calq0's emission must equal the seed's (fixed arrays): {}",
        diff_hint(&got.output, &expected)
    );
    let emitted = run_artifact(&got.output).expect("the emitted array probe runs");
    assert_eq!(emitted.output, "15|6\n");
}

#[test]
fn seed_struct_spread() {
    // The `..base` spread in `%T{..base, f: v}` construction: fields
    // copy semantically, explicit fields override, the base is
    // untouched.
    let files = vec![stdlib_str(), unit("sp.calq", None, SP_SRC)];
    let artifact = seed_translate_cached(&files).expect("the seed emits the spread probe");
    let outcome = run_artifact(&artifact).expect("the spread probe runs");
    assert_eq!(outcome.output, "Matt 42 41\n");
}

const SP_SRC: &str = "const User = struct {\n    name: Str,\n    score: i64,\n    active: bool,\n};\n\npub fn main() i64 {\n    const before = %User{name: \"Matt\", score: 41, active: true};\n    const after = %User{..before, score: 42};\n    io.print(after.name);\n    io.print(\" \");\n    io.print(str.fromI64(after.score));\n    io.print(\" \");\n    io.print(str.fromI64(before.score));\n    io.print(\"\\n\");\n    0\n}\n";

#[test]
fn calq0_matches_the_seed_on_struct_spread() {
    let files = vec![stdlib_str(), unit("sp.calq", None, SP_SRC)];
    let artifact = compile_artifact();
    let expected = seed_translate_cached(&files).expect("the seed emits the spread probe");
    let got = run_artifact_with_input_cached(artifact, &files).expect("calq0 compiles the spread probe");
    assert_eq!(
        got.output,
        expected,
        "calq0's emission must equal the seed's (spread): {}",
        diff_hint(&got.output, &expected)
    );
    let emitted = run_artifact(&got.output).expect("the emitted spread probe runs");
    assert_eq!(emitted.output, "Matt 42 41\n");
}

#[test]
fn seed_imports() {
    // D-116: `import NAME;` resolves against the unit set; cycles and
    // unknown names are loud.
    let std = stdlib_str();
    let files = vec![
        std,
        unit("a.calq", Some("a"), "import b;\npub fn fa(x: i64) i64 { b.fb(x) + 1 }\n"),
        unit("b.calq", Some("b"), "pub fn fb(x: i64) i64 { x * 2 }\n"),
        unit(
            "main.calq",
            None,
"import a;\npub fn main() i64 { io.print(str.fromI64(a.fa(20))); io.print(\"\\n\"); 0 }\n",
        ),
    ];
    let artifact = seed_translate_cached(&files).expect("the import set emits");
    let outcome = run_artifact(&artifact).expect("the import set runs");
    assert_eq!(outcome.output, "41\n");

    // a cycle is an error naming one edge
    let cyc = vec![
        stdlib_str(),
        unit("a.calq", Some("a"), "import b;\npub fn fa(x: i64) i64 { x }\n"),
        unit("b.calq", Some("b"), "import a;\npub fn fb(x: i64) i64 { x }\n"),
    ];
    let err = seed_translate(&cyc).expect_err("the cycle is refused");
    assert!(err.contains("import cycle detected"), "{err}");

    // an unknown name is refused with its position
    let unk = vec![
        stdlib_str(),
        unit("a.calq", Some("a"), "import nope;\npub fn fa(x: i64) i64 { x }\n"),
    ];
    let err2 = seed_translate(&unk).expect_err("the unknown import is refused");
    assert!(err2.contains("unknown import `nope`"), "{err2}");
}

#[test]
fn calq0_modules_and_receiver_calls() {
    // D-128: a module with a primary struct + `Self`; the receiver
    // spelling `recv.name(args)` and the qualified `M.name(recv, ...)`
    // agree; `mut self` receivers write through the caller's binding.
    let case_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../goldens/calque/module_counter/case.calq"
    );
    let src = std::fs::read_to_string(case_path).expect("the module case exists");
    let artifact = calque::compile_source("module_counter.calq", &src).expect("calq0 compiles it");
    let outcome = run_artifact(&artifact).expect("it runs");
    assert_eq!(outcome.output, "2\n2\n");

    // A missing associated function is a clean error naming the
    // alternative spelling.
    let bad = "pub const B = struct { n: i64 = 0, };\npub fn main() i64 { const b = %B{}; b.missing(); 0 }\n";
    let err = calque::compile_source("moderr.calq", bad).expect_err("no associated fn");
    assert!(
        err.contains("no associated function `missing` for `B`"),
        "{err}"
    );

    // D-130: a lexical alias to a dotted module; the collision check.
    let alias_case = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../goldens/calque/module_alias/case.calq"
    );
    let asrc = std::fs::read_to_string(alias_case).expect("the alias case exists");
    let aart = calque::compile_source("module_alias.calq", &asrc).expect("calq0 compiles it");
    let aout = run_artifact(&aart).expect("it runs");
    assert_eq!(aout.output, "3\n3\n");

    let collide = "alias A.B;\nalias C.B;\npub fn main() i64 { 0 }\n";
    let cerr = calque::compile_source("aliasc.calq", collide).expect_err("alias collision");
    assert!(cerr.contains("alias collision"), "{cerr}");
}

#[test]
fn calq0_collection_receiver_methods() {
    // D-137: items.push/len/get/set resolve through the collection's
    // associated functions; the mutating ones keep the vector.*
    // in-place semantics.
    let case_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../goldens/calque/vec_methods/case.calq"
    );
    let src = std::fs::read_to_string(case_path).expect("the methods case exists");
    let artifact = calque::compile_source("vec_methods.calq", &src).expect("calq0 compiles it");
    let outcome = run_artifact(&artifact).expect("it runs");
    assert_eq!(outcome.output, "3|4|9|6\n");

    // an unknown method names the available ones
    let bad = "pub fn main() i64 {\n    var xs: [i64] = [];\n    xs.frob(1);\n    0\n}\n";
    let err = calque::compile_source("vm_bad.calq", bad).expect_err("unknown method");
    assert!(err.contains("no collection method `frob`"), "{err}");
}

#[test]
fn calq0_comptime_evaluates_and_reifies() {
    // D-134: the block runs at compile time through the evaluation
    // service; the result reifies as a constant. The reified constant
    // is asserted in the artifact (the feature's output), and the
    // runtime-local rejection is asserted as the required-comptime
    // diagnostic.
    let case_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../goldens/calque/comptime/case.calq"
    );
    let src = std::fs::read_to_string(case_path).expect("the comptime case exists");
    let artifact = calque::compile_source("comptime.calq", &src).expect("calq0 compiles it");
    assert!(
        artifact.contains("arith.constant 55 : i64"),
        "the comptime result must be reified into the artifact"
    );
    let outcome = run_artifact(&artifact).expect("it runs");
    assert_eq!(outcome.output, "55\n");

    // a runtime local inside a comptime block is refused loudly
    let bad = "pub fn main() i64 {\n    const y: i64 = 3;\n    const x: i64 = comptime { y + 1 };\n    x\n}\n";
    let err = calque::compile_source("ctrej.calq", bad).expect_err("runtime dependency");
    assert!(
        err.contains("compile-time-computable"),
        "{err}"
    );

    // D-139: assert.comptime - the false path surfaces the custom
    // message through the evaluation service.
    let afail = "pub fn main() i64 {\n    const x: i64 = comptime {\n        assert.comptime(1 == 2, \"one is not two\");\n        0\n    };\n    x\n}\n";
    let aerr = calque::compile_source("ctafail.calq", afail).expect_err("assert fails");
    assert!(aerr.contains("one is not two"), "{aerr}");

    // D-136: a STRING result reifies as a lit constant.
    let ssrc = "pub fn main() i64 {\n    const s: Str = comptime { str.concat(\"com\", \"ptime\") };\n    io.print(s);\n    io.print(\"\\n\");\n    0\n}\n";
    let sart = calque::compile_source("ctstr.calq", ssrc).expect("calq0 compiles it");
    assert!(
        sart.contains("text = \"comptime\""),
        "the comptime string must reify as a lit constant"
    );
    let sout = run_artifact(&sart).expect("it runs");
    assert_eq!(sout.output, "comptime\n");
}

#[test]
fn calq0_selective_imports() {
    // D-131: `import m.{quad};` brings a pub member into the bare
    // scope; ambiguity is a diagnosed collision; a non-pub member is
    // refused. calq0-only (post-freeze surface).
    let compiler = calque::canonical_compiler().expect("the chain builds it");
    let m = calque::UnitFile {
        name: "m.calq".to_string(),
        ns: Some("m".to_string()),
        text: "pub fn quad(x: i64) i64 { x * x }\nfn hidden(x: i64) i64 { x }\n".to_string(),
    };
    let n = calque::UnitFile {
        name: "n.calq".to_string(),
        ns: Some("n".to_string()),
        text: "pub fn quad(x: i64) i64 { x + 1 }\n".to_string(),
    };
    let main_ok = calque::UnitFile {
        name: "main.calq".to_string(),
        ns: None,
        text: "import m.{quad};\npub fn main() i64 { io.print(str.fromI64(quad(5))); io.print(\"\\n\"); 0 }\n".to_string(),
    };
    let files = vec![stdlib_str(), m.clone(), n.clone(), main_ok];
    let compiled = run_artifact_with_input_cached(compiler, &files).expect("it compiles");
    let outcome = run_artifact(&compiled.output).expect("the program runs");
    assert_eq!(outcome.output, "25\n", "{}", outcome.output);

    // ambiguity between two imports is a collision, not an order pick
    let main_amb = calque::UnitFile {
        name: "main.calq".to_string(),
        ns: None,
        text: "import m.{quad};\nimport n.{quad};\npub fn main() i64 { quad(5) }\n".to_string(),
    };
    let files2 = vec![stdlib_str(), m.clone(), n.clone(), main_amb];
    let out2 = run_artifact_with_input_cached(compiler, &files2).expect("runs");
    assert!(out2.output.contains("import collision"), "{}", out2.output);

    // a non-pub member is refused
    let main_hid = calque::UnitFile {
        name: "main.calq".to_string(),
        ns: None,
        text: "import m.{hidden};\npub fn main() i64 { hidden(5) }\n".to_string(),
    };
    let files3 = vec![stdlib_str(), m.clone(), main_hid];
    let out3 = run_artifact_with_input_cached(compiler, &files3).expect("runs");
    assert!(out3.output.contains("is not pub"), "{}", out3.output);
}

#[test]
fn chain_compiler_agrees_with_the_direct_build() {
    std::thread::Builder::new()
        .stack_size(frk_interp::STACK_SIZE)
        .spawn(chain_compiler_agrees_body)
        .expect("spawn")
        .join()
        .expect("join");
}

fn chain_compiler_agrees_body() {
    // D-123: the recorded chain (frozen seed -> c3v1 -> current
    // sources) must be a WORKING compiler that agrees with the direct
    // seed build at the only level byte-equality is promised: the
    // artifacts it produces for the same program. (Seed-built and
    // calq0-built compilers differ in incidental identifiers; §5's
    // fixed point is calq0-stability, not seed equality.)
    // The migration-gate shape: the c3v1 compiler (the pre-migration
    // state the seed can build) and the current chain-built compiler
    // must produce byte-identical artifacts for the same probe - any
    // behavior change the migration introduced shows up here.
    let snapshot_bin = calque::chain_fold_to(&calque::newest_snapshot())
        .expect("the newest snapshot's compiler builds via the fold");
    let current = calque::canonical_compiler().expect("the chain builds the current compiler");
    let units = vec![
        stdlib_str(),
        unit("probe.calq", None, "pub fn main() i64 { io.print(str.fromI64(6 * 7)); 0 }\n"),
    ];
    let via_snapshot = run_artifact_with_input_cached(&snapshot_bin, &units)
        .expect("the snapshot compiler runs")
        .output;
    let via_current = run_artifact_with_input_cached(current, &units)
        .expect("the current compiler runs")
        .output;
    assert_eq!(
        via_current,
        via_snapshot,
        "the current compiler's output drifted from the c3v1 compiler's: {}",
        diff_hint(&via_current, &via_snapshot)
    );
}

#[test]
fn calq0_indexed_writes_are_persistent() {
    // D-120 slice 1, the acceptance example: a write through one
    // binding leaves another binding's vector intact. calq0-only (the
    // frozen seed predates the surface).
    let case_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../goldens/calque/idx_persist/case.calq"
    );
    let src = std::fs::read_to_string(case_path).expect("the acceptance case exists");
    let artifact = calque::compile_source("idx_persist.calq", &src).expect("calq0 compiles it");
    let outcome = run_artifact(&artifact).expect("it runs");
    assert_eq!(outcome.output, "20|99|35|30\n");
}

#[test]
fn calq0_matches_the_seed_on_visibility() {
    // D-118: bare own-unit calls in a namespaced unit resolve; a
    // dotted member needs an import and pub, with the seed's texts.
    let artifact = compile_artifact();

    // vis1: bare own-call + imported pub member: runs.
    let files = vec![
        stdlib_str(),
        unit("m.calq", Some("m"), "fn helper() i64 { 7 }\npub fn mf() i64 { helper() + 1 }\n"),
        unit(
            "main.calq",
            None,
            "import m;\npub fn main() i64 { io.print(str.fromI64(m.mf())); io.print(\"\\n\"); 0 }\n",
        ),
    ];
    let expected = seed_translate_cached(&files).expect("the seed emits the bare-own set");
    let got = run_artifact_with_input_cached(artifact, &files).expect("calq0 compiles the bare-own set");
    assert_eq!(
        got.output,
        expected,
        "calq0's emission must equal the seed's (bare own-call): {}",
        diff_hint(&got.output, &expected)
    );
    let emitted = run_artifact(&got.output).expect("the emitted bare-own set runs");
    assert_eq!(emitted.output, "8\n");

    // vis2: a dotted member of a unit that was not imported.
    let files2 = vec![
        stdlib_str(),
        unit("m.calq", Some("m"), "pub fn mf() i64 { 7 }\n"),
        unit(
            "main.calq",
            None,
            "pub fn main() i64 { io.print(str.fromI64(m.mf())); io.print(\"\\n\"); 0 }\n",
        ),
    ];
    let mid = run_artifact_with_input_cached(artifact, &files2).expect("calq0 runs on vis2");
    assert!(
        mid.output.contains("module `m` is not imported (add `import m;`)"),
        "{}",
        mid.output
    );

    // vis3: imported but not pub.
    let files3 = vec![
        stdlib_str(),
        unit("m.calq", Some("m"), "fn hidden() i64 { 9 }\n"),
        unit(
            "main.calq",
            None,
            "import m;\npub fn main() i64 { io.print(str.fromI64(m.hidden())); io.print(\"\\n\"); 0 }\n",
        ),
    ];
    let mid3 = run_artifact_with_input_cached(artifact, &files3).expect("calq0 runs on vis3");
    assert!(mid3.output.contains("`m.hidden` is not pub"), "{}", mid3.output);
}

#[test]
fn calq0_matches_the_seed_on_imports() {
    // D-116's differential: the clean set is byte-equal and runs; the
    // cycle and the unknown name are refused with the seed's texts.
    let files = vec![
        stdlib_str(),
        unit("a.calq", Some("a"), "import b;\npub fn fa(x: i64) i64 { b.fb(x) + 1 }\n"),
        unit("b.calq", Some("b"), "pub fn fb(x: i64) i64 { x * 2 }\n"),
        unit(
            "main.calq",
            None,
            "import a;\npub fn main() i64 { io.print(str.fromI64(a.fa(20))); io.print(\"\\n\"); 0 }\n",
        ),
    ];
    let artifact = compile_artifact();
    let expected = seed_translate_cached(&files).expect("the seed emits the import set");
    let got = run_artifact_with_input_cached(artifact, &files).expect("calq0 compiles the import set");
    assert_eq!(
        got.output,
        expected,
        "calq0's emission must equal the seed's (imports): {}",
        diff_hint(&got.output, &expected)
    );
    let emitted = run_artifact(&got.output).expect("the emitted import set runs");
    assert_eq!(emitted.output, "41\n");

    let cyc = vec![
        stdlib_str(),
        unit("a.calq", Some("a"), "import b;\npub fn fa(x: i64) i64 { x }\n"),
        unit("b.calq", Some("b"), "import a;\npub fn fb(x: i64) i64 { x }\n"),
    ];
    let cg = run_artifact_with_input_cached(artifact, &cyc).expect("calq0 runs on the cycle set");
    assert!(cg.output.contains("import cycle detected"), "{}", cg.output);
    let unk = vec![
        stdlib_str(),
        unit("a.calq", Some("a"), "import nope;\npub fn fa(x: i64) i64 { x }\n"),
    ];
    let ug = run_artifact_with_input_cached(artifact, &unk).expect("calq0 runs on the unknown set");
    assert!(ug.output.contains("unknown import `nope`"), "{}", ug.output);
}

#[test]
fn calq0_compiles_itself() {
    // The C1 self-compilation gate, restated for the chain era
    // (D-124): calq0 (the chain's product for the current sources)
    // compiles its OWN sources, and the result must be byte-identical
    // to calq0 itself - the calq0 -> calq0 fixed point. The seed-built
    // leg no longer exists (the seed's surface closed at D-124); the
    // recorded chain in `make calque-bootstrap` carries the seed -> ...
    // -> calq0 history instead.
    let built = compile_artifact();
    let files = compiler_files();
    let got = compile_via_calq0_cached("self", built, &files);
    assert_eq!(
        got.len(),
        built.len(),
        "the self-translation must have calq0's length; head: {}",
        got.chars().take(160).collect::<String>()
    );
    assert_eq!(
        got,
        *built,
        "calq0 -> calq0 must reproduce calq0 byte-exactly: {}",
        diff_hint(&got, built)
    );

    // Round 2: the SELF-PRODUCED compiler is a working compiler - it
    // compiles fib.calq byte-equal to the seed, and the result runs.
    let fib: Vec<UnitFile> = vec![
        stdlib_str(),
        unit(
            "fib.calq",
            None,
            include_str!("../../../specimens/calque/examples/fib.calq"),
        ),
    ];
    let expected = seed_translate_cached(&fib).expect("the seed emits fib");
    let got2 = run_artifact_with_input_cached(&got, &fib).expect("the self-produced calq0 compiles fib");
    assert_eq!(
        got2.output,
        expected,
        "the self-produced compiler must emit fib byte-equal: {}",
        diff_hint(&got2.output, &expected)
    );
    let emitted = run_artifact(&got2.output).expect("the fib artifact runs");
    assert_eq!(emitted.output, "55\n");
}

#[test]
fn calq0_compiles_the_corpus_examples() {
    // Slice 3c: the independent corpus - every example that the seed's
    // b0 subset admits, compiled by BOTH compilers and executed.
    // identity_pair.calq (comptime generics) and active_score.calq
    // (pipelines + closures) are C2 frontier and stay fenced.
    let artifact = compile_artifact();
    let cases: &[(&str, &str, &str)] = &[
        (
            "fib.calq",
            include_str!("../../../specimens/calque/examples/fib.calq"),
            "55\n",
        ),
        (
            "squares.calq",
            include_str!("../../../specimens/calque/examples/squares.calq"),
            "385\n",
        ),
        (
            "user_badge.calq",
            include_str!("../../../specimens/calque/examples/user_badge.calq"),
            "Hello, Matt\nPerfect, Matt\n",
        ),
    ];
    for (name, src, want) in cases {
        let files = vec![stdlib_str(), unit(name, None, src)];
        let expected = seed_translate_cached(&files).expect("the seed emits the example");
        let got = run_artifact_with_input_cached(artifact, &files).expect("calq0 compiles the example");
        assert_eq!(
            got.output,
            expected,
            "calq0's emission must equal the seed's ({name}): {}",
            diff_hint(&got.output, &expected)
        );
        let emitted = run_artifact(&got.output).expect("the emitted example runs");
        assert_eq!(emitted.output, *want, "{name} must print its hand-derived output");
    }
}

#[test]
fn seed_translates_the_compiler_set() {
    // The seed compiles the NEWEST SNAPSHOT set through the JIT path
    // (cached like the self-gate): a fast smoke of the one job the
    // frozen seed still has after D-124 - the current sources ride the
    // chain (canonical_compiler), never the seed. The seed's failure
    // mode on post-freeze surface is a runtime trap the JIT binds
    // REAL, so asking it for the current set would abort the process.
    let seed_dir = calque::newest_seed_buildable().expect("a seed-buildable snapshot exists");
    let files = calque::snapshot_sources(&seed_dir).expect("the seed-buildable snapshot reads");
    let seed_src = include_str!("../../../tools/calque/bootstrap/seed.lua");
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    seed_src.hash(&mut hasher);
    for f in &files {
        f.name.hash(&mut hasher);
        f.ns.hash(&mut hasher);
        f.text.hash(&mut hasher);
    }
    let key = format!("calque-artifacts/setcheck-{:016x}.mlir", hasher.finish());
    let path = std::path::Path::new("../../target").join(&key);
    if let Ok(cached) = std::fs::read_to_string(&path) {
        println!("SET OK {} (cached)", cached.len());
        return;
    }
    println!("SET COMPUTING");
    let result = std::thread::Builder::new()
        .stack_size(frk_interp::STACK_SIZE)
        .spawn(move || calque::seed_translate_jit(&files))
        .expect("spawn")
        .join()
        .expect("join");
    match result {
        Ok(a) => {
            let _ = std::fs::create_dir_all(path.parent().unwrap());
            let _ = std::fs::write(&path, &a);
            println!("SET OK {}", a.len());
        }
        Err(e) => println!("SET ERR: {e}"),
    }
}

#[test]
fn calq0_comptime_flushes_lifted_closures() {
    let source = "pub fn answer() i64 {
    const base: i64 = 40;
    const add: fn (i64) i64 = fn (n: i64) i64 { base + n };
    add(2)
}
pub fn main() i64 {
    const value: i64 = comptime { answer() };
    io.print(str.fromI64(value));
    io.print(\"\\n\");
    0
}
";
    let artifact = calque::compile_source("ct_closure.calq", source)
        .expect("comptime sub-modules include their lifted closure definitions");
    let outcome = run_artifact(&artifact).expect("the reified result runs");
    assert_eq!(outcome.output, "42\n");
}

// D-142 (modules-access section 6): the emitter's owning operations
// live in `module Emitter`, its helpers are unit-private, and the
// match join state is scalar fields updated through an explicit
// `mut c: MatchCtx`. Behavior: nested value-joining matches, a match
// whose arms all return (no join block), and the arm-type mismatch
// diagnostic all flow through the migrated join state.
#[test]
fn calq0_emitter_domain_model() {
    let source = "pub fn pick(x: i64) i64 {
    match x {
        0 => { return 7; },
        _ => { return x * 2; },
    };
}
pub fn label(x: i64, y: i64) Str {
    const inner: Str = match y {
        0 => \"zero\",
        _ => \"some\",
    };
    match x {
        1 => str.concat(\"one-\", inner),
        2 => match y {
            0 => \"two-zero\",
            _ => str.concat(\"two-\", inner),
        },
        _ => \"other\",
    }
}
pub fn main() i64 {
    io.print(label(1, 0));
    io.print(\"|\");
    io.print(label(2, 5));
    io.print(\"|\");
    io.print(label(2, 0));
    io.print(\"|\");
    io.print(str.fromI64(pick(0) + pick(4)));
    io.print(\"\\n\");
    0
}
";
    let artifact = calque::compile_source("emitter_model.calq", source)
        .expect("nested matches compile through the scalar join state");
    let outcome = run_artifact(&artifact).expect("runs");
    assert_eq!(outcome.output, "one-zero|two-some|two-zero|15\n");

    let mismatch = "pub fn main() i64 {
    const x: i64 = 1;
    const v: i64 = match x {
        1 => 2,
        _ => \"no\",
    };
    v
}
";
    let err = calque::compile_source("arm_mismatch.calq", mismatch)
        .expect_err("arms of different types are refused");
    assert!(err.contains("match arms produce different types"), "{err}");

    // The source contract: no top-level `pub fn` outside the module
    // (the unit's public API is the Emitter module), and MatchCtx
    // carries no collection-typed cells.
    let emit = include_str!("../../../tools/calque/compiler/emit.calq");
    let module_start = emit.find("pub module Emitter {").expect("the Emitter module");
    let module_end = module_start + emit[module_start..].find("\n}\n").expect("module end");
    for (i, line) in emit.lines().enumerate() {
        if line.starts_with("pub fn ") {
            panic!("emit.calq:{}: top-level helper is pub: {line}", i + 1);
        }
    }
    assert!(emit[module_start..module_end].contains("pub fn emit_program(mut self: Self, a: Ast)"));
    let mc = emit.find("const MatchCtx = struct {").expect("MatchCtx");
    let mc_body = &emit[mc..mc + emit[mc..].find("};").unwrap()];
    assert!(!mc_body.contains(": ["), "MatchCtx must not hold collection cells: {mc_body}");
}

// D-143 (D-139's defect): a comptime expression in statement position
// or as a const/var initializer INSIDE a comptime block. The parser
// admits `comptime` as a statement head; the entry's result type is
// inferred with the block's own locals in scope. Failures keep each
// enclosing comptime's position, and runtime locals stay refused.
#[test]
fn calq0_nested_comptime_statements() {
    let source = include_str!("../../../goldens/calque/comptime_nested_stmt/case.calq");
    let artifact = calque::compile_source("comptime_nested_stmt.calq", source)
        .expect("nested comptime statements and initializers compile");
    assert!(artifact.contains("arith.constant 41 : i64"), "the outer result is reified");
    assert!(artifact.contains("text = \"comptime\""), "the Str result is reified");
    let outcome = run_artifact(&artifact).expect("runs");
    assert_eq!(outcome.output, "41|comptime|46\n");

    let failing = "pub fn main() i64 {
    const x: i64 = comptime {
        const a: i64 = comptime {
            assert.comptime(1 == 2, \"inner boom\");
            1
        };
        a
    };
    x
}
";
    let err = calque::compile_source("nested_fail.calq", failing).expect_err("the inner assertion fails");
    assert!(err.contains("inner boom"), "{err}");

    let runtime = "pub fn main() i64 {
    const r: i64 = 5;
    const x: i64 = comptime {
        comptime { r };
        1
    };
    x
}
";
    let err = calque::compile_source("nested_runtime.calq", runtime).expect_err("a runtime local is refused");
    assert!(err.contains("error 3:20 the comptime block must be compile-time-computable (error 4:9 the comptime block must be compile-time-computable"), "{err}");
    assert!(err.contains("unknown name `r`"), "{err}");
}

// D-144: comptime VALUE parameters. Each distinct typed constant is
// its own instance (typed keys in the symbol), arguments are literals
// or evaluated through the shared service, and the refusals are
// positioned.
#[test]
fn calq0_comptime_value_parameters() {
    let source = include_str!("../../../goldens/calque/comptime_values/case.calq");
    let artifact = calque::compile_source("comptime_values.calq", source)
        .expect("comptime value parameters compile");
    for sym in ["@tag__ci41", "@tag__ci16", "@scale__ci3", "@pick__cb1", "@pick__cb0", "@greet__cscalque", "@chain__ci5", "@folded__ci4"] {
        assert!(artifact.contains(&format!("func.func {sym}(")), "missing instance {sym}");
    }
    let outcome = run_artifact(&artifact).expect("runs");
    assert_eq!(outcome.output, "42|18|7007|hi calque|17|16|40\n");

    // Str constants mangle exactly: distinct texts never share a key.
    let strs = "fn g(comptime S: Str) i64 { str.len(S) }
pub fn main() i64 { io.print(str.fromI64(g(\"a b_c\") + g(\"a_b c\"))); io.print(\"\\n\"); 0 }
";
    let artifact = calque::compile_source("ct_strs.calq", strs).expect("compiles");
    assert!(artifact.contains("@g__csa_20b_5Fc("), "{artifact}");
    assert!(artifact.contains("@g__csa_5Fb_20c("), "{artifact}");

    let cases = [
        (
            "fn tag(comptime K: i64) i64 { K + 1 }\npub fn main() i64 { const r: i64 = 3; tag(r) }\n",
            "error 2:42 the comptime argument for `K` must be compile-time-computable",
        ),
        (
            "fn tag(comptime K: i64) i64 { K + 1 }\npub fn main() i64 { tag(\"x\") }\n",
            "error 2:24 the comptime argument for `K` is Str, expected i64",
        ),
        (
            "fn tag(x: i64, comptime K: i64) i64 { K + x }\npub fn main() i64 { tag(1, 2) }\n",
            "error 1:25 a comptime value parameter must precede the runtime parameters",
        ),
        (
            "fn tag(comptime K: i64) i64 { K + 1 }\npub fn main() i64 { tag() }\n",
            "error 2:24 missing the comptime argument for `K`",
        ),
        (
            "const P = struct { x: i64 };\npub fn main() i64 { const p: P = comptime { %P{x: 1} }; 0 }\n",
            "error 2:34 the comptime block must produce i64, bool, or Str (aggregate reification is not implemented)",
        ),
    ];
    for (src, want) in cases {
        let err = calque::compile_source("ct_neg.calq", src).expect_err(want);
        assert!(err.contains(want), "want {want:?}, got {err}");
    }
}

// D-145: conditional type construction. Type functions elaborate in
// the Calque emitter; conditions run through the shared service; only
// the taken branch elaborates; equal applications share one identity;
// cycles, unbounded construction, and misuse are refused.
#[test]
fn calq0_conditional_type_construction() {
    let source = include_str!("../../../goldens/calque/cond_type/case.calq");
    let artifact = calque::compile_source("cond_type.calq", source)
        .expect("type functions elaborate");
    assert!(!artifact.contains("@Choose"), "type functions are compile-time only");
    assert!(artifact.contains("func.func @describe(%env: !frk_closure.envref, %x: !frk_bstr.str) -> !frk_bstr.str"), "Choose(true) elaborated to Str in the signature");
    let outcome = run_artifact(&artifact).expect("runs");
    assert_eq!(outcome.output, "<wide>|50|10|12|nest|11\n");

    let cases = [
        ("fn Loop(comptime N: i64) type { Loop(N) }\npub fn main() i64 { const x: Loop(1) = 1; x }\n",
         "error 1:33 type-construction cycle through `Loop__ci1`"),
        ("fn Inf(comptime N: i64) type { Inf(N + 1) }\npub fn main() i64 { const x: Inf(0) = 1; x }\n",
         "type construction depth exceeds 64 at `Inf__ci64`"),
        ("fn notty(x: i64) i64 { x }\npub fn main() i64 { const x: notty(1) = 1; x }\n",
         "error 2:30 `notty` is not a type function (it must return `type`)"),
        ("fn C(comptime B: bool) type { if B { i64 } else { Str } }\npub fn main() i64 { const x: C(true, 1) = 1; x }\n",
         "error 2:30 type function `C` takes 1 argument(s), got 2"),
        ("fn C(comptime B: bool) type { if B { i64 } else { Str } }\npub fn main() i64 { const r: bool = true; const x: C(r) = 1; x }\n",
         "error 2:52 the comptime argument for `B` must be compile-time-computable"),
        ("fn C(comptime B: bool) type { if B { i64 } else { Str } }\npub fn main() i64 { const x: i64 = C(true); x }\n",
         "error 2:37 `C` is a type function; apply it in a type position"),
        ("fn C(comptime N: i64) type { if N { i64 } else { Str } }\npub fn main() i64 { const x: C(1) = 1; x }\n",
         "error 1:30 a type-level `if` condition must be a comptime bool"),
        ("fn grow(comptime N: i64) i64 { grow(N + 1) }\npub fn main() i64 { grow(0) }\n",
         "specialization depth exceeds 64 at `grow__ci64`"),
    ];
    for (src, want) in cases {
        let err = calque::compile_source("ct_type_neg.calq", src).expect_err(want);
        assert!(err.contains(want), "want {want:?}, got {err}");
    }
}

// D-146: compile-time reflection - fields, variants, signatures, and
// canonical type identities as reified data, also inside generic
// instances and comptime blocks; out-of-range and misapplied queries
// are positioned refusals.
#[test]
fn calq0_reflection() {
    let source = include_str!("../../../goldens/calque/reflect/case.calq");
    let artifact = calque::compile_source("reflect.calq", source).expect("reflection compiles");
    assert!(artifact.contains("@field_list__Point("), "the generic instance reflects over its T");
    assert!(!artifact.contains("reflect"), "no runtime reflection calls remain");
    let outcome = run_artifact(&artifact).expect("runs");
    assert_eq!(outcome.output, "3 label:[i64]|3 Named:Point dot-bare|2 scale:Shape->?i64|union,struct|Point{x,label}|30\n");

    let cases = [
        ("const P = struct { x: i64 };\npub fn main() i64 { io.print(reflect.field_name(P, 3)); 0 }\n",
         "error 2:48 field index 3 is out of range for `P` (1)"),
        ("const P = struct { x: i64 };\npub fn main() i64 { reflect.variant_count(P) }\n",
         "error 2:42 reflect.variant_count needs a union type, got `P`"),
        ("pub fn main() i64 { reflect.frobnicate(i64) }\n",
         "error 1:39 unknown reflection query `reflect.frobnicate`"),
        ("fn id(comptime T: type, v: T) T { v }\npub fn main() i64 { reflect.param_count(id) }\n",
         "error 2:40 reflect.param_count needs a concrete signature; `id` has comptime parameters"),
        ("const P = struct { x: i64 };\npub fn main() i64 { const r: i64 = 0; io.print(reflect.field_name(P, r)); 0 }\n",
         "error 2:66 the comptime argument for `index` must be compile-time-computable"),
    ];
    for (src, want) in cases {
        let err = calque::compile_source("reflect_neg.calq", src).expect_err(want);
        assert!(err.contains(want), "want {want:?}, got {err}");
    }
}

// D-147: type functions returning anonymous structs (the goal's
// `Pair(T)`): one nominal type per application key, constructible,
// usable as a type argument (including inside generic instances), and
// visible to reflection; recursion through an application is a cycle.
#[test]
fn calq0_type_functions_build_structs() {
    let source = include_str!("../../../goldens/calque/type_struct/case.calq");
    let artifact = calque::compile_source("type_struct.calq", source).expect("anonymous structs elaborate");
    assert!(artifact.contains("@identity__Pair__i64("), "Pair(i64) is one nominal type in instance keys");
    assert!(artifact.contains("@swap__str("), "the generic instance constructs %Pair(T)");
    let outcome = run_artifact(&artifact).expect("runs");
    assert_eq!(outcome.output, "7|ba|wide7|Pair__i64 21 Str\n");

    let cases = [
        ("fn C(comptime B: bool) type { if B { Str } else { i64 } }\npub fn main() i64 { const x = %C(true){}; 0 }\n",
         "error 2:31 `C(...)` does not denote a struct type"),
        ("fn Node(comptime T: type) type { struct { v: T, next: ?Node(T) } }\npub fn main() i64 { const n: Node(i64) = %Node(i64){v: 1, next: .None}; 0 }\n",
         "error 1:56 type-construction cycle through `Node__i64`"),
    ];
    for (src, want) in cases {
        let err = calque::compile_source("type_struct_neg.calq", src).expect_err(want);
        assert!(err.contains(want), "want {want:?}, got {err}");
    }
}

// D-148: `comptime if` prunes elaboration (recursion over a comptime
// index terminates; an untaken branch may be ill-typed for this
// instance), `reflect.field`/`reflect.with` access fields by comptime
// index, a type name passed inside an instance resolves through the
// active substitution, and evaluation sub-modules include only the
// fns they reference.
#[test]
fn calq0_comptime_if_and_field_access() {
    let source = "const Point = struct {
    x: i64,
    y: i64,
};
fn cnt(comptime T: type, comptime I: i64) i64 {
    comptime if I == reflect.field_count(T) {
        0
    } else {
        1 + cnt(T, I + 1)
    }
}
fn fill(comptime T: type, comptime I: i64, acc: T) T {
    comptime if I == reflect.field_count(T) {
        acc
    } else {
        fill(T, I + 1, reflect.with(acc, I, reflect.field(acc, I) * 10))
    }
}
fn pick(comptime B: bool) i64 {
    comptime if B { 1 } else { str.len(42) }
}
pub fn main() i64 {
    const p: Point = fill(Point, 0, %Point{x: 3, y: 4});
    io.print(str.fromI64(cnt(Point, 0)));
    io.print(\"|\");
    io.print(str.fromI64(p.x + p.y));
    io.print(\"|\");
    io.print(str.fromI64(pick(true)));
    io.print(\"\\n\");
    0
}
";
    let artifact = calque::compile_source("ct_if.calq", source).expect("comptime if prunes");
    assert!(artifact.contains("@cnt__Point__ci2("), "the recursion stops at the field count");
    assert!(!artifact.contains("@cnt__Point__ci3("), "and does not instantiate past it");
    let outcome = run_artifact(&artifact).expect("runs");
    assert_eq!(outcome.output, "2|70|1\n");

    let cases = [
        ("pub fn main() i64 { comptime if 1 { 1 } else { 2 } }\n",
         "error 1:30 a `comptime if` condition must be a bool"),
        ("pub fn main() i64 { const r: bool = true; comptime if r { 1 } else { 2 } }\n",
         "error 1:52 the `comptime if` condition must be compile-time-computable"),
        ("const P = struct { x: i64 };\npub fn main() i64 { const p: P = %P{x: 1}; reflect.field(p, 2) }\n",
         "error 2:57 field index 2 is out of range for `P` (1)"),
        ("pub fn main() i64 { const v: i64 = 3; reflect.field(v, 0) }\n",
         "error 1:52 reflect.field needs a struct value, got `i64`"),
    ];
    for (src, want) in cases {
        let err = calque::compile_source("ct_if_neg.calq", src).expect_err(want);
        assert!(err.contains(want), "want {want:?}, got {err}");
    }
}

// D-149 (AC-9.11): the reflection-driven codec round-trips, refuses a
// malformed input, and survives a 60-value property sweep whose names
// are built from the format's own delimiters.
#[test]
fn calq0_reflection_codec_round_trips() {
    let source = include_str!("../../../goldens/calque/codec/case.calq");
    let artifact = calque::compile_source("codec.calq", source).expect("the codec compiles");
    for sym in ["@enc_from__User__ci0(", "@enc_from__User__ci4(", "@dec_from__User__ci4(", "@enc_value__str("] {
        assert!(artifact.contains(sym), "missing instance {sym}");
    }
    let outcome = run_artifact(&artifact).expect("runs");
    assert_eq!(
        outcome.output,
        "id=42;name=6:ada;=x;admin=t;karma=-7;\nid=42;name=6:ada;=x;admin=t;karma=-7;\nnone\nx=3;y=-4;\nroundtrip 60/60\n"
    );
}

// D-150: the evaluation service's resource contract - a deterministic
// step budget turns a non-terminating comptime block into a
// diagnostic (not a hung compiler), and nested evaluations stay
// within the nesting bound.
#[test]
fn calq0_comptime_budget_and_nesting() {
    let spin = "fn spin(n: i64) i64 {
    var i: i64 = n;
    while i >= 0 {
        i = i + 1;
    }
    i
}
pub fn main() i64 {
    const x: i64 = comptime { spin(1) };
    x
}
";
    let err = calque::compile_source("spin.calq", spin).expect_err("an unbounded comptime loop is cut off");
    assert!(err.contains("step budget exhausted after 10000000 operations"), "{err}");

    let nested = "fn d(comptime K: i64) i64 {
    comptime if K == 0 { 0 } else { comptime { d(K - 1) } + 1 }
}
pub fn main() i64 {
    io.print(str.fromI64(d(3)));
    io.print(\"\\n\");
    0
}
";
    let artifact = calque::compile_source("nest.calq", nested).expect("bounded nesting evaluates");
    assert_eq!(run_artifact(&artifact).expect("runs").output, "3\n");
}

// D-151 phase A: the compiler KNOWS the limited service entries (the
// emission case a later hop's compiler needs before the compiler's
// own source may call them); `comptime(steps: N)` parses and refuses
// until that hop; multi-digit literals parse (the parse_digits fix).
#[test]
fn calq0_limited_service_emission_and_digits() {
    let direct = "pub fn main() i64 {
    const m: Str = \"func.func @inner(%x: i64) -> i64 {\\n  %a = arith.muli %x, %x : i64\\n  return %a : i64\\n}\\n\";
    const v: i64 = __calque_eval_i64_limited(m, \"inner\", 7, 100);
    const s: Str = __calque_eval_s_limited(\"func.func @t(%x: i64) -> !frk_bstr.str {\\n  %s = \\\"frk_bstr.lit\\\"() {text = \\\"ok\\\"} : () -> !frk_bstr.str\\n  return %s : !frk_bstr.str\\n}\\n\", \"t\", 0, 0);
    io.print(str.fromI64(v));
    io.print(s);
    io.print(\"\\n\");
    0
}
";
    let artifact = calque::compile_source("limited.calq", direct).expect("the emission case exists");
    assert!(artifact.contains("func.call @__calque_eval_i64_limited("), "{artifact}");
    assert_eq!(run_artifact(&artifact).expect("runs").output, "49ok\n");

    let gated = "pub fn main() i64 {\n    const x: i64 = comptime(steps: 100) { 1 + 2 };\n    x\n}\n";
    let err = calque::compile_source("gated.calq", gated).expect_err("the surface waits for the hop");
    assert!(err.contains("error 2:20 explicit comptime limits activate with the c8v1 bootstrap hop"), "{err}");

    let digits = "pub fn main() i64 {
    const a: [12]i64 = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    io.print(str.fromI64(a[11]));
    io.print(\"\\n\");
    0
}
";
    let artifact = calque::compile_source("digits.calq", digits).expect("[12]i64 parses");
    assert_eq!(run_artifact(&artifact).expect("runs").output, "12\n");
}
