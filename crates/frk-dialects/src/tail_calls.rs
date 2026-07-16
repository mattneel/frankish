//! frk-tail-calls (M14, D-059 rung 2): the native half of the tail-
//! call law. Runs LAST in the pipeline, over final LLVM-dialect form:
//! any DIRECT `llvm.call` in tail shape — its results are exactly the
//! operands of the immediately following `llvm.return` — whose callee
//! has an LLVM function type IDENTICAL to its caller's gets
//! `TailCallKind = musttail`, which LLVM guarantees to lower as a
//! frame-replacing jump.
//!
//! Two qualifying cases:
//! - DIRECT (M14): the callee's LLVM function type is IDENTICAL to
//!   the caller's — self-recursion always qualifies, equal-signature
//!   mutual recursion too.
//! - INDIRECT (M18, D-063): the CALLSITE prototype — reconstructed
//!   from the call's operand/result types — equals the caller's
//!   function type. Under the uniform-signature convention every
//!   closure-carried function of a pack language is `(ptr, ptr) ->
//!   ptr`, so lua tail applies qualify by construction.
//! Cross-signature tails remain unmarked; the interpreter's
//! trampoline covers ALL shapes — reference semantics leads, native
//! follows. NOTE (D-063 fence): under the rc strategy, block-exit
//! releases sit between a tail call and its return, breaking the tail
//! shape — rc-native deep recursion stays unguaranteed until release
//! scheduling gets its own rung.

use std::collections::HashMap;

use melior::ir::attribute::{Attribute, FlatSymbolRefAttribute, StringAttribute, TypeAttribute};
use melior::ir::operation::{OperationLike, OperationMutLike};
use melior::ir::r#type::TypeId;
use melior::ir::{BlockLike, OperationRef, RegionLike, ValueLike};
use melior::pass::{ExternalPass, Pass, create_external};

#[repr(align(8))]
struct PassId;
static TAIL_CALLS_PASS_ID: PassId = PassId;

pub fn tail_calls_pass() -> Pass {
    create_external(
        |operation: OperationRef, pass: ExternalPass| {
            if let Err(message) = mark_tail_calls(operation) {
                eprintln!("frk-tail-calls: {message}");
                pass.signal_failure();
            }
        },
        TypeId::create(&TAIL_CALLS_PASS_ID),
        "frk-tail-calls",
        "frk-tail-calls",
        "rewrite identical-signature direct tail calls to musttail (D-059)",
        "",
        &[],
    )
}

fn attr_string(op: OperationRef<'_, '_>, name: &str) -> Option<String> {
    op.attribute(name)
        .ok()
        .and_then(|attribute| StringAttribute::try_from(attribute).ok())
        .map(|attribute| attribute.value().to_string())
}

fn mark_tail_calls(module: OperationRef<'_, '_>) -> Result<(), String> {
    let context = unsafe { module.context().to_ref() };

    // Pass 1: llvm.func symbol → printed function type.
    let mut signatures: HashMap<String, String> = HashMap::new();
    let body = module
        .region(0)
        .map_err(|e| e.to_string())?
        .first_block()
        .ok_or_else(|| "module without a body".to_string())?;
    let mut next = body.first_operation();
    while let Some(op) = next {
        if op
            .name()
            .as_string_ref()
            .as_str()
            .is_ok_and(|name| name == "llvm.func")
        {
            if let (Some(symbol), Ok(ty)) = (attr_string(op, "sym_name"), op.attribute("function_type")) {
                if let Ok(type_attr) = TypeAttribute::try_from(ty) {
                    signatures.insert(symbol, type_attr.value().to_string());
                }
            }
        }
        next = op.next_in_block();
    }

    let musttail = Attribute::parse(context, "#llvm.tailcallkind<musttail>")
        .ok_or_else(|| "unparsable musttail kind".to_string())?;

    // Pass 2 (read-only): collect qualifying call ops by raw pointer.
    let mut qualifying: Vec<usize> = Vec::new();
    let mut next_fn = body.first_operation();
    while let Some(function) = next_fn {
        let is_func = function
            .name()
            .as_string_ref()
            .as_str()
            .is_ok_and(|name| name == "llvm.func");
        if is_func {
            let caller_type = function
                .attribute("function_type")
                .ok()
                .and_then(|a| TypeAttribute::try_from(a).ok())
                .map(|a| a.value().to_string());
            if let (Some(caller_type), Ok(region)) = (caller_type, function.region(0)) {
                let mut block = region.first_block();
                while let Some(current) = block {
                    let mut next_op = current.first_operation();
                    while let Some(op) = next_op {
                        if qualifies(op, &caller_type, &signatures) {
                            qualifying.push(op.to_raw().ptr as usize);
                        }
                        next_op = op.next_in_block();
                    }
                    block = current.next_in_region();
                }
            }
        }
        next_fn = function.next_in_block();
    }

    // Pass 3 (mutating): set musttail on the collected ops.
    let qualifying: std::collections::HashSet<usize> = qualifying.into_iter().collect();
    let mut next_fn = body.first_operation_mut();
    while let Some(function) = next_fn {
        let following_fn = function.next_in_block_mut();
        if let Ok(region) = function.region(0) {
            let mut block = region.first_block();
            while let Some(current) = block {
                let next_block = current.next_in_region();
                let mut next_op = current.first_operation_mut();
                while let Some(mut op) = next_op {
                    let following = op.next_in_block_mut();
                    if qualifying.contains(&(op.to_raw().ptr as usize)) {
                        op.set_attribute("TailCallKind", musttail);
                    }
                    next_op = following;
                }
                block = next_block;
            }
        }
        next_fn = following_fn;
    }
    Ok(())
}

