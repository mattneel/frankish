//! The interpreter's runtime value domain. v0: two's-complement integers
//! up to 64 bits, plus structured ADT values (M3). Adt is internal to
//! interpretation — the entry protocol still renders scalars only
//! (docs/canon.md §2); widening what goldens can *print* is a canon
//! event, widening what they can *compute with* is not.

use crate::error::EvalError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// An integer of `width` bits (1..=64), stored sign-agnostically in
    /// the low bits of `bits`; bits above `width` are always zero.
    Int { bits: u64, width: u32 },
    /// A structured value: which variant (`tag`) plus its field values.
    /// Products are single-variant sums at runtime: tag 0.
    Adt { tag: usize, fields: Vec<Value> },
    /// A first-class function: the lifted callee's symbol plus the
    /// captured values (by value, D-035). Applying it calls
    /// `callee(captures..., args...)`.
    Closure { callee: String, captures: Vec<Value> },
}

fn mask(width: u32) -> u64 {
    if width >= 64 { u64::MAX } else { (1u64 << width) - 1 }
}

impl Value {
    pub fn int(bits: u64, width: u32) -> Result<Self, EvalError> {
        if width == 0 || width > 64 {
            return Err(EvalError::Unsupported(format!("i{width} values")));
        }
        Ok(Self::Int { bits: bits & mask(width), width })
    }

    pub fn from_signed(value: i64, width: u32) -> Result<Self, EvalError> {
        Self::int(value as u64, width)
    }

    pub fn bool(value: bool) -> Self {
        Self::Int { bits: value as u64, width: 1 }
    }

    pub fn adt(tag: usize, fields: Vec<Value>) -> Self {
        Self::Adt { tag, fields }
    }

    pub fn closure(callee: impl Into<String>, captures: Vec<Value>) -> Self {
        Self::Closure { callee: callee.into(), captures }
    }

    fn int_parts(&self) -> Result<(u64, u32), EvalError> {
        match self {
            Self::Int { bits, width } => Ok((*bits, *width)),
            Self::Adt { .. } => Err(EvalError::TypeMismatch(
                "expected an integer, got an adt value".into(),
            )),
            Self::Closure { .. } => Err(EvalError::TypeMismatch(
                "expected an integer, got a closure".into(),
            )),
        }
    }

    pub fn width(&self) -> Result<u32, EvalError> {
        Ok(self.int_parts()?.1)
    }

    /// The unsigned reading: the masked bits.
    pub fn as_unsigned(&self) -> Result<u64, EvalError> {
        Ok(self.int_parts()?.0)
    }

    /// The signed reading: sign-extended to i64.
    pub fn as_signed(&self) -> Result<i64, EvalError> {
        let (bits, width) = self.int_parts()?;
        Ok(if width >= 64 {
            bits as i64
        } else {
            let shift = 64 - width;
            ((bits << shift) as i64) >> shift
        })
    }

    pub fn as_bool(&self) -> Result<bool, EvalError> {
        match self.int_parts()? {
            (bits, 1) => Ok(bits != 0),
            (_, width) => Err(EvalError::TypeMismatch(format!("expected i1, got i{width}"))),
        }
    }

    pub fn as_adt(&self) -> Result<(usize, &[Value]), EvalError> {
        match self {
            Self::Adt { tag, fields } => Ok((*tag, fields)),
            Self::Int { width, .. } => Err(EvalError::TypeMismatch(format!(
                "expected an adt value, got i{width}"
            ))),
            Self::Closure { .. } => Err(EvalError::TypeMismatch(
                "expected an adt value, got a closure".into(),
            )),
        }
    }

    pub fn as_closure(&self) -> Result<(&str, &[Value]), EvalError> {
        match self {
            Self::Closure { callee, captures } => Ok((callee, captures)),
            Self::Int { width, .. } => Err(EvalError::TypeMismatch(format!(
                "expected a closure, got i{width}"
            ))),
            Self::Adt { .. } => Err(EvalError::TypeMismatch(
                "expected a closure, got an adt value".into(),
            )),
        }
    }
}

/// The most negative signed value at `width` (needed for div overflow).
pub fn min_signed(width: u32) -> i64 {
    if width >= 64 { i64::MIN } else { -(1i64 << (width - 1)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_masks_to_width() {
        assert_eq!(Value::int(0x1ff, 8).unwrap().as_unsigned().unwrap(), 0xff);
        assert_eq!(
            Value::int(u64::MAX, 64).unwrap().as_unsigned().unwrap(),
            u64::MAX
        );
    }

    #[test]
    fn zero_and_over_wide_widths_are_unsupported() {
        assert!(Value::int(0, 0).is_err());
        assert!(Value::int(0, 65).is_err());
    }

    #[test]
    fn signed_reading_sign_extends() {
        assert_eq!(Value::int(0xff, 8).unwrap().as_signed().unwrap(), -1);
        assert_eq!(Value::int(0x7f, 8).unwrap().as_signed().unwrap(), 127);
        assert_eq!(Value::from_signed(-1, 64).unwrap().as_signed().unwrap(), -1);
        // i1 reads as 0 / -1 signed — MLIR semantics, surprising but law.
        assert_eq!(Value::bool(true).as_signed().unwrap(), -1);
    }

    #[test]
    fn bool_reading_requires_width_one() {
        assert!(Value::bool(true).as_bool().unwrap());
        assert!(!Value::bool(false).as_bool().unwrap());
        assert!(Value::int(1, 64).unwrap().as_bool().is_err());
    }

    #[test]
    fn adt_values_nest_and_read_back() {
        let inner = Value::adt(0, vec![Value::bool(true)]);
        let outer = Value::adt(3, vec![inner.clone(), Value::int(9, 64).unwrap()]);
        let (tag, fields) = outer.as_adt().unwrap();
        assert_eq!(tag, 3);
        assert_eq!(fields[0], inner);
        assert_eq!(fields[1].as_signed().unwrap(), 9);
    }

    #[test]
    fn integer_readers_reject_adt_values_loudly() {
        let value = Value::adt(0, vec![]);
        assert!(value.as_signed().is_err());
        assert!(value.as_unsigned().is_err());
        assert!(value.as_bool().is_err());
        assert!(value.width().is_err());
        assert!(Value::bool(true).as_adt().is_err());
    }

    #[test]
    fn closures_nest_and_readers_stay_honest() {
        let closure = Value::closure("f", vec![Value::bool(true)]);
        let (callee, captures) = closure.as_closure().unwrap();
        assert_eq!(callee, "f");
        assert_eq!(captures.len(), 1);
        // Closures capture closures and travel inside adt values.
        let outer = Value::adt(1, vec![Value::closure("g", vec![closure.clone()])]);
        let (_, fields) = outer.as_adt().unwrap();
        let (inner_callee, inner_captures) = fields[0].as_closure().unwrap();
        assert_eq!(inner_callee, "g");
        assert_eq!(inner_captures[0], closure);
        // And every reader rejects the wrong shape loudly.
        assert!(closure.as_signed().is_err());
        assert!(closure.as_adt().is_err());
        assert!(Value::bool(true).as_closure().is_err());
        assert!(Value::adt(0, vec![]).as_closure().is_err());
    }

    #[test]
    fn min_signed_matches_two_complement() {
        assert_eq!(min_signed(64), i64::MIN);
        assert_eq!(min_signed(8), -128);
        assert_eq!(min_signed(1), -1);
    }
}
