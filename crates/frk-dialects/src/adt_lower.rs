//! K3 for frk.adt: lowering to LLVM-dialect struct values, packaged as a
//! real MLIR external pass so it slots into the shared pipeline table —
//! and therefore into stage dumps — like any upstream pass (D-032).
//!
//! v0 representation (D-032): a sum with variants V and K = max field
//! count lowers to `!llvm.struct<(i64, i64 × K)>` — the tag plus K
//! uniform i64 slots; a product with F fields lowers to
//! `!llvm.struct<(i64 × F)>` (no tag). Narrow integer fields zero-extend
//! into their slot and truncate back out. Deliberately wasteful and
//! obviously correct: niche/tag-packing is a later, separately-goldened
//! pass (D-025, SPEC §4.1).
//!
//! v0 fence: field types must be builtin integers ≤ 64 bits. Anything
//! else — including nested adts — fails the pass loudly; representation
//! work for those arrives with the memory axis (frk.mem, M7).
//!
//! Wrong-variant extract is *unspecified* in lowered code (it reads
//! whatever the slot holds) while the interpreter traps on it (D-029):
//! extracts must be tag-guarded, which is exactly what the decision-tree
//! pass emits. An unguarded extract can therefore never be a golden.
//!
//! Mechanics: plan-then-apply. One walk collects (a) op rewrite plans
//! with layouts decoded from the *original* frk types, (b) every
//! non-frk-op value needing a type swap (block arguments, op results),
//! (c) function signatures mentioning frk types. Apply then swaps types
//! in place (`set_type`), rewrites `function_type` attributes, and
//! replaces each frk op with its llvm/arith expansion in program order
//! (operands are re-read live, so producer replacements are visible).

use melior::dialect::llvm;
use melior::ir::attribute::{DenseI64ArrayAttribute, IntegerAttribute, TypeAttribute};
use melior::ir::operation::{OperationBuilder, OperationLike, OperationMutLike};
use melior::ir::r#type::{FunctionType, IntegerType, TypeId};
use melior::ir::{BlockLike, Location, OperationRef, RegionLike, Type, Value, ValueLike};
use melior::pass::{ExternalPass, Pass, create_external};
use melior::{Context, IrRewriter, RewriterBase};
use std::collections::HashMap;

use crate::adt::{decode_product, decode_sum};

#[repr(align(8))]
struct PassId;
static LOWER_ADT_PASS_ID: PassId = PassId;

/// Constructs the pass; the shared pipeline table calls this exactly
/// like the upstream `create_*` constructors.
pub fn lower_adt_pass() -> Pass {
    create_external(
        |operation: OperationRef, pass: ExternalPass| {
            if let Err(message) = lower_adt(operation) {
                eprintln!("lower-frk-adt: {message}");
                pass.signal_failure();
            }
        },
        TypeId::create(&LOWER_ADT_PASS_ID),
        "lower-frk-adt",
        "lower-frk-adt",
        "lower frk_adt ops and types to LLVM struct values (D-032)",
        // Anchor on any op; the pipeline adds it at module level.
        "",
        // frk contexts load every dialect eagerly (frk-core policy), so
        // no dependent-dialect loading is needed here.
        &[],
    )
}

enum Planned<'c, 'a> {
    /// make_sum: tag slot + verbatim copy of the payload product's
    /// (already-widened) slots.
    MakeSum {
        op: OperationRef<'c, 'a>,
        tag: i64,
        container: Type<'c>,
        payload_slots: usize,
    },
    TagOf {
        op: OperationRef<'c, 'a>,
    },
    Read {
        op: OperationRef<'c, 'a>,
        slot: i64,
        width: u32,
    },
    ProductNew {
        op: OperationRef<'c, 'a>,
        container: Type<'c>,
    },
    /// product_snoc: copy old slots verbatim, widen and append one.
    ProductSnoc {
        op: OperationRef<'c, 'a>,
        container: Type<'c>,
        old_slots: usize,
        width: u32,
    },
}

/// Lowers every frk_adt op and type under `module` (any op works as the
/// root; the pipeline anchors it on builtin.module).
pub fn lower_adt(module: OperationRef<'_, '_>) -> Result<(), String> {
    // Sound: the context strictly outlives every IR object walked here.
    let context = unsafe { module.context().to_ref() };

    let mut plans = Vec::new();
    let mut retypes = Vec::new();
    let mut signatures = HashMap::new();
    collect(context, module, &mut plans, &mut retypes, &mut signatures)?;

    for (value, mapped) in &retypes {
        value.set_type(*mapped);
    }

    rewrite_signatures(module, &signatures);

    let rewriter = IrRewriter::new(context);
    let rewriter = rewriter.as_rewriter_base();
    for plan in plans {
        apply(context, &rewriter, plan)?;
    }
    Ok(())
}

