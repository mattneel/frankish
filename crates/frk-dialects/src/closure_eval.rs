//! K2 for frk.closure: Eval implementations (SPEC §3 K2). Runtime
//! representation: `Value::Closure { callee, captures }` — captures
//! snapshot the env product's fields by value at make time (D-035).
//! Apply re-enters the interpreter's function machinery (which owns the
//! D-029 depth guard) with `captures ++ unpacked args`.

use frk_interp::eval_util::continue_with_result;
use frk_interp::{Eval, EvalError, Frame, Interp, Step, Value};
use melior::ir::OperationRef;
use melior::ir::operation::OperationLike;

pub(crate) fn register_eval(interp: &mut Interp<'_, '_>) {
    interp.register_eval("frk_closure.make", Box::new(Make));
    interp.register_eval("frk_closure.apply", Box::new(Apply));
    interp.register_eval("frk_closure.env_load", Box::new(EnvLoad));
}

fn operand_value(
    frame: &Frame,
    op: OperationRef<'_, '_>,
    index: usize,
    what: &str,
) -> Result<Value, EvalError> {
    frame.get(
        op.operand(index)
            .map_err(|_| EvalError::Malformed(format!("frk_closure op without {what}")))?,
    )
}

struct Make;
impl Eval for Make {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let callee = crate::closure::callee_name(op).map_err(EvalError::Malformed)?;
        let env = operand_value(frame, op, 0, "an env operand")?;
        let (_, captures) = env.as_adt()?;
        continue_with_result(frame, op, Value::closure(callee, captures.to_vec()))
    }
}

struct Apply;
impl Eval for Apply {
    fn eval<'c, 'a>(
        &self,
        interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let closure = operand_value(frame, op, 0, "a closure operand")?;
        let args_pack = operand_value(frame, op, 1, "an args operand")?;
        let call_args;
        let callee;
        {
            let (name, captures) = closure.as_closure()?;
            let (_, args) = args_pack.as_adt()?;

            // κ_frk v1 (D-069): applying a RESUMER marks its one-shot
            // marker (second application traps) and returns the
            // received pack — the identity-on-pack thunk, mirrored by
            // the native lowering's synthesized resumer. Checked
            // before the convention probe: the resumer has no
            // func.func body.
            if name == crate::ctl_eval::RESUMER {
                let marker = captures
                    .first()
                    .ok_or_else(|| {
                        EvalError::Malformed("resumer without a marker capture".into())
                    })?
                    .as_signed()?;
                interp.ctl_consume_marker(marker)?;
                let [pack] = args else {
                    return Err(EvalError::Malformed(
                        "resumer applied without exactly one pack".into(),
                    ));
                };
                let pack = pack.clone();
                return continue_with_result(frame, op, pack);
            }
            // Two conventions (D-063): a UNIFORM callee (first input is
            // the envref) receives the closure value itself as its env —
            // env_load then reads captures out of it. A legacy callee
            // receives the captures unpacked as leading arguments.
            let uniform = interp
                .function_input_types(name)
                .and_then(|inputs| inputs.first().cloned())
                .is_some_and(|first| first == "!frk_closure.envref");
            call_args = if uniform {
                let mut v = Vec::with_capacity(1 + args.len());
                v.push(closure.clone());
                v.extend(args.iter().cloned());
                v
            } else {
                let mut v = Vec::with_capacity(captures.len() + args.len());
                v.extend(captures.iter().cloned());
                v.extend(args.iter().cloned());
                v
            };
            callee = name.to_string();
        }

        // The tail shape (D-063, generalizing M14): an apply whose sole
        // result feeds the immediately following func.return REPLACES
        // the frame — Step::TailCall rides the same trampoline as
        // func.call tails, so deep closure recursion runs at one depth
        // unit. Works for both conventions and every frontend.
        if apply_is_tail(op) {
            return Ok(Step::TailCall(callee, call_args));
        }

        let results = interp.eval_function(&callee, &call_args)?;
        let [result] = results.as_slice() else {
            return Err(EvalError::Malformed(format!(
                "@{callee} returned {} value(s); closures return exactly one (D-036)",
                results.len()
            )));
        };
        continue_with_result(frame, op, result.clone())
    }
}

/// apply's single result is exactly the operand of the immediately
/// following func.return.
fn apply_is_tail(op: OperationRef<'_, '_>) -> bool {
    let Some(following) = op.next_in_block() else {
        return false;
    };
    let is_return = following
        .name()
        .as_string_ref()
        .as_str()
        .is_ok_and(|name| name == "func.return");
    if !is_return || op.result_count() != 1 || following.operand_count() != 1 {
        return false;
    }
    match (op.result(0), following.operand(0)) {
        (Ok(result), Ok(operand)) => {
            use melior::ir::ValueLike;
            result.to_raw().ptr == operand.to_raw().ptr
        }
        _ => false,
    }
}

/// env_load under the uniform convention: the envref argument IS the
/// closure value; the field is its capture at `index`.
struct EnvLoad;
impl Eval for EnvLoad {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let env = operand_value(frame, op, 0, "an env operand")?;
        let (_, captures) = env.as_closure()?;
        let index = crate::adt::index_attr(op, "index").map_err(EvalError::Malformed)?;
        let value = captures.get(index).cloned().ok_or_else(|| {
            EvalError::Malformed(format!(
                "env_load index {index} out of range for {} capture(s)",
                captures.len()
            ))
        })?;
        continue_with_result(frame, op, value)
    }
}
