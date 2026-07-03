//! arith: constants, integer arithmetic (wrapping, per MLIR's modulo-2^n
//! semantics), float arithmetic (IEEE, M9/TS-0 — no UB, no traps),
//! comparisons, select, bitcast. Signed-division UB traps (D-029).

use std::collections::HashMap;

use melior::ir::attribute::{BoolAttribute, IntegerAttribute};
use melior::ir::operation::OperationLike;
use melior::ir::r#type::IntegerType;
use melior::ir::{OperationRef, Type, ValueLike};

use super::{binary_operands, continue_with_result, operand_values};
use crate::error::EvalError;
use crate::interp::{Eval, Frame, Interp, Step};
use crate::value::{Value, min_signed};

pub(crate) fn register(registry: &mut HashMap<&'static str, Box<dyn Eval>>) {
    registry.insert("arith.constant", Box::new(Constant));
    registry.insert("arith.addi", Box::new(AddI));
    registry.insert("arith.subi", Box::new(SubI));
    registry.insert("arith.muli", Box::new(MulI));
    registry.insert("arith.divsi", Box::new(DivSI));
    registry.insert("arith.cmpi", Box::new(CmpI));
    registry.insert("arith.select", Box::new(Select));
    registry.insert("arith.addf", Box::new(FloatBin(|a, b| a + b)));
    registry.insert("arith.subf", Box::new(FloatBin(|a, b| a - b)));
    registry.insert("arith.mulf", Box::new(FloatBin(|a, b| a * b)));
    registry.insert("arith.divf", Box::new(FloatBin(|a, b| a / b)));
    registry.insert("arith.remf", Box::new(FloatBin(|a, b| a % b)));
    registry.insert("arith.negf", Box::new(NegF));
    registry.insert("arith.cmpf", Box::new(CmpF));
    registry.insert("arith.bitcast", Box::new(Bitcast));
    registry.insert("arith.xori", Box::new(XorI));
}

/// Two operands with no integer-width insistence (float ops).
fn float_operands(
    frame: &Frame,
    op: OperationRef<'_, '_>,
) -> Result<(Value, Value), EvalError> {
    let mut values = operand_values(frame, op, 0, 2)?;
    let rhs = values.pop().expect("two operands");
    let lhs = values.pop().expect("two operands");
    Ok((lhs, rhs))
}

/// IEEE binary op: total, deterministic (NaN propagates; never traps).
struct FloatBin(fn(f64, f64) -> f64);
impl Eval for FloatBin {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let (lhs, rhs) = float_operands(frame, op)?;
        let value = (self.0)(lhs.as_float()?, rhs.as_float()?);
        continue_with_result(frame, op, Value::float(value))
    }
}

struct NegF;
impl Eval for NegF {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let values = operand_values(frame, op, 0, 1)?;
        continue_with_result(frame, op, Value::float(-values[0].as_float()?))
    }
}

/// arith.cmpf with the ordered/unordered predicates TS-0 emits:
/// oeq/one/olt/ole/ogt/oge (false on NaN) and une (true on NaN — JS
/// NaN !== NaN).
struct CmpF;
impl Eval for CmpF {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let (lhs, rhs) = float_operands(frame, op)?;
        let (a, b) = (lhs.as_float()?, rhs.as_float()?);
        let predicate = op
            .attribute("predicate")
            .ok()
            .and_then(|attribute| IntegerAttribute::try_from(attribute).ok())
            .ok_or_else(|| EvalError::Malformed("arith.cmpf without predicate".into()))?
            .value();
        // MLIR CmpFPredicate numbering (llvm-project ArithBase.td).
        let result = match predicate {
            1 => a == b,                       // oeq
            2 => a > b,                        // ogt
            3 => a >= b,                       // oge
            4 => a < b,                        // olt
            5 => a <= b,                       // ole
            6 => a != b && !(a.is_nan() || b.is_nan()), // one
            13 => a != b || a.is_nan() || b.is_nan(),   // une (JS !==)
            other => {
                return Err(EvalError::Unsupported(format!(
                    "cmpf predicate {other}"
                )));
            }
        };
        continue_with_result(frame, op, Value::bool(result))
    }
}

/// Same-width bit reinterpretation: f64 <-> i64 (the slot model's
/// float path, D-046).
struct Bitcast;
impl Eval for Bitcast {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let values = operand_values(frame, op, 0, 1)?;
        let result_type = op
            .result(0)
            .map_err(|_| EvalError::Malformed("bitcast without result".into()))?
            .r#type();
        let value = if result_type.to_string() == "f64" {
            let (bits, width) = values[0].int_parts()?;
            if width != 64 {
                return Err(EvalError::Unsupported(format!(
                    "bitcast i{width} -> f64"
                )));
            }
            Value::float(f64::from_bits(bits))
        } else if IntegerType::try_from(result_type).is_ok_and(|t| t.width() == 64) {
            Value::int(values[0].as_float()?.to_bits(), 64)?
        } else {
            return Err(EvalError::Unsupported(format!(
                "bitcast to {result_type}"
            )));
        };
        continue_with_result(frame, op, value)
    }
}

