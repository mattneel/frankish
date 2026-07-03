//! K2 for frk.mem: strategy-agnostic reference semantics (D-041) —
//! boxes are shared mutable cells (`Value::Box`), whatever the lowering
//! later does about their storage.

use frk_interp::eval_util::{continue_with_result, continue_with_results};
use frk_interp::{Eval, EvalError, Frame, Interp, Step, Value};
use melior::ir::OperationRef;
use melior::ir::operation::OperationLike;

pub(crate) fn register_eval(interp: &mut Interp<'_, '_>) {
    interp.register_eval("frk_mem.box_new", Box::new(BoxNew));
    interp.register_eval("frk_mem.box_get", Box::new(BoxGet));
    interp.register_eval("frk_mem.box_set", Box::new(BoxSet));
}

fn operand_value(
    frame: &Frame,
    op: OperationRef<'_, '_>,
    index: usize,
) -> Result<Value, EvalError> {
    frame.get(
        op.operand(index)
            .map_err(|_| EvalError::Malformed(format!("frk_mem op missing operand {index}")))?,
    )
}

struct BoxNew;
impl Eval for BoxNew {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let value = operand_value(frame, op, 0)?;
        continue_with_result(frame, op, Value::boxed(value))
    }
}

struct BoxGet;
impl Eval for BoxGet {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let boxed = operand_value(frame, op, 0)?;
        let cell = boxed.as_box()?;
        let value = cell.borrow().clone();
        continue_with_result(frame, op, value)
    }
}

struct BoxSet;
impl Eval for BoxSet {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let boxed = operand_value(frame, op, 0)?;
        let value = operand_value(frame, op, 1)?;
        *boxed.as_box()?.borrow_mut() = value;
        continue_with_results(frame, op, &[])
    }
}
