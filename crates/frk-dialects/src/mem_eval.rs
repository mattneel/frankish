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
    interp.register_eval("frk_mem.field_get", Box::new(FieldGet));
    interp.register_eval("frk_mem.field_set", Box::new(FieldSet));
    interp.register_eval("frk_mem.rec_ref", Box::new(RecIdentity));
    interp.register_eval("frk_mem.rec_cast", Box::new(RecIdentity));
    interp.register_eval("frk_mem.recref_null", Box::new(RecrefNull));
    interp.register_eval("frk_mem.array_new", Box::new(ArrayNew));
    interp.register_eval("frk_mem.array_get", Box::new(ArrayGet));
    interp.register_eval("frk_mem.array_set", Box::new(ArraySet));
    interp.register_eval("frk_mem.array_len", Box::new(ArrayLen));
    interp.register_eval("frk_mem.dispose", Box::new(Dispose));
}

/// Bounds discipline (D-049): OOB traps deterministically here; the
/// native path is unchecked — the corpus stays in-bounds by law.
fn index_of(value: &Value, len: usize) -> Result<usize, EvalError> {
    let index = value.as_signed()?;
    if index < 0 || index as usize >= len {
        return Err(EvalError::Trap(format!(
            "array index {index} out of bounds for length {len} (D-049)"
        )));
    }
    Ok(index as usize)
}

struct ArrayNew;
impl Eval for ArrayNew {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let len = operand_value(frame, op, 0)?.as_signed()?;
        if len < 0 {
            return Err(EvalError::Trap(format!("array_new with negative length {len}")));
        }
        // Zero-filled f64 slots by default; sets follow immediately in
        // literal lowerings. Typed zero: use Float(0) — TS-0 arrays
        // are number[]; a bool[] case would overwrite before reading.
        let items = vec![Value::float(0.0); len as usize];
        continue_with_result(frame, op, Value::array(items))
    }
}

/// Attaches the op's threaded source location (§6.5) to a trap — the
/// whole point of span threading is that THIS message points home.
fn locate(error: EvalError, op: OperationRef<'_, '_>) -> EvalError {
    match error {
        EvalError::Trap(message) => {
            EvalError::Trap(format!("{message} at {}", op.location()))
        }
        other => other,
    }
}

struct ArrayGet;
impl Eval for ArrayGet {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let array = operand_value(frame, op, 0)?;
        let items = array.as_array()?.borrow();
        let index = index_of(&operand_value(frame, op, 1)?, items.len())
            .map_err(|e| locate(e, op))?;
        let value = items[index].clone();
        drop(items);
        continue_with_result(frame, op, value)
    }
}

struct ArraySet;
impl Eval for ArraySet {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let array = operand_value(frame, op, 0)?;
        let mut items = array.as_array()?.borrow_mut();
        let index = index_of(&operand_value(frame, op, 1)?, items.len())
            .map_err(|e| locate(e, op))?;
        items[index] = operand_value(frame, op, 2)?;
        drop(items);
        continue_with_results(frame, op, &[])
    }
}

struct ArrayLen;
impl Eval for ArrayLen {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let array = operand_value(frame, op, 0)?;
        let len = array.as_array()?.borrow().len() as u64;
        continue_with_result(frame, op, Value::int(len, 64)?)
    }
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

/// Field-granular record reads (D-073): the box holds a product
/// (tag-0 adt); project the named field out of the shared cell.
struct FieldGet;
impl Eval for FieldGet {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let boxed = operand_value(frame, op, 0)?;
        let field = crate::adt::index_attr(op, "field").map_err(EvalError::Malformed)?;
        let cell = boxed.as_box()?;
        let value = {
            let record = cell.borrow();
            let (_, fields) = record.as_adt()?;
            fields
                .get(field)
                .cloned()
                .ok_or_else(|| {
                    EvalError::Malformed(format!("field {field} out of range"))
                })?
        };
        continue_with_result(frame, op, value)
    }
}

/// Field-granular record writes (D-073): replace one field in place;
/// aliases of the box observe the write.
struct FieldSet;
impl Eval for FieldSet {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let boxed = operand_value(frame, op, 0)?;
        let value = operand_value(frame, op, 1)?;
        let field = crate::adt::index_attr(op, "field").map_err(EvalError::Malformed)?;
        let cell = boxed.as_box()?;
        {
            let mut record = cell.borrow_mut();
            let (tag, fields) = record.as_adt()?;
            let mut fields = fields.to_vec();
            if field >= fields.len() {
                return Err(EvalError::Malformed(format!("field {field} out of range")));
            }
            fields[field] = value;
            *record = Value::adt(tag, fields);
        }
        continue_with_results(frame, op, &[])
    }
}

/// D-074: type-erased record references. Both directions are pure
/// identity — the value IS the shared box; erasure is a static-type
/// fact only, so object identity survives the knot.
struct RecIdentity;
impl Eval for RecIdentity {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let value = operand_value(frame, op, 0)?;
        value.as_box()?; // must be a record either way
        continue_with_result(frame, op, value)
    }
}

/// D-074 construction knot: the placeholder a self-referential slot
/// holds between box_new and the immediate rec_ref back-patch.
/// Reading one (via rec_cast → as_box) errors — a frontend bug, never
/// a program outcome.
struct RecrefNull;
impl Eval for RecrefNull {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        // A zero word: not a box, so any read through it errors.
        continue_with_result(frame, op, Value::Int { bits: 0, width: 64 })
    }
}

/// D-067: end-of-ownership marker. The reference interpreter does not
/// count references, so dispose is a semantic no-op — its meaning is
/// entirely a strategy-lowering fact (Rc: release; Arena: erased).
struct Dispose;
impl Eval for Dispose {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        // Validate the operand is bound (catches malformed IR), then continue.
        operand_value(frame, op, 0)?;
        continue_with_results(frame, op, &[])
    }
}
