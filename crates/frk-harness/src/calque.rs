//! calque — the Calque workstream's seed-invocation services (M37,
//! D-087): unit assembly, the seed call, input injection, artifact
//! composition, and the run path. NO Calque frontend logic lives in
//! Rust (goal §4): the lexer, parser, and emitter are Calque source
//! under `tools/calque/`; this module only drives the SEED (D-086) and
//! composes/parses/verifies/runs artifacts.
//!
//! Home: frk-harness, because the golden/grid machinery must reach it
//! (the harness cannot depend on the `calque` crate — that crate
//! depends on the harness). `crates/calque` re-exports this surface.

use frk_interp::Value;

/// The artifact version marker an emitted module MUST carry as its
/// first line (BOOTSTRAP §2). Bumping it is a D-entry that must
/// preserve existing producers.
pub const ARTIFACT_MARKER: &str = "// calque-artifact v0";

/// One source file of a compilation unit. `ns` is the optional
/// namespace tag (D-094): namespaced declarations mangle as
/// `ns__name`; `None` is the main namespace.
#[derive(Clone, Debug)]
pub struct UnitFile {
    pub name: String,
    pub ns: Option<String>,
    pub text: String,
}

/// What a successful artifact run produced: the entry's values and the
/// captured output stream (`io.print` bytes).
#[derive(Debug)]
pub struct RunOutcome {
    pub values: Vec<Value>,
    pub output: String,
}

/// Calque's kernel-IR intrinsics module — the D-062 authoring-surface
/// pattern, Calque's turn (the fourth frontend after lua/scheme/ts).
pub fn intrinsics_source() -> &'static str {
    include_str!("../../calque/src/intrinsics.mlir")
}

/// Refuses an artifact whose version marker is missing or different
/// (BOOTSTRAP §2: the driver never guesses a version).
pub fn check_artifact_marker(artifact: &str) -> Result<(), String> {
    match artifact.lines().next() {
        Some(line) if line.trim_end() == ARTIFACT_MARKER => Ok(()),
        other => Err(format!(
            "artifact must begin with {ARTIFACT_MARKER:?}, got {:?}",
            other.unwrap_or("<empty artifact>")
        )),
    }
}

/// Composes a runnable module text from an artifact: intrinsics first
/// (the seed-module role), then the artifact's own functions.
pub fn compose_unit(artifact: &str) -> String {
    format!("{}\n{}\n", intrinsics_source(), artifact)
}

/// Composes a runnable module text from an artifact plus the input
/// injection (BOOTSTRAP §2.1): intrinsics first, the artifact's own
/// functions, then the compiler's "stdin".
pub fn compose_unit_with_input(artifact: &str, units: &[UnitFile]) -> String {
    format!(
        "{}\n{}\n{}\n",
        intrinsics_source(),
        artifact,
        input_injection(units)
    )
}

/// Escapes one byte-string into an MLIR string-attribute body. The
/// compiler's input channel is UNRESTRICTED (BOOTSTRAP §2.1) — unlike
/// the seed's ASCII-only one: any byte rides as `\XX` hex outside
/// printable ASCII.
fn mlir_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    for &b in text.as_bytes() {
        match b {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            0x20..=0x7E => out.push(b as char),
            _ => out.push_str(&format!("\\{b:02X}")),
        }
    }
    out
}

