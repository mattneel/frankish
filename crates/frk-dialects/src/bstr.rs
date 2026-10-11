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
  irdl.operation @sub {
    %s = irdl.base @frk_bstr::@str
    %n = irdl.is i64
    irdl.operands(value: %s, from: %n, to: %n)
    irdl.results(out: %s)
  }
  irdl.operation @rep {
    %s = irdl.base @frk_bstr::@str
    %n = irdl.is i64
    irdl.operands(value: %s, count: %n)
    irdl.results(out: %s)
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
            // v0.2 (D-098, amending D-056's v0.1 literal fence): the
            // decoded attribute bytes are admitted AS-IS — arbitrary
            // 8-bit bytes included. The IR attribute carries decoded
            // bytes; intern/len/sub are byte-based end to end. The
            // fence ruled on the ENCODING; byte strings have none.
            let _ = crate::attr_util::string_attr_bytes(text)?;
            Ok(())
        }
        "concat" | "eq" | "len" | "sub" | "rep" => Ok(()),
        other => Err(format!("no semantic verifier for frk_bstr.{other}")),
    }
}
