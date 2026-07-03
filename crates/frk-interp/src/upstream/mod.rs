//! Eval adapters for upstream dialects (SPEC §7.1: the interpreter walks
//! "any mix of kernel + supported upstream ops"). Coverage grows with the
//! corpus: an op lands here with its tests in the same commit, or not at
//! all (law L1). Unlisted ops fail loudly as [`EvalError::UnknownOp`].

use std::collections::HashMap;

use crate::interp::Eval;

mod arith;
mod cf;
mod func;
mod scf;

pub(crate) fn register_all() -> HashMap<&'static str, Box<dyn Eval>> {
    let mut registry: HashMap<&'static str, Box<dyn Eval>> = HashMap::new();
    arith::register(&mut registry);
    cf::register(&mut registry);
    func::register(&mut registry);
    scf::register(&mut registry);
    registry
}

// Shared accessor helpers, used by every dialect module.

use crate::error::EvalError;
use crate::interp::{Frame, Step};
use crate::value::Value;
use melior::ir::OperationRef;
use melior::ir::operation::OperationLike;

/// Reads operands `[from, from+count)` out of the frame.
pub(crate) fn operand_values(
    frame: &Frame,
    op: OperationRef<'_, '_>,
    from: usize,
    count: usize,
) -> Result<Vec<Value>, EvalError> {
    (from..from + count)
        .map(|index| {
            let operand = op
                .operand(index)
                .map_err(|_| EvalError::Malformed(format!("missing operand {index}")))?;
            frame.get(operand)
        })
        .collect()
}

/// Reads the two operands of a binary op, insisting on matching widths.
pub(crate) fn binary_operands(
    frame: &Frame,
    op: OperationRef<'_, '_>,
) -> Result<(Value, Value), EvalError> {
    if op.operand_count() != 2 {
        return Err(EvalError::Malformed(format!(
            "binary op with {} operand(s)",
            op.operand_count()
        )));
    }
    let values = operand_values(frame, op, 0, 2)?;
    let (lhs, rhs) = (values[0], values[1]);
    if lhs.width() != rhs.width() {
        return Err(EvalError::TypeMismatch(format!(
            "i{} vs i{}",
            lhs.width(),
            rhs.width()
        )));
    }
    Ok((lhs, rhs))
}

/// Binds a single-result op's value and continues.
pub(crate) fn continue_with_result<'c, 'a>(
    frame: &mut Frame,
    op: OperationRef<'c, 'a>,
    value: Value,
) -> Result<Step<'c, 'a>, EvalError> {
    let result = op
        .result(0)
        .map_err(|_| EvalError::Malformed("op has no result 0".into()))?;
    frame.set(result.into(), value);
    Ok(Step::Continue)
}

/// Binds an N-result op's values and continues.
pub(crate) fn continue_with_results<'c, 'a>(
    frame: &mut Frame,
    op: OperationRef<'c, 'a>,
    values: &[Value],
) -> Result<Step<'c, 'a>, EvalError> {
    if op.result_count() != values.len() {
        return Err(EvalError::Malformed(format!(
            "op yields {} value(s) for {} result(s)",
            values.len(),
            op.result_count()
        )));
    }
    for (index, value) in values.iter().enumerate() {
        let result = op
            .result(index)
            .map_err(|_| EvalError::Malformed("result out of range".into()))?;
        frame.set(result.into(), *value);
    }
    Ok(Step::Continue)
}