/// Generates the input injection (BOOTSTRAP §2.1): `__calque_input_count()`
/// and 0-based `__calque_input(i)` as kernel-IR definitions carrying the
/// unit texts as bstr literals. This is the compiler's "stdin"; it is the
/// ONLY difference between compiling a unit and compiling the compiler
/// itself.
pub fn input_injection(units: &[UnitFile]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "func.func @__calque_input_count() -> i64 {{\n  %n = arith.constant {} : i64\n  return %n : i64\n}}\n",
        units.len()
    ));
    out.push_str("func.func @__calque_input(%arg0: i64) -> !frk_bstr.str {\n");
    for (index, unit) in units.iter().enumerate() {
        out.push_str(&format!("  %c{index} = arith.constant {index} : i64\n"));
        out.push_str(&format!(
            "  %eq{index} = arith.cmpi eq, %arg0, %c{index} : i64\n"
        ));
        out.push_str(&format!(
            "  cf.cond_br %eq{index}, ^lit{index}, ^next{index}\n"
        ));
        out.push_str(&format!("^lit{index}:\n"));
        out.push_str(&format!(
            "  %s{index} = \"frk_bstr.lit\"() {{text = \"{}\"}} : () -> !frk_bstr.str\n",
            mlir_escape(&unit.text)
        ));
        out.push_str(&format!("  return %s{index} : !frk_bstr.str\n"));
        out.push_str(&format!("^next{index}:\n"));
    }
    out.push_str(
        "  %msg = \"frk_bstr.lit\"() {text = \"__calque_input: index out of range\"} : () -> !frk_bstr.str\n",
    );
    out.push_str("  func.call @frk_rt_calque_trap(%msg) : (!frk_bstr.str) -> ()\n");
    out.push_str(
        "  %empty = \"frk_bstr.lit\"() {text = \"\"} : () -> !frk_bstr.str\n  return %empty : !frk_bstr.str\n}\n",
    );
    // The NAMED channel: the unit's file name (the seed's CURRENT_FILE;
    // diagnostics carry it, so the artifact text depends on it).
    out.push_str("func.func @__calque_input_name(%arg0: i64) -> !frk_bstr.str {\n");
    for (index, unit) in units.iter().enumerate() {
        out.push_str(&format!("  %c{index} = arith.constant {index} : i64\n"));
        out.push_str(&format!(
            "  %eq{index} = arith.cmpi eq, %arg0, %c{index} : i64\n"
        ));
        out.push_str(&format!(
            "  cf.cond_br %eq{index}, ^lit{index}, ^next{index}\n"
        ));
        out.push_str(&format!("^lit{index}:\n"));
        out.push_str(&format!(
            "  %s{index} = \"frk_bstr.lit\"() {{text = \"{}\"}} : () -> !frk_bstr.str\n",
            mlir_escape(&unit.name)
        ));
        out.push_str(&format!("  return %s{index} : !frk_bstr.str\n"));
        out.push_str(&format!("^next{index}:\n"));
    }
    out.push_str(
        "  %msg2 = \"frk_bstr.lit\"() {text = \"__calque_input_name: index out of range\"} : () -> !frk_bstr.str\n",
    );
    out.push_str("  func.call @frk_rt_calque_trap(%msg2) : (!frk_bstr.str) -> ()\n");
    out.push_str(
        "  %empty2 = \"frk_bstr.lit\"() {text = \"\"} : () -> !frk_bstr.str\n  return %empty2 : !frk_bstr.str\n}\n",
    );
    // The NAMESPACE channel (D-094): the seed mangles namespaced decls
    // as ns__name, so the compiler must know each unit's ns.
    out.push_str("func.func @__calque_input_ns(%arg0: i64) -> !frk_bstr.str {\n");
    for (index, unit) in units.iter().enumerate() {
        out.push_str(&format!("  %c{index} = arith.constant {index} : i64\n"));
        out.push_str(&format!(
            "  %eq{index} = arith.cmpi eq, %arg0, %c{index} : i64\n"
        ));
        out.push_str(&format!(
            "  cf.cond_br %eq{index}, ^lit{index}, ^next{index}\n"
        ));
        out.push_str(&format!("^lit{index}:\n"));
        out.push_str(&format!(
            "  %s{index} = \"frk_bstr.lit\"() {{text = \"{}\"}} : () -> !frk_bstr.str\n",
            mlir_escape(unit.ns.as_deref().unwrap_or(""))
        ));
        out.push_str(&format!("  return %s{index} : !frk_bstr.str\n"));
        out.push_str(&format!("^next{index}:\n"));
    }
    out.push_str(
        "  %msg3 = \"frk_bstr.lit\"() {text = \"__calque_input_ns: index out of range\"} : () -> !frk_bstr.str\n",
    );
    out.push_str("  func.call @frk_rt_calque_trap(%msg3) : (!frk_bstr.str) -> ()\n");
    out.push_str(
        "  %empty3 = \"frk_bstr.lit\"() {text = \"\"} : () -> !frk_bstr.str\n  return %empty3 : !frk_bstr.str\n}\n",
    );
    out
}

