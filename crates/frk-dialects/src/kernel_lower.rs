//! The kernel lowering: ONE external MLIR pass ("lower-frk-kernel")
//! taking every frk_adt and frk_closure op/type to LLVM-dialect form
//! (D-032 representation, D-035 strategy, D-037 merge + slot model).
//! One pass, not two: adt products carry closure-typed fields (church's
//! env is `product<[fn<...>]>`) and closure envs/args are adt products —
//! the value nesting is mutual, so the type mapping must be solved
//! together.
//!
//! Representations:
//! - sum      → `!llvm.struct<(i64 tag, i64 × K)>`, K = max variant slots
//! - product  → `!llvm.struct<(i64 × S)>`
//! - fn       → `!llvm.struct<(ptr thunk, ptr env)>`
//!
//! Slot model (D-037): an integer field ≤64 occupies one i64 slot
//! (extui in / trunci out); a closure field occupies TWO slots, its two
//! pointers ptrtoint'd in and inttoptr'd back out. Nested adt fields
//! stay fenced until frk.mem (M7).
//!
//! Closures (D-035): `make` heap-allocates the env via `frk_rt_alloc`
//! (declared once per module; the JIT runner registers the symbol, AOT
//! links frk-rt), stores the env product's slots, and builds
//! {thunk, env} — the thunk is a synthesized per-make-site func.func
//! that reloads captures and calls the lifted callee. The thunk's
//! address is taken as `func.constant` + one
//! `builtin.unrealized_conversion_cast` to `!llvm.ptr`: FuncToLLVM
//! turns the constant into llvm.mlir.addressof and
//! reconcile-unrealized-casts folds the cast away (llvm.mlir.addressof
//! cannot reference a func.func directly). `apply` extracts {thunk,
//! env}, unpacks the arg product, and calls indirectly.

use std::collections::HashMap;

use melior::dialect::llvm;
use melior::ir::attribute::{
    Attribute, DenseI64ArrayAttribute, FlatSymbolRefAttribute, IntegerAttribute, StringAttribute,
    TypeAttribute,
};
use melior::ir::operation::{OperationBuilder, OperationLike, OperationMutLike};
use melior::ir::r#type::{FunctionType, IntegerType, TypeId};
use melior::ir::{
    Block, BlockLike, Location, Operation, OperationRef, Region, RegionLike, Type, Value,
    ValueLike,
};
use melior::pass::{ExternalPass, Pass, create_external};
use melior::{Context, IrRewriter, RewriterBase};

use crate::adt::{decode_product, decode_sum};
use crate::closure::decode_fn;

#[repr(align(8))]
struct PassId;
static LOWER_KERNEL_PASS_ID: PassId = PassId;

/// Constructs the pass; the shared pipeline table calls this exactly
/// like the upstream `create_*` constructors.
pub fn lower_kernel_pass() -> Pass {
    create_external(
        |operation: OperationRef, pass: ExternalPass| {
            if let Err(message) = lower_kernel(operation) {
                eprintln!("lower-frk-kernel: {message}");
                pass.signal_failure();
            }
        },
        TypeId::create(&LOWER_KERNEL_PASS_ID),
        "lower-frk-kernel",
        "lower-frk-kernel",
        "lower frk_adt + frk_closure ops and types to LLVM form (D-032/D-035/D-037)",
        "",
        &[],
    )
}

/// What one field/param/capture is, slot-wise.
#[derive(Clone, Copy, Debug)]
enum SlotKind {
    /// An integer of the given width: one slot, extui/trunci adapted.
    Int(u32),
    /// A closure {ptr, ptr}: two slots, ptrtoint/inttoptr adapted.
    Closure,
}

impl SlotKind {
    fn slots(self) -> usize {
        match self {
            Self::Int(_) => 1,
            Self::Closure => 2,
        }
    }
}

