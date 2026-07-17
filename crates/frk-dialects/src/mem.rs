//! frk.mem — the allocation/ownership surface (SPEC §4.3, D-041).
//! Dialect namespace `frk_mem`. One surface, swappable lowerings: the
//! memory strategy is a lowering parameter (Arena | Rc today), never a
//! language feature — the same IR runs under every strategy.
//!
//! Ops (packed/trait-free, D-031/D-036):
//! - `box_new(value) -> !frk_mem.box<T>` — allocate + initialize.
//! - `box_get(box) -> T`
//! - `box_set(box, value)` — the mutable-cell primitive (zero results).
//!
//! IRDL enforces bases; the frk verification pass enforces the elem
//! type equations (box_new's T = operand type; get/set against the
//! box's parameter).

use melior::Context;
use melior::ir::attribute::TypeAttribute;
use melior::ir::operation::OperationLike;
use melior::ir::{OperationRef, Type, ValueLike};

use crate::attr_util::type_params;

pub const IRDL: &str = r##"
irdl.dialect @frk_mem {
  irdl.type @box {
    %elem = irdl.any
    irdl.parameters(elem: %elem)
  }
  irdl.type @arr {
    %elem = irdl.any
    irdl.parameters(elem: %elem)
  }
  irdl.operation @array_new {
    %len = irdl.is i64
    %a = irdl.base @frk_mem::@arr
    irdl.operands(len: %len)
    irdl.results(arr: %a)
  }
  irdl.operation @array_get {
    %a = irdl.base @frk_mem::@arr
    %i = irdl.is i64
    %v = irdl.any
    irdl.operands(arr: %a, index: %i)
    irdl.results(value: %v)
  }
  irdl.operation @array_set {
    %a = irdl.base @frk_mem::@arr
    %i = irdl.is i64
    %v = irdl.any
    irdl.operands(arr: %a, index: %i, value: %v)
  }
  irdl.operation @array_len {
    %a = irdl.base @frk_mem::@arr
    %n = irdl.is i64
    irdl.operands(arr: %a)
    irdl.results(len: %n)
  }
  irdl.operation @dispose {
    %v = irdl.any
    irdl.operands(value: %v)
  }
  irdl.operation @box_new {
    %v = irdl.any
    %b = irdl.base @frk_mem::@box
    irdl.operands(value: %v)
    irdl.results(box: %b)
  }
  irdl.operation @box_get {
    %b = irdl.base @frk_mem::@box
    %v = irdl.any
    irdl.operands(box: %b)
    irdl.results(value: %v)
  }
  irdl.operation @box_set {
    %b = irdl.base @frk_mem::@box
    %v = irdl.any
    irdl.operands(box: %b, value: %v)
  }
  irdl.operation @field_get {
    %b = irdl.base @frk_mem::@box
    %v = irdl.any
    %f = irdl.base "#builtin.integer"
    irdl.operands(box: %b)
    irdl.results(value: %v)
    irdl.attributes { "field" = %f }
  }
  irdl.operation @field_set {
    %b = irdl.base @frk_mem::@box
    %v = irdl.any
    %f = irdl.base "#builtin.integer"
    irdl.operands(box: %b, value: %v)
    irdl.attributes { "field" = %f }
  }
}
"##;

/// Decodes a record op's box operand: the box must hold a product
/// (D-073 — a class instance is a managed box of a product), and the
/// `field` attribute must index into it. Returns (field types, index).
pub(crate) fn record_field<'c>(
    context: &'c Context,
    op: OperationRef<'c, '_>,
    box_type: Type<'c>,
) -> Result<(Vec<Type<'c>>, usize), String> {
    let elem = decode_box(context, box_type)?;
    let fields = crate::adt::decode_product(context, elem)
        .map_err(|_| format!("field ops need a box of a product, got box<{elem}>"))?;
    let field = crate::adt::index_attr(op, "field")?;
    if field >= fields.len() {
        return Err(format!(
            "field {field} out of range: the record has {} field(s)",
            fields.len()
        ));
    }
    Ok((fields, field))
}