/// Escapes one byte-string into a femto_lua string literal. The seed
/// input channel is ASCII-only (SPEC F18): bytes >= 0x80 are a LOUD
/// error here, never a silent mis-lex.
fn lua_literal(text: &str) -> Result<String, String> {
    let mut out = String::from("\"");
    for &b in text.as_bytes() {
        match b {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            b'\n' => out.push_str("\\n"),
            b'\t' => out.push_str("\\t"),
            b'\r' => out.push_str("\\r"),
            0x20..=0x7E => out.push(b as char),
            0x80.. => {
                return Err(format!(
                    "non-ASCII byte 0x{b:02X} in the seed input channel \
                     (ASCII-only per SPEC F18)"
                ));
            }
            _ => out.push_str(&format!("\\{b:03}")),
        }
    }
    out.push('"');
    Ok(out)
}

/// Runs Calque's bootstrap seed over a unit and returns the artifact
/// text (BOOTSTRAP §1). Seed failures are PROCESS failures: the seed
/// prints a `__calque_seed_error:` marker line before trapping, and
/// that marker — read from the captured output — is the diagnostic.
fn seed_source(files: &[UnitFile]) -> Result<String, String> {
    let mut source = String::from(include_str!("../../../tools/calque/bootstrap/seed.lua"));
    source.push_str("\nlocal __files = {\n");
    for file in files {
        source.push_str("  { name = ");
        source.push_str(&lua_literal(&file.name)?);
        source.push_str(", ns = ");
        match &file.ns {
            Some(ns) => source.push_str(&lua_literal(ns)?),
            None => source.push_str("nil"),
        }
        source.push_str(", text = ");
        source.push_str(&lua_literal(&file.text)?);
        source.push_str(" },\n");
    }
    source.push_str("}\nrun(__files)\n");
    Ok(source)
}

/// The seed translated through the JIT runners - the development
/// loop's fast path (the reference interpreter is ~1000x slower on the
/// seed's 50 KB parser input). The artifact is byte-identical to
/// `seed_translate`'s by the differential law (L3); the ignored
/// verify test asserts the two agree. Seed FAILURES must use
/// `seed_translate`: the seed's protocol ends in a runtime trap, which
/// the JIT binds REAL (stderr + abort), so a broken unit would kill
/// the process instead of yielding the marker diagnostic.
pub fn seed_translate_jit(files: &[UnitFile]) -> Result<String, String> {
    let source = seed_source(files)?;
    let context = frk_core::context();
    frk_dialects::register(&context).map_err(|error| format!("register: {error}"))?;
    let module = frk_front::lua::compile_lua(&context, "calque_seed.lua", &source)
        .map_err(|error| format!("seed compile: {error}"))?;
    let text = module.as_operation().to_string();
    let (output, _code) = crate::runner::jit_run_text(
        &context,
        &text,
        "main",
        frk_dialects::Strategy::Rc,
    )?;
    Ok(output)
}

pub fn seed_translate(files: &[UnitFile]) -> Result<String, String> {
    let source = seed_source(files)?;

    let context = frk_core::context();
    frk_dialects::register(&context).map_err(|error| format!("register: {error}"))?;
    let module = frk_front::lua::compile_lua(&context, "calque_seed.lua", &source)
        .map_err(|error| format!("seed compile: {error}"))?;
    frk_dialects::verify(&context, &module).map_err(|error| format!("seed verify: {error:?}"))?;
    let mut interp =
        frk_interp::Interp::new(&module).map_err(|error| format!("seed interp: {error}"))?;
    frk_dialects::register_eval(&mut interp);
    crate::runner::register_protocol_builtins(&mut interp);

    let result = interp.eval_function("main", &[]);
    let output = interp.take_output();
    if let Some(marker) = output
        .lines()
        .find(|line| line.starts_with("__calque_seed_error: "))
    {
        return Err(marker.to_string());
    }
    result.map_err(|error| format!("seed process failure: {error}"))?;
    Ok(output)
}

/// Parses, verifies, and interprets an artifact end to end (the
/// composed-module path every runner shares, BOOTSTRAP §2.1).
pub fn run_artifact(artifact: &str) -> Result<RunOutcome, String> {
    check_artifact_marker(artifact)?;
    run_composed(&compose_unit(artifact))
}

