//! frk_ctl — control effects (κ_frk, docs/ctl-calculus.md; D-060).
//!
//! v0 is the drop-clause subset: escape continuations as
//! prompt/abort with first-class prompt tokens.
//!
//! - `prompt` installs a fresh prompt and calls its body closure
//!   (`fn<[i64],[dyn]>` — the body receives the token); yields the
//!   body's return, or the aborted value if an abort targeted THIS
//!   prompt.
//! - `abort` unwinds to the live prompt whose token matches; a dead
//!   token traps "escape past extent (κ_frk)". In the reference
//!   interpreter this is a real unwind; natively it is result-passing
//!   through the runtime's pending flag (D-011 — Tier-0 friendly,
//!   no unwinder, works on wasm32).
//! - `pending` is the result-passing carrier: the FRONTEND threads
//!   explicit pending-checks after calls (κ_frk §3). The interpreter
//!   answers 0 always (real unwinds never re-enter frames); native
//!   code reads the runtime flag. Program outputs agree — that
//!   agreement is the license gate, enforced by L3.

use melior::Context;
use melior::ir::operation::OperationLike;
use melior::ir::{OperationRef, ValueLike};

use crate::closure::decode_fn;

pub const IRDL: &str = r#"
irdl.dialect @frk_ctl {
  irdl.operation @prompt {
    %body = irdl.base @frk_closure::@fn
    %r = irdl.base @frk_dyn::@dyn
    irdl.operands(body: %body)
    irdl.results(value: %r)
  }
  irdl.operation @abort {
    %tok = irdl.is i64
    %v = irdl.base @frk_dyn::@dyn
    irdl.operands(token: %tok, value: %v)
  }
  irdl.operation @pending {
    %p = irdl.is i64
    irdl.results(pending: %p)
  }
  irdl.operation @handle {
    %clause = irdl.base @frk_closure::@fn
    %body = irdl.base @frk_closure::@fn
    %r = irdl.base @frk_dyn::@dyn
    irdl.operands(clause: %clause, body: %body)
    irdl.results(value: %r)
  }
  irdl.operation @perform {
    %v = irdl.base @frk_dyn::@dyn
    %r = irdl.base @frk_dyn::@dyn
    irdl.operands(value: %v)
    irdl.results(out: %r)
  }
  irdl.operation @wind {
    %b = irdl.base @frk_closure::@fn
    %t = irdl.base @frk_closure::@fn
    %a = irdl.base @frk_closure::@fn
    %r = irdl.base @frk_dyn::@dyn
    irdl.operands(before: %b, thunk: %t, after: %a)
    irdl.results(value: %r)
  }
  irdl.operation @resume {
    %m = irdl.is i64
    %v = irdl.base @frk_dyn::@dyn
    %r = irdl.base @frk_dyn::@dyn
    irdl.operands(marker: %m, value: %v)
    irdl.results(out: %r)
  }
}
"#;

pub(crate) fn verify_op<'c>(
    context: &'c Context,
    name: &str,
    op: OperationRef<'c, '_>,
) -> Result<(), String> {
    match name {
        "prompt" => {
            let body_type = op
                .operand(0)
                .map_err(|_| "prompt without a body operand".to_string())?
                .r#type();
            let (params, results) = decode_fn(context, body_type)?;
            let param_ok = params.len() == 1 && params[0].to_string() == "i64";
            let result_ok =
                results.len() == 1 && results[0].to_string() == "!frk_dyn.dyn";
            if param_ok && result_ok {
                Ok(())
            } else {
                Err(format!(
                    "prompt body must be fn<[i64],[!frk_dyn.dyn]>, got {body_type}"
                ))
            }
        }
        "abort" | "pending" | "resume" | "wind" => Ok(()),
        "handle" => {
            // clause: the uniform pack fn; body: fn<[i64],[dyn]>
            // (prompt-shaped); label attribute present.
            op.attribute("label")
                .map_err(|_| "handle without a label attribute".to_string())?;
            let body_type = op
                .operand(1)
                .map_err(|_| "handle without a body operand".to_string())?
                .r#type();
            let (params, results) = decode_fn(context, body_type)?;
            let param_ok = params.len() == 1 && params[0].to_string() == "i64";
            let result_ok =
                results.len() == 1 && results[0].to_string() == "!frk_dyn.dyn";
            if param_ok && result_ok {
                Ok(())
            } else {
                Err(format!(
                    "handle body must be fn<[i64],[!frk_dyn.dyn]>, got {body_type}"
                ))
            }
        }
        "perform" => {
            op.attribute("label")
                .map_err(|_| "perform without a label attribute".to_string())?;
            Ok(())
        }
        other => Err(format!("no semantic verifier for frk_ctl.{other}")),
    }
}
