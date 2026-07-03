//! K2 for frk.bstr (D-056): reference semantics uses Value::Bytes
//! with CONTENT equality — observably identical to the native path's
//! interned pointer identity (intern ⇒ ptr-eq ⟺ content-eq), so the
//! interpreter needs no intern table. Deliberate; noted in D-056.

use frk_interp::eval_util::continue_with_result;
use frk_interp::{Eval, EvalError, Frame, Interp, Step, Value};
use melior::ir::OperationRef;
use melior::ir::operation::OperationLike;

pub(crate) fn register_eval(interp: &mut Interp<'_, '_>) {
    interp.register_eval("frk_bstr.lit", Box::new(Lit));
    interp.register_eval("frk_bstr.concat", Box::new(Concat));
    interp.register_eval("frk_bstr.eq", Box::new(Eq));
    interp.register_eval("frk_bstr.len", Box::new(Len));
    interp.register_eval("frk_bstr.sub", Box::new(Sub));
    interp.register_eval("frk_bstr.rep", Box::new(Rep));
}

/// Lua string.sub semantics (D-058): 1-based, negative counts from
/// the end, clamped; empty when the range inverts.
pub(crate) fn sub_range(len: usize, from: i64, to: i64) -> (usize, usize) {
    let len = len as i64;
    let mut i = if from < 0 { len + from + 1 } else { from };
    let mut j = if to < 0 { len + to + 1 } else { to };
    if i < 1 {
        i = 1;
    }
    if j > len {
        j = len;
    }
    if i > j {
        (0, 0)
    } else {
        ((i - 1) as usize, j as usize)
    }
}

struct Sub;
impl Eval for Sub {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let value = operand_value(frame, op, 0)?;
        let bytes = value.as_bytes()?;
        let from = operand_value(frame, op, 1)?.as_signed()?;
        let to = operand_value(frame, op, 2)?.as_signed()?;
        let (start, end) = sub_range(bytes.len(), from, to);
        continue_with_result(frame, op, Value::bytes(bytes[start..end].to_vec()))
    }
}

struct Rep;
impl Eval for Rep {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let value = operand_value(frame, op, 0)?;
        let bytes = value.as_bytes()?;
        let count = operand_value(frame, op, 1)?.as_signed()?.max(0) as usize;
        let mut out = Vec::with_capacity(bytes.len() * count);
        for _ in 0..count {
            out.extend_from_slice(bytes);
        }
        continue_with_result(frame, op, Value::bytes(out))
    }
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
        let attribute = op
            .attribute("text")
            .map_err(|_| EvalError::Malformed("frk_bstr.lit without text".into()))?;
        let bytes = crate::attr_util::string_attr_bytes(attribute)
            .map_err(EvalError::Malformed)?;
        continue_with_result(frame, op, Value::bytes(bytes))
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
