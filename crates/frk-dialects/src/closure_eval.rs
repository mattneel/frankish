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
        let (callee, captures) = closure.as_closure()?;
        let args_pack = operand_value(frame, op, 1, "an args operand")?;
        let (_, args) = args_pack.as_adt()?;

        let mut call_args = Vec::with_capacity(captures.len() + args.len());
        call_args.extend(captures.iter().cloned());
        call_args.extend(args.iter().cloned());

        let callee = callee.to_string();
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
