//! calque — the host side of the Calque workstream (M37, D-087):
//! driver services over shared Frankish machinery. NO Calque frontend
//! logic lives here (goal §4's prohibition): the lexer, parser, and
//! emitter are Calque source under `tools/calque/`; this crate
//! assembles units, invokes the SEED (D-086), and composes, parses,
//! verifies, and runs artifacts.
//!
//! Composition contract (BOOTSTRAP §2.1): a runnable module is
//! `intrinsics.mlir` + the artifact text + input injection. v0 ships
//! the first two pieces and the marker check; input injection and the
//! CLI land with the driver slab.

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
    include_str!("intrinsics.mlir")
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
    let (output, _code) = frk_harness::runner::jit_run_text(
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
    frk_harness::runner::register_protocol_builtins(&mut interp);

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

fn run_composed(text: &str) -> Result<RunOutcome, String> {
    let context = frk_core::context();
    frk_dialects::register(&context).map_err(|error| format!("register: {error}"))?;
    let module = melior::ir::Module::parse(&context, text)
        .ok_or_else(|| "composed module failed to parse".to_string())?;
    frk_dialects::verify(&context, &module).map_err(|error| format!("verify: {error:?}"))?;
    let mut interp =
        frk_interp::Interp::new(&module).map_err(|error| format!("interp: {error}"))?;
    frk_dialects::register_eval(&mut interp);
    frk_harness::runner::register_protocol_builtins(&mut interp);
    let values = interp
        .eval_function("main", &[])
        .map_err(|error| error.to_string())?;
    Ok(RunOutcome {
        values,
        output: interp.take_output(),
    })
}
