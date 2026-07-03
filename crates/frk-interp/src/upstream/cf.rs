//! cf: unstructured branches. Successor operand splitting needs no
//! segment attribute: each destination's block-argument count determines
//! its share of the operand list.

use std::collections::HashMap;

use melior::ir::operation::OperationLike;
use melior::ir::{BlockLike, OperationRef};

use super::operand_values;
use crate::error::EvalError;
use crate::interp::{Eval, Frame, Interp, Step};

pub(crate) fn register(registry: &mut HashMap<&'static str, Box<dyn Eval>>) {
    registry.insert("cf.br", Box::new(Branch));
    registry.insert("cf.cond_br", Box::new(CondBranch));
}

struct Branch;
impl Eval for Branch {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let target = op
            .successor(0)
            .map_err(|_| EvalError::Malformed("cf.br without successor".into()))?;
        let args = operand_values(frame, op, 0, op.operand_count())?;
        if args.len() != target.argument_count() {
            return Err(EvalError::Malformed(
                "cf.br operand count != successor argument count".into(),
            ));
        }
        Ok(Step::Branch(target, args))
    }
}

struct CondBranch;
impl Eval for CondBranch {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let on_true = op
            .successor(0)
            .map_err(|_| EvalError::Malformed("cf.cond_br without true successor".into()))?;
        let on_false = op
            .successor(1)
            .map_err(|_| EvalError::Malformed("cf.cond_br without false successor".into()))?;

        let true_count = on_true.argument_count();
        let false_count = on_false.argument_count();
        if op.operand_count() != 1 + true_count + false_count {
            return Err(EvalError::Malformed(format!(
                "cf.cond_br has {} operand(s); successors expect 1+{}+{}",
                op.operand_count(),
                true_count,
                false_count
            )));
        }

        let condition = frame
            .get(op.operand(0).map_err(|_| {
                EvalError::Malformed("cf.cond_br without condition".into())
            })?)?
            .as_bool()?;

        if condition {
            Ok(Step::Branch(
                on_true,
                operand_values(frame, op, 1, true_count)?,
            ))
        } else {
            Ok(Step::Branch(
                on_false,
                operand_values(frame, op, 1 + true_count, false_count)?,
            ))
        }
    }
}
