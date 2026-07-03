//! K2 for frk.bstr (D-056): reference semantics uses Value::Bytes
//! with CONTENT equality — observably identical to the native path's
//! interned pointer identity (intern ⇒ ptr-eq ⟺ content-eq), so the
//! interpreter needs no intern table. Deliberate; noted in D-056.

use frk_interp::eval_util::continue_with_result;
use frk_interp::{Eval, EvalError, Frame, Interp, Step, Value};
use melior::ir::OperationRef;
use melior::ir::attribute::StringAttribute;
use melior::ir::operation::OperationLike;

pub(crate) fn register_eval(interp: &mut Interp<'_, '_>) {
    interp.register_eval("frk_bstr.lit", Box::new(Lit));
    interp.register_eval("frk_bstr.concat", Box::new(Concat));
    interp.register_eval("frk_bstr.eq", Box::new(Eq));
    interp.register_eval("frk_bstr.len", Box::new(Len));
}

fn operand_value(
    frame: &Frame,
    op: OperationRef<'_, '_>,
    index: usize,
) -> Result<Value, EvalError> {
    frame.get(
        op.operand(index)
            .map_err(|_| EvalError::Malformed(format!("frk_bstr op missing operand {index}")))?,
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
        let text = op
            .attribute("text")
            .ok()
            .and_then(|attribute| StringAttribute::try_from(attribute).ok())
            .ok_or_else(|| EvalError::Malformed("frk_bstr.lit without text".into()))?
            .value()
            .to_string();
        continue_with_result(frame, op, Value::bytes(text.into_bytes()))
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
        let mut bytes = lhs.as_bytes()?.as_ref().clone();
        bytes.extend_from_slice(rhs.as_bytes()?);
        continue_with_result(frame, op, Value::bytes(bytes))
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
        let equal = lhs.as_bytes()? == rhs.as_bytes()?;
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
        let len = value.as_bytes()?.len() as u64;
        continue_with_result(frame, op, Value::int(len, 64)?)
    }
}