/// Tail shape + direct callee + identical caller/callee LLVM type.
fn qualifies(
    op: OperationRef<'_, '_>,
    caller_type: &str,
    signatures: &HashMap<String, String>,
) -> bool {
    let is_call = op
        .name()
        .as_string_ref()
        .as_str()
        .is_ok_and(|name| name == "llvm.call");
    if !is_call {
        return false;
    }
    // DIRECT: a flat-symbol callee that resolves to an llvm.func with
    // the caller's exact type. INDIRECT (D-063): no callee attribute —
    // operand 0 is the function pointer; the callsite prototype
    // (reconstructed from operand/result types) must equal the
    // caller's type.
    let callee = op
        .attribute("callee")
        .ok()
        .and_then(|attribute| FlatSymbolRefAttribute::try_from(attribute).ok())
        .map(|attribute| attribute.value().to_string());
    match callee {
        Some(callee) => {
            let Some(callee_type) = signatures.get(&callee) else {
                return false;
            };
            if callee_type != caller_type {
                return false;
            }
        }
        None => {
            if op.operand_count() == 0 {
                return false;
            }
            let Some((caller_ret, caller_args)) = split_llvm_fn_type(caller_type) else {
                return false;
            };
            // Standalone types print with the "!llvm." prefix, but
            // inside !llvm.func<…> they print in bare LLVM shorthand
            // ("ptr", "struct<(i64, i64)>") — normalize before
            // comparing.
            let norm = |printed: String| -> String {
                printed
                    .strip_prefix("!llvm.")
                    .map(str::to_string)
                    .unwrap_or(printed)
            };
            let callsite_args = (1..op.operand_count())
                .map(|index| {
                    norm(
                        op.operand(index)
                            .map(|operand| operand.r#type().to_string())
                            .unwrap_or_default(),
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            let callsite_ret = if op.result_count() == 1 {
                norm(
                    op.result(0)
                        .map(|result| result.r#type().to_string())
                        .unwrap_or_default(),
                )
            } else {
                "void".to_string()
            };
            if caller_ret != callsite_ret || caller_args != callsite_args {
                return false;
            }
        }
    }
    // Tail shape: immediately followed by llvm.return of exactly the
    // call's results.
    let Some(following) = op.next_in_block() else {
        return false;
    };
    let is_return = following
        .name()
        .as_string_ref()
        .as_str()
        .is_ok_and(|name| name == "llvm.return");
    if !is_return || op.result_count() != following.operand_count() {
        return false;
    }
    for index in 0..op.result_count() {
        let (Ok(result), Ok(operand)) = (op.result(index), following.operand(index)) else {
            return false;
        };
        if result.to_raw().ptr != operand.to_raw().ptr {
            return false;
        }
    }
    true
}

/// Splits a printed `!llvm.func<RET (ARGS)>` into (RET, ARGS).
fn split_llvm_fn_type(printed: &str) -> Option<(String, String)> {
    let inner = printed.strip_prefix("!llvm.func<")?.strip_suffix('>')?;
    let open = inner.find(" (")?;
    let ret = inner[..open].to_string();
    let args = inner[open + 2..].strip_suffix(')')?.to_string();
    Some((ret, args))
}
