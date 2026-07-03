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
use crate::case::{Case, ResultKind};
use crate::pipeline;

/// A named way to execute a case and produce raw (pre-canonicalization)
/// output. Implementations must be deterministic under docs/canon.md.
pub trait Runner {
    fn name(&self) -> &'static str;
    fn run(&self, case: &Case) -> Result<String, RunError>;
}

#[derive(Debug)]
pub enum RunError {
    Io(String),
    Parse(String),
    Verify(String),
    Lower(String),
    Invoke(String),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(m) => write!(f, "io: {m}"),
            Self::Parse(m) => write!(f, "parse: {m}"),
            Self::Verify(m) => write!(f, "verify: {m}"),
            Self::Lower(m) => write!(f, "lower: {m}"),
            Self::Invoke(m) => write!(f, "invoke: {m}"),
        }
    }
}

impl std::error::Error for RunError {}

/// The ORC JIT runner: parse → verify → shared lowering pipeline →
/// ExecutionEngine → render the entry's return per docs/canon.md §2.
pub struct JitRunner;

impl Runner for JitRunner {
    fn name(&self) -> &'static str {
        "jit"
    }

    fn run(&self, case: &Case) -> Result<String, RunError> {
        let source = fs::read_to_string(&case.source_path)
            .map_err(|e| RunError::Io(format!("{}: {e}", case.source_path.display())))?;

        let context = frk_core::context();
        let mut module = Module::parse(&context, &source)
            .ok_or_else(|| RunError::Parse(format!("{}", case.source_path.display())))?;

        if !module.as_operation().verify() {
            return Err(RunError::Verify(format!(
                "{}: module failed MLIR verification",
                case.source_path.display()
            )));
        }

        pipeline::lower_to_llvm(&context, &mut module)
            .map_err(|e| RunError::Lower(format!("{e}")))?;

        // Entry functions carry llvm.emit_c_interface (goldens/README.md);
        // invoke_packed resolves the _mlir_ciface_ wrapper by entry name.
        let engine = ExecutionEngine::new(&module, 2, &[], false, false);
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
