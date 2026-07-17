//! K2 for frk.contract: the interpreter executes EVERY narrow as a
//! real check (D-072 — reference semantics is maximal checking; the
//! promotion pass exists only on native paths, so a wrong promotion
//! shows up as a divergence against this evaluator, never as silence).

use frk_interp::eval_util::{continue_with_result, operand_values};
use frk_interp::{Eval, EvalError, Frame, Interp, Step};
use melior::ir::OperationRef;

pub fn register_eval(interp: &mut Interp<'_, '_>) {
    interp.register_eval("frk_contract.narrow", Box::new(Narrow));
}

struct Narrow;
impl Eval for Narrow {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let values = operand_values(frame, op, 0, 1)?;
        let (tag, _) = values[0].as_adt()?;
        let claimed = crate::adt::index_attr(op, "variant")
            .map_err(EvalError::Malformed)?;
        if tag != claimed {
            let blame =
                crate::contract::blame_of(op).map_err(EvalError::Malformed)?;
            return Err(EvalError::Trap(format!(
                "contract: narrowing refuted: expected variant {claimed}, got {tag} — {blame} (D-072)"
            )));
        }
        continue_with_result(frame, op, values.into_iter().next().expect("one operand"))
    }
}
