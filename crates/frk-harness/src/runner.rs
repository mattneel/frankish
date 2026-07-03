//! Runners — the executable semantics a golden can be judged against
//! (SPEC §7.2). M1 ships `jit`; `interp` joins at M2 (and becomes the
//! reference semantics per law L3), `aot` at M7, specimen oracles with
//! their specimens.

use std::fmt;
use std::fs;

use melior::ExecutionEngine;
use melior::ir::Module;
use melior::ir::operation::OperationLike;

use crate::canon;
use crate::case::{Case, ResultKind, SourceKind};
use crate::pipeline;

/// A named way to execute a case and produce raw (pre-canonicalization)
/// output. Implementations must be deterministic under docs/canon.md.
pub trait Runner {
    fn name(&self) -> &'static str;
    fn run(&self, case: &Case) -> Result<String, RunError>;
    /// Whether this runner can execute this case at all (specimen
    /// oracles only speak their specimen's language). Combined with the
    /// case's `runners=` directive by the golden/diff engines.
    fn applicable(&self, case: &Case) -> bool {
        let _ = case;
        true
    }
}

#[derive(Debug)]
pub enum RunError {
    /// Harness-side defect (dialect registration, thread spawning) — not
    /// a property of the case.
    Setup(String),
    Io(String),
    Parse(String),
    Verify(String),
    Lower(String),
    Invoke(String),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Setup(m) => write!(f, "setup: {m}"),
            Self::Io(m) => write!(f, "io: {m}"),
            Self::Parse(m) => write!(f, "parse: {m}"),
            Self::Verify(m) => write!(f, "verify: {m}"),
            Self::Lower(m) => write!(f, "lower: {m}"),
            Self::Invoke(m) => write!(f, "invoke: {m}"),
        }
    }
}

impl std::error::Error for RunError {}

/// The shared front half of every runner: kernel-aware context, parse,
/// MLIR verify, frankish semantic verify (SPEC §3 K1 as amended by
/// D-031: semantic verification runs before ANY execution or lowering).
/// Returns the context; the caller re-parses into it (melior modules
/// borrow their context, so a helper can't return both).
fn frk_context(case: &Case) -> Result<(melior::Context, String), RunError> {
    let source = fs::read_to_string(&case.source_path)
        .map_err(|e| RunError::Io(format!("{}: {e}", case.source_path.display())))?;
    let context = frk_core::context();
    frk_dialects::register(&context)
        .map_err(|e| RunError::Setup(format!("kernel dialect registration: {e}")))?;
    Ok((context, source))
}

/// Runs the loanword producer (tools/loanword-ts via node) over a .ts
/// case and returns the canonical artifact text (D-047).
fn produce_loanword(case: &Case) -> Result<String, RunError> {
    let root = repo_root_from(&case.dir)?;
    let output = std::process::Command::new("node")
        .arg(root.join("tools/loanword-ts/src/main.ts"))
        .arg(&case.source_path)
        .env("LC_ALL", "C")
        .output()
        .map_err(|e| RunError::Invoke(format!("node (loanword producer): {e}")))?;
    if !output.status.success() {
        return Err(RunError::Parse(format!(
            "loanword-ts: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    String::from_utf8(output.stdout)
        .map_err(|_| RunError::Parse("non-UTF-8 loanword artifact".into()))
}

fn parse_and_verify<'c>(
    context: &'c melior::Context,
    source: &str,
    case: &Case,
) -> Result<Module<'c>, RunError> {
    let module = match case.kind {
        SourceKind::Mlir => Module::parse(context, source)
            .ok_or_else(|| RunError::Parse(format!("{}", case.source_path.display())))?,
        SourceKind::Ml => frk_front::compile_ml(context, source).map_err(|e| {
            RunError::Parse(format!("{}: {e}", case.source_path.display()))
        })?,
        SourceKind::Ts => {
            let artifact = produce_loanword(case)?;
            frk_front::loanword::compile_loanword(context, &artifact)
                .map_err(|e| RunError::Parse(format!("{}: {e}", case.source_path.display())))?
        }
        SourceKind::Lua => {
            let file = case
                .source_path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "case.lua".to_string());
            frk_front::lua::compile_lua(context, &file, source)
                .map_err(|e| RunError::Parse(format!("{}: {e}", case.source_path.display())))?
        }
        SourceKind::Scheme => {
            let file = case
                .source_path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "case.scm".to_string());
            frk_front::scheme::compile_scheme(context, &file, source)
                .map_err(|e| RunError::Parse(format!("{}: {e}", case.source_path.display())))?
        }
        SourceKind::Transcript => {
            return Err(RunError::Parse(format!(
                "{}: transcripts run only under the repl runner",
                case.source_path.display()
            )));
        }
    };
    if !module.as_operation().verify() {
        return Err(RunError::Verify(format!(
            "{}: module failed MLIR verification",
            case.source_path.display()
        )));
    }
    frk_dialects::verify(context, &module)
        .map_err(|errors| RunError::Verify(format!("{errors}")))?;
    Ok(module)
}

