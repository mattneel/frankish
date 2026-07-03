//! frk.closure — first-class functions as `closure.make` (lifted
//! function symbol + captured values) and `closure.apply` (SPEC §4.2).
//! Dialect namespace `frk_closure`. Trait-free per D-031; strategy and
//! fences ruled in D-035.
//!
//! Type: `!frk_closure.fn<[param types], [result types]>` — the call
//! signature only; captures are existential (that is the point).
//!
//! Ops (packed surface, D-036 — no variadics):
//! - `make(env) {callee = @f} -> !frk_closure.fn<[p...], [r]>`
//!   `env` is a `!frk_adt.product` holding the captures. Calling
//!   convention: the lifted function takes the captures first, then the
//!   closure's parameters: `@f : (captures..., p...) -> r`. Captures are
//!   taken BY VALUE at make time (D-035; by-ref capture becomes
//!   meaningful when frk.mem introduces locations, M7).
//! - `apply(closure, args) -> r` — `args` is a `!frk_adt.product` of the
//!   closure's parameters; exactly one result (multi-result closures
//!   deferred, D-036 — every v1 specimen is single-valued).
//!
//! IRDL enforces shape (closure base types, symbol-ref attribute kind,
//! variadic arities); the frk verification pass enforces the deep
//! contract: the callee exists and its signature equals
//! (capture types ++ params) -> results; apply's args and results match
//! the closure type exactly.

use melior::Context;
use melior::ir::attribute::FlatSymbolRefAttribute;
use melior::ir::operation::OperationLike;
use melior::ir::r#type::FunctionType;
use melior::ir::{OperationRef, Type, ValueLike};
use std::collections::HashMap;

use crate::adt::decode_field_list;
use crate::attr_util::{array_elements, type_params};

/// The dialect definition, loaded together with frk_adt's in ONE module
/// by [`crate::register`] — the `@frk_adt::@product` references resolve
/// only within a combined module. (`irdl.base "#builtin.symbol_ref"`:
/// FlatSymbolRef has no separate registered name — it IS symbol_ref
/// without nesting.)
pub const IRDL: &str = r##"
irdl.dialect @frk_closure {
  irdl.type @fn {
    %params = irdl.any
    %results = irdl.any
    irdl.parameters(params: %params, results: %results)
  }
  irdl.operation @make {
    %env = irdl.base @frk_adt::@product
    %fn = irdl.base @frk_closure::@fn
    %sym = irdl.base "#builtin.symbol_ref"
    irdl.operands(env: %env)
    irdl.results(closure: %fn)
    irdl.attributes { "callee" = %sym }
  }
  irdl.operation @apply {
    %fn = irdl.base @frk_closure::@fn
    %args = irdl.base @frk_adt::@product
    %res = irdl.any
    irdl.operands(closure: %fn, args: %args)
    irdl.results(value: %res)
  }
}
"##;

/// Module-level function signatures, prebuilt by the verify driver.
pub(crate) type SymbolTable<'c> = HashMap<String, FunctionType<'c>>;

