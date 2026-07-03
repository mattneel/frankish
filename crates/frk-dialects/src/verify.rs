//! The frankish semantic verifier (K1 second half, D-031): invariants
//! the IRDL constraint language cannot express, enforced by walking the
//! IR. Harness runners call this right after MLIR's own verifier and
//! before any execution or lowering (SPEC §3 K1 as amended).

use std::fmt;

use melior::Context;
use melior::ir::attribute::{StringAttribute, TypeAttribute};
use melior::ir::operation::OperationLike;
use melior::ir::r#type::FunctionType;
use melior::ir::{BlockLike, Module, OperationRef, RegionLike};

use crate::closure::SymbolTable;

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
    let symbols = symbol_table(module);
    let mut findings = Vec::new();
    let mut next = module.body().first_operation();
    while let Some(op) = next {
        walk(context, &symbols, op, &mut findings);
        next = op.next_in_block();
    }
    if findings.is_empty() {
        Ok(())
    } else {
        Err(VerifyErrors(findings))
    }
}

/// Module-level func.func signatures — the deep checks (closure callees)
/// resolve against this.
fn symbol_table<'c>(module: &Module<'c>) -> SymbolTable<'c> {
    let mut symbols = SymbolTable::new();
    let mut next = module.body().first_operation();
    while let Some(op) = next {
        let is_func = op
            .name()
            .as_string_ref()
            .as_str()
            .is_ok_and(|name| name == "func.func");
        if is_func {
            let name = op
                .attribute("sym_name")
                .ok()
                .and_then(|attribute| StringAttribute::try_from(attribute).ok())
                .map(|attribute| attribute.value().to_string());
            let function_type = op
                .attribute("function_type")
                .ok()
                .and_then(|attribute| TypeAttribute::try_from(attribute).ok())
                .and_then(|attribute| FunctionType::try_from(attribute.value()).ok());
            if let (Some(name), Some(function_type)) = (name, function_type) {
                symbols.insert(name, function_type);
            }
        }
        next = op.next_in_block();
    }
    symbols
}

fn walk<'c>(
    context: &'c Context,
    symbols: &SymbolTable<'c>,
    op: OperationRef<'c, '_>,
    findings: &mut Vec<Finding>,
) {
    check(context, symbols, op, findings);
    for region_index in 0..op.region_count() {
        let Ok(region) = op.region(region_index) else {
            continue;
        };
        let mut block = region.first_block();
        while let Some(current_block) = block {
            let mut inner = current_block.first_operation();
            while let Some(inner_op) = inner {
                walk(context, symbols, inner_op, findings);
                inner = inner_op.next_in_block();
            }
            block = current_block.next_in_region();
        }
    }
}

fn check<'c>(
    context: &'c Context,
    symbols: &SymbolTable<'c>,
    op: OperationRef<'c, '_>,
    findings: &mut Vec<Finding>,
) {
    let name = op.name();
    let Ok(name) = name.as_string_ref().as_str() else {
        return;
    };
    let outcome = if let Some(suffix) = name.strip_prefix("frk_adt.") {
        crate::adt::verify_op(context, suffix, op)
            .map_err(|message| format!("frk_adt.{suffix}: {message}"))
    } else if let Some(suffix) = name.strip_prefix("frk_closure.") {
        crate::closure::verify_op(context, symbols, suffix, op)
            .map_err(|message| format!("frk_closure.{suffix}: {message}"))
    } else {
        Ok(())
    };
    if let Err(message) = outcome {
        findings.push(Finding { op: op.to_string(), message });
    }
}