/// Runs an artifact with the input injection for `units` — the
/// compiler-as-a-program path (BOOTSTRAP §2.1/§3.2).
pub fn run_artifact_with_input(artifact: &str, units: &[UnitFile]) -> Result<RunOutcome, String> {
    check_artifact_marker(artifact)?;
    run_composed(&compose_unit_with_input(artifact, units))
}

/// Runs a compiled artifact's `test` blocks (C2, F33): every
/// `__calque_test_<N>` entry is evaluated on a fresh interpreter; a
/// trap marks that block failed. Returns (name, ok) in symbol order.
pub fn run_test_blocks(artifact: &str) -> Result<Vec<(String, bool)>, String> {
    check_artifact_marker(artifact)?;
    let composed = compose_unit(artifact);
    let context = frk_core::context();
    frk_dialects::register(&context).map_err(|error| format!("register: {error}"))?;
    let module = melior::ir::Module::parse(&context, &composed)
        .ok_or_else(|| "composed module failed to parse".to_string())?;
    frk_dialects::verify(&context, &module).map_err(|error| format!("verify: {error:?}"))?;
    let mut names: Vec<(String, String)> = Vec::new();
    let mut pending = String::new();
    for line in composed.lines() {
        if let Some(rest) = line.strip_prefix("// test: ") {
            pending = rest.trim().to_string();
        }
        if let Some(rest) = line.strip_prefix("func.func @__calque_test_entry_") {
            let suffix: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            let full = format!("__calque_test_entry_{suffix}");
            let label = if pending.is_empty() {
                full.clone()
            } else {
                pending.clone()
            };
            names.push((label, full));
            pending.clear();
        }
    }
    let mut results = Vec::new();
    for (label, sym) in names {
        let mut interp =
            frk_interp::Interp::new(&module).map_err(|error| format!("interp: {error}"))?;
        frk_dialects::register_eval(&mut interp);
        crate::runner::register_protocol_builtins(&mut interp);
        let ok = interp.eval_function(&sym, &[]).is_ok();
        results.push((label, ok));
    }
    Ok(results)
}

fn run_composed(text: &str) -> Result<RunOutcome, String> {
    let context = frk_core::context();
    frk_dialects::register(&context).map_err(|error| format!("register: {error}"))?;
    let module = melior::ir::Module::parse(&context, text)
        .ok_or_else(|| "composed module failed to parse".to_string())?;
    frk_dialects::verify(&context, &module).map_err(|error| format!("verify: {error:?}"))?;
    let mut interp =
        frk_interp::Interp::new(&module).map_err(|error| format!("interp: {error}"))?;
    frk_dialects::register_eval(&mut interp);
    crate::runner::register_protocol_builtins(&mut interp);
    let values = interp
        .eval_function("main", &[])
        .map_err(|error| error.to_string())?;
    Ok(RunOutcome {
        values,
        output: interp.take_output(),
    })
}


// ------------------------------------------------------------------
// The canonical compiler, as the golden harness's Calque front-end.

/// The Calque standard library, as source (one unit, namespace `str`).
/// The pull-sequence stdlib, as source (one unit, namespace `seq`).
pub fn seq_source() -> &'static str {
    include_str!("../../../tools/calque/std/seq.calq")
}

/// The Option helpers, as source (one unit, namespace `opt`).
pub fn opt_source() -> &'static str {
    include_str!("../../../tools/calque/std/opt.calq")
}

/// The Result helpers, as source (one unit, namespace `res`).
pub fn res_source() -> &'static str {
    include_str!("../../../tools/calque/std/res.calq")
}

pub fn stdlib_source() -> &'static str {
    include_str!("../../../tools/calque/std/str.calq")
}