/// Decodes `!frk_mem.box<T>` to T.
pub(crate) fn decode_box<'c>(context: &'c Context, r#type: Type<'c>) -> Result<Type<'c>, String> {
    let param = type_params(context, r#type, "!frk_mem.box<", false)?;
    TypeAttribute::try_from(param)
        .map(|attribute| attribute.value())
        .map_err(|_| format!("box parameter must be a type: {type}", type = r#type))
}

/// Decodes `!frk_mem.arr<T>` to T.
pub(crate) fn decode_arr<'c>(context: &'c Context, r#type: Type<'c>) -> Result<Type<'c>, String> {
    let param = type_params(context, r#type, "!frk_mem.arr<", false)?;
    TypeAttribute::try_from(param)
        .map(|attribute| attribute.value())
        .map_err(|_| format!("arr parameter must be a type: {type}", type = r#type))
}

pub(crate) fn verify_op<'c>(
    context: &'c Context,
    name: &str,
    op: OperationRef<'c, '_>,
) -> Result<(), String> {
    let operand_type = |index: usize| {
        op.operand(index)
            .map(|operand| operand.r#type())
            .map_err(|_| format!("missing operand {index}"))
    };
    match name {
        "box_new" => {
            let elem = decode_box(
                context,
                op.result(0)
                    .map_err(|_| "box_new without a result".to_string())?
                    .r#type(),
            )?;
            let value = operand_type(0)?;
            if value == elem {
                Ok(())
            } else {
                Err(format!("box_new stores a {value} into a box<{elem}>"))
            }
        }
        "box_get" => {
            let elem = decode_box(context, operand_type(0)?)?;
            let result = op
                .result(0)
                .map_err(|_| "box_get without a result".to_string())?
                .r#type();
            if result == elem {
                Ok(())
            } else {
                Err(format!("box_get yields {result} from a box<{elem}>"))
            }
        }
        "box_set" => {
            let elem = decode_box(context, operand_type(0)?)?;
            let value = operand_type(1)?;
            if value == elem {
                Ok(())
            } else {
                Err(format!("box_set stores a {value} into a box<{elem}>"))
            }
        }
        // Field-granular record mutation (D-073): the box must hold a
        // product; the field index must be in range; the value/result
        // type must equal the named field's type.
        "field_get" => {
            let (fields, field) = record_field(context, op, operand_type(0)?)?;
            let result = op
                .result(0)
                .map_err(|_| "field_get without a result".to_string())?
                .r#type();
            if result == fields[field] {
                Ok(())
            } else {
                Err(format!(
                    "field_get yields {result}, field {field} is {}",
                    fields[field]
                ))
            }
        }
        "field_set" => {
            let (fields, field) = record_field(context, op, operand_type(0)?)?;
            let value = operand_type(1)?;
            if value == fields[field] {
                Ok(())
            } else {
                Err(format!(
                    "field_set stores a {value}, field {field} is {}",
                    fields[field]
                ))
            }
        }
        "array_new" => {
            decode_arr(
                context,
                op.result(0)
                    .map_err(|_| "array_new without a result".to_string())?
                    .r#type(),
            )?;
            Ok(())
        }
        "array_get" => {
            let elem = decode_arr(context, operand_type(0)?)?;
            let result = op
                .result(0)
                .map_err(|_| "array_get without a result".to_string())?
                .r#type();
            if result == elem {
                Ok(())
            } else {
                Err(format!("array_get yields {result} from an arr<{elem}>"))
            }
        }
        "array_set" => {
            let elem = decode_arr(context, operand_type(0)?)?;
            let value = operand_type(2)?;
            if value == elem {
                Ok(())
            } else {
                Err(format!("array_set stores a {value} into an arr<{elem}>"))
            }
        }
        "array_len" => {
            decode_arr(context, operand_type(0)?)?;
            Ok(())
        }
        "dispose" => {
            // End-of-ownership for a RECEIVED managed value (D-067:
            // callee-owned packs). Operand must be a managed kernel
            // type; frame-created values are the release discipline's
            // job, not dispose's.
            let printed = operand_type(0)?.to_string();
            if printed.starts_with("!frk_mem.arr<") || printed.starts_with("!frk_mem.box<") {
                Ok(())
            } else {
                Err(format!("dispose takes a managed mem value, got {printed}"))
            }
        }
        other => Err(format!("no semantic verifier for frk_mem.{other}")),
    }
}