fn collect<'c, 'a>(
    context: &'c Context,
    op: OperationRef<'c, 'a>,
    plans: &mut Vec<Planned<'c, 'a>>,
    retypes: &mut Vec<(Value<'c, 'a>, Type<'c>)>,
    signatures: &mut HashMap<usize, Type<'c>>,
) -> Result<(), String> {
    let name = op
        .name()
        .as_string_ref()
        .as_str()
        .map_err(|_| "non-UTF-8 op name".to_string())?
        .to_string();

    if let Some(suffix) = name.strip_prefix("frk_adt.") {
        plans.push(plan_op(context, suffix, op)?);
    } else {
        if name == "func.func" {
            if let Some(mapped) = mapped_signature(context, op)? {
                signatures.insert(op.to_raw().ptr as usize, mapped);
            }
        }
        for index in 0..op.result_count() {
            let result = op.result(index).map_err(|e| e.to_string())?;
            if is_frk_type(result.r#type()) {
                retypes.push((result.into(), map_type(context, result.r#type())?));
            }
        }
    }

    for region_index in 0..op.region_count() {
        let Ok(region) = op.region(region_index) else {
            continue;
        };
        let mut block = region.first_block();
        while let Some(current) = block {
            for arg_index in 0..current.argument_count() {
                let argument = current.argument(arg_index).map_err(|e| e.to_string())?;
                if is_frk_type(argument.r#type()) {
                    retypes.push((argument.into(), map_type(context, argument.r#type())?));
                }
            }
            let mut inner = current.first_operation();
            while let Some(inner_op) = inner {
                collect(context, inner_op, plans, retypes, signatures)?;
                inner = inner_op.next_in_block();
            }
            block = current.next_in_region();
        }
    }
    Ok(())
}

fn plan_op<'c, 'a>(
    context: &'c Context,
    suffix: &str,
    op: OperationRef<'c, 'a>,
) -> Result<Planned<'c, 'a>, String> {
    let index = |name: &str| crate::adt::index_attr(op, name);
    let result_type = || {
        op.result(0)
            .map(|result| result.r#type())
            .map_err(|_| "frk op without a result".to_string())
    };
    let operand_type = || {
        op.operand(0)
            .map(|operand| operand.r#type())
            .map_err(|_| "frk op without an operand".to_string())
    };

    match suffix {
        "product_new" => Ok(Planned::ProductNew {
            op,
            container: map_type(context, result_type()?)?,
        }),
        "product_snoc" => {
            let old_fields = decode_product(context, operand_type()?)?;
            field_widths(&old_fields)?;
            let appended = op
                .operand(1)
                .map_err(|_| "snoc without a value operand".to_string())?
                .r#type();
            let width = field_widths(&[appended])?[0];
            Ok(Planned::ProductSnoc {
                op,
                container: map_type(context, result_type()?)?,
                old_slots: old_fields.len(),
                width,
            })
        }
        "make_sum" => {
            let variants = decode_sum(context, result_type()?)?;
            let tag = index("variant")? as i64;
            field_widths(
                variants
                    .get(tag as usize)
                    .ok_or_else(|| format!("variant {tag} out of range"))?,
            )?;
            let payload = decode_product(context, operand_type()?)?;
            Ok(Planned::MakeSum {
                op,
                tag,
                container: map_type(context, result_type()?)?,
                payload_slots: payload.len(),
            })
        }
        "tag_of" => Ok(Planned::TagOf { op }),
        "extract" => {
            let variants = decode_sum(context, operand_type()?)?;
            let variant = index("variant")?;
            let field = index("field")?;
            let width = field_widths(
                variants
                    .get(variant)
                    .ok_or_else(|| format!("variant {variant} out of range"))?,
            )?
            .get(field)
            .copied()
            .ok_or_else(|| format!("field {field} out of range"))?;
            Ok(Planned::Read {
                op,
                slot: 1 + field as i64,
                width,
            })
        }

        "get" => {
            let fields = decode_product(context, operand_type()?)?;
            let field = index("field")?;
            let width = field_widths(&fields)?
                .get(field)
                .copied()
                .ok_or_else(|| format!("field {field} out of range"))?;
            Ok(Planned::Read {
                op,
                slot: field as i64,
                width,
            })
        }
        other => Err(format!("no lowering for frk_adt.{other}")),
    }
}

fn is_frk_type(r#type: Type<'_>) -> bool {
    r#type.to_string().starts_with("!frk_adt.")
}

/// v0 fence: fields must be builtin integers ≤ 64 bits (D-032).
fn field_widths(fields: &[Type<'_>]) -> Result<Vec<u32>, String> {
    fields
        .iter()
        .map(|field| {
            let width = IntegerType::try_from(*field)
                .map_err(|_| {
                    format!(
                        "v0 lowering supports integer fields only, got {field} \
                         (nested adts arrive with the memory axis, M7)"
                    )
                })?
                .width();
            if width > 64 {
                return Err(format!("field width {width} exceeds 64"));
            }
            Ok(width)
        })
        .collect()
}

fn map_type<'c>(context: &'c Context, r#type: Type<'c>) -> Result<Type<'c>, String> {
    let printed = r#type.to_string();
    if printed.starts_with("!frk_adt.sum<") {
        let variants = decode_sum(context, r#type)?;
        let mut max_fields = 0;
        for fields in &variants {
            field_widths(fields)?;
            max_fields = max_fields.max(fields.len());
        }
        Ok(slots_struct(context, 1 + max_fields))
    } else if printed.starts_with("!frk_adt.product<") {
        let fields = decode_product(context, r#type)?;
        field_widths(&fields)?;
        Ok(slots_struct(context, fields.len()))
    } else {
        Ok(r#type)
    }
}

fn slots_struct(context: &Context, count: usize) -> Type<'_> {
    let i64_type: Type = IntegerType::new(context, 64).into();
    llvm::r#type::r#struct(context, &vec![i64_type; count], false)
}

fn mapped_signature<'c>(
    context: &'c Context,
    op: OperationRef<'c, '_>,
) -> Result<Option<Type<'c>>, String> {
    let attribute = op
        .attribute("function_type")
        .ok()
        .and_then(|attribute| TypeAttribute::try_from(attribute).ok())
        .ok_or_else(|| "func.func without function_type".to_string())?;
    let function = FunctionType::try_from(attribute.value())
        .map_err(|_| "function_type is not a FunctionType".to_string())?;

    let mut any = false;
    let mut inputs = Vec::with_capacity(function.input_count());
    for index in 0..function.input_count() {
        let input = function.input(index).map_err(|e| e.to_string())?;
        any |= is_frk_type(input);
        inputs.push(map_type(context, input)?);
    }
    let mut results = Vec::with_capacity(function.result_count());
    for index in 0..function.result_count() {
        let result = function.result(index).map_err(|e| e.to_string())?;
        any |= is_frk_type(result);
        results.push(map_type(context, result)?);
    }
    Ok(any.then(|| FunctionType::new(context, &inputs, &results).into()))
}

/// Rewrites collected function_type attributes; needs the mutable walk
/// because set_attribute demands OperationRefMut.
fn rewrite_signatures(module: OperationRef<'_, '_>, signatures: &HashMap<usize, Type<'_>>) {
    if signatures.is_empty() {
        return;
    }
    let Ok(region) = module.region(0) else {
        return;
    };
    let Some(block) = region.first_block() else {
        return;
    };
    let mut next = block.first_operation_mut();
    while let Some(mut op) = next {
        let following = op.next_in_block_mut();
        if let Some(mapped) = signatures.get(&(op.to_raw().ptr as usize)) {
            op.set_attribute("function_type", TypeAttribute::new(*mapped).into());
        }
        next = following;
    }
}

fn apply<'c>(
    context: &'c Context,
    rewriter: &RewriterBase<'c, '_>,
    plan: Planned<'c, '_>,
) -> Result<(), String> {
    match plan {
        Planned::TagOf { op } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let container = operand(op, 0)?;
            let read = rewriter.insert(llvm::extract_value(
                context,
                container,
                DenseI64ArrayAttribute::new(context, &[0]),
                IntegerType::new(context, 64).into(),
                location,
            ));
            finish(rewriter, op, result_value(read)?)
        }
        Planned::Read { op, slot, width } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let container = operand(op, 0)?;
            let read = rewriter.insert(llvm::extract_value(
                context,
                container,
                DenseI64ArrayAttribute::new(context, &[slot]),
                IntegerType::new(context, 64).into(),
                location,
            ));
            let mut value = result_value(read)?;
            if width < 64 {
                let narrowed = rewriter.insert(cast(
                    "arith.trunci",
                    value,
                    IntegerType::new(context, width).into(),
                    location,
                )?);
                value = result_value(narrowed)?;
            }
            finish(rewriter, op, value)
        }
        Planned::MakeSum {
            op,
            tag,
            container,
            payload_slots,
        } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let i64_type: Type = IntegerType::new(context, 64).into();
            let mut acc = result_value(rewriter.insert(llvm::undef(container, location)))?;

            let tag_value = result_value(rewriter.insert(melior::dialect::arith::constant(
                context,
                IntegerAttribute::new(i64_type, tag).into(),
                location,
            )))?;
            acc = result_value(rewriter.insert(llvm::insert_value(
                context,
                acc,
                DenseI64ArrayAttribute::new(context, &[0]),
                tag_value,
                location,
            )))?;

            // Payload slots are already widened i64s — copy verbatim.
            let payload = operand(op, 0)?;
            for index in 0..payload_slots {
                let slot = result_value(rewriter.insert(llvm::extract_value(
                    context,
                    payload,
                    DenseI64ArrayAttribute::new(context, &[index as i64]),
                    i64_type,
                    location,
                )))?;
                acc = result_value(rewriter.insert(llvm::insert_value(
                    context,
                    acc,
                    DenseI64ArrayAttribute::new(context, &[1 + index as i64]),
                    slot,
                    location,
                )))?;
            }
            finish(rewriter, op, acc)
        }
        Planned::ProductNew { op, container } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let acc = result_value(rewriter.insert(llvm::undef(container, location)))?;
            finish(rewriter, op, acc)
        }
        Planned::ProductSnoc {
            op,
            container,
            old_slots,
            width,
        } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let i64_type: Type = IntegerType::new(context, 64).into();
            let mut acc = result_value(rewriter.insert(llvm::undef(container, location)))?;

            let old = operand(op, 0)?;
            for index in 0..old_slots {
                let slot = result_value(rewriter.insert(llvm::extract_value(
                    context,
                    old,
                    DenseI64ArrayAttribute::new(context, &[index as i64]),
                    i64_type,
                    location,
                )))?;
                acc = result_value(rewriter.insert(llvm::insert_value(
                    context,
                    acc,
                    DenseI64ArrayAttribute::new(context, &[index as i64]),
                    slot,
                    location,
                )))?;
            }
            let appended = widened(context, rewriter, operand(op, 1)?, width, location)?;
            acc = result_value(rewriter.insert(llvm::insert_value(
                context,
                acc,
                DenseI64ArrayAttribute::new(context, &[old_slots as i64]),
                appended,
                location,
            )))?;
            finish(rewriter, op, acc)
        }
    }
}

fn operand<'c, 'a>(op: OperationRef<'c, 'a>, index: usize) -> Result<Value<'c, 'a>, String> {
    op.operand(index)
        .map_err(|_| format!("missing operand {index}"))
}

/// First result of an inserted op, with a caller-chosen use lifetime.
/// Sound: the value lives in the module, which outlives the whole pass;
/// melior's second lifetime is phantom tracking we deliberately reset
/// here to escape the rewriter borrow.
fn result_value<'c, 'r>(op: OperationRef<'c, '_>) -> Result<Value<'c, 'r>, String> {
    let raw = op
        .result(0)
        .map_err(|_| "inserted op has no result".to_string())?
        .to_raw();
    Ok(unsafe { Value::from_raw(raw) })
}

fn cast<'c>(
    name: &str,
    value: Value<'c, '_>,
    to: Type<'c>,
    location: Location<'c>,
) -> Result<melior::ir::Operation<'c>, String> {
    OperationBuilder::new(name, location)
        .add_operands(&[value])
        .add_results(&[to])
        .build()
        .map_err(|e| e.to_string())
}

/// Zero-extends a narrow field into its i64 slot (identity at 64).
fn widened<'c, 'r>(
    context: &'c Context,
    rewriter: &RewriterBase<'c, '_>,
    value: Value<'c, '_>,
    width: u32,
    location: Location<'c>,
) -> Result<Value<'c, 'r>, String> {
    if width < 64 {
        let widened = rewriter.insert(cast(
            "arith.extui",
            value,
            IntegerType::new(context, 64).into(),
            location,
        )?);
        result_value(widened)
    } else {
        Ok(unsafe { Value::from_raw(value.to_raw()) })
    }
}

fn finish<'c>(
    rewriter: &RewriterBase<'c, '_>,
    op: OperationRef<'c, '_>,
    replacement: Value<'c, '_>,
) -> Result<(), String> {
    let old = op
        .result(0)
        .map_err(|_| "frk op without a result".to_string())?;
    rewriter.replace_all_uses_with(old.into(), replacement);
    rewriter.erase_op(op);
    Ok(())
}
