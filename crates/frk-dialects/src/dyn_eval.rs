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
    interp.register_eval("frk_dyn.table_new", Box::new(TableNew));
    interp.register_eval("frk_dyn.raw_get", Box::new(RawGet));
    interp.register_eval("frk_dyn.raw_set", Box::new(RawSet));
    interp.register_eval("frk_dyn.table_len", Box::new(TableLen));
    interp.register_eval("frk_dyn.set_meta", Box::new(SetMeta));
    interp.register_eval("frk_dyn.get_meta", Box::new(GetMeta));
    interp.register_eval("frk_dyn.payload_word", Box::new(PayloadWord));
    interp.register_eval("frk_dyn.table_next", Box::new(TableNext));
    interp.register_eval("frk_dyn.iface_make", Box::new(IfaceMake));
    interp.register_eval("frk_dyn.iface_call", Box::new(IfaceCall));
}

/// Iteration for pairs/next (D-058): nil key → first entry; else the
/// entry AFTER the given key. Order here is INSERTION order; the
/// native path iterates slot order — both are legal Lua (pairs order
/// is implementation-defined), and the canon rule (D-058) keeps
/// corpus output order-independent.
/// Structural interfaces (D-075), REFERENCE representation: the
/// interface value is a DICTIONARY — a product of bound closures, one
/// per method, each capturing the object. Native lowers the same ops
/// to a Go-style itab; the differential matrix arbitrates.
struct IfaceMake;
impl Eval for IfaceMake {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let object = frame
            .get(op.operand(0).map_err(|_| {
                EvalError::Malformed("iface_make without an object".into())
            })?.into())?;
        let methods =
            crate::dyn_dialect::iface_methods(op).map_err(EvalError::Malformed)?;
        let dictionary: Vec<Value> = methods
            .into_iter()
            .map(|symbol| Value::closure(symbol, vec![object.clone()]))
            .collect();
        continue_with_result(frame, op, Value::adt(0, dictionary))
    }
}

struct IfaceCall;
impl Eval for IfaceCall {
    fn eval<'c, 'a>(
        &self,
        interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let iface = frame
            .get(op.operand(0).map_err(|_| {
                EvalError::Malformed("iface_call without an iface".into())
            })?.into())?;
        let args_pack = frame
            .get(op.operand(1).map_err(|_| {
                EvalError::Malformed("iface_call without args".into())
            })?.into())?;
        let method = crate::adt::index_attr(op, "method").map_err(EvalError::Malformed)?;
        let (_, dictionary) = iface.as_adt()?;
        let bound = dictionary.get(method).ok_or_else(|| {
            EvalError::Malformed(format!("method index {method} out of range"))
        })?;
        let (callee, captures) = bound.as_closure()?;
        let (_, args) = args_pack.as_adt()?;
        let mut call_args = Vec::with_capacity(captures.len() + args.len());
        call_args.extend(captures.iter().cloned());
        call_args.extend(args.iter().cloned());
        let results = interp.eval_function(callee, &call_args)?;
        let [result] = results.as_slice() else {
            return Err(EvalError::Malformed(format!(
                "@{callee} returned {} value(s); iface_call yields exactly one (D-075)",
                results.len()
            )));
        };
        continue_with_result(frame, op, result.clone())
    }
}

struct TableNext;
impl Eval for TableNext {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let table = table_of(&operand_value(frame, op, 0)?, op)?;
        let key = operand_value(frame, op, 1)?;
        let (key_tag, _) = key.as_dyn()?;
        let data = table.borrow();
        let start = if key_tag == crate::dyn_dialect::TAG_NIL as u64 {
            0
        } else {
            match data.entries.iter().position(|(k, _)| *k == key) {
                Some(index) => index + 1,
                None => {
                    return Err(EvalError::Trap(format!(
                        "invalid key to next at {}",
                        op.location()
                    )));
                }
            }
        };
        let (next_key, next_value) = match data.entries.get(start) {
            Some((k, v)) => (k.clone(), v.clone()),
            None => (nil_dyn(), nil_dyn()),
        };
        drop(data);
        frk_interp::eval_util::continue_with_results(frame, op, &[next_key, next_value])
    }
}