fn slot_kind(r#type: Type<'_>) -> Result<SlotKind, String> {
    let printed = r#type.to_string();
    if printed.starts_with("!frk_closure.fn<") {
        return Ok(SlotKind::Closure);
    }
    if printed.starts_with("!frk_adt.") {
        return Err(format!(
            "nested adt fields are fenced until frk.mem (M7): {printed}"
        ));
    }
    let width = IntegerType::try_from(r#type)
        .map_err(|_| format!("unsupported field type {printed} (integers ≤64 and closures only)"))?
        .width();
    if width > 64 {
        return Err(format!("field width {width} exceeds 64"));
    }
    Ok(SlotKind::Int(width))
}

fn kinds_of(fields: &[Type<'_>]) -> Result<Vec<SlotKind>, String> {
    fields.iter().map(|field| slot_kind(*field)).collect()
}

fn total_slots(kinds: &[SlotKind]) -> usize {
    kinds.iter().map(|kind| kind.slots()).sum()
}

enum Planned<'c, 'a> {
    MakeSum {
        op: OperationRef<'c, 'a>,
        tag: i64,
        container: Type<'c>,
        payload_slots: usize,
    },
    TagOf {
        op: OperationRef<'c, 'a>,
    },
    /// extract/get: read `kind` starting at slot `offset`.
    Read {
        op: OperationRef<'c, 'a>,
        offset: usize,
        kind: SlotKind,
    },
    ProductNew {
        op: OperationRef<'c, 'a>,
        container: Type<'c>,
    },
    ProductSnoc {
        op: OperationRef<'c, 'a>,
        container: Type<'c>,
        old_slots: usize,
        kind: SlotKind,
    },
    MakeClosure {
        op: OperationRef<'c, 'a>,
        callee: String,
        env_kinds: Vec<SlotKind>,
        /// Lowered parameter/result types for the thunk signature.
        params: Vec<Type<'c>>,
        result: Type<'c>,
        thunk: String,
    },
    ApplyClosure {
        op: OperationRef<'c, 'a>,
        param_kinds: Vec<SlotKind>,
        result: Type<'c>,
    },
}

/// Lowers every kernel op and type under `module` (the pipeline anchors
/// this on builtin.module).
pub fn lower_kernel(module: OperationRef<'_, '_>) -> Result<(), String> {
    // Sound: the context strictly outlives every IR object walked here.
    let context = unsafe { module.context().to_ref() };

    let mut plans = Vec::new();
    let mut retypes = Vec::new();
    let mut signatures = HashMap::new();
    let mut thunk_counter = 0usize;
    collect(
        context,
        module,
        &mut plans,
        &mut retypes,
        &mut signatures,
        &mut thunk_counter,
    )?;

    for (value, mapped) in &retypes {
        value.set_type(*mapped);
    }
    rewrite_signatures(module, &signatures);

    // Thunks + the frk_rt_alloc declaration are built against retyped
    // callee signatures, so this happens after the sweeps.
    let needs_allocator = plans
        .iter()
        .any(|plan| matches!(plan, Planned::MakeClosure { .. }));
    if needs_allocator {
        declare_allocator(context, module)?;
        synthesize_thunks(context, module, &plans)?;
    }

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
    thunk_counter: &mut usize,
) -> Result<(), String> {
    let name = op
        .name()
        .as_string_ref()
        .as_str()
        .map_err(|_| "non-UTF-8 op name".to_string())?
        .to_string();

    if let Some(suffix) = name.strip_prefix("frk_adt.") {
        plans.push(plan_adt(context, suffix, op)?);
    } else if let Some(suffix) = name.strip_prefix("frk_closure.") {
        plans.push(plan_closure(context, suffix, op, thunk_counter)?);
    } else {
        if name == "func.func" {
            if let Some(mapped) = mapped_signature(context, op)? {
                signatures.insert(op.to_raw().ptr as usize, mapped);
            }
        }
        for index in 0..op.result_count() {
            let result = op.result(index).map_err(|e| e.to_string())?;
            if is_kernel_type(result.r#type()) {
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
                if is_kernel_type(argument.r#type()) {
                    retypes.push((argument.into(), map_type(context, argument.r#type())?));
                }
            }
            let mut inner = current.first_operation();
            while let Some(inner_op) = inner {
                collect(context, inner_op, plans, retypes, signatures, thunk_counter)?;
                inner = inner_op.next_in_block();
            }
            block = current.next_in_region();
        }
    }
    Ok(())
}

fn plan_adt<'c, 'a>(
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
            let old = kinds_of(&decode_product(context, operand_type()?)?)?;
            let appended = op
                .operand(1)
                .map_err(|_| "snoc without a value operand".to_string())?
                .r#type();
            Ok(Planned::ProductSnoc {
                op,
                container: map_type(context, result_type()?)?,
                old_slots: total_slots(&old),
                kind: slot_kind(appended)?,
            })
        }
        "make_sum" => {
            let variants = decode_sum(context, result_type()?)?;
            let tag = index("variant")? as i64;
            kinds_of(
                variants
                    .get(tag as usize)
                    .ok_or_else(|| format!("variant {tag} out of range"))?,
            )?;
            let payload = kinds_of(&decode_product(context, operand_type()?)?)?;
            Ok(Planned::MakeSum {
                op,
                tag,
                container: map_type(context, result_type()?)?,
                payload_slots: total_slots(&payload),
            })
        }
        "tag_of" => Ok(Planned::TagOf { op }),
        "extract" => {
            let variants = decode_sum(context, operand_type()?)?;
            let variant = index("variant")?;
            let field = index("field")?;
            let kinds = kinds_of(
                variants
                    .get(variant)
                    .ok_or_else(|| format!("variant {variant} out of range"))?,
            )?;
            if field >= kinds.len() {
                return Err(format!("field {field} out of range"));
            }
            Ok(Planned::Read {
                op,
                offset: 1 + total_slots(&kinds[..field]),
                kind: kinds[field],
            })
        }
        "get" => {
            let kinds = kinds_of(&decode_product(context, operand_type()?)?)?;
            let field = index("field")?;
            if field >= kinds.len() {
                return Err(format!("field {field} out of range"));
            }
            Ok(Planned::Read {
                op,
                offset: total_slots(&kinds[..field]),
                kind: kinds[field],
            })
        }
        other => Err(format!("no lowering for frk_adt.{other}")),
    }
}

fn plan_closure<'c, 'a>(
    context: &'c Context,
    suffix: &str,
    op: OperationRef<'c, 'a>,
    thunk_counter: &mut usize,
) -> Result<Planned<'c, 'a>, String> {
    match suffix {
        "make" => {
            let callee = crate::closure::callee_name(op)?;
            let env_kinds = kinds_of(&decode_product(
                context,
                op.operand(0)
                    .map_err(|_| "make without an env operand".to_string())?
                    .r#type(),
            )?)?;
            let (params, results) = decode_fn(
                context,
                op.result(0)
                    .map_err(|_| "make without a result".to_string())?
                    .r#type(),
            )?;
            let [result] = results.as_slice() else {
                return Err("closures return exactly one value (D-036)".to_string());
            };
            let params = params
                .iter()
                .map(|param| map_type(context, *param))
                .collect::<Result<Vec<_>, _>>()?;
            let result = map_type(context, *result)?;

            let thunk = format!("__frk_thunk_{}", *thunk_counter);
            *thunk_counter += 1;
            Ok(Planned::MakeClosure {
                op,
                callee,
                env_kinds,
                params,
                result,
                thunk,
            })
        }
        "apply" => {
            let (params, results) = decode_fn(
                context,
                op.operand(0)
                    .map_err(|_| "apply without a closure operand".to_string())?
                    .r#type(),
            )?;
            let [result] = results.as_slice() else {
                return Err("closures return exactly one value (D-036)".to_string());
            };
            Ok(Planned::ApplyClosure {
                op,
                param_kinds: kinds_of(&params)?,
                result: map_type(context, *result)?,
            })
        }
        other => Err(format!("no lowering for frk_closure.{other}")),
    }
}

fn is_kernel_type(r#type: Type<'_>) -> bool {
    let printed = r#type.to_string();
    printed.starts_with("!frk_adt.") || printed.starts_with("!frk_closure.")
}

fn closure_struct(context: &Context) -> Type<'_> {
    let ptr = llvm::r#type::pointer(context, 0);
    llvm::r#type::r#struct(context, &[ptr, ptr], false)
}

fn slots_struct(context: &Context, count: usize) -> Type<'_> {
    let i64_type: Type = IntegerType::new(context, 64).into();
    llvm::r#type::r#struct(context, &vec![i64_type; count], false)
}

fn map_type<'c>(context: &'c Context, r#type: Type<'c>) -> Result<Type<'c>, String> {
    let printed = r#type.to_string();
    if printed.starts_with("!frk_adt.sum<") {
        let variants = decode_sum(context, r#type)?;
        let mut max_slots = 0;
        for fields in &variants {
            max_slots = max_slots.max(total_slots(&kinds_of(fields)?));
        }
        Ok(slots_struct(context, 1 + max_slots))
    } else if printed.starts_with("!frk_adt.product<") {
        let kinds = kinds_of(&decode_product(context, r#type)?)?;
        Ok(slots_struct(context, total_slots(&kinds)))
    } else if printed.starts_with("!frk_closure.fn<") {
        Ok(closure_struct(context))
    } else {
        Ok(r#type)
    }
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
        any |= is_kernel_type(input);
        inputs.push(map_type(context, input)?);
    }
    let mut results = Vec::with_capacity(function.result_count());
    for index in 0..function.result_count() {
        let result = function.result(index).map_err(|e| e.to_string())?;
        any |= is_kernel_type(result);
        results.push(map_type(context, result)?);
    }
    Ok(any.then(|| FunctionType::new(context, &inputs, &results).into()))
}

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

/// `llvm.func @frk_rt_alloc(i64) -> !llvm.ptr` — external declaration,
/// resolved by the JIT's registered symbol or by linking frk-rt.
fn declare_allocator(context: &Context, module: OperationRef<'_, '_>) -> Result<(), String> {
    let location = module.location();
    let i64_type: Type = IntegerType::new(context, 64).into();
    let ptr = llvm::r#type::pointer(context, 0);
    let function_type = llvm::r#type::function(ptr, &[i64_type], false);

    let declaration = OperationBuilder::new("llvm.func", location)
        .add_attributes(&[
            (
                melior::ir::Identifier::new(context, "sym_name"),
                StringAttribute::new(context, "frk_rt_alloc").into(),
            ),
            (
                melior::ir::Identifier::new(context, "function_type"),
                TypeAttribute::new(function_type).into(),
            ),
        ])
        .add_regions([Region::new()])
        .build()
        .map_err(|e| e.to_string())?;

    module
        .region(0)
        .map_err(|e| e.to_string())?
        .first_block()
        .ok_or_else(|| "module without a body".to_string())?
        .append_operation(declaration);
    Ok(())
}

/// One thunk per make-site: `func.func @__frk_thunk_N(env: ptr,
/// params...) -> result` reloading captures and calling the callee.
fn synthesize_thunks(
    context: &Context,
    module: OperationRef<'_, '_>,
    plans: &[Planned<'_, '_>],
) -> Result<(), String> {
    let module_block = module
        .region(0)
        .map_err(|e| e.to_string())?
        .first_block()
        .ok_or_else(|| "module without a body".to_string())?;
    let location = module.location();
    let ptr = llvm::r#type::pointer(context, 0);
    let i64_type: Type = IntegerType::new(context, 64).into();

    for plan in plans {
        let Planned::MakeClosure {
            callee,
            env_kinds,
            params,
            result,
            thunk,
            ..
        } = plan
        else {
            continue;
        };

        let mut inputs = Vec::with_capacity(1 + params.len());
        inputs.push(ptr);
        inputs.extend(params.iter().copied());

        let block = Block::new(
            &inputs
                .iter()
                .map(|input| (*input, location))
                .collect::<Vec<_>>(),
        );
        let env_ptr: Value = block.argument(0).map_err(|e| e.to_string())?.into();

        // Reload captures from the env, slot by slot.
        let mut call_args: Vec<Value> = Vec::with_capacity(env_kinds.len() + params.len());
        let mut offset = 0usize;
        for kind in env_kinds {
            match kind {
                SlotKind::Int(width) => {
                    let slot = load_slot(context, &block, env_ptr, offset, location)?;
                    let value = if *width < 64 {
                        let narrowed = block.append_operation(cast_op(
                            "arith.trunci",
                            slot,
                            IntegerType::new(context, *width).into(),
                            location,
                        )?);
                        narrowed.result(0).map_err(|e| e.to_string())?.into()
                    } else {
                        slot
                    };
                    call_args.push(value);
                    offset += 1;
                }
                SlotKind::Closure => {
                    let lo = load_slot(context, &block, env_ptr, offset, location)?;
                    let hi = load_slot(context, &block, env_ptr, offset + 1, location)?;
                    let p0 = block.append_operation(cast_op("llvm.inttoptr", lo, ptr, location)?);
                    let p1 = block.append_operation(cast_op("llvm.inttoptr", hi, ptr, location)?);
                    let rebuilt = build_pair(
                        context,
                        &block,
                        closure_struct(context),
                        p0.result(0).map_err(|e| e.to_string())?.into(),
                        p1.result(0).map_err(|e| e.to_string())?.into(),
                        location,
                    )?;
                    call_args.push(rebuilt);
                    offset += 2;
                }
            }
        }
        for param_index in 0..params.len() {
            call_args.push(
                block
                    .argument(1 + param_index)
                    .map_err(|e| e.to_string())?
                    .into(),
            );
        }

        let call = block.append_operation(
            OperationBuilder::new("func.call", location)
                .add_attributes(&[(
                    melior::ir::Identifier::new(context, "callee"),
                    FlatSymbolRefAttribute::new(context, callee).into(),
                )])
                .add_operands(&call_args)
                .add_results(&[*result])
                .build()
                .map_err(|e| e.to_string())?,
        );
        block.append_operation(
            OperationBuilder::new("func.return", location)
                .add_operands(&[call.result(0).map_err(|e| e.to_string())?.into()])
                .build()
                .map_err(|e| e.to_string())?,
        );

        let region = Region::new();
        region.append_block(block);
        let function = melior::dialect::func::func(
            context,
            StringAttribute::new(context, thunk),
            TypeAttribute::new(FunctionType::new(context, &inputs, &[*result]).into()),
            region,
            &[],
            location,
        );
        module_block.append_operation(function);
        let _ = i64_type; // slot loads use it via load_slot
    }
    Ok(())
}

fn load_slot<'c>(
    context: &'c Context,
    block: &Block<'c>,
    base: Value<'c, '_>,
    slot: usize,
    location: Location<'c>,
) -> Result<Value<'c, 'c>, String> {
    let i64_type: Type = IntegerType::new(context, 64).into();
    let ptr = llvm::r#type::pointer(context, 0);
    let gep = block.append_operation(gep_op(context, base, slot, ptr, location)?);
    let load = block.append_operation(
        OperationBuilder::new("llvm.load", location)
            .add_attributes(&[(
                melior::ir::Identifier::new(context, "ordering"),
                Attribute::parse(context, "0 : i64").ok_or("ordering attr")?,
            )])
            .add_operands(&[gep.result(0).map_err(|e| e.to_string())?.into()])
            .add_results(&[i64_type])
            .build()
            .map_err(|e| e.to_string())?,
    );
    Ok(unsafe { Value::from_raw(load.result(0).map_err(|e| e.to_string())?.to_raw()) })
}

fn gep_op<'c>(
    context: &'c Context,
    base: Value<'c, '_>,
    slot: usize,
    ptr: Type<'c>,
    location: Location<'c>,
) -> Result<Operation<'c>, String> {
    OperationBuilder::new("llvm.getelementptr", location)
        .add_attributes(&[
            (
                melior::ir::Identifier::new(context, "elem_type"),
                TypeAttribute::new(IntegerType::new(context, 64).into()).into(),
            ),
            (
                melior::ir::Identifier::new(context, "noWrapFlags"),
                Attribute::parse(context, "0 : i32").ok_or("noWrapFlags attr")?,
            ),
            (
                melior::ir::Identifier::new(context, "rawConstantIndices"),
                Attribute::parse(context, &format!("array<i32: {slot}>"))
                    .ok_or("rawConstantIndices attr")?,
            ),
        ])
        .add_operands(&[base])
        .add_results(&[ptr])
        .build()
        .map_err(|e| e.to_string())
}

