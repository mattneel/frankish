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
    // The registered-ABI declaration check (M17, D-062): any func.func
    // DECLARATION (bodyless) named frk_rt_* — hand-written in a
    // frontend or carried by an intrinsics module — must project to
    // the frk-abi registry row. Catches signature drift at verify
    // time, before any execution.
    if name == "func.func" {
        if let Err(message) = check_rt_declaration(op) {
            findings.push(Finding { op: op.to_string(), message });
            return;
        }
    }
    let outcome = if let Some(suffix) = name.strip_prefix("frk_adt.") {
        crate::adt::verify_op(context, suffix, op)
            .map_err(|message| format!("frk_adt.{suffix}: {message}"))
    } else if let Some(suffix) = name.strip_prefix("frk_closure.") {
        crate::closure::verify_op(context, symbols, suffix, op)
            .map_err(|message| format!("frk_closure.{suffix}: {message}"))
    } else if let Some(suffix) = name.strip_prefix("frk_mem.") {
        crate::mem::verify_op(context, suffix, op)
            .map_err(|message| format!("frk_mem.{suffix}: {message}"))
    } else if let Some(suffix) = name.strip_prefix("frk_bstr.") {
        crate::bstr::verify_op(context, suffix, op)
            .map_err(|message| format!("frk_bstr.{suffix}: {message}"))
    } else if let Some(suffix) = name.strip_prefix("frk_dyn.") {
        crate::dyn_dialect::verify_op(context, suffix, op)
            .map_err(|message| format!("frk_dyn.{suffix}: {message}"))
    } else if let Some(suffix) = name.strip_prefix("frk_str.") {
        crate::str_dialect::verify_op(context, suffix, op)
            .map_err(|message| format!("frk_str.{suffix}: {message}"))
    } else if let Some(suffix) = name.strip_prefix("frk_ctl.") {
        crate::ctl::verify_op(context, suffix, op)
            .map_err(|message| format!("frk_ctl.{suffix}: {message}"))
    } else {
        Ok(())
    };
    if let Err(message) = outcome {
        findings.push(Finding { op: op.to_string(), message });
    }
}

/// Projects a pre-lowering declaration type onto the ABI vocabulary
/// and compares with the registry (D-062). Classes, not exact types:
/// every `!frk_*` kernel type and `!llvm.ptr` are pointer-class; i64
/// matches I64/U64; i1/i8 match U8 (the widening rule); f64 matches
/// F64. A declaration for an UNREGISTERED frk_rt_* symbol is itself a
/// finding — the registry is the roster.
fn check_rt_declaration(op: OperationRef<'_, '_>) -> Result<(), String> {
    let name = match op
        .attribute("sym_name")
        .ok()
        .and_then(|a| StringAttribute::try_from(a).ok())
    {
        Some(attribute) => attribute.value().to_string(),
        None => return Ok(()),
    };
    if !name.starts_with("frk_rt_") {
        return Ok(());
    }
    // Declarations only: a bodyless first region.
    let is_declaration = op
        .region(0)
        .map(|region| region.first_block().is_none())
        .unwrap_or(true);
    if !is_declaration {
        return Ok(());
    }
    let entry = frk_abi::find(&name)
        .ok_or_else(|| format!("{name} is declared but not in the frk-abi registry (D-062)"))?;
    let function_type = op
        .attribute("function_type")
        .ok()
        .and_then(|a| TypeAttribute::try_from(a).ok())
        .map(|a| a.value())
        .and_then(|t| FunctionType::try_from(t).ok())
        .ok_or_else(|| format!("{name}: declaration without a function type"))?;

    let class_of = |printed: &str| -> &'static str {
        if printed.starts_with("!frk_") || printed.starts_with("!llvm.ptr") {
            "ptr"
        } else if printed == "i64" {
            "i64"
        } else if printed == "i1" || printed == "i8" {
            "u8"
        } else if printed == "f64" {
            "f64"
        } else {
            "other"
        }
    };
    let abi_class = |ty: frk_abi::AbiTy| -> &'static str {
        if ty.is_pointer() {
            "ptr"
        } else {
            match ty {
                frk_abi::AbiTy::F64 => "f64",
                frk_abi::AbiTy::U8 => "u8",
                _ => "i64",
            }
        }
    };
    // u8-class registry rows accept i64-class declarations too (the
    // widening direction is always safe; the twins take the narrow
    // type only on legacy print flags).
    let matches = |declared: &str, registered: &'static str| {
        declared == registered || (registered == "u8" && declared == "i64") ||
            (registered == "i64" && declared == "u8")
    };

    let declared_inputs = function_type.input_count();
    if declared_inputs != entry.args.len() {
        return Err(format!(
            "{name}: declared with {declared_inputs} argument(s); the registry says {}",
            entry.args.len()
        ));
    }
    for index in 0..declared_inputs {
        let input = function_type
            .input(index)
            .map_err(|_| format!("{name}: unreadable input {index}"))?;
        let declared = class_of(&input.to_string());
        let registered = abi_class(entry.args[index]);
        if !matches(declared, registered) {
            return Err(format!(
                "{name}: argument {index} is {declared}-class; the registry says {registered} (D-062)"
            ));
        }
    }
    let declared_results = function_type.result_count();
    match (declared_results, entry.ret) {
        (0, None) => {}
        (1, Some(ret)) => {
            let result = function_type
                .result(0)
                .map_err(|_| format!("{name}: unreadable result"))?;
            let declared = class_of(&result.to_string());
            let registered = abi_class(ret);
            if !matches(declared, registered) {
                return Err(format!(
                    "{name}: result is {declared}-class; the registry says {registered} (D-062)"
                ));
            }
        }
        (got, expected) => {
            return Err(format!(
                "{name}: declared with {got} result(s); the registry says {}",
                if expected.is_some() { 1 } else { 0 }
            ));
        }
    }
    Ok(())
}
