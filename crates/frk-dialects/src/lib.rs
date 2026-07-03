//! frk-dialects — the kernel dialect library, one module per dialect
//! (SPEC §4), each shipped whole under the K1–K7 contract (SPEC §3,
//! D-007). Registration is IRDL runtime loading and nothing else
//! (D-031): dialect designs stay trait-free — no custom terminators,
//! successors, or trait-relaxed regions.
//!
//! Residents: [`adt`] (M3, in progress). Runners obtain kernel-aware
//! contexts by calling [`register`] right after `frk_core::context()`.

pub mod adt;
pub mod adt_dtree;
pub mod dtree_emit;
mod adt_eval;
mod attr_util;
pub mod closure;
mod closure_eval;
pub mod mem;
mod mem_eval;
pub mod bstr;
mod bstr_eval;
pub mod dyn_dialect;
mod dyn_eval;
pub mod str_dialect;
mod str_eval;
pub mod ctl;
mod ctl_eval;
pub mod tail_calls;
pub mod verify;

mod kernel_lower;

pub use kernel_lower::{Strategy, lower_kernel_pass};
pub use tail_calls::tail_calls_pass;
pub use verify::{Finding, VerifyErrors, verify};

/// Registers every kernel dialect's evaluators into an interpreter —
/// the K2 hook harness runners call right after `Interp::new`.
pub fn register_eval(interp: &mut frk_interp::Interp<'_, '_>) {
    adt_eval::register_eval(interp);
    closure_eval::register_eval(interp);
    mem_eval::register_eval(interp);
    str_eval::register_eval(interp);
    dyn_eval::register_eval(interp);
    bstr_eval::register_eval(interp);
    ctl_eval::register_eval(interp);
}

use std::fmt;

use melior::Context;
use melior::ir::Module;
use melior::utility::load_irdl_dialects;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegisterError {
    /// The embedded IRDL source failed to parse — a defect in this
    /// crate, not an input condition (the source is a constant).
    Parse(&'static str),
    /// mlirLoadIRDLDialects rejected the definitions.
    Load(&'static str),
}

impl fmt::Display for RegisterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(dialect) => write!(f, "embedded IRDL for {dialect} failed to parse"),
            Self::Load(dialect) => write!(f, "IRDL loading failed for {dialect}"),
        }
    }
}

impl std::error::Error for RegisterError {}

/// Registers every frankish kernel dialect into `context`.
///
/// Precondition: the upstream `irdl` dialect is loaded in `context` —
/// `frk_core::context()` qualifies (it loads all available dialects).
pub fn register(context: &Context) -> Result<(), RegisterError> {
    // One combined module: frk_closure's IRDL references
    // @frk_adt::@product, and IRDL symbol refs resolve only within the
    // module being loaded.
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        adt::IRDL,
        closure::IRDL,
        mem::IRDL,
        str_dialect::IRDL,
        dyn_dialect::IRDL,
        bstr::IRDL,
        ctl::IRDL
    );
    register_one(context, &combined, "frk kernel dialects")
}

fn register_one(
    context: &Context,
    source: &str,
    dialect: &'static str,
) -> Result<(), RegisterError> {
    let definitions =
        Module::parse(context, source).ok_or(RegisterError::Parse(dialect))?;
    if load_irdl_dialects(&definitions) {
        Ok(())
    } else {
        Err(RegisterError::Load(dialect))
    }
}
