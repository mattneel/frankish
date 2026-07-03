//! func: return and direct calls. Calls re-enter the interpreter's
//! function machinery, which owns the depth guard (D-029).

use std::collections::HashMap;

use melior::ir::OperationRef;
use melior::ir::attribute::FlatSymbolRefAttribute;
use melior::ir::operation::OperationLike;

use super::{continue_with_results, operand_values};
use crate::error::EvalError;
use crate::interp::{Eval, Frame, Interp, Step};

pub(crate) fn register(registry: &mut HashMap<&'static str, Box<dyn Eval>>) {
    registry.insert("func.return", Box::new(Return));
    registry.insert("func.call", Box::new(Call));
}

struct Return;
impl Eval for Return {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        Ok(Step::Return(operand_values(
            frame,
            op,
            0,
            op.operand_count(),
        )?))
    }
}

struct Call;
impl Eval for Call {
    fn eval<'c, 'a>(
        &self,
        interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let callee = op
            .attribute("callee")
            .ok()
            .and_then(|attribute| FlatSymbolRefAttribute::try_from(attribute).ok())
            .ok_or_else(|| EvalError::Malformed("func.call without callee".into()))?
            .value()
            .to_string();
        let arguments = operand_values(frame, op, 0, op.operand_count())?;
        let results = interp.eval_function(&callee, &arguments)?;
        continue_with_results(frame, op, &results)
    }
}
