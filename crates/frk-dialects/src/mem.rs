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

pub const IRDL: &str = r#"
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
}
"#;

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
        other => Err(format!("no semantic verifier for frk_mem.{other}")),
    }
}
