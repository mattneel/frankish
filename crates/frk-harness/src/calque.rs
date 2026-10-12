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
    let (output, _code) = crate::runner::jit_run_text_opt(
        &context,
        &text,
        "main",
        frk_dialects::Strategy::Rc,
        0,
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

/// JIT + disk-cached seed translation for arbitrary unit sets - the
/// fast path the differential tests and the bootstrap chain use. The
/// result is byte-identical to `seed_translate` (differential law L3,
/// asserted by the ignored verify test); success paths only - a seed
/// FAILURE must go through `seed_translate`, whose interpreter
/// surfaces the marker (the JIT binds the trap for real).
pub fn seed_translate_cached(files: &[UnitFile]) -> Result<String, String> {
    let seed = include_str!("../../../tools/calque/bootstrap/seed.lua");
    let key = cache_key(seed, "seed-set", files);
    let path = workspace_target().join(format!("seed-{key}.mlir"));
    if let Ok(cached) = std::fs::read_to_string(&path) {
        return Ok(cached);
    }
    let artifact = seed_translate_jit(files)?;
    let _ = std::fs::create_dir_all(path.parent().unwrap());
    let _ = std::fs::write(&path, &artifact);
    Ok(artifact)
}

/// JIT execution of an artifact with input injection - the fast path
/// for the compiler-as-a-program runs (calq0 compiling itself). The
/// outcome carries the entry i64 (what the JIT reports) and the
/// captured output; the run is deterministic, so the entry value and
/// output stream are cached on disk keyed by artifact + units.
pub fn run_artifact_with_input_cached(
    artifact: &str,
    units: &[UnitFile],
) -> Result<RunOutcome, String> {
    check_artifact_marker(artifact)?;
    // The composed module = intrinsics + artifact + injection, so the
    // intrinsics source belongs in the key.
    let key = cache_key(
        &format!("{artifact}\n---intrinsics---\n{}", intrinsics_source()),
        "run-in",
        units,
    );
    let path = workspace_target().join(format!("run-{key}.out"));
    let decode = |raw: &str| -> Result<RunOutcome, String> {
        let (code_line, output) = raw
            .split_once('\n')
            .ok_or_else(|| "run cache: bad header".to_string())?;
        let code: i64 = code_line
            .parse()
            .map_err(|e| format!("run cache: bad code: {e}"))?;
        Ok(RunOutcome {
            values: vec![frk_interp::Value::int(code as u64, 64)
                .map_err(|e| format!("run cache: {e:?}"))?],
            output: output.to_string(),
        })
    };
    if let Ok(cached) = std::fs::read_to_string(&path) {
        return decode(&cached);
    }
    let context = frk_core::context();
    frk_dialects::register(&context).map_err(|error| format!("register: {error}"))?;
    let composed = compose_unit_with_input(artifact, units);
    let (output, code) = crate::runner::jit_run_text_opt(
        &context,
        &composed,
        "main",
        frk_dialects::Strategy::Rc,
        0,
    )?;
    let _ = std::fs::create_dir_all(path.parent().unwrap());
    let _ = std::fs::write(&path, format!("{code}\n{output}"));
    Ok(RunOutcome {
        values: vec![frk_interp::Value::int(code as u64, 64).map_err(|e| format!("jit: {e:?}"))?],
        output,
    })
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

/// A workspace-root-relative path, resolved regardless of the
/// process's CWD (tests run from the crate directory, the CLI from
/// the repository root).
fn workspace_path(rel: &str) -> std::path::PathBuf {
    let p = std::path::Path::new(rel);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// One snapshot's unit set, from disk: std/str.calq (namespace
/// `str`) plus the five compiler units (no namespace, the v0 flat
/// model) - the same shape as `compiler_sources()`.
pub fn snapshot_sources(dir: &str) -> Result<Vec<UnitFile>, String> {
    let mut files = Vec::new();
    for (rel, ns) in [
        ("std/str.calq", Some("str")),
        ("compiler/lex.calq", None),
        ("compiler/report.calq", None),
        ("compiler/parse.calq", None),
        ("compiler/emit.calq", None),
        ("compiler/compile.calq", None),
    ] {
        let rel_path = format!("{dir}/{rel}");
        let text = std::fs::read_to_string(workspace_path(&rel_path))
            .map_err(|e| format!("snapshot: cannot read {rel_path}: {e}"))?;
        files.push(UnitFile {
            name: rel_path,
            ns: ns.map(|s| s.to_string()),
            text,
        });
    }
    Ok(files)
}

/// The recorded bootstrap chain (D-119/D-123): the FROZEN seed builds
/// the newest seed-buildable snapshot, and that compiler translates
/// the CURRENT sources. Used when the current sources have outgrown
/// the seed's surface (they then cannot be built directly).
pub fn snapshot_compiler(snapshot_dir: &str) -> Result<String, String> {
    let snap = snapshot_sources(snapshot_dir)?;
    let seed = include_str!("../../../tools/calque/bootstrap/seed.lua");
    let key0 = cache_key(seed, "snapshot-compiler", &snap);
    let path0 = workspace_target().join(format!("snap-{key0}.mlir"));
    match std::fs::read_to_string(&path0) {
        Ok(cached) => Ok(cached),
        Err(_) => {
            let a = seed_translate_jit(&snap)?;
            let _ = std::fs::create_dir_all(path0.parent().unwrap());
            let _ = std::fs::write(&path0, &a);
            Ok(a)
        }
    }
}

/// `chain_compiler`'s second stage alone: the snapshot compiler
/// translates `current`.
pub fn snapshot_translate(snapshot_dir: &str, current: &[UnitFile]) -> Result<String, String> {
    let artifact0 = snapshot_compiler(snapshot_dir)?;
    let outcome = run_artifact_with_input_cached(&artifact0, current)?;
    if outcome.output.contains("error ") && outcome.output.starts_with("error ") {
        return Err(format!(
            "chain: the snapshot compiler rejected the sources: {}",
            &outcome.output[..outcome.output.len().min(200)]
        ));
    }
    Ok(outcome.output)
}

pub fn chain_compiler(snapshot_dir: &str, current: &[UnitFile]) -> Result<String, String> {
    snapshot_translate(snapshot_dir, current)
}

/// The canonical compiler (calq0): the SEED translates the compiler
/// sources; the result is cached in-process and on disk (target/
/// calque-artifacts), keyed by the seed and every source. When the
/// current sources use surface the FROZEN seed cannot parse, the
/// recorded chain (seed -> newest snapshot -> current) takes over
/// (D-119's freeze discipline: growth rides calq0 + snapshots).
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
            // The canonical path is the recorded chain (D-123): the
            // frozen seed can no longer be ASKED to parse the current
            // sources - after D-122 they may use surface it lacks, and
            // its failure mode is a runtime trap the JIT binds REAL
            // (the seed_translate_jit note above). The seed must only
            // ever translate its own recorded snapshot.
            let chain = chain_compiler("tools/calque/bootstrap/snapshots/c3v1", &files)?;
            let _ = std::fs::create_dir_all(path.parent().unwrap());
            let _ = std::fs::write(&path, &chain);
            Ok(chain)
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