fn cast_op<'c>(
    name: &str,
    value: Value<'c, '_>,
    to: Type<'c>,
    location: Location<'c>,
) -> Result<Operation<'c>, String> {
    OperationBuilder::new(name, location)
        .add_operands(&[value])
        .add_results(&[to])
        .build()
        .map_err(|e| e.to_string())
}

fn build_pair<'c>(
    context: &'c Context,
    block: &Block<'c>,
    container: Type<'c>,
    first: Value<'c, '_>,
    second: Value<'c, '_>,
    location: Location<'c>,
) -> Result<Value<'c, 'c>, String> {
    let undef = block.append_operation(llvm::undef(container, location));
    let with_first = block.append_operation(llvm::insert_value(
        context,
        undef.result(0).map_err(|e| e.to_string())?.into(),
        DenseI64ArrayAttribute::new(context, &[0]),
        first,
        location,
    ));
    let with_both = block.append_operation(llvm::insert_value(
        context,
        with_first.result(0).map_err(|e| e.to_string())?.into(),
        DenseI64ArrayAttribute::new(context, &[1]),
        second,
        location,
    ));
    Ok(unsafe { Value::from_raw(with_both.result(0).map_err(|e| e.to_string())?.to_raw()) })
}

// ---- the rewriter-driven op replacements ----

