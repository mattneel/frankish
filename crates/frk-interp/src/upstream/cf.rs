//! cf: unstructured branches. Successor operand splitting needs no
//! segment attribute: each destination's block-argument count determines
//! its share of the operand list (operands are ordered [flag/cond]
//! ++ successor0's ++ successor1's ++ …, matching successor order).

use std::collections::HashMap;

use melior::ir::attribute::DenseElementsAttribute;
use melior::ir::operation::OperationLike;
use melior::ir::{BlockLike, OperationRef};

use super::operand_values;
use crate::error::EvalError;
use crate::interp::{Eval, Frame, Interp, Step};

pub(crate) fn register(registry: &mut HashMap<&'static str, Box<dyn Eval>>) {
    registry.insert("cf.br", Box::new(Branch));
    registry.insert("cf.cond_br", Box::new(CondBranch));
    registry.insert("cf.switch", Box::new(Switch));
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

/// cf.switch: successor 0 is the default; case i maps to successor i+1
/// with case_values (a dense int elements attribute) supplying the
/// comparison keys. Comparison is on masked bits at the flag's width.
struct Switch;
impl Eval for Switch {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let flag = frame.get(op.operand(0).map_err(|_| {
            EvalError::Malformed("cf.switch without a flag operand".into())
        })?)?;
        let flag_bits = flag.as_unsigned()?;
        let flag_width = flag.width()?;

        let successor_count = op.successor_count();
        if successor_count == 0 {
            return Err(EvalError::Malformed("cf.switch without successors".into()));
        }
        let case_count = successor_count - 1;

        let case_values = read_case_values(op, case_count, flag_width)?;
        let chosen = case_values
            .iter()
            .position(|value| *value == flag_bits)
            .map(|index| index + 1)
            .unwrap_or(0);

        // Operands: [flag] ++ default's ++ case0's ++ …, in successor
        // order; walk cumulative offsets to the chosen successor.
        let mut offset = 1;
        for index in 0..chosen {
            let successor = op
                .successor(index)
                .map_err(|_| EvalError::Malformed("cf.switch successor out of range".into()))?;
            offset += successor.argument_count();
        }
        let target = op
            .successor(chosen)
            .map_err(|_| EvalError::Malformed("cf.switch successor out of range".into()))?;
        let args = operand_values(frame, op, offset, target.argument_count())?;
        Ok(Step::Branch(target, args))
    }
}

fn read_case_values(
    op: OperationRef<'_, '_>,
    case_count: usize,
    flag_width: u32,
) -> Result<Vec<u64>, EvalError> {
    if case_count == 0 {
        return Ok(Vec::new());
    }
    let attribute = op
        .attribute("case_values")
        .ok()
        .and_then(|attribute| DenseElementsAttribute::try_from(attribute).ok())
        .ok_or_else(|| {
            EvalError::Malformed("cf.switch cases without a case_values attribute".into())
        })?;
    if attribute.len() != case_count {
        return Err(EvalError::Malformed(format!(
            "cf.switch has {case_count} case successor(s) but {} case value(s)",
            attribute.len()
        )));
    }
    (0..case_count)
        .map(|index| match flag_width {
            8 => attribute
                .i8_element(index)
                .map(|value| value as u8 as u64)
                .map_err(|e| EvalError::Malformed(e.to_string())),
            16 => attribute
                .i16_element(index)
                .map(|value| value as u16 as u64)
                .map_err(|e| EvalError::Malformed(e.to_string())),
            32 => attribute
                .i32_element(index)
                .map(|value| value as u32 as u64)
                .map_err(|e| EvalError::Malformed(e.to_string())),
            64 => attribute
                .i64_element(index)
                .map(|value| value as u64)
                .map_err(|e| EvalError::Malformed(e.to_string())),
            other => Err(EvalError::Unsupported(format!(
                "cf.switch on an i{other} flag"
            ))),
        })
        .collect()
}
