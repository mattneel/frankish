//! frk-dialects — the kernel dialect library, one module per dialect
//! (SPEC §4), each shipped whole under the K1–K7 contract (SPEC §3,
//! D-007). Registration is IRDL runtime loading and nothing else
//! (D-031): dialect designs stay trait-free — no custom terminators,
//! successors, or trait-relaxed regions.
//!
//! Residents: [`adt`] (M3, in progress). Runners obtain kernel-aware
//! contexts by calling [`register`] right after `frk_core::context()`.

pub mod adt;
mod adt_eval;
mod adt_lower;
pub mod verify;

pub use adt_eval::register_eval;
pub use adt_lower::lower_adt_pass;
pub use verify::{Finding, VerifyErrors, verify};

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
    register_one(context, adt::IRDL, "frk_adt")
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