fn apply<'c>(
    context: &'c Context,
    rewriter: &RewriterBase<'c, '_>,
    plan: Planned<'c, '_>,
) -> Result<(), String> {
    match plan {
        Planned::TagOf { op } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let read = rewriter.insert(llvm::extract_value(
                context,
                operand(op, 0)?,
                DenseI64ArrayAttribute::new(context, &[0]),
                IntegerType::new(context, 64).into(),
                location,
            ));
            finish(rewriter, op, result_value(read)?)
        }
        Planned::Read { op, offset, kind } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let container = operand(op, 0)?;
            let value = read_slots(context, rewriter, container, offset, kind, location)?;
            finish(rewriter, op, value)
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
            kind,
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
            let appended = operand(op, 1)?;
            write_slots(
                context, rewriter, &mut acc, old_slots, kind, appended, location,
            )?;
            finish(rewriter, op, acc)
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
        Planned::MakeClosure {
            op,
            env_kinds,
            thunk,
            params,
            result,
            ..
        } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let i64_type: Type = IntegerType::new(context, 64).into();
            let ptr = llvm::r#type::pointer(context, 0);

            // Heap-allocate the env and store the product's slots.
            let env_slots = total_slots(&env_kinds);
            let size = result_value(rewriter.insert(melior::dialect::arith::constant(
                context,
                IntegerAttribute::new(i64_type, (env_slots.max(1) * 8) as i64).into(),
                location,
            )))?;
            let env_ptr = result_value(rewriter.insert(direct_call(
                context,
                "frk_rt_alloc",
                &[size],
                ptr,
                location,
            )?))?;
            let env_value = operand(op, 0)?;
            for slot in 0..env_slots {
                let word = result_value(rewriter.insert(llvm::extract_value(
                    context,
                    env_value,
                    DenseI64ArrayAttribute::new(context, &[slot as i64]),
                    i64_type,
                    location,
                )))?;
                let address =
                    result_value(rewriter.insert(gep_op(context, env_ptr, slot, ptr, location)?))?;
                rewriter.insert(
                    OperationBuilder::new("llvm.store", location)
                        .add_attributes(&[(
                            melior::ir::Identifier::new(context, "ordering"),
                            Attribute::parse(context, "0 : i64").ok_or("ordering attr")?,
                        )])
                        .add_operands(&[word, address])
                        .build()
                        .map_err(|e| e.to_string())?,
                );
            }

            // Thunk address: func.constant + unrealized cast to ptr
            // (folded away after FuncToLLVM; see the module docs).
            let mut thunk_inputs = Vec::with_capacity(1 + params.len());
            thunk_inputs.push(ptr);
            thunk_inputs.extend(params.iter().copied());
            let thunk_type = FunctionType::new(context, &thunk_inputs, &[result]);
            let constant = result_value(rewriter.insert(
                OperationBuilder::new("func.constant", location)
                    .add_attributes(&[(
                        melior::ir::Identifier::new(context, "value"),
                        FlatSymbolRefAttribute::new(context, &thunk).into(),
                    )])
                    .add_results(&[thunk_type.into()])
                    .build()
                    .map_err(|e| e.to_string())?,
            ))?;
            let thunk_ptr = result_value(rewriter.insert(
                OperationBuilder::new("builtin.unrealized_conversion_cast", location)
                    .add_operands(&[constant])
                    .add_results(&[ptr])
                    .build()
                    .map_err(|e| e.to_string())?,
            ))?;

            // {thunk, env}
            let closure_type = closure_struct(context);
            let undef = result_value(rewriter.insert(llvm::undef(closure_type, location)))?;
            let with_fn = result_value(rewriter.insert(llvm::insert_value(
                context,
                undef,
                DenseI64ArrayAttribute::new(context, &[0]),
                thunk_ptr,
                location,
            )))?;
            let closure = result_value(rewriter.insert(llvm::insert_value(
                context,
                with_fn,
                DenseI64ArrayAttribute::new(context, &[1]),
                env_ptr,
                location,
            )))?;
            finish(rewriter, op, closure)
        }
        Planned::ApplyClosure {
            op,
            param_kinds,
            result,
        } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let ptr = llvm::r#type::pointer(context, 0);

            let closure = operand(op, 0)?;
            let fn_ptr = result_value(rewriter.insert(llvm::extract_value(
                context,
                closure,
                DenseI64ArrayAttribute::new(context, &[0]),
                ptr,
                location,
            )))?;
            let env_ptr = result_value(rewriter.insert(llvm::extract_value(
                context,
                closure,
                DenseI64ArrayAttribute::new(context, &[1]),
                ptr,
                location,
            )))?;

            let arg_pack = operand(op, 1)?;
            let mut call_operands: Vec<Value> = vec![fn_ptr, env_ptr];
            let mut offset = 0usize;
            for kind in &param_kinds {
                let value =
                    read_slots(context, rewriter, arg_pack, offset, *kind, location)?;
                call_operands.push(value);
                offset += kind.slots();
            }

            let n = call_operands.len() as i32;
            let call = rewriter.insert(
                OperationBuilder::new("llvm.call", location)
                    .add_attributes(&[
                        (
                            melior::ir::Identifier::new(context, "CConv"),
                            Attribute::parse(context, "#llvm.cconv<ccc>").ok_or("CConv")?,
                        ),
                        (
                            melior::ir::Identifier::new(context, "TailCallKind"),
                            Attribute::parse(context, "#llvm.tailcallkind<none>")
                                .ok_or("TailCallKind")?,
                        ),
                        (
                            melior::ir::Identifier::new(context, "fastmathFlags"),
                            Attribute::parse(context, "#llvm.fastmath<none>")
                                .ok_or("fastmathFlags")?,
                        ),
                        (
                            melior::ir::Identifier::new(context, "op_bundle_sizes"),
                            Attribute::parse(context, "array<i32>").ok_or("op_bundle_sizes")?,
                        ),
                        (
                            melior::ir::Identifier::new(context, "operandSegmentSizes"),
                            Attribute::parse(context, &format!("array<i32: {n}, 0>"))
                                .ok_or("operandSegmentSizes")?,
                        ),
                    ])
                    .add_operands(&call_operands)
                    .add_results(&[result])
                    .build()
                    .map_err(|e| e.to_string())?,
            );
            finish(rewriter, op, result_value(call)?)
        }
    }
}

