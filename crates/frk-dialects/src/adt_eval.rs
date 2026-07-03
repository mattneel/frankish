//! K2 for frk.adt: Eval implementations plugged into the derived
//! interpreter (SPEC §3 K2 — the dialect ships its own reference
//! semantics). Runtime representation: `Value::Adt { tag, fields }`;
//! products are tag-0 adts.
//!
//! Wrong-variant extraction is a deterministic trap (D-029 spirit): the
//! decision-tree pass only emits extracts guarded by tag dispatch, so a
//! trap here indicates a compiler bug upstream of the interpreter, never
//! a legitimate program outcome.

use frk_interp::eval_util::{continue_with_result, operand_values};
use frk_interp::{Eval, EvalError, Frame, Interp, Step, Value};
use melior::ir::OperationRef;
use melior::ir::operation::OperationLike;

/// Registers the frk.adt evaluators into an interpreter. Harness runners
/// call this right after `Interp::new`.
pub fn register_eval(interp: &mut Interp<'_, '_>) {
    interp.register_eval("frk_adt.make_sum", Box::new(MakeSum));
    interp.register_eval("frk_adt.tag_of", Box::new(TagOf));
    interp.register_eval("frk_adt.extract", Box::new(Extract));
    interp.register_eval("frk_adt.make_product", Box::new(MakeProduct));
    interp.register_eval("frk_adt.get", Box::new(Get));
}

fn index_attr(op: OperationRef<'_, '_>, name: &str) -> Result<usize, EvalError> {
    crate::adt::index_attr(op, name).map_err(EvalError::Malformed)
}

struct MakeSum;
impl Eval for MakeSum {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let variant = index_attr(op, "variant")?;
        let fields = operand_values(frame, op, 0, op.operand_count())?;
        continue_with_result(frame, op, Value::adt(variant, fields))
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
        let value = frame.get(op.operand(0).map_err(|_| {
            EvalError::Malformed("frk_adt.tag_of without an operand".into())
        })?)?;
        let (tag, _) = value.as_adt()?;
        continue_with_result(frame, op, Value::int(tag as u64, 64)?)
    }
}

struct Extract;
impl Eval for Extract {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let variant = index_attr(op, "variant")?;
        let field = index_attr(op, "field")?;
        let value = frame.get(op.operand(0).map_err(|_| {
            EvalError::Malformed("frk_adt.extract without an operand".into())
        })?)?;
        let (tag, fields) = value.as_adt()?;
        if tag != variant {
            return Err(EvalError::Trap(format!(
                "frk_adt.extract: value holds variant {tag}, extract names variant {variant}"
            )));
        }
        let field_value = fields.get(field).ok_or_else(|| {
            EvalError::Malformed(format!(
                "frk_adt.extract: field {field} out of range ({} present)",
                fields.len()
            ))
        })?;
        continue_with_result(frame, op, field_value.clone())
    }
}

struct MakeProduct;
impl Eval for MakeProduct {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let fields = operand_values(frame, op, 0, op.operand_count())?;
        continue_with_result(frame, op, Value::adt(0, fields))
    }
}

struct Get;
impl Eval for Get {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let field = index_attr(op, "field")?;
        let value = frame.get(op.operand(0).map_err(|_| {
            EvalError::Malformed("frk_adt.get without an operand".into())
        })?)?;
        let (_, fields) = value.as_adt()?;
        let field_value = fields.get(field).ok_or_else(|| {
            EvalError::Malformed(format!(
                "frk_adt.get: field {field} out of range ({} present)",
                fields.len()
            ))
        })?;
        continue_with_result(frame, op, field_value.clone())
    }
}