struct XorI;
impl Eval for XorI {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let (lhs, rhs) = binary_operands(frame, op)?;
        let (a, width) = lhs.int_parts()?;
        let (b, _) = rhs.int_parts()?;
        continue_with_result(frame, op, Value::int(a ^ b, width)?)
    }
}

fn int_width(r#type: Type<'_>) -> Result<u32, EvalError> {
    IntegerType::try_from(r#type)
        .map(|t| t.width())
        .map_err(|_| EvalError::Unsupported(format!("non-integer type {type}", type = r#type)))
}

struct Constant;
impl Eval for Constant {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let result = op
            .result(0)
            .map_err(|_| EvalError::Malformed("arith.constant without result".into()))?;
        let width = int_width(result.r#type());
        let attribute = op
            .attribute("value")
            .map_err(|_| EvalError::Malformed("arith.constant without value".into()))?;

        let value = if let Ok(float) = melior::ir::attribute::FloatAttribute::try_from(attribute)
        {
            Value::float(float.value())
        } else if let Ok(integer) = IntegerAttribute::try_from(attribute) {
            Value::int(integer.value() as u64, width?)?
        } else if let Ok(boolean) = BoolAttribute::try_from(attribute) {
            Value::bool(boolean.value())
        } else {
            return Err(EvalError::Unsupported(
                "non-integer arith.constant".into(),
            ));
        };
        continue_with_result(frame, op, value)
    }
}

/// Wrapping binary ops: MLIR arith without overflow flags is arithmetic
/// modulo 2^width — masking in Value::int completes the wrap.
macro_rules! wrapping_binary {
    ($name:ident, $method:ident) => {
        struct $name;
        impl Eval for $name {
            fn eval<'c, 'a>(
                &self,
                _interp: &Interp<'c, 'a>,
                frame: &mut Frame,
                op: OperationRef<'c, 'a>,
            ) -> Result<Step<'c, 'a>, EvalError> {
                let (lhs, rhs) = binary_operands(frame, op)?;
                let bits = lhs.as_unsigned()?.$method(rhs.as_unsigned()?);
                continue_with_result(frame, op, Value::int(bits, lhs.width()?)?)
            }
        }
    };
}

wrapping_binary!(AddI, wrapping_add);
wrapping_binary!(SubI, wrapping_sub);
wrapping_binary!(MulI, wrapping_mul);

struct DivSI;
impl Eval for DivSI {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let (lhs, rhs) = binary_operands(frame, op)?;
        let (dividend, divisor) = (lhs.as_signed()?, rhs.as_signed()?);
        if divisor == 0 {
            return Err(EvalError::Trap("arith.divsi: division by zero".into()));
        }
        let width = lhs.width()?;
        if divisor == -1 && dividend == min_signed(width) {
            return Err(EvalError::Trap("arith.divsi: signed overflow".into()));
        }
        continue_with_result(frame, op, Value::from_signed(dividend / divisor, width)?)
    }
}

struct CmpI;
impl Eval for CmpI {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let (lhs, rhs) = binary_operands(frame, op)?;
        let predicate = op
            .attribute("predicate")
            .ok()
            .and_then(|attribute| IntegerAttribute::try_from(attribute).ok())
            .ok_or_else(|| EvalError::Malformed("arith.cmpi without predicate".into()))?
            .value();

        // Predicate numbering is MLIR's arith::CmpIPredicate.
        let outcome = match predicate {
            0 => lhs.as_unsigned()? == rhs.as_unsigned()?,
            1 => lhs.as_unsigned()? != rhs.as_unsigned()?,
            2 => lhs.as_signed()? < rhs.as_signed()?,
            3 => lhs.as_signed()? <= rhs.as_signed()?,
            4 => lhs.as_signed()? > rhs.as_signed()?,
            5 => lhs.as_signed()? >= rhs.as_signed()?,
            6 => lhs.as_unsigned()? < rhs.as_unsigned()?,
            7 => lhs.as_unsigned()? <= rhs.as_unsigned()?,
            8 => lhs.as_unsigned()? > rhs.as_unsigned()?,
            9 => lhs.as_unsigned()? >= rhs.as_unsigned()?,
            other => {
                return Err(EvalError::Unsupported(format!(
                    "cmpi predicate {other}"
                )));
            }
        };
        continue_with_result(frame, op, Value::bool(outcome))
    }
}

struct Select;
impl Eval for Select {
    fn eval<'c, 'a>(
        &self,
        _interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let mut values = operand_values(frame, op, 0, 3)?;
        let on_false = values.pop().expect("three operands");
        let on_true = values.pop().expect("three operands");
        let condition = values.pop().expect("three operands");
        let chosen = if condition.as_bool()? { on_true } else { on_false };
        continue_with_result(frame, op, chosen)
    }
}
