//! frk.str — immutable UTF-16 string values (M9, D-049). JS semantics:
//! `.length` counts code units (surrogate pairs count 2). Everything
//! lowers to rt calls; literals become UTF-16 globals. Strings are
//! rt-owned (plain malloc inside the rt, strategy-uniform) — NOT rc
//! headers; the slot model treats them as unmanaged pointers.

use melior::Context;
use melior::ir::OperationRef;
use melior::ir::operation::OperationLike;

pub const IRDL: &str = r#"
irdl.dialect @frk_str {
  irdl.type @str {}
  irdl.operation @lit {
    %s = irdl.base @frk_str::@str
    irdl.results(value: %s)
  }
  irdl.operation @concat {
    %s = irdl.base @frk_str::@str
    irdl.operands(lhs: %s, rhs: %s)
    irdl.results(value: %s)
  }
  irdl.operation @eq {
    %s = irdl.base @frk_str::@str
    %b = irdl.is i1
    irdl.operands(lhs: %s, rhs: %s)
    irdl.results(value: %b)
  }
  irdl.operation @len {
    %s = irdl.base @frk_str::@str
    %n = irdl.is i64
    irdl.operands(value: %s)
    irdl.results(len: %n)
  }
}
"#;

pub(crate) fn verify_op<'c>(
    _context: &'c Context,
    name: &str,
    op: OperationRef<'c, '_>,
) -> Result<(), String> {
    match name {
        "lit" => {
            // The text attribute is mandatory (UTF-8 in IR; lowering
            // re-encodes UTF-16).
            op.attribute("text")
                .map_err(|_| "frk_str.lit without a text attribute".to_string())?;
            Ok(())
        }
        "concat" | "eq" | "len" => Ok(()),
        other => Err(format!("no semantic verifier for frk_str.{other}")),
    }
}