/// Every source file of the canonical compiler (the same set the
/// bootstrap chain compiles): the stdlib plus the compiler's units.
pub fn compiler_sources() -> Vec<UnitFile> {
    vec![
        UnitFile {
            name: "tools/calque/std/str.calq".to_string(),
            ns: Some("str".to_string()),
            text: stdlib_source().to_string(),
        },
        UnitFile {
            name: "tools/calque/compiler/lex.calq".to_string(),
            ns: None,
            text: include_str!("../../../tools/calque/compiler/lex.calq").to_string(),
        },
        UnitFile {
            name: "tools/calque/compiler/report.calq".to_string(),
            ns: None,
            text: include_str!("../../../tools/calque/compiler/report.calq").to_string(),
        },
        UnitFile {
            name: "tools/calque/compiler/parse.calq".to_string(),
            ns: None,
            text: include_str!("../../../tools/calque/compiler/parse.calq").to_string(),
        },
        UnitFile {
            name: "tools/calque/compiler/emit.calq".to_string(),
            ns: None,
            text: include_str!("../../../tools/calque/compiler/emit.calq").to_string(),
        },
        UnitFile {
            name: "tools/calque/compiler/compile.calq".to_string(),
            ns: None,
            text: include_str!("../../../tools/calque/compiler/compile.calq").to_string(),
        },
    ]
}

fn workspace_target() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target")
        .join("calque-artifacts")
}

fn cache_key(seed: &str, base: &str, files: &[UnitFile]) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    seed.hash(&mut hasher);
    base.hash(&mut hasher);
    for f in files {
        f.name.hash(&mut hasher);
        f.ns.hash(&mut hasher);
        f.text.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

/// The canonical compiler (calq0): the SEED translates the compiler
/// sources; the result is cached in-process and on disk (target/
/// calque-artifacts), keyed by the seed and every source. Cold cost is
/// minutes (the reference interpreter); warm cost is a file read.
pub fn canonical_compiler() -> Result<&'static str, String> {
    static COMPILER: std::sync::OnceLock<Result<String, String>> = std::sync::OnceLock::new();
    COMPILER
        .get_or_init(|| {
            let seed = include_str!("../../../tools/calque/bootstrap/seed.lua");
            let files = compiler_sources();
            let key = cache_key(seed, "canonical-compiler", &files);
            let path = workspace_target().join(format!("canonical-{key}.mlir"));
            if let Ok(cached) = std::fs::read_to_string(&path) {
                return Ok(cached);
            }
            // The JIT path: seed-translating the compiler set is a
            // pure function and a success path; failures would use
            // seed_translate (the trap marker only the interpreter
            // surfaces).
            let artifact = seed_translate_jit(&files)?;
            let _ = std::fs::create_dir_all(path.parent().unwrap());
            let _ = std::fs::write(&path, &artifact);
            Ok(artifact)
        })
        .as_deref()
        .map_err(|e| e.clone())
}

/// Compiles one program (stdlib + the unit) with the CANONICAL compiler
/// - the harness's Calque front-end. Cached on disk under the same key
/// discipline as canonical_compiler.
pub fn compile_source(name: &str, source: &str) -> Result<String, String> {
    let compiler = canonical_compiler()?;
    let files = vec![
        UnitFile {
            name: "tools/calque/std/str.calq".to_string(),
            ns: Some("str".to_string()),
            text: stdlib_source().to_string(),
        },
        UnitFile {
            name: "tools/calque/std/seq.calq".to_string(),
            ns: Some("seq".to_string()),
            text: seq_source().to_string(),
        },
        UnitFile {
            name: "tools/calque/std/opt.calq".to_string(),
            ns: Some("opt".to_string()),
            text: opt_source().to_string(),
        },
        UnitFile {
            name: "tools/calque/std/res.calq".to_string(),
            ns: Some("res".to_string()),
            text: res_source().to_string(),
        },
        UnitFile {
            name: name.to_string(),
            ns: None,
            text: source.to_string(),
        },
    ];
    let seed = include_str!("../../../tools/calque/bootstrap/seed.lua");
    let key = cache_key(seed, compiler, &files);
    let path = workspace_target().join(format!("case-{key}.mlir"));
    if let Ok(cached) = std::fs::read_to_string(&path) {
        return Ok(cached);
    }
    let outcome = run_artifact_with_input(compiler, &files)?;
    let artifact = outcome.output;
    if !artifact.starts_with(ARTIFACT_MARKER) {
        return Err(format!(
            "compiler did not emit an artifact (head: {:?})",
            artifact.chars().take(160).collect::<String>()
        ));
    }
    let _ = std::fs::create_dir_all(path.parent().unwrap());
    let _ = std::fs::write(&path, &artifact);
    Ok(artifact)
}
