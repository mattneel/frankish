//! The interpreter's runtime value domain. v0: two's-complement integers
//! up to 64 bits, plus structured ADT values (M3). Adt is internal to
//! interpretation — the entry protocol still renders scalars only
//! (docs/canon.md §2); widening what goldens can *print* is a canon
//! event, widening what they can *compute with* is not.

use crate::error::EvalError;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug)]
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
    /// A frk_mem box: a shared mutable cell (D-041). Clones alias the
    /// same cell; equality is cell identity.
    Box(Rc<RefCell<Value>>),
    /// An f64 (M9, TS-0). Equality is BIT equality — deterministic
    /// under diffing, NaN == NaN by bits.
    Float(f64),
    /// A frk_mem array: shared mutable, identity equality — JS
    /// reference semantics (D-049). Out-of-bounds access TRAPS.
    Array(Rc<RefCell<Vec<Value>>>),
    /// A frk_str string: immutable UTF-16 code units (D-049).
    Str(Rc<Vec<u16>>),
    /// A frk_dyn fat value (D-051): closed-enum tag + payload.
    Dyn(u64, Rc<Value>),
    /// A frk_bstr byte string (D-056): content equality here is
    /// observably identical to the native path's interned identity.
    Bytes(Rc<Vec<u8>>),
    /// A frk_dyn table (D-056): association list + metatable slot,
    /// identity equality — Lua reference semantics. Linear lookup is
    /// deliberate at corpus scale (reference semantics, not speed).
    Table(Rc<RefCell<TableData>>),
}

#[derive(Debug, Default)]
pub struct TableData {
    /// Insertion-ordered (key, value) pairs; keys compare by the dyn
    /// key rule (Value equality — bit-eq nums, content-eq bytes,
    /// identity tables). -0.0 and NaN keys are fenced (D-056).
    pub entries: Vec<(Value, Value)>,
    pub meta: Option<Value>,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Int { bits: a, width: wa }, Self::Int { bits: b, width: wb }) => {
                a == b && wa == wb
            }
            (Self::Adt { tag: a, fields: fa }, Self::Adt { tag: b, fields: fb }) => {
                a == b && fa == fb
            }
            (
                Self::Closure { callee: a, captures: ca },
                Self::Closure { callee: b, captures: cb },
            ) => a == b && ca == cb,
            (Self::Box(a), Self::Box(b)) => Rc::ptr_eq(a, b),
            (Self::Float(a), Self::Float(b)) => a.to_bits() == b.to_bits(),
            (Self::Array(a), Self::Array(b)) => Rc::ptr_eq(a, b),
            (Self::Str(a), Self::Str(b)) => a == b,
            (Self::Dyn(ta, pa), Self::Dyn(tb, pb)) => ta == tb && pa == pb,
            (Self::Bytes(a), Self::Bytes(b)) => a == b,
            (Self::Table(a), Self::Table(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl Eq for Value {}

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

    pub fn float(value: f64) -> Self {
        Self::Float(value)
    }

    pub fn as_float(&self) -> Result<f64, EvalError> {
        match self {
            Self::Float(value) => Ok(*value),
            other => Err(EvalError::TypeMismatch(format!(
                "expected an f64, got {other:?}"
            ))),
        }
    }

    pub fn array(items: Vec<Value>) -> Self {
        Self::Array(Rc::new(RefCell::new(items)))
    }

    pub fn as_array(&self) -> Result<&Rc<RefCell<Vec<Value>>>, EvalError> {
        match self {
            Self::Array(items) => Ok(items),
            other => Err(EvalError::TypeMismatch(format!(
                "expected an array, got {other:?}"
            ))),
        }
    }

    pub fn str_from(text: &str) -> Self {
        Self::Str(Rc::new(text.encode_utf16().collect()))
    }

    pub fn as_str_units(&self) -> Result<&Rc<Vec<u16>>, EvalError> {
        match self {
            Self::Str(units) => Ok(units),
            other => Err(EvalError::TypeMismatch(format!(
                "expected a string, got {other:?}"
            ))),
        }
    }

    pub fn bytes(data: Vec<u8>) -> Self {
        Self::Bytes(Rc::new(data))
    }

    pub fn as_bytes(&self) -> Result<&Rc<Vec<u8>>, EvalError> {
        match self {
            Self::Bytes(data) => Ok(data),
            other => Err(EvalError::TypeMismatch(format!(
                "expected a byte string, got {other:?}"
            ))),
        }
    }

    pub fn table() -> Self {
        Self::Table(Rc::new(RefCell::new(TableData::default())))
    }

    pub fn as_table(&self) -> Result<&Rc<RefCell<TableData>>, EvalError> {
        match self {
            Self::Table(data) => Ok(data),
            other => Err(EvalError::TypeMismatch(format!(
                "expected a table, got {other:?}"
            ))),
        }
    }

    pub fn dyn_value(tag: u64, payload: Value) -> Self {
        Self::Dyn(tag, Rc::new(payload))
    }

    pub fn as_dyn(&self) -> Result<(u64, &Value), EvalError> {
        match self {
            Self::Dyn(tag, payload) => Ok((*tag, payload)),
            other => Err(EvalError::TypeMismatch(format!(
                "expected a dyn value, got {other:?}"
            ))),
        }
    }

    pub fn boxed(value: Value) -> Self {
        Self::Box(Rc::new(RefCell::new(value)))
    }

    pub fn as_box(&self) -> Result<&Rc<RefCell<Value>>, EvalError> {
        match self {
            Self::Box(cell) => Ok(cell),
            other => Err(EvalError::TypeMismatch(format!(
                "expected a box, got {other:?}"
            ))),
        }
    }

    pub(crate) fn int_parts(&self) -> Result<(u64, u32), EvalError> {
        match self {
            Self::Int { bits, width } => Ok((*bits, *width)),
            other => Err(EvalError::TypeMismatch(format!(
                "expected an integer, got {other:?}"
            ))),
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
            other => Err(EvalError::TypeMismatch(format!(
                "expected an adt value, got {other:?}"
            ))),
        }
    }

    pub fn as_closure(&self) -> Result<(&str, &[Value]), EvalError> {
        match self {
            Self::Closure { callee, captures } => Ok((callee, captures)),
            Self::Int { width, .. } => Err(EvalError::TypeMismatch(format!(
                "expected a closure, got i{width}"
            ))),
            other => Err(EvalError::TypeMismatch(format!(
                "expected a closure, got {other:?}"
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
    fn boxes_are_shared_mutable_cells_with_identity_equality() {
        let a = Value::boxed(Value::bool(false));
        let alias = a.clone();
        *alias.as_box().unwrap().borrow_mut() = Value::int(9, 64).unwrap();
        assert_eq!(
            a.as_box().unwrap().borrow().as_signed().unwrap(),
            9,
            "clones alias the same cell"
        );
        let b = Value::boxed(Value::int(9, 64).unwrap());
        assert_eq!(a, alias);
        assert_ne!(a, b, "equality is identity, not contents");
        assert!(a.as_signed().is_err());
        assert!(a.as_adt().is_err());
        assert!(a.as_closure().is_err());
        assert!(Value::bool(true).as_box().is_err());
    }

    #[test]
    fn min_signed_matches_two_complement() {
        assert_eq!(min_signed(64), i64::MIN);
        assert_eq!(min_signed(8), -128);
        assert_eq!(min_signed(1), -1);
    }
}
