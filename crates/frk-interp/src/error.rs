//! Interpreter failure taxonomy. `Trap` is the deliberate, deterministic
//! answer to MLIR-level UB (D-029): the reference semantics must be total,
//! so what native codegen leaves undefined, the interpreter defines as a
//! trap — and the golden corpus is required to stay UB-free.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvalError {
    /// No Eval registered for this op — not an input error, a coverage
    /// boundary. Extending coverage means landing the op's tests with it.
    UnknownOp(String),
    /// Recognized but outside what v0 chooses to support.
    Unsupported(String),
    /// IR shape the MLIR verifier should have rejected (or an interpreter
    /// bug); never a normal program outcome.
    Malformed(String),
    TypeMismatch(String),
    /// Deterministic runtime trap (division by zero, signed div overflow,
    /// call-depth exhaustion, non-positive scf.for step).
    Trap(String),
    CalleeNotFound(String),
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownOp(name) => write!(f, "no Eval registered for {name:?}"),
            Self::Unsupported(what) => write!(f, "unsupported: {what}"),
            Self::Malformed(what) => write!(f, "malformed IR: {what}"),
            Self::TypeMismatch(what) => write!(f, "type mismatch: {what}"),
            Self::Trap(what) => write!(f, "trap: {what}"),
            Self::CalleeNotFound(name) => write!(f, "callee not found: @{name}"),
        }
    }
}

impl std::error::Error for EvalError {}
