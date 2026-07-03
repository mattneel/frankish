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
        Box::new(JitRunner),
        Box::new(OcamlOracle),
    ]
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

/// The ORC JIT runner: parse → verify → shared lowering pipeline →
/// ExecutionEngine → render the entry's return per docs/canon.md §2.
pub struct JitRunner;

impl Runner for JitRunner {
    fn name(&self) -> &'static str {
        "jit"
    }

    fn run(&self, case: &Case) -> Result<String, RunError> {
        let (context, source) = frk_context(case)?;
        let mut module = parse_and_verify(&context, &source, case)?;

        pipeline::lower_to_llvm(&context, &mut module)
            .map_err(|e| RunError::Lower(format!("{e}")))?;

        // Entry functions carry llvm.emit_c_interface (goldens/README.md);
        // invoke_packed resolves the _mlir_ciface_ wrapper by entry name.
        let engine = ExecutionEngine::new(&module, 2, &[], false, false);
        // The kernel lowering calls frk_rt_alloc for closure envs
        // (D-035); the harness process hosts frk-rt, so hand the JIT the
        // symbol directly. AOT (M7) links the staticlib instead.
        unsafe {
            engine.register_symbol("frk_rt_alloc", frk_rt::frk_rt_alloc as *mut ());
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
