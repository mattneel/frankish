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
    interp.register_eval("frk_ctl.handle", Box::new(Handle));
    interp.register_eval("frk_ctl.perform", Box::new(Perform));
    interp.register_eval("frk_ctl.resume", Box::new(Resume));
}

/// The synthesized resumer's callee name: κ is BORN UNIFORM (D-069) —
/// a closure over its marker whose application marks-or-traps and
/// returns its pack. The closure Apply path special-cases this name;
/// the native lowering synthesizes the matching thunk.
pub const RESUMER: &str = "__frk_ctl_resume__";

fn label_of(op: OperationRef<'_, '_>) -> Result<String, EvalError> {
    let attribute = op
        .attribute("label")
        .map_err(|_| EvalError::Malformed("frk_ctl op without a label".into()))?;
    crate::attr_util::string_attr_bytes(attribute)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .map_err(EvalError::Malformed)
}

/// Applies a UNIFORM clause closure to a pack value (mirrors
/// closure_eval::Apply's calling convention).
fn apply_uniform(
    interp: &Interp<'_, '_>,
    clause: &Value,
    pack: Value,
) -> Result<Value, EvalError> {
    let (callee, captures) = clause.as_closure()?;
    // Both conventions, mirroring closure_eval::Apply (D-063): a
    // uniform callee takes (envref = the closure value, pack); a
    // legacy callee takes (captures…, pack).
    let uniform = interp
        .function_input_types(callee)
        .and_then(|inputs| inputs.first().cloned())
        .is_some_and(|first| first == "!frk_closure.envref");
    let call_args = if uniform {
        vec![clause.clone(), pack]
    } else {
        let mut v = Vec::with_capacity(captures.len() + 1);
        v.extend(captures.iter().cloned());
        v.push(pack);
        v
    };
    let callee = callee.to_string();
    let results = interp.eval_function(&callee, &call_args)?;
    let [result] = results.as_slice() else {
        return Err(EvalError::Malformed(format!(
            "@{callee} returned {} value(s); closures return exactly one (D-036)",
            results.len()
        )));
    };
    Ok(result.clone())
}

/// pack[0] with nil-fill — the perform-site read of a clause's return.
fn pack_head(pack: &Value) -> Result<Value, EvalError> {
    let items = pack.as_array()?.borrow();
    Ok(items
        .first()
        .cloned()
        .unwrap_or(Value::dyn_value(0, Value::int(0, 64)?)))
}

struct Handle;
impl Eval for Handle {
    fn eval<'c, 'a>(
        &self,
        interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        // handle = prompt PLUS handler registration (D-069): the body
        // still receives the token, so escapes compose with v0.
        let label = label_of(op)?;
        let clause = operand_value(frame, op, 0)?;
        let body = operand_value(frame, op, 1)?;
        let (callee, captures) = body.as_closure()?;
        let token = interp.ctl_push_prompt();
        interp.ctl_push_handler(&label, clause, token);
        let mut call_args = Vec::with_capacity(captures.len() + 1);
        call_args.extend(captures.iter().cloned());
        call_args.push(Value::int(token as u64, 64)?);
        let callee = callee.to_string();
        let outcome = interp.eval_function(&callee, &call_args);
        interp.ctl_pop_handler(token);
        interp.ctl_pop_prompt(token);
        let value = match outcome {
            Ok(results) => {
                let [result] = results.as_slice() else {
                    return Err(EvalError::Malformed(format!(
                        "handle body @{callee} returned {} value(s)",
                        results.len()
                    )));
                };
                result.clone()
            }
            // An abortive clause (or a v0 escape) targeting THIS
            // handle: yield the parked value.
            Err(EvalError::Abort { token: t }) if t == token => {
                interp.ctl_take_aborted()?
            }
            Err(other) => return Err(other),
        };
        continue_with_result(frame, op, value)
    }
}

struct Perform;
impl Eval for Perform {
    fn eval<'c, 'a>(
        &self,
        interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let label = label_of(op)?;
        let value = operand_value(frame, op, 0)?;
        let Some((index, clause, token)) = interp.ctl_find_and_mask(&label) else {
            return Err(EvalError::Trap(format!(
                "unhandled effect \"{label}\" (κ_frk)"
            )));
        };
        // κ: born uniform — a closure over a fresh one-shot marker.
        let marker = interp.ctl_new_marker();
        let kappa = Value::dyn_value(
            5, // TAG_FUN
            Value::closure(RESUMER, vec![Value::int(marker as u64, 64)?]),
        );
        let pack = Value::array(vec![value, kappa]);
        // The clause runs AT THE PERFORM SITE with H masked; the mask
        // lifting afterwards is the deep reinstall (κ_frk v1).
        let outcome = apply_uniform(interp, &clause, pack);
        interp.ctl_unmask(index);
        let clause_pack = outcome?;
        let result = pack_head(&clause_pack)?;
        if interp.ctl_marker_consumed(marker) {
            // Tail-resume: the clause's return IS the resume value;
            // the body continues under the handler.
            continue_with_result(frame, op, result)
        } else {
            // Abortive: the handle yields the clause's value.
            interp.ctl_set_aborted(result);
            Err(EvalError::Abort { token })
        }
    }
}

struct Resume;
impl Eval for Resume {
    fn eval<'c, 'a>(
        &self,
        interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let marker = operand_value(frame, op, 0)?.as_signed()?;
        let value = operand_value(frame, op, 1)?;
        interp.ctl_consume_marker(marker)?;
        continue_with_result(frame, op, value)
    }
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
