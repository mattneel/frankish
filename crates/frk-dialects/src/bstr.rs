//! frk.bstr — interned byte strings (M11; D-052/D-056). Lua's string
//! semantics: 8-bit values, INTERNED at creation, identity-equal —
//! which is why eq lowers to a pointer comparison and len to a header
//! load; only intern/concat/from_num touch the runtime.
//!
//! Deliberately a sibling of frk_str, not an overload: UTF-16
//! code-unit semantics is TS's law, byte semantics is Lua's.

use melior::Context;
use melior::ir::OperationRef;
use melior::ir::operation::OperationLike;

pub const IRDL: &str = r#"
irdl.dialect @frk_bstr {
  irdl.type @str {}
  irdl.operation @lit {
    %s = irdl.base @frk_bstr::@str
    irdl.results(value: %s)
  }
  irdl.operation @concat {
    %s = irdl.base @frk_bstr::@str
    irdl.operands(lhs: %s, rhs: %s)
    irdl.results(value: %s)
  }
  irdl.operation @eq {
    %s = irdl.base @frk_bstr::@str
    %b = irdl.is i1
    irdl.operands(lhs: %s, rhs: %s)
    irdl.results(value: %b)
  }
  irdl.operation @len {
    %s = irdl.base @frk_bstr::@str
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
            let text = op
                .attribute("text")
                .map_err(|_| "frk_bstr.lit without a text attribute".to_string())?;
            // v0.1 literal fence (D-056): decoded bytes are printable
            // ASCII + \t \n — the attribute round-trips losslessly.
            let printed = text.to_string();
            if printed.bytes().any(|byte| byte >= 0x80) {
                return Err(
                    "non-ASCII byte-string literal (fenced in v0.1, D-056)".to_string()
                );
            }
            Ok(())
        }
        "concat" | "eq" | "len" => Ok(()),
        other => Err(format!("no semantic verifier for frk_bstr.{other}")),
    }
}
