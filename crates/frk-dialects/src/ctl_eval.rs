//! Reference semantics for frk_ctl (κ_frk §2). The interpreter is
//! the oracle: aborts REALLY unwind (EvalError::Abort threads up the
//! frame stack), prompts catch exactly their own token, and both κ_frk
//! traps carry the calculus's wording.

use frk_interp::eval_util::continue_with_result;
use frk_interp::{Eval, EvalError, Frame, Interp, Step, Value};
use melior::ir::OperationRef;
use melior::ir::operation::OperationLike;

pub(crate) fn register_eval(interp: &mut Interp<'_, '_>) {
    interp.register_eval("frk_ctl.prompt", Box::new(Prompt));
    interp.register_eval("frk_ctl.abort", Box::new(Abort));
    interp.register_eval("frk_ctl.pending", Box::new(Pending));
}

fn operand_value(
    frame: &Frame,
    op: OperationRef<'_, '_>,
    index: usize,
) -> Result<Value, EvalError> {
    frame.get(
        op.operand(index)
            .map_err(|_| EvalError::Malformed(format!("frk_ctl op missing operand {index}")))?,
    )
}

struct Prompt;
impl Eval for Prompt {
    fn eval<'c, 'a>(
        &self,
        interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let closure = operand_value(frame, op, 0)?;
        let (callee, captures) = closure.as_closure()?;
        let token = interp.ctl_push_prompt();
        let mut call_args = Vec::with_capacity(captures.len() + 1);
        call_args.extend(captures.iter().cloned());
        call_args.push(Value::int(token as u64, 64)?);
        let callee = callee.to_string();
        let outcome = interp.eval_function(&callee, &call_args);
        interp.ctl_pop_prompt(token);
        let value = match outcome {
            Ok(results) => {
                let [result] = results.as_slice() else {
                    return Err(EvalError::Malformed(format!(
                        "prompt body @{callee} returned {} value(s)",
                        results.len()
                    )));
                };
                result.clone()
            }
            // H-op-drop, landed: an abort aimed at THIS prompt. The
            // value was parked by `abort`; collect it.
            Err(EvalError::Abort { token: t }) if t == token => {
                interp.ctl_take_aborted()?
            }
            Err(other) => return Err(other),
        };
        continue_with_result(frame, op, value)
    }
}

struct Abort;
impl Eval for Abort {
    fn eval<'c, 'a>(
        &self,
        interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let token = operand_value(frame, op, 0)?.as_signed()?;
        let value = operand_value(frame, op, 1)?;
        if !interp.ctl_prompt_live(token) {
            return Err(EvalError::Trap("escape past extent (κ_frk)".into()));
        }
        interp.ctl_set_aborted(value);
        Err(EvalError::Abort { token })
    }
}

struct Pending;
impl Eval for Pending {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        // Real unwinds never re-enter a frame: the check is dead in
        // the reference semantics by construction (κ_frk §3).
        let result = op
            .result(0)
            .map_err(|_| EvalError::Malformed("pending without a result".into()))?;
        frame.set(result.into(), Value::int(0, 64)?);
        Ok(Step::Continue)
    }
}