/// Raw payload word for IDENTITY comparison (D-056; __lua_eq's
/// table arm). The numeric value is meaningless outside equality:
/// reference types yield a stable per-object address, everything
/// else the address of the payload cell.
struct PayloadWord;
impl Eval for PayloadWord {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let value = operand_value(frame, op, 0)?;
        let (_, payload) = value.as_dyn()?;
        let word = match payload {
            Value::Table(table) => std::rc::Rc::as_ptr(table) as usize as u64,
            other => other as *const Value as usize as u64,
        };
        continue_with_result(frame, op, Value::int(word, 64)?)
    }
}

fn nil_dyn() -> Value {
    Value::dyn_value(0, Value::int(0, 64).expect("nil payload"))
}

/// Projects a dyn operand to its table, with a located trap otherwise
/// (Lua "attempt to index a non-table" semantics, D-052 fences).
fn table_of(
    value: &Value,
    op: OperationRef<'_, '_>,
) -> Result<std::rc::Rc<std::cell::RefCell<frk_interp::TableData>>, EvalError> {
    let (tag, payload) = value.as_dyn()?;
    if tag != crate::dyn_dialect::TAG_TABLE as u64 {
        return Err(EvalError::Trap(format!(
            "attempt to index a non-table (tag {tag}) at {}",
            op.location()
        )));
    }
    Ok(payload.as_table()?.clone())
}

struct TableNew;
impl Eval for TableNew {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        continue_with_result(
            frame,
            op,
            Value::dyn_value(crate::dyn_dialect::TAG_TABLE as u64, Value::table()),
        )
    }
}

struct RawGet;
impl Eval for RawGet {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let table = table_of(&operand_value(frame, op, 0)?, op)?;
        let key = operand_value(frame, op, 1)?;
        let entries = table.borrow();
        let found = entries
            .entries
            .iter()
            .find(|(existing, _)| *existing == key)
            .map(|(_, value)| value.clone())
            .unwrap_or_else(nil_dyn);
        drop(entries);
        continue_with_result(frame, op, found)
    }
}

struct RawSet;
impl Eval for RawSet {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let table = table_of(&operand_value(frame, op, 0)?, op)?;
        let key = operand_value(frame, op, 1)?;
        let (key_tag, _) = key.as_dyn()?;
        if key_tag == crate::dyn_dialect::TAG_NIL as u64 {
            return Err(EvalError::Trap(format!(
                "table index is nil at {}",
                op.location()
            )));
        }
        let value = operand_value(frame, op, 2)?;
        let (value_tag, _) = value.as_dyn()?;
        let mut data = table.borrow_mut();
        let existing = data.entries.iter().position(|(k, _)| *k == key);
        if value_tag == crate::dyn_dialect::TAG_NIL as u64 {
            // Lua: assigning nil DELETES the key.
            if let Some(index) = existing {
                data.entries.remove(index);
            }
        } else if let Some(index) = existing {
            data.entries[index].1 = value;
        } else {
            data.entries.push((key, value));
        }
        drop(data);
        frk_interp::eval_util::continue_with_results(frame, op, &[])
    }
}

struct TableLen;
impl Eval for TableLen {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let table = table_of(&operand_value(frame, op, 0)?, op)?;
        let data = table.borrow();
        // The border probe (D-056): # = largest n with t[1..n] all
        // present — pure-hash representation, O(n) probe, corpus scale.
        let mut length: u64 = 0;
        loop {
            let probe = Value::dyn_value(
                crate::dyn_dialect::TAG_NUM as u64,
                Value::float((length + 1) as f64),
            );
            if data.entries.iter().any(|(k, _)| *k == probe) {
                length += 1;
            } else {
                break;
            }
        }
        drop(data);
        continue_with_result(frame, op, Value::int(length, 64)?)
    }
}

struct SetMeta;
impl Eval for SetMeta {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let table = table_of(&operand_value(frame, op, 0)?, op)?;
        let meta = operand_value(frame, op, 1)?;
        let (meta_tag, _) = meta.as_dyn()?;
        table.borrow_mut().meta = if meta_tag == crate::dyn_dialect::TAG_NIL as u64 {
            None
        } else {
            Some(meta)
        };
        frk_interp::eval_util::continue_with_results(frame, op, &[])
    }
}

struct GetMeta;
impl Eval for GetMeta {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let table = table_of(&operand_value(frame, op, 0)?, op)?;
        let meta = table.borrow().meta.clone().unwrap_or_else(nil_dyn);
        continue_with_result(frame, op, meta)
    }
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
