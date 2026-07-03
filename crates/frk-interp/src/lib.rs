//! frk-interp — the per-op Eval interface and the derived interpreter
//! (SPEC §7.1). From M2 on this is the project's reference semantics
//! (D-008): the JIT/AOT paths must byte-match it on every golden (law
//! L3). Deterministic trap semantics and the call-depth ceiling are
//! ruled in D-029.

mod error;
mod interp;
mod upstream;
mod value;

pub use error::EvalError;
pub use interp::{Eval, Frame, Interp, MAX_CALL_DEPTH, STACK_SIZE, Step};
pub use value::Value;

use melior::ir::Module;

/// One-shot convenience: interpret `entry(args)` inside `module` and
/// return the call's results.
pub fn interpret_entry(
    module: &Module,
    entry: &str,
    args: &[Value],
) -> Result<Vec<Value>, EvalError> {
    Interp::new(module)?.eval_function(entry, args)
}