/// Direct llvm.call by symbol (the allocator).
fn direct_call<'c>(
    context: &'c Context,
    callee: &str,
    arguments: &[Value<'c, '_>],
    result: Type<'c>,
    location: Location<'c>,
) -> Result<Operation<'c>, String> {
    let n = arguments.len() as i32;
    OperationBuilder::new("llvm.call", location)
        .add_attributes(&[
            (
                melior::ir::Identifier::new(context, "callee"),
                FlatSymbolRefAttribute::new(context, callee).into(),
            ),
            (
                melior::ir::Identifier::new(context, "CConv"),
                Attribute::parse(context, "#llvm.cconv<ccc>").ok_or("CConv")?,
            ),
            (
                melior::ir::Identifier::new(context, "TailCallKind"),
                Attribute::parse(context, "#llvm.tailcallkind<none>").ok_or("TailCallKind")?,
            ),
            (
                melior::ir::Identifier::new(context, "fastmathFlags"),
                Attribute::parse(context, "#llvm.fastmath<none>").ok_or("fastmathFlags")?,
            ),
            (
                melior::ir::Identifier::new(context, "op_bundle_sizes"),
                Attribute::parse(context, "array<i32>").ok_or("op_bundle_sizes")?,
            ),
            (
                melior::ir::Identifier::new(context, "operandSegmentSizes"),
                Attribute::parse(context, &format!("array<i32: {n}, 0>"))
                    .ok_or("operandSegmentSizes")?,
            ),
        ])
        .add_operands(arguments)
        .add_results(&[result])
        .build()
        .map_err(|e| e.to_string())
}

