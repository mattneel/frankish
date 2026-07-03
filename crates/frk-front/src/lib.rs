//! frk-front — the frontend kit: readers, binder, type kit, and (M9)
//! the loanword consumer (SPEC §6). First resident: the ml_core
//! specimen frontend (M5) — scaffolding-grade lexer/parser (D-019),
//! HM inference with let-polymorphism over ena (SPEC §6.4), and
//! emission into the kernel dialects.
//!
//! v0 debts, all ledgered: spans don't thread into MLIR locations yet
//! (§6.5 — a diagnostic pointing at IR instead of source is a known
//! bug class until then); the green tree (§6.2) is deferred with the
//! rowan-vs-custom decision; polymorphic multi-instantiation emission
//! and recursive ADTs are fenced (D-038).

pub mod ast;
pub mod loanword;
pub mod lua;
pub mod emit;
pub mod infer;
pub mod lex;
pub mod types;

use melior::Context;
use melior::ir::Module;

#[derive(Debug)]
pub enum CompileError {
    Parse(String),
    Type(String),
    Emit(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(m) => write!(f, "parse: {m}"),
            Self::Type(m) => write!(f, "type: {m}"),
            Self::Emit(m) => write!(f, "emit: {m}"),
        }
    }
}

impl std::error::Error for CompileError {}

/// Compiles an ml_core program to a kernel-dialect module inside
/// `context`. Precondition: the kernel dialects are registered
/// (`frk_dialects::register`).
/// Typecheck only, REPL policy (main optional, any concrete result).
/// The REPL validates decl lines and answers `:type` with this.
pub fn check_ml(source: &str) -> Result<infer::TypedProgram, CompileError> {
    let program = ast::parse(source).map_err(|e| CompileError::Parse(e.to_string()))?;
    infer::infer_with(&program, infer::MainPolicy::OptionalAny)
        .map_err(|e| CompileError::Type(e.0))
}

/// Compile under the REPL policy; returns the module plus main's
/// zonked result type for value rendering (D-043). Errors if the
/// source has no main.
pub fn compile_ml_any<'c>(
    context: &'c melior::Context,
    source: &str,
) -> Result<(melior::ir::Module<'c>, types::Ty), CompileError> {
    let program = ast::parse(source).map_err(|e| CompileError::Parse(e.to_string()))?;
    let typed = infer::infer_with(&program, infer::MainPolicy::OptionalAny)
        .map_err(|e| CompileError::Type(e.0))?;
    let result = typed
        .main_result
        .clone()
        .ok_or_else(|| CompileError::Emit("REPL compilation needs a main".into()))?;
    let module = emit::emit(context, &typed).map_err(CompileError::Emit)?;
    Ok((module, result))
}

pub fn compile_ml<'c>(
    context: &'c Context,
    source: &str,
) -> Result<Module<'c>, CompileError> {
    let program = ast::parse(source).map_err(|e| CompileError::Parse(e.to_string()))?;
    let typed = infer::infer(&program).map_err(|e| CompileError::Type(e.to_string()))?;
    emit::emit(context, &typed).map_err(CompileError::Emit)
}