/// Decodes `!frk_closure.fn<[p...], [r...]>` into (params, results).
/// The two parameters print comma-separated, so `type_params` wraps
/// them into one array before parsing.
pub(crate) fn decode_fn<'c>(
    context: &'c Context,
    r#type: Type<'c>,
) -> Result<(Vec<Type<'c>>, Vec<Type<'c>>), String> {
    let both = type_params(context, r#type, "!frk_closure.fn<", true)?;
    let both = array_elements(both)
        .map_err(|attribute| format!("closure parameters must be arrays, got {attribute}"))?;
    let [params, results] = both.as_slice() else {
        return Err(format!(
            "closure type needs exactly [params], [results]; got {} parameter(s)",
            both.len()
        ));
    };
    Ok((
        decode_field_list(*params, "closure params")?,
        decode_field_list(*results, "closure results")?,
    ))
}

pub(crate) fn callee_name(op: OperationRef<'_, '_>) -> Result<String, String> {
    Ok(op
        .attribute("callee")
        .ok()
        .and_then(|attribute| FlatSymbolRefAttribute::try_from(attribute).ok())
        .ok_or_else(|| "closure.make without a callee symbol".to_string())?
        .value()
        .to_string())
}

/// Semantic verification (K1 second half) for `frk_closure.<name>`.
pub(crate) fn verify_op<'c>(
    context: &'c Context,
    symbols: &SymbolTable<'c>,
    name: &str,
    op: OperationRef<'c, '_>,
) -> Result<(), String> {
    match name {
        "make" => {
            let callee = callee_name(op)?;
            let function = symbols
                .get(&callee)
                .ok_or_else(|| format!("callee @{callee} is not a func.func in this module"))?;
            let (params, results) = decode_fn(
                context,
                op.result(0)
                    .map_err(|_| "make without a result".to_string())?
                    .r#type(),
            )?;
            let [result] = results.as_slice() else {
                return Err(format!(
                    "closure type declares {} result(s); exactly one (D-036)",
                    results.len()
                ));
            };

            let env = crate::adt::decode_product(
                context,
                op.operand(0)
                    .map_err(|_| "make without an env operand".to_string())?
                    .r#type(),
            )?;

            let expected_inputs = env.len() + params.len();
            if function.input_count() != expected_inputs {
                return Err(format!(
                    "@{callee} takes {} input(s); {} capture(s) + {} param(s) = {expected_inputs} expected",
                    function.input_count(),
                    env.len(),
                    params.len()
                ));
            }
            for (index, capture) in env.iter().enumerate() {
                let input = function.input(index).map_err(|e| e.to_string())?;
                if *capture != input {
                    return Err(format!(
                        "capture {index} has type {capture}, @{callee} input {index} is {input}"
                    ));
                }
            }
            for (offset, param) in params.iter().enumerate() {
                let input = function
                    .input(env.len() + offset)
                    .map_err(|e| e.to_string())?;
                if *param != input {
                    return Err(format!(
                        "closure param {offset} is {param}, @{callee} input {} is {input}",
                        env.len() + offset
                    ));
                }
            }
            if function.result_count() != 1 {
                return Err(format!(
                    "@{callee} returns {} value(s); closures return exactly one (D-036)",
                    function.result_count()
                ));
            }
            let actual = function.result(0).map_err(|e| e.to_string())?;
            if *result != actual {
                return Err(format!(
                    "closure result is {result}, @{callee} returns {actual}"
                ));
            }
            Ok(())
        }
        "apply" => {
            let closure_type = op
                .operand(0)
                .map_err(|_| "apply without a closure operand".to_string())?
                .r#type();
            let (params, results) = decode_fn(context, closure_type)?;
            let [result] = results.as_slice() else {
                return Err(format!(
                    "closure type declares {} result(s); exactly one (D-036)",
                    results.len()
                ));
            };

            let args = crate::adt::decode_product(
                context,
                op.operand(1)
                    .map_err(|_| "apply without an args operand".to_string())?
                    .r#type(),
            )?;
            if args.len() != params.len() {
                return Err(format!(
                    "arg pack has {} field(s), the closure takes {}",
                    args.len(),
                    params.len()
                ));
            }
            for (index, param) in params.iter().enumerate() {
                if args[index] != *param {
                    return Err(format!(
                        "arg {index} has type {}, the closure takes {param}",
                        args[index]
                    ));
                }
            }
            let actual = op
                .result(0)
                .map_err(|_| "apply without a result".to_string())?
                .r#type();
            if actual != *result {
                return Err(format!(
                    "apply result has type {actual}, the closure returns {result}"
                ));
            }
            Ok(())
        }
        other => Err(format!("no semantic verifier for frk_closure.{other}")),
    }
}
