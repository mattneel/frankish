//! scf: structured control flow. Regions are single-block in v0 (the
//! interpreter rejects more, loudly). scf.for follows MLIR semantics:
//! signed [lb, ub) iteration; a non-positive step traps (D-029) rather
//! than looping forever.

use std::collections::HashMap;

use melior::ir::operation::OperationLike;
use melior::ir::{BlockRef, OperationRef, RegionLike};

use super::{continue_with_results, operand_values};
use crate::error::EvalError;
use crate::interp::{Eval, Frame, Interp, Step};
use crate::value::Value;

pub(crate) fn register(registry: &mut HashMap<&'static str, Box<dyn Eval>>) {
    registry.insert("scf.for", Box::new(For));
    registry.insert("scf.if", Box::new(If));
    registry.insert("scf.yield", Box::new(Yield));
}

fn single_block<'c, 'a>(
    op: OperationRef<'c, 'a>,
    region_index: usize,
) -> Result<Option<BlockRef<'c, 'a>>, EvalError> {
    match op.region(region_index) {
        Ok(region) => Ok(region.first_block()),
        Err(_) => Ok(None),
    }
}

struct For;
impl Eval for For {
    fn eval<'c, 'a>(
        &self,
        interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        if op.operand_count() < 3 {
            return Err(EvalError::Malformed("scf.for needs lb, ub, step".into()));
        }
        let bounds = operand_values(frame, op, 0, 3)?;
        let (lb, ub, step) = (bounds[0], bounds[1], bounds[2]);
        let width = lb.width();
        let step_value = step.as_signed();
        if step_value <= 0 {
            return Err(EvalError::Trap(format!(
                "scf.for: non-positive step {step_value}"
            )));
        }

        let body = single_block(op, 0)?
            .ok_or_else(|| EvalError::Malformed("scf.for without a body block".into()))?;

        let mut carried = operand_values(frame, op, 3, op.operand_count() - 3)?;
        let mut induction = lb.as_signed();
        while induction < ub.as_signed() {
            let mut args = Vec::with_capacity(1 + carried.len());
            args.push(Value::from_signed(induction, width)?);
            args.extend(carried.iter().copied());
            carried = interp.run_structured_block(frame, body, args)?;
            // Wrapping induction would mean ub was unreachable; the ub
            // comparison bounds it first, so plain addition is safe here.
            induction += step_value;
        }
        continue_with_results(frame, op, &carried)
    }
}

struct If;
impl Eval for If {
    fn eval<'c, 'a>(
        &self,
        interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let condition = frame
            .get(op.operand(0).map_err(|_| {
                EvalError::Malformed("scf.if without condition".into())
            })?)?
            .as_bool()?;

        let chosen = if condition {
            single_block(op, 0)?
        } else {
            single_block(op, 1)?
        };

        let yielded = match chosen {
            Some(block) => interp.run_structured_block(frame, block, Vec::new())?,
            // Absent else region: legal only when the op has no results.
            None => Vec::new(),
        };
        continue_with_results(frame, op, &yielded)
    }
}

struct Yield;
impl Eval for Yield {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        Ok(Step::Yield(operand_values(
            frame,
            op,
            0,
            op.operand_count(),
        )?))
    }
}
