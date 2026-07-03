//! The interpreter's runtime value domain. v0: two's-complement integers
//! up to 64 bits — exactly what the upstream corpus and the early kernel
//! dialects need. Widening this enum is a docs/canon.md §2 event, because
//! it widens what goldens can print.

use crate::error::EvalError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Value {
    /// An integer of `width` bits (1..=64), stored sign-agnostically in
    /// the low bits of `bits`; bits above `width` are always zero.
    Int { bits: u64, width: u32 },
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

    pub fn width(&self) -> u32 {
        let Self::Int { width, .. } = self;
        *width
    }

    /// The unsigned reading: the masked bits.
    pub fn as_unsigned(&self) -> u64 {
        let Self::Int { bits, .. } = self;
        *bits
    }

    /// The signed reading: sign-extended to i64.
    pub fn as_signed(&self) -> i64 {
        let Self::Int { bits, width } = *self;
        if width >= 64 {
            bits as i64
        } else {
            let shift = 64 - width;
            ((bits << shift) as i64) >> shift
        }
    }

    pub fn as_bool(&self) -> Result<bool, EvalError> {
        match *self {
            Self::Int { bits, width: 1 } => Ok(bits != 0),
            other => Err(EvalError::TypeMismatch(format!(
                "expected i1, got {other:?}"
            ))),
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
        assert_eq!(Value::int(0x1ff, 8).unwrap().as_unsigned(), 0xff);
        assert_eq!(Value::int(u64::MAX, 64).unwrap().as_unsigned(), u64::MAX);
    }

    #[test]
    fn zero_and_over_wide_widths_are_unsupported() {
        assert!(Value::int(0, 0).is_err());
        assert!(Value::int(0, 65).is_err());
    }

    #[test]
    fn signed_reading_sign_extends() {
        assert_eq!(Value::int(0xff, 8).unwrap().as_signed(), -1);
        assert_eq!(Value::int(0x7f, 8).unwrap().as_signed(), 127);
        assert_eq!(Value::from_signed(-1, 64).unwrap().as_signed(), -1);
        // i1 reads as 0 / -1 signed — MLIR semantics, surprising but law.
        assert_eq!(Value::bool(true).as_signed(), -1);
    }

    #[test]
    fn bool_reading_requires_width_one() {
        assert!(Value::bool(true).as_bool().unwrap());
        assert!(!Value::bool(false).as_bool().unwrap());
        assert!(Value::int(1, 64).unwrap().as_bool().is_err());
    }

    #[test]
    fn min_signed_matches_two_complement() {
        assert_eq!(min_signed(64), i64::MIN);
        assert_eq!(min_signed(8), -128);
        assert_eq!(min_signed(1), -1);
    }
}
