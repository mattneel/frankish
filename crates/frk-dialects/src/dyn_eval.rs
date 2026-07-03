//! K2 for frk.dyn (D-051): fat values in the reference semantics —
//! Value::Dyn(tag, payload). Unwrap on the wrong tag TRAPS with the
//! op's threaded source location (total semantics, D-029; the §6.5
//! discipline from D-050.3 applied from birth).

use frk_interp::eval_util::continue_with_result;
use frk_interp::{Eval, EvalError, Frame, Interp, Step, Value};
use melior::ir::OperationRef;
use melior::ir::operation::OperationLike;

use crate::dyn_dialect::tag_attr;

pub(crate) fn register_eval(interp: &mut Interp<'_, '_>) {
    interp.register_eval("frk_dyn.wrap", Box::new(Wrap));
    interp.register_eval("frk_dyn.unwrap", Box::new(Unwrap));
    interp.register_eval("frk_dyn.tag_of", Box::new(TagOf));
}

fn operand_value(
    frame: &Frame,
    op: OperationRef<'_, '_>,
    index: usize,
) -> Result<Value, EvalError> {
    frame.get(
        op.operand(index)
            .map_err(|_| EvalError::Malformed(format!("frk_dyn op missing operand {index}")))?,
    )
}

struct Wrap;
impl Eval for Wrap {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let tag = tag_attr(op).map_err(EvalError::Malformed)?;
        let payload = operand_value(frame, op, 0)?;
        continue_with_result(frame, op, Value::dyn_value(tag as u64, payload))
    }
}

struct Unwrap;
impl Eval for Unwrap {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let expected = tag_attr(op).map_err(EvalError::Malformed)? as u64;
        let value = operand_value(frame, op, 0)?;
        let (tag, payload) = value.as_dyn()?;
        if tag != expected {
            return Err(EvalError::Trap(format!(
                "dyn tag mismatch: expected {expected}, got {tag} (D-051) at {}",
                op.location()
            )));
        }
        continue_with_result(frame, op, payload.clone())
    }
}

struct TagOf;
impl Eval for TagOf {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let value = operand_value(frame, op, 0)?;
        let (tag, _) = value.as_dyn()?;
        continue_with_result(frame, op, Value::int(tag, 64)?)
    }
}