/// Reads one field (per its slot kind) out of a slots VALUE struct via
/// extractvalue, adapting representation.
fn read_slots<'c>(
    context: &'c Context,
    rewriter: &RewriterBase<'c, '_>,
    container: Value<'c, '_>,
    offset: usize,
    kind: SlotKind,
    location: Location<'c>,
) -> Result<Value<'c, 'c>, String> {
    let i64_type: Type = IntegerType::new(context, 64).into();
    let ptr = llvm::r#type::pointer(context, 0);
    match kind {
        SlotKind::Int(width) => {
            let slot = result_value(rewriter.insert(llvm::extract_value(
                context,
                container,
                DenseI64ArrayAttribute::new(context, &[offset as i64]),
                i64_type,
                location,
            )))?;
            if width < 64 {
                let narrowed = rewriter.insert(cast_op(
                    "arith.trunci",
                    slot,
                    IntegerType::new(context, width).into(),
                    location,
                )?);
                result_value(narrowed)
            } else {
                Ok(slot)
            }
        }
        SlotKind::Closure => {
            let lo = result_value(rewriter.insert(llvm::extract_value(
                context,
                container,
                DenseI64ArrayAttribute::new(context, &[offset as i64]),
                i64_type,
                location,
            )))?;
            let hi = result_value(rewriter.insert(llvm::extract_value(
                context,
                container,
                DenseI64ArrayAttribute::new(context, &[offset as i64 + 1]),
                i64_type,
                location,
            )))?;
            let p0 = result_value(rewriter.insert(cast_op("llvm.inttoptr", lo, ptr, location)?))?;
            let p1 = result_value(rewriter.insert(cast_op("llvm.inttoptr", hi, ptr, location)?))?;
            let undef =
                result_value(rewriter.insert(llvm::undef(closure_struct(context), location)))?;
            let with_first = result_value(rewriter.insert(llvm::insert_value(
                context,
                undef,
                DenseI64ArrayAttribute::new(context, &[0]),
                p0,
                location,
            )))?;
            result_value(rewriter.insert(llvm::insert_value(
                context,
                with_first,
                DenseI64ArrayAttribute::new(context, &[1]),
                p1,
                location,
            )))
        }
    }
}