/// Every runner applicable to the corpus today — the list `make diff`
/// executes and the corpus tests hold in pairwise agreement (law L3).
/// interp + jit since M2, the ocaml oracle since M5 (ml cases only);
/// M7 adds the AOT path.
pub fn default_runners() -> Vec<Box<dyn Runner>> {
    vec![
        Box::new(InterpRunner),
        Box::new(JitRunner { strategy: frk_dialects::Strategy::Arena }),
        Box::new(JitRunner { strategy: frk_dialects::Strategy::Rc }),
        Box::new(OcamlOracle),
        Box::new(NodeOracle),
        Box::new(LuaOracle),
        Box::new(SchemeOracle),
        Box::new(ReplRunner),
    ]
}

/// The upstream oracle for femto_lua (M11): lua5.1 (the 5.1.5 pin IS
/// the spec) runs the SAME .lua file, LC_ALL=C, through canon.
pub struct LuaOracle;

impl Runner for LuaOracle {
    fn name(&self) -> &'static str {
        "lua"
    }

    fn applicable(&self, case: &Case) -> bool {
        case.kind == SourceKind::Lua
    }

    fn run(&self, case: &Case) -> Result<String, RunError> {
        let output = std::process::Command::new("lua5.1")
            .arg(&case.source_path)
            .env("LC_ALL", "C")
            .output()
            .map_err(|e| RunError::Invoke(format!("running lua5.1: {e}")))?;
        if !output.status.success() {
            return Err(RunError::Invoke(format!(
                "lua5.1 exited with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        String::from_utf8(output.stdout)
            .map_err(|_| RunError::Invoke("lua5.1 produced non-UTF-8 output".into()))
    }
}

/// The upstream oracle for r7rs_core (M15, D-060): chibi-scheme runs
/// the SAME .scm file (`-q`, so `(import (scheme base) (scheme write))`
/// supplies call/cc + display), LC_ALL=C, STDOUT only. chibi's
/// `CHIBI_VERSION_TESTED` pin is the spec reference.
pub struct SchemeOracle;

impl Runner for SchemeOracle {
    fn name(&self) -> &'static str {
        "scheme"
    }

    fn applicable(&self, case: &Case) -> bool {
        case.kind == SourceKind::Scheme
    }

    fn run(&self, case: &Case) -> Result<String, RunError> {
        let output = std::process::Command::new("chibi-scheme")
            .arg("-q")
            .arg(&case.source_path)
            .env("LC_ALL", "C")
            .output()
            .map_err(|e| RunError::Invoke(format!("running chibi-scheme: {e}")))?;
        if !output.status.success() {
            return Err(RunError::Invoke(format!(
                "chibi-scheme exited with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        String::from_utf8(output.stdout)
            .map_err(|_| RunError::Invoke("chibi-scheme produced non-UTF-8 output".into()))
    }
}

/// The upstream oracle for TS-0 (M9): node runs the SAME .ts file
/// directly (native type stripping, node >= 20 per versions.env),
/// LC_ALL=C, through the same canon filter. node/V8 is ground truth
/// (specimens/typescript/MANIFEST.md).
pub struct NodeOracle;

impl Runner for NodeOracle {
    fn name(&self) -> &'static str {
        "node"
    }

    fn applicable(&self, case: &Case) -> bool {
        case.kind == SourceKind::Ts
    }

    fn run(&self, case: &Case) -> Result<String, RunError> {
        let output = std::process::Command::new("node")
            .arg(&case.source_path)
            .env("LC_ALL", "C")
            .output()
            .map_err(|e| RunError::Invoke(format!("running node: {e}")))?;
        if !output.status.success() {
            return Err(RunError::Invoke(format!(
                "node exited with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        String::from_utf8(output.stdout)
            .map_err(|_| RunError::Invoke("node produced non-UTF-8 output".into()))
    }
}

/// The scripted-transcript runner (M8 exit bar): drives the EXACT
/// library engine the interactive shell runs, over transcript.in,
/// with :load resolved against the case directory.
pub struct ReplRunner;

impl Runner for ReplRunner {
    fn name(&self) -> &'static str {
        "repl"
    }

    fn applicable(&self, case: &Case) -> bool {
        case.kind == SourceKind::Transcript
    }

    fn run(&self, case: &Case) -> Result<String, RunError> {
        let input = fs::read_to_string(&case.source_path)
            .map_err(|e| RunError::Io(format!("{}: {e}", case.source_path.display())))?;
        Ok(frk_repl::run_transcript(&input, case.dir.clone()))
    }
}

/// The runner blessing writes goldens from: the derived interpreter,
/// which *is* the reference semantics (D-008) — everything else must
/// agree with it byte-exactly (L3).
pub fn reference_runner() -> Box<dyn Runner> {
    Box::new(InterpRunner)
}

/// The derived-interpreter runner (SPEC §7.1) — reference semantics
/// since M2 (D-008). Interpretation runs on a `frk_interp::STACK_SIZE`
/// thread so depth-ceiling programs trap per D-029 instead of exhausting
/// a skinny caller stack.
pub struct InterpRunner;

impl Runner for InterpRunner {
    fn name(&self) -> &'static str {
        "interp"
    }

    fn applicable(&self, case: &Case) -> bool {
        case.kind != SourceKind::Transcript
    }

    fn run(&self, case: &Case) -> Result<String, RunError> {
        std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(frk_interp::STACK_SIZE)
                .spawn_scoped(scope, || interpret_case(case))
                .map_err(|e| RunError::Io(format!("spawning interpreter thread: {e}")))?
                .join()
                .map_err(|_| RunError::Invoke("interpreter thread panicked".into()))?
        })
    }
}

fn interpret_case(case: &Case) -> Result<String, RunError> {
    let (context, source) = frk_context(case)?;
    let module = parse_and_verify(&context, &source, case)?;

    let mut interp =
        frk_interp::Interp::new(&module).map_err(|e| RunError::Invoke(e.to_string()))?;
    frk_dialects::register_eval(&mut interp);

    if matches!(case.kind, SourceKind::Ts | SourceKind::Lua | SourceKind::Scheme) {
        // TS-0/Lua protocol (D-047/D-054): void entry; output = prints.
        // scheme display protocol (M15): no trailing newline; newline
        // emits one; booleans as #t/#f.
        interp.register_builtin(
            "frk_rt_scm_display_num",
            Box::new(|arguments, output| {
                output.push_str(&frk_rt::format_lua_num(arguments[0].as_float()?));
                Ok(vec![])
            }),
        );
        interp.register_builtin(
            "frk_rt_scm_display_bool",
            Box::new(|arguments, output| {
                output.push_str(if arguments[0].as_signed()? != 0 { "#t" } else { "#f" });
                Ok(vec![])
            }),
        );
        interp.register_builtin(
            "frk_rt_scm_newline",
            Box::new(|_arguments, output| {
                output.push('\n');
                Ok(vec![])
            }),
        );
        interp.register_builtin(
            frk_front::loanword::PRINT_F64,
            Box::new(|arguments, output| {
                output.push_str(&frk_rt::format_f64(arguments[0].as_float()?));
                output.push('\n');
                Ok(vec![])
            }),
        );
        interp.register_builtin(
            frk_front::loanword::PRINT_BOOL,
            Box::new(|arguments, output| {
                output.push_str(if arguments[0].as_bool()? { "true" } else { "false" });
                output.push('\n');
                Ok(vec![])
            }),
        );
        interp.register_builtin(
            frk_front::loanword::PRINT_STR,
            Box::new(|arguments, output| {
                let units = arguments[0].as_str_units()?;
                output.push_str(&String::from_utf16_lossy(units));
                output.push('\n');
                Ok(vec![])
            }),
        );
        interp.register_builtin(
            "frk_rt_bstr_from_num",
            Box::new(|arguments, _output| {
                Ok(vec![frk_interp::Value::bytes(
                    frk_rt::format_lua_num(arguments[0].as_float()?).into_bytes(),
                )])
            }),
        );
        interp.register_builtin(
            "frk_rt_print_lua_str",
            Box::new(|arguments, output| {
                let bytes = arguments[0].as_bytes()?;
                output.push_str(&String::from_utf8_lossy(bytes));
                output.push('\n');
                Ok(vec![])
            }),
        );
        interp.register_builtin(
            "frk_rt_lua_error",
            Box::new(|arguments, _output| {
                Err(frk_interp::EvalError::Trap(format!(
                    "lua runtime error {} (D-056)",
                    arguments[0].as_signed()?
                )))
            }),
        );
        interp
            .eval_function(&case.entry, &[])
            .map_err(|e| RunError::Invoke(e.to_string()))?;
        return Ok(interp.take_output());
    }

    let results = interp
        .eval_function(&case.entry, &[])
        .map_err(|e| RunError::Invoke(e.to_string()))?;
    match case.result {
        ResultKind::I64 => {
            let [value] = results.as_slice() else {
                return Err(RunError::Invoke(format!(
                    "entry returned {} value(s); the protocol expects one i64",
                    results.len()
                )));
            };
            match value.width() {
                Ok(64) => Ok(canon::render_i64(
                    value
                        .as_signed()
                        .map_err(|e| RunError::Invoke(e.to_string()))?,
                )),
                Ok(other) => Err(RunError::Invoke(format!(
                    "entry returned i{other}, protocol expects i64"
                ))),
                Err(_) => Err(RunError::Invoke(
                    "entry returned a non-scalar value; the protocol expects i64".into(),
                )),
            }
        }
    }
}

/// The ORC JIT runner, one per memory strategy (D-041): parse → verify
/// → shared lowering pipeline at the strategy → ExecutionEngine →
/// render the entry's return per docs/canon.md §2. Both strategies sit
/// in default_runners, so the diff matrix holds them byte-equal.
pub struct JitRunner {
    pub strategy: frk_dialects::Strategy,
}

impl Runner for JitRunner {
    fn name(&self) -> &'static str {
        match self.strategy {
            frk_dialects::Strategy::Arena => "jit",
            frk_dialects::Strategy::Rc => "jit-rc",
        }
    }

    fn applicable(&self, case: &Case) -> bool {
        case.kind != SourceKind::Transcript
    }

    fn run(&self, case: &Case) -> Result<String, RunError> {
        let (context, source) = frk_context(case)?;
        let mut module = parse_and_verify(&context, &source, case)?;

        pipeline::lower_to_llvm(&context, &mut module, self.strategy)
            .map_err(|e| RunError::Lower(format!("{e}")))?;

        // Entry functions carry llvm.emit_c_interface (goldens/README.md);
        // invoke_packed resolves the _mlir_ciface_ wrapper by entry name.
        let engine = ExecutionEngine::new(&module, 2, &[], false, false);
        // The kernel lowering calls the strategy's runtime (D-041); the
        // harness process hosts frk-rt, so hand the JIT every symbol.
        // AOT (M7 grid) links the staticlib instead.
        unsafe {
            engine.register_symbol(
                "frk_rt_arena_alloc",
                frk_rt::frk_rt_arena_alloc as *mut (),
            );
            engine.register_symbol("frk_rt_rc_alloc", frk_rt::frk_rt_rc_alloc as *mut ());
            engine.register_symbol("frk_rt_rc_retain", frk_rt::frk_rt_rc_retain as *mut ());
            engine.register_symbol(
                "frk_rt_rc_release",
                frk_rt::frk_rt_rc_release as *mut (),
            );
            // TS prints resolve to in-process CAPTURING symbols: the
            // JIT shares our stdout, so the real runtime's println
            // would interleave with harness output (D-047).
            engine.register_symbol(
                frk_front::loanword::PRINT_F64,
                capture_print_f64 as *mut (),
            );
            engine.register_symbol(
                frk_front::loanword::PRINT_BOOL,
                capture_print_bool as *mut (),
            );
            engine.register_symbol(
                "frk_rt_str_from_units",
                frk_rt::frk_rt_str_from_units as *mut (),
            );
            engine.register_symbol("frk_rt_str_concat", frk_rt::frk_rt_str_concat as *mut ());
            engine.register_symbol("frk_rt_str_eq", frk_rt::frk_rt_str_eq as *mut ());
            engine.register_symbol("frk_rt_str_len", frk_rt::frk_rt_str_len as *mut ());
            engine.register_symbol("frk_rt_dyn_check", frk_rt::frk_rt_dyn_check as *mut ());
            engine.register_symbol(
                "frk_rt_bstr_intern",
                frk_rt::frk_rt_bstr_intern as *mut (),
            );
            engine.register_symbol(
                "frk_rt_bstr_concat",
                frk_rt::frk_rt_bstr_concat as *mut (),
            );
            engine.register_symbol(
                "frk_rt_bstr_from_num",
                frk_rt::frk_rt_bstr_from_num as *mut (),
            );
            engine.register_symbol("frk_rt_table_init", frk_rt::frk_rt_table_init as *mut ());
            engine.register_symbol(
                "frk_rt_table_raw_get",
                frk_rt::frk_rt_table_raw_get as *mut (),
            );
            engine.register_symbol(
                "frk_rt_table_raw_set",
                frk_rt::frk_rt_table_raw_set as *mut (),
            );
            engine.register_symbol("frk_rt_table_len", frk_rt::frk_rt_table_len as *mut ());
            engine.register_symbol("frk_rt_lua_error", frk_rt::frk_rt_lua_error as *mut ());
            engine.register_symbol(
                "frk_rt_table_next",
                frk_rt::frk_rt_table_next as *mut (),
            );
            engine.register_symbol("frk_rt_bstr_sub", frk_rt::frk_rt_bstr_sub as *mut ());
            engine.register_symbol("frk_rt_bstr_rep", frk_rt::frk_rt_bstr_rep as *mut ());
            // scheme display resolves to CAPTURING symbols (shared
            // stdout, D-047) — no trailing newline for display.
            engine.register_symbol(
                "frk_rt_scm_display_num",
                capture_scm_display_num as *mut (),
            );
            engine.register_symbol(
                "frk_rt_scm_display_bool",
                capture_scm_display_bool as *mut (),
            );
            engine.register_symbol("frk_rt_scm_newline", capture_scm_newline as *mut ());
            // Control effects (κ_frk, D-060): the pending-cell carrier.
            engine.register_symbol(
                "frk_rt_ctl_prompt_enter",
                frk_rt::frk_rt_ctl_prompt_enter as *mut (),
            );
            engine.register_symbol(
                "frk_rt_ctl_prompt_exit",
                frk_rt::frk_rt_ctl_prompt_exit as *mut (),
            );
            engine.register_symbol("frk_rt_ctl_abort", frk_rt::frk_rt_ctl_abort as *mut ());
            engine.register_symbol(
                "frk_rt_ctl_pending",
                frk_rt::frk_rt_ctl_pending as *mut (),
            );
            engine.register_symbol(
                "frk_rt_ctl_resolve",
                frk_rt::frk_rt_ctl_resolve as *mut (),
            );
            engine.register_symbol(
                "frk_rt_print_lua_str",
                capture_print_lua_str as *mut (),
            );
            engine.register_symbol(
                frk_front::loanword::PRINT_STR,
                capture_print_str as *mut (),
            );
        }
        if matches!(case.kind, SourceKind::Ts | SourceKind::Lua | SourceKind::Scheme) {
            CAPTURE.with(|buffer| buffer.borrow_mut().clear());
            unsafe {
                engine
                    .invoke_packed(&case.entry, &mut [])
                    .map_err(|e| RunError::Invoke(format!("{}: {e}", case.entry)))?;
            }
            return Ok(CAPTURE.with(|buffer| buffer.borrow_mut().split_off(0)));
        }
        match case.result {
            ResultKind::I64 => {
                let mut result: i64 = 0;
                unsafe {
                    engine
                        .invoke_packed(&case.entry, &mut [&mut result as *mut i64 as *mut ()])
                        .map_err(|e| RunError::Invoke(format!("{}: {e}", case.entry)))?;
                }
                Ok(canon::render_i64(result))
            }
        }
    }
}

thread_local! {
    /// Print capture for in-process JIT runs of ts cases (D-047).
    static CAPTURE: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

extern "C" fn capture_print_f64(value: f64) {
    CAPTURE.with(|buffer| {
        let mut buffer = buffer.borrow_mut();
        buffer.push_str(&frk_rt::format_f64(value));
        buffer.push('\n');
    });
}

extern "C" fn capture_print_bool(value: u8) {
    CAPTURE.with(|buffer| {
        let mut buffer = buffer.borrow_mut();
        buffer.push_str(if value != 0 { "true" } else { "false" });
        buffer.push('\n');
    });
}

extern "C" fn capture_print_lua_str(s: *const u8) {
    let text = unsafe {
        let len = *(s as *const u64) as usize;
        let bytes = std::slice::from_raw_parts(s.add(8), len);
        String::from_utf8_lossy(bytes).into_owned()
    };
    CAPTURE.with(|buffer| {
        let mut buffer = buffer.borrow_mut();
        buffer.push_str(&text);
        buffer.push('\n');
    });
}

extern "C" fn capture_print_str(s: *const u8) {
    let text = unsafe {
        let len = *(s as *const u64) as usize;
        let units = std::slice::from_raw_parts(s.add(8) as *const u16, len);
        String::from_utf16_lossy(units)
    };
    CAPTURE.with(|buffer| {
        let mut buffer = buffer.borrow_mut();
        buffer.push_str(&text);
        buffer.push('\n');
    });
}

// scheme display protocol (M15): display has NO trailing newline;
// newline emits one. Capturing versions for the in-process JIT.
extern "C" fn capture_scm_display_num(value: f64) {
    CAPTURE.with(|buffer| buffer.borrow_mut().push_str(&frk_rt::format_lua_num(value)));
}

extern "C" fn capture_scm_display_bool(value: u8) {
    CAPTURE.with(|buffer| buffer.borrow_mut().push_str(if value != 0 { "#t" } else { "#f" }));
}

extern "C" fn capture_scm_newline() {
    CAPTURE.with(|buffer| buffer.borrow_mut().push('\n'));
}

/// The upstream oracle for ml_core (SPEC §7.2/§8; oracle policy in
/// docs/canon.md §5): the SAME source file the frankish runners
/// compile, run by `ocaml` with `print_int (main ())` appended, under
/// LC_ALL=C, through the same canon filter. The int-width divergence
/// (OCaml 63-bit vs our i64) is a corpus rule: values stay within 62
/// bits (specimens/ml_core/MANIFEST.md).
pub struct OcamlOracle;

impl Runner for OcamlOracle {
    fn name(&self) -> &'static str {
        "ocaml"
    }

    fn applicable(&self, case: &Case) -> bool {
        case.kind == SourceKind::Ml
    }

    fn run(&self, case: &Case) -> Result<String, RunError> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);

        let source = fs::read_to_string(&case.source_path)
            .map_err(|e| RunError::Io(format!("{}: {e}", case.source_path.display())))?;
        let wrapped = format!("{source}\nlet () = print_int (main ())\n");

        let path = std::env::temp_dir().join(format!(
            "frk-oracle-{}-{}.ml",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&path, wrapped)
            .map_err(|e| RunError::Io(format!("{}: {e}", path.display())))?;

        let output = std::process::Command::new("ocaml")
            .arg(&path)
            .env("LC_ALL", "C")
            .output();
        let _ = fs::remove_file(&path);
        let output = output.map_err(|e| RunError::Invoke(format!("running ocaml: {e}")))?;

        if !output.status.success() {
            return Err(RunError::Invoke(format!(
                "ocaml exited with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        String::from_utf8(output.stdout)
            .map_err(|_| RunError::Invoke("ocaml produced non-UTF-8 output".into()))
    }
}

/// Tier-0 grid triples (D-017/D-042). Musl-static (zig bundles libc)
/// so qemu-user executes sysroot-free; wasm32-wasi runs on wasmtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Triple {
    X86_64Linux,
    Aarch64Linux,
    Riscv64Linux,
    S390xLinux,
    Wasm32Wasi,
}

impl Triple {
    pub const GRID: [Triple; 4] = [
        Triple::X86_64Linux,
        Triple::Aarch64Linux,
        Triple::Riscv64Linux,
        Triple::Wasm32Wasi,
    ];

    pub fn target(self) -> &'static str {
        match self {
            Self::X86_64Linux => "x86_64-linux-musl",
            Self::Aarch64Linux => "aarch64-linux-musl",
            Self::Riscv64Linux => "riscv64-linux-musl",
            Self::S390xLinux => "s390x-linux-musl",
            Self::Wasm32Wasi => "wasm32-wasi",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Self::X86_64Linux => "x86_64",
            Self::Aarch64Linux => "aarch64",
            Self::Riscv64Linux => "riscv64",
            Self::S390xLinux => "s390x",
            Self::Wasm32Wasi => "wasm32",
        }
    }

    /// The executor prefix, if any (native runs directly).
    fn executor(self) -> Option<Vec<String>> {
        match self {
            Self::X86_64Linux => None,
            Self::Aarch64Linux => Some(vec!["qemu-aarch64".into()]),
            Self::Riscv64Linux => Some(vec!["qemu-riscv64".into()]),
            Self::S390xLinux => Some(vec!["qemu-s390x".into()]),
            Self::Wasm32Wasi => Some(vec![wasmtime_path(), "run".into()]),
        }
    }
}

fn wasmtime_path() -> String {
    if which("wasmtime") {
        return "wasmtime".into();
    }
    if let Ok(home) = std::env::var("HOME") {
        let candidate = format!("{home}/.wasmtime/bin/wasmtime");
        if std::path::Path::new(&candidate).is_file() {
            return candidate;
        }
    }
    "wasmtime".into()
}

fn which(name: &str) -> bool {
    std::env::var("PATH").is_ok_and(|path| {
        path.split(':')
            .any(|dir| std::path::Path::new(dir).join(name).is_file())
    })
}

fn mlir_prefix() -> String {
    std::env::var("MLIR_SYS_220_PREFIX").unwrap_or_else(|_| "/usr/lib/llvm-22".into())
}

/// Walks up from a case directory to the repo root (the directory
/// holding versions.env) — the AOT runner needs scripts/ and
/// crates/frk-rt/c/ regardless of the harness process's cwd.
fn repo_root_from(start: &std::path::Path) -> Result<std::path::PathBuf, RunError> {
    let mut dir = start
        .canonicalize()
        .map_err(|e| RunError::Io(format!("{}: {e}", start.display())))?;
    loop {
        if dir.join("versions.env").is_file() {
            return Ok(dir);
        }
        if !dir.pop() {
            return Err(RunError::Io(
                "no versions.env above the corpus — AOT needs the repo root".into(),
            ));
        }
    }
}

/// The AOT runner (D-042): strategy pipeline → mlir-translate →
/// llvm-22 clang compiles IR to a per-triple object (IR version
/// safety) → zig cc links it with the generated shim and the C
/// runtime mirror (bundled musl/wasi libc) → execute (qemu/wasmtime
/// off-native) → canon over stdout.
pub struct AotRunner {
    pub triple: Triple,
    pub strategy: frk_dialects::Strategy,
    /// Leaked once at construction: Runner::name returns &'static str.
    name: &'static str,
}

impl AotRunner {
    pub fn new(triple: Triple, strategy: frk_dialects::Strategy) -> Self {
        let name = match strategy {
            frk_dialects::Strategy::Arena => format!("aot-{}", triple.short()),
            frk_dialects::Strategy::Rc => format!("aot-{}-rc", triple.short()),
        };
        Self {
            triple,
            strategy,
            name: Box::leak(name.into_boxed_str()),
        }
    }
}

impl Runner for AotRunner {
    fn name(&self) -> &'static str {
        self.name
    }

    fn applicable(&self, case: &Case) -> bool {
        case.kind != SourceKind::Transcript
    }

    fn run(&self, case: &Case) -> Result<String, RunError> {
        use std::process::Command;
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);

        let (context, source) = frk_context(case)?;
        let mut module = parse_and_verify(&context, &source, case)?;

        // Rename the entry symbol pre-lowering: the C shim owns main()
        // (D-042; entry functions are externally-invoked-only).
        rename_entry(&module, &case.entry, "frk_entry").map_err(RunError::Verify)?;

        pipeline::lower_to_llvm(&context, &mut module, self.strategy)
            .map_err(|e| RunError::Lower(format!("{e}")))?;

        let root = repo_root_from(&case.dir)?;
        let work = std::env::temp_dir().join(format!(
            "frk-aot-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&work).map_err(|e| RunError::Io(format!("{e}")))?;
        let cleanup = |result: Result<String, RunError>| {
            let _ = fs::remove_dir_all(&work);
            result
        };

        let run = || -> Result<String, RunError> {
            let mlir_path = work.join("lowered.mlir");
            fs::write(&mlir_path, module.as_operation().to_string())
                .map_err(|e| RunError::Io(format!("{e}")))?;

            let ll_path = work.join("case.ll");
            let translate = Command::new(format!("{}/bin/mlir-translate", mlir_prefix()))
                .args(["--mlir-to-llvmir"])
                .arg(&mlir_path)
                .args(["-o"])
                .arg(&ll_path)
                .output()
                .map_err(|e| RunError::Invoke(format!("mlir-translate: {e}")))?;
            if !translate.status.success() {
                return Err(RunError::Lower(format!(
                    "mlir-translate: {}",
                    String::from_utf8_lossy(&translate.stderr)
                )));
            }

            // IR → object with the PINNED LLVM's clang (the IR may be
            // newer than zig's bundled LLVM); zig links with its libc.
            let obj_path = work.join("case.o");
            let mut compile_cmd = Command::new(format!("{}/bin/clang", mlir_prefix()));
            compile_cmd.args(["-target", self.triple.target(), "-O1", "-c"]);
            if self.triple == Triple::Wasm32Wasi {
                // musttail needs the wasm tail-call feature (D-059);
                // wasmtime 46 has the proposal on by default.
                compile_cmd.arg("-mtail-call");
            }
            let compile = compile_cmd
                .arg(&ll_path)
                .args(["-o"])
                .arg(&obj_path)
                .output()
                .map_err(|e| RunError::Invoke(format!("clang: {e}")))?;
            if !compile.status.success() {
                return Err(RunError::Lower(format!(
                    "clang -c: {}",
                    String::from_utf8_lossy(&compile.stderr)
                )));
            }

            let shim_path = work.join("shim.c");
            let shim: &str = if matches!(case.kind, SourceKind::Ts | SourceKind::Lua | SourceKind::Scheme) {
                // TS entry protocol (D-047): void main; output happens
                // through the linked C runtime's print functions.
                "extern void frk_entry(void);\nint main(void) { frk_entry(); return 0; }\n"
            } else {
                "#include <stdio.h>\nextern long long frk_entry(void);\nint main(void) { printf(\"%lld\\n\", frk_entry()); return 0; }\n"
            };
            fs::write(&shim_path, shim).map_err(|e| RunError::Io(format!("{e}")))?;

            let exe_path = work.join("case.exe");
            let link = Command::new("sh")
                .arg(root.join("scripts/zigcc.sh"))
                .args(["-target", self.triple.target(), "-O1"])
                .arg(&obj_path)
                .arg(&shim_path)
                .arg(root.join("crates/frk-rt/c/frk_rt.c"))
                .args(["-o"])
                .arg(&exe_path)
                .current_dir(&root)
                .output()
                .map_err(|e| RunError::Invoke(format!("zigcc: {e}")))?;
            if !link.status.success() {
                return Err(RunError::Lower(format!(
                    "zig cc link: {}",
                    String::from_utf8_lossy(&link.stderr)
                )));
            }

            let output = match self.triple.executor() {
                None => Command::new(&exe_path)
                    .output()
                    .map_err(|e| RunError::Invoke(format!("exec: {e}")))?,
                Some(executor) => {
                    let mut command = Command::new(&executor[0]);
                    for arg in &executor[1..] {
                        command.arg(arg);
                    }
                    command
                        .arg(&exe_path)
                        .output()
                        .map_err(|e| RunError::Invoke(format!("{}: {e}", executor[0])))?
                }
            };
            if !output.status.success() {
                return Err(RunError::Invoke(format!(
                    "target exited with {}: {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                )));
            }
            String::from_utf8(output.stdout)
                .map_err(|_| RunError::Invoke("non-UTF-8 target output".into()))
        };
        cleanup(run())
    }
}

/// Renames the module-level func.func `from` to `to` (sym_name only —
/// valid because entry symbols are externally-invoked-only, D-042).
fn rename_entry(module: &Module, from: &str, to: &str) -> Result<(), String> {
    use melior::ir::BlockLike;
    use melior::ir::attribute::StringAttribute;
    use melior::ir::operation::OperationMutLike;

    let context = unsafe { module.context().to_ref() };
    let body = module.body();
    let mut next = body.first_operation_mut();
    while let Some(mut op) = next {
        let following = op.next_in_block_mut();
        let name_matches = op
            .attribute("sym_name")
            .ok()
            .and_then(|attribute| StringAttribute::try_from(attribute).ok())
            .is_some_and(|attribute| attribute.value() == from);
        if name_matches {
            op.set_attribute("sym_name", StringAttribute::new(context, to).into());
            return Ok(());
        }
        next = following;
    }
    Err(format!("entry symbol {from:?} not found for AOT rename"))
}
