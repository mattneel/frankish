//! The frankish semantic verifier (K1 second half, D-031): invariants
//! the IRDL constraint language cannot express, enforced by walking the
//! IR. Harness runners call this right after MLIR's own verifier and
//! before any execution or lowering (SPEC §3 K1 as amended).

use std::fmt;

use melior::Context;
use melior::ir::operation::OperationLike;
use melior::ir::{BlockLike, Module, OperationRef, RegionLike};

#[derive(Debug)]
pub struct Finding {
    /// The offending op, printed in its generic form.
    pub op: String,
    pub message: String,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}\n  at: {}", self.message, self.op)
    }
}

#[derive(Debug)]
pub struct VerifyErrors(pub Vec<Finding>);

impl fmt::Display for VerifyErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, finding) in self.0.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            write!(f, "{finding}")?;
        }
        Ok(())
    }
}

impl std::error::Error for VerifyErrors {}

/// Walks every op in `module` (recursively through regions) and checks
/// frankish semantic invariants. `context` must be the module's own
/// context (runners hold it already).
pub fn verify<'c>(context: &'c Context, module: &Module<'c>) -> Result<(), VerifyErrors> {
    let mut findings = Vec::new();
    let mut next = module.body().first_operation();
    while let Some(op) = next {
        walk(context, op, &mut findings);
        next = op.next_in_block();
    }
    if findings.is_empty() {
        Ok(())
    } else {
        Err(VerifyErrors(findings))
    }
}

fn walk<'c>(context: &'c Context, op: OperationRef<'c, '_>, findings: &mut Vec<Finding>) {
    check(context, op, findings);
    for region_index in 0..op.region_count() {
        let Ok(region) = op.region(region_index) else {
            continue;
        };
        let mut block = region.first_block();
        while let Some(current_block) = block {
            let mut inner = current_block.first_operation();
            while let Some(inner_op) = inner {
                walk(context, inner_op, findings);
                inner = inner_op.next_in_block();
            }
            block = current_block.next_in_region();
        }
    }
}

fn check<'c>(context: &'c Context, op: OperationRef<'c, '_>, findings: &mut Vec<Finding>) {
    let name = op.name();
    let Ok(name) = name.as_string_ref().as_str() else {
        return;
    };
    if let Some(suffix) = name.strip_prefix("frk_adt.") {
        if let Err(message) = crate::adt::verify_op(context, suffix, op) {
            findings.push(Finding {
                op: op.to_string(),
                message: format!("frk_adt.{suffix}: {message}"),
            });
        }
    }
}