/// Writes one value into a slots VALUE struct at `offset` (mutating the
/// accumulator), adapting representation per kind.
fn write_slots<'c>(
    context: &'c Context,
    rewriter: &RewriterBase<'c, '_>,
    accumulator: &mut Value<'c, 'c>,
    offset: usize,
    kind: SlotKind,
    value: Value<'c, '_>,
    location: Location<'c>,
) -> Result<(), String> {
    let i64_type: Type = IntegerType::new(context, 64).into();
    match kind {
        SlotKind::Int(width) => {
            let widened = if width < 64 {
                result_value(rewriter.insert(cast_op("arith.extui", value, i64_type, location)?))?
            } else {
                unsafe { Value::from_raw(value.to_raw()) }
            };
            *accumulator = result_value(rewriter.insert(llvm::insert_value(
                context,
                *accumulator,
                DenseI64ArrayAttribute::new(context, &[offset as i64]),
                widened,
                location,
            )))?;
        }
        SlotKind::Closure => {
            let ptr_type = llvm::r#type::pointer(context, 0);
            let p0 = result_value(rewriter.insert(llvm::extract_value(
                context,
                value,
                DenseI64ArrayAttribute::new(context, &[0]),
                ptr_type,
                location,
            )))?;
            let p1 = result_value(rewriter.insert(llvm::extract_value(
                context,
                value,
                DenseI64ArrayAttribute::new(context, &[1]),
                ptr_type,
                location,
            )))?;
            let lo = result_value(rewriter.insert(cast_op("llvm.ptrtoint", p0, i64_type, location)?))?;
            let hi = result_value(rewriter.insert(cast_op("llvm.ptrtoint", p1, i64_type, location)?))?;
            *accumulator = result_value(rewriter.insert(llvm::insert_value(
                context,
                *accumulator,
                DenseI64ArrayAttribute::new(context, &[offset as i64]),
                lo,
                location,
            )))?;
            *accumulator = result_value(rewriter.insert(llvm::insert_value(
                context,
                *accumulator,
                DenseI64ArrayAttribute::new(context, &[offset as i64 + 1]),
                hi,
                location,
            )))?;
        }
    }
    Ok(())
}

fn operand<'c, 'a>(op: OperationRef<'c, 'a>, index: usize) -> Result<Value<'c, 'a>, String> {
    op.operand(index)
        .map_err(|_| format!("missing operand {index}"))
}

/// First result of an inserted op, with a caller-chosen use lifetime.
/// Sound: the value lives in the module, which outlives the whole pass.
fn result_value<'c, 'r>(op: OperationRef<'c, '_>) -> Result<Value<'c, 'r>, String> {
    let raw = op
        .result(0)
        .map_err(|_| "inserted op has no result".to_string())?
        .to_raw();
    Ok(unsafe { Value::from_raw(raw) })
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
