//! frk.dyn — uniform dynamic values (SPEC §4.5; D-051, M10 "contract
//! underway"). v0 representation is FAT VALUES: {tag: i64, payload} —
//! two slots, the closure precedent; NaN-boxing/pointer-tagging are
//! later representation swaps behind this same surface.
//!
//! K1/K2 land at M10. K3 (lowering) is scheduled with the femto_lua
//! implementation milestone — dyn goldens ride `runners=interp` until
//! then (the fence mechanism built for exactly this).
//!
//! Tag space v0 (closed enum, femto_lua's six — D-051):
//! nil=0, bool=1, num=2, str=3, table=4, fun=5.

use melior::Context;
use melior::ir::OperationRef;
use melior::ir::attribute::IntegerAttribute;
use melior::ir::operation::OperationLike;

pub const IRDL: &str = r#"
irdl.dialect @frk_dyn {
  irdl.type @dyn {}
  irdl.operation @wrap {
    %v = irdl.any
    %d = irdl.base @frk_dyn::@dyn
    irdl.operands(value: %v)
    irdl.results(out: %d)
  }
  irdl.operation @unwrap {
    %d = irdl.base @frk_dyn::@dyn
    %v = irdl.any
    irdl.operands(value: %d)
    irdl.results(out: %v)
  }
  irdl.operation @tag_of {
    %d = irdl.base @frk_dyn::@dyn
    %n = irdl.is i64
    irdl.operands(value: %d)
    irdl.results(tag: %n)
  }
  irdl.operation @table_new {
    %d = irdl.base @frk_dyn::@dyn
    irdl.results(table: %d)
  }
  irdl.operation @raw_get {
    %d = irdl.base @frk_dyn::@dyn
    irdl.operands(table: %d, key: %d)
    irdl.results(value: %d)
  }
  irdl.operation @raw_set {
    %d = irdl.base @frk_dyn::@dyn
    irdl.operands(table: %d, key: %d, value: %d)
  }
  irdl.operation @table_len {
    %d = irdl.base @frk_dyn::@dyn
    %n = irdl.is i64
    irdl.operands(table: %d)
    irdl.results(len: %n)
  }
  irdl.operation @set_meta {
    %d = irdl.base @frk_dyn::@dyn
    irdl.operands(table: %d, meta: %d)
  }
  irdl.operation @get_meta {
    %d = irdl.base @frk_dyn::@dyn
    irdl.operands(table: %d)
    irdl.results(meta: %d)
  }
  irdl.operation @table_next {
    %d = irdl.base @frk_dyn::@dyn
    irdl.operands(table: %d, key: %d)
    irdl.results(next_key: %d, next_value: %d)
  }
  irdl.operation @payload_word {
    %d = irdl.base @frk_dyn::@dyn
    %n = irdl.is i64
    irdl.operands(value: %d)
    irdl.results(word: %n)
  }
}
"#;

pub const TAG_NIL: i64 = 0;
pub const TAG_BOOL: i64 = 1;
pub const TAG_NUM: i64 = 2;
pub const TAG_STR: i64 = 3;
pub const TAG_TABLE: i64 = 4;
pub const TAG_FUN: i64 = 5;
const TAG_LIMIT: i64 = 6;

pub(crate) fn tag_attr(op: OperationRef<'_, '_>) -> Result<i64, String> {
    let tag = op
        .attribute("tag")
        .ok()
        .and_then(|attribute| IntegerAttribute::try_from(attribute).ok())
        .ok_or_else(|| "missing integer `tag` attribute".to_string())?
        .value();
    if !(0..TAG_LIMIT).contains(&tag) {
        return Err(format!(
            "tag {tag} outside the closed v0 space 0..{TAG_LIMIT} (D-051)"
        ));
    }
    Ok(tag)
}

pub(crate) fn verify_op<'c>(
    _context: &'c Context,
    name: &str,
    op: OperationRef<'c, '_>,
) -> Result<(), String> {
    match name {
        "wrap" | "unwrap" => {
            tag_attr(op)?;
            Ok(())
        }
        "tag_of" | "table_new" | "raw_get" | "raw_set" | "table_len" | "set_meta"
        | "get_meta" | "payload_word" | "table_next" => Ok(()),
        other => Err(format!("no semantic verifier for frk_dyn.{other}")),
    }
}
