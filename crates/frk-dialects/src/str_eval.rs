//! K2 for frk.str (D-049): UTF-16 code-unit semantics, immutable
//! values, structural equality.

use frk_interp::eval_util::continue_with_result;
use frk_interp::{Eval, EvalError, Frame, Interp, Step, Value};
use melior::ir::OperationRef;
use melior::ir::operation::OperationLike;

pub(crate) fn register_eval(interp: &mut Interp<'_, '_>) {
    interp.register_eval("frk_str.lit", Box::new(Lit));
    interp.register_eval("frk_str.concat", Box::new(Concat));
    interp.register_eval("frk_str.eq", Box::new(Eq));
    interp.register_eval("frk_str.len", Box::new(Len));
}

fn operand_value(
    frame: &Frame,
    op: OperationRef<'_, '_>,
    index: usize,
) -> Result<Value, EvalError> {
    frame.get(
        op.operand(index)
            .map_err(|_| EvalError::Malformed(format!("frk_str op missing operand {index}")))?,
    )
}

struct Lit;
impl Eval for Lit {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let attribute = op
            .attribute("text")
            .map_err(|_| EvalError::Malformed("frk_str.lit without text".into()))?;
        let bytes = crate::attr_util::string_attr_bytes(attribute)
            .map_err(EvalError::Malformed)?;
        let text = String::from_utf8(bytes)
            .map_err(|_| EvalError::Malformed("non-UTF-8 str literal".into()))?;
        continue_with_result(frame, op, Value::str_from(&text))
    }
}

struct Concat;
impl Eval for Concat {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let lhs = operand_value(frame, op, 0)?;
        let rhs = operand_value(frame, op, 1)?;
        let mut units = lhs.as_str_units()?.as_ref().clone();
        units.extend_from_slice(rhs.as_str_units()?);
        continue_with_result(frame, op, Value::Str(std::rc::Rc::new(units)))
    }
}

struct Eq;
impl Eval for Eq {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let lhs = operand_value(frame, op, 0)?;
        let rhs = operand_value(frame, op, 1)?;
        let equal = lhs.as_str_units()? == rhs.as_str_units()?;
        continue_with_result(frame, op, Value::bool(equal))
    }
}

struct Len;
impl Eval for Len {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let value = operand_value(frame, op, 0)?;
        let len = value.as_str_units()?.len() as u64;
        continue_with_result(frame, op, Value::int(len, 64)?)
    }
}
