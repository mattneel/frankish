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

/// The memory strategy (D-041): a lowering parameter, never IR. Arena
/// bump-allocates (process-lifetime v0); Rc adds refcount headers and
/// retain calls at owning stores (elided on ownership transfer);
/// releases arrive with the M10 GC-gate liveness work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Strategy {
    Arena,
    Rc,
}

/// What an owning store must retain under rc (computed at plan time
/// from slot kind + source type; D-057's frontier symmetry: retain
/// coverage EQUALS trace coverage or cascades free live objects).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RetainKind {
    None,
    /// A managed pointer (box/array).
    Ptr,
    /// A closure value: its env pointer (word 1) is managed.
    ClosureEnv,
    /// A dyn pair: retained iff the tag is table/fun — emitted
    /// branch-free (select to null; retain(null) is a no-op).
    DynPair,
}

fn retain_kind(kind: &SlotKind<'_>, ty: Type<'_>) -> RetainKind {
    match kind {
        SlotKind::Ptr { managed: true } => RetainKind::Ptr,
        SlotKind::Closure => RetainKind::ClosureEnv,
        SlotKind::Words { .. } if ty.to_string() == "!frk_dyn.dyn" => RetainKind::DynPair,
        _ => RetainKind::None,
    }
}

/// D-057 layout words, mirroring frk-rt's encoding (a dev-dependency
/// parity test holds the two in lockstep). Codes: 0 skip, 1 managed
/// pointer, 2 dyn-tag (pair with the next word).
#[allow(dead_code)] // documented anchor of the encoding (D-057)
pub(crate) const LAYOUT_LEAF: u64 = 0;
pub(crate) const LAYOUT_TABLE_SHELL: u64 = 1;
pub(crate) const LAYOUT_ARRAY_LEAF: u64 = 2;
pub(crate) const LAYOUT_ARRAY_PTR: u64 = 2 | (1 << 2);
pub(crate) const LAYOUT_ARRAY_DYN: u64 = 2 | (2 << 2);

pub(crate) fn layout_wordmap(codes: &[u8]) -> u64 {
    let mut layout = 0u64;
    for (index, &code) in codes.iter().enumerate().take(30) {
        layout |= (code as u64 & 0b11) << (4 + 2 * index);
    }
    layout
}

/// Per-word trace codes for a slot-kind sequence (the D-049 knowledge
/// made runtime-visible — D-055's rung). Words (embedded aggregates)
/// code as skip: the conservative leak-biased frontier, same as the
/// retain side.
fn kinds_layout(kinds: &[SlotKind<'_>], types: &[Type<'_>]) -> u64 {
    let mut codes = Vec::new();
    for (kind, ty) in kinds.iter().zip(types) {
        match kind {
            SlotKind::Int(_) | SlotKind::F64 => codes.push(0),
            SlotKind::Ptr { managed } => codes.push(if *managed { 1 } else { 0 }),
            SlotKind::Closure => {
                codes.push(0); // thunk fn-ptr
                codes.push(1); // env ptr (managed)
            }
            SlotKind::Words { slots, .. } => {
                if ty.to_string() == "!frk_dyn.dyn" {
                    codes.push(2); // dyn tag; pair traced by tag
                    codes.push(0);
                } else {
                    codes.extend(std::iter::repeat_n(0u8, *slots));
                }
            }
        }
    }
    layout_wordmap(&codes)
}

impl Strategy {
    fn alloc_symbol(self) -> &'static str {
        match self {
            Self::Arena => "frk_rt_arena_alloc",
            Self::Rc => "frk_rt_rc_alloc",
        }
    }
}

/// Constructs the pass for one strategy; the pipeline builds it fresh
/// per run exactly like the upstream `create_*` constructors.
pub fn lower_kernel_pass(strategy: Strategy) -> Pass {
    create_external(
        move |operation: OperationRef, pass: ExternalPass| {
            if let Err(message) = lower_kernel(operation, strategy) {
                eprintln!("lower-frk-kernel: {message}");
                pass.signal_failure();
            }
        },
        TypeId::create(&LOWER_KERNEL_PASS_ID),
        "lower-frk-kernel",
        "lower-frk-kernel",
        "lower frk_adt/closure/mem ops and types to LLVM form (D-032/D-035/D-037/D-041)",
        "",
        &[],
    )
}

/// What one field/param/capture is, slot-wise.
#[derive(Clone, Debug)]
enum SlotKind<'c> {
    /// An integer of the given width: one slot, extui/trunci adapted.
    Int(u32),
    /// A closure {ptr, ptr}: two slots, ptrtoint/inttoptr adapted.
    Closure,
    /// A nested adt value: `slots` verbatim i64 words (its own lowered
    /// struct is all-i64), rebuilt as `mapped` on read. Finite by
    /// construction — recursive ADTs cannot even be spelled (D-038).
    Words { slots: usize, mapped: Type<'c> },
    /// One !llvm.ptr slot, ptrtoint in / inttoptr out. `managed` iff
    /// it carries an rc header at ptr-8 (boxes, arrays) — strings do
    /// NOT, and a retain on them would corrupt memory (D-049).
    Ptr { managed: bool },
    /// An f64 (M9/TS-0): one slot, arith.bitcast in and out.
    F64,
}

impl SlotKind<'_> {
    fn slots(&self) -> usize {
        match self {
            Self::Int(_) => 1,
            Self::Closure => 2,
            Self::Words { slots, .. } => *slots,
            Self::Ptr { .. } => 1,
            Self::F64 => 1,
        }
    }
}

fn slot_kind<'c>(context: &'c Context, r#type: Type<'c>) -> Result<SlotKind<'c>, String> {
    let printed = r#type.to_string();
    if printed.starts_with("!frk_closure.fn<") {
        return Ok(SlotKind::Closure);
    }
    if printed.starts_with("!frk_mem.box<") || printed.starts_with("!frk_mem.arr<") {
        return Ok(SlotKind::Ptr { managed: true });
    }
    if printed == "!frk_str.str" || printed == "!frk_bstr.str" {
        return Ok(SlotKind::Ptr { managed: false });
    }
    if printed == "!frk_dyn.dyn" {
        // Fat value (D-051): two verbatim word slots.
        return Ok(SlotKind::Words { slots: 2, mapped: map_type(context, r#type)? });
    }
    if printed == "f64" {
        return Ok(SlotKind::F64);
    }
    if printed.starts_with("!frk_adt.") {
        let mapped = map_type(context, r#type)?;
        let slots = struct_field_count(&mapped)?;
        return Ok(SlotKind::Words { slots, mapped });
    }
    let width = IntegerType::try_from(r#type)
        .map_err(|_| format!("unsupported field type {printed} (integers ≤64, closures, adts)"))?
        .width();
    if width > 64 {
        return Err(format!("field width {width} exceeds 64"));
    }
    Ok(SlotKind::Int(width))
}

/// Counts the fields of an !llvm.struct<(i64 × N)> by its printed form —
/// the structs this pass makes are always uniform i64 tuples.
fn struct_field_count(mapped: &Type<'_>) -> Result<usize, String> {
    let printed = mapped.to_string();
    let inner = printed
        .strip_prefix("!llvm.struct<(")
        .and_then(|rest| rest.strip_suffix(")>"))
        .ok_or_else(|| format!("expected a struct type, got {printed}"))?;
    if inner.is_empty() {
        return Ok(0);
    }
    Ok(inner.split(',').count())
}

fn kinds_of<'c>(context: &'c Context, fields: &[Type<'c>]) -> Result<Vec<SlotKind<'c>>, String> {
    fields.iter().map(|field| slot_kind(context, *field)).collect()
}

fn total_slots(kinds: &[SlotKind<'_>]) -> usize {
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
        kind: SlotKind<'c>,
    },
    ProductNew {
        op: OperationRef<'c, 'a>,
        container: Type<'c>,
    },
    ProductSnoc {
        op: OperationRef<'c, 'a>,
        container: Type<'c>,
        old_slots: usize,
        kind: SlotKind<'c>,
        appended_retain: RetainKind,
    },
    MakeClosure {
        op: OperationRef<'c, 'a>,
        callee: String,
        env_kinds: Vec<SlotKind<'c>>,
        env_layout: u64,
        /// Lowered parameter/result types for the thunk signature.
        params: Vec<Type<'c>>,
        result: Type<'c>,
        thunk: String,
    },
    ApplyClosure {
        op: OperationRef<'c, 'a>,
        param_kinds: Vec<SlotKind<'c>>,
        result: Type<'c>,
    },
    BoxNew {
        op: OperationRef<'c, 'a>,
        payload_bytes: usize,
        payload_retain: RetainKind,
        /// GC ladder step 1 (D-053/D-054): when this allocation dies
        /// block-locally, the terminator to release before.
        die_at: Option<OperationRef<'c, 'a>>,
        /// D-057 layout word for the tracer.
        layout: u64,
    },
    BoxGet {
        op: OperationRef<'c, 'a>,
        result: Type<'c>,
    },
    BoxSet {
        op: OperationRef<'c, 'a>,
        payload_retain: RetainKind,
    },
    ArrayNew {
        op: OperationRef<'c, 'a>,
        die_at: Option<OperationRef<'c, 'a>>,
        layout: u64,
        elem_slots: usize,
    },
    ArrayGet {
        op: OperationRef<'c, 'a>,
        elem_kind: SlotKind<'c>,
        elem_mapped: Type<'c>,
    },
    ArraySet {
        op: OperationRef<'c, 'a>,
        elem_kind: SlotKind<'c>,
        elem_mapped: Type<'c>,
        elem_retain: RetainKind,
    },
    ArrayLen {
        op: OperationRef<'c, 'a>,
    },
    StrLit {
        op: OperationRef<'c, 'a>,
        text: String,
        symbol: String,
    },
    /// concat / eq / len — a straight rt call, result adapted.
    StrCall {
        op: OperationRef<'c, 'a>,
        callee: &'static str,
        /// eq returns i64 from the rt and truncates to i1 here.
        trunc_to_i1: bool,
    },
    DynWrap {
        op: OperationRef<'c, 'a>,
        tag: i64,
        payload_kind: SlotKind<'c>,
        payload_retain: RetainKind,
        /// Layout for the heap box when the payload is multi-word.
        boxed_layout: u64,
    },
    DynUnwrap {
        op: OperationRef<'c, 'a>,
        tag: i64,
        result_kind: SlotKind<'c>,
        result_mapped: Type<'c>,
    },
    DynTagOf {
        op: OperationRef<'c, 'a>,
    },
    BstrLit {
        op: OperationRef<'c, 'a>,
        text: String,
        symbol: String,
    },
    BstrConcat {
        op: OperationRef<'c, 'a>,
    },
    /// Interned identity: eq is an inline pointer comparison (D-056).
    BstrEq {
        op: OperationRef<'c, 'a>,
    },
    /// Inline header load (D-056).
    BstrLen {
        op: OperationRef<'c, 'a>,
    },
    TableNew {
        op: OperationRef<'c, 'a>,
    },
    TableRawGet {
        op: OperationRef<'c, 'a>,
    },
    TableRawSet {
        op: OperationRef<'c, 'a>,
    },
    TableLen {
        op: OperationRef<'c, 'a>,
    },
    /// Meta is shell word 3 — set/get lower INLINE (D-056).
    TableSetMeta {
        op: OperationRef<'c, 'a>,
    },
    TableGetMeta {
        op: OperationRef<'c, 'a>,
    },
    DynPayloadWord {
        op: OperationRef<'c, 'a>,
    },
    TableNext {
        op: OperationRef<'c, 'a>,
    },
    BstrSub {
        op: OperationRef<'c, 'a>,
    },
    BstrRep {
        op: OperationRef<'c, 'a>,
    },
    /// frk_ctl.prompt (κ_frk, D-060): enter → apply body(token) →
    /// exit → resolve-overwrites → yield. `param_kinds` is the body
    /// closure's parameter kinds (v0: a single Int token).
    CtlPrompt {
        op: OperationRef<'c, 'a>,
        result: Type<'c>,
    },
    /// frk_ctl.abort: park the dyn value + set pending. Control
    /// diversion (early return) is the frk-ctl-guard pass's job.
    CtlAbort {
        op: OperationRef<'c, 'a>,
    },
    /// frk_ctl.pending: read the result-passing carrier.
    CtlPending {
        op: OperationRef<'c, 'a>,
    },
}

/// Lowers every kernel op and type under `module` (the pipeline anchors
/// this on builtin.module).
pub fn lower_kernel(module: OperationRef<'_, '_>, strategy: Strategy) -> Result<(), String> {
    // Sound: the context strictly outlives every IR object walked here.
    let context = unsafe { module.context().to_ref() };

    let mut plans = Vec::new();
    let mut retypes = Vec::new();
    let mut signatures = HashMap::new();
    let mut thunk_counter = 0usize;
    let mut str_counter = 0usize;
    let mut use_counts: HashMap<usize, usize> = HashMap::new();
    let mut use_sites: HashMap<usize, (Vec<usize>, bool)> = HashMap::new();
    let mut op_blocks: HashMap<usize, usize> = HashMap::new();
    collect(
        context,
        module,
        &mut plans,
        &mut retypes,
        &mut signatures,
        &mut thunk_counter,
        &mut str_counter,
        &mut use_counts,
        &mut use_sites,
        &mut op_blocks,
    )?;

    // GC ladder step 1 (D-053/D-054, rc only): a box/array allocation
    // whose every use sits in its own block — none escaping through a
    // branch, call, or return — dies at that block's end; mark it to
    // release before the terminator. Cross-block lifetimes leak (the
    // documented conservative frontier; the cycle collector's ladder
    // continues from here).
    //
    // TRANSFER-vs-RELEASE exclusion (D-057, found by the corpus UAF):
    // a value whose ONLY use is an owning store TRANSFERRED its one
    // reference there (the retain was elided) — a block-exit release
    // would spend that reference twice and free an object its new
    // owner still holds. Such values get no die_at.
    if strategy == Strategy::Rc {
        // Owning consumption sites: (consumer op, operand index) whose
        // stores take ownership of a managed operand.
        let mut owned_operands: HashMap<usize, usize> = HashMap::new();
        for plan in &plans {
            let sites: &[(OperationRef, usize)] = match plan {
                Planned::ProductSnoc { op, appended_retain, .. }
                    if *appended_retain != RetainKind::None =>
                {
                    &[(*op, 1)]
                }
                Planned::BoxNew { op, payload_retain, .. }
                    if *payload_retain != RetainKind::None =>
                {
                    &[(*op, 0)]
                }
                Planned::BoxSet { op, payload_retain }
                    if *payload_retain != RetainKind::None =>
                {
                    &[(*op, 1)]
                }
                Planned::ArraySet { op, elem_retain, .. }
                    if *elem_retain != RetainKind::None =>
                {
                    &[(*op, 2)]
                }
                Planned::DynWrap { op, payload_retain, .. }
                    if *payload_retain != RetainKind::None =>
                {
                    &[(*op, 0)]
                }
                Planned::TableRawSet { op } => &[(*op, 1), (*op, 2)],
                Planned::TableSetMeta { op } => &[(*op, 1)],
                _ => continue,
            };
            for (site, index) in sites {
                if let Ok(value) = site.operand(*index) {
                    *owned_operands.entry(value.to_raw().ptr as usize).or_insert(0) += 1;
                }
            }
        }

        for plan in &mut plans {
            let (op, die_at) = match plan {
                Planned::BoxNew { op, die_at, .. } => (*op, die_at),
                Planned::ArrayNew { op, die_at, .. } => (*op, die_at),
                _ => continue,
            };
            let Ok(result) = op.result(0) else { continue };
            let key = result.to_raw().ptr as usize;
            let def_block = op.block().map(|b| b.to_raw().ptr as usize).unwrap_or(0);
            let local = match use_sites.get(&key) {
                // Unused allocations die immediately too.
                None => true,
                Some((users, escapes)) => {
                    let transferred = users.len() == 1
                        && owned_operands.get(&key).copied().unwrap_or(0) >= 1;
                    !*escapes
                        && !transferred
                        && users
                            .iter()
                            .all(|user| op_blocks.get(user).copied() == Some(def_block))
                }
            };
            if local {
                *die_at = block_terminator(op);
            }
        }
    }

    for (value, mapped) in &retypes {
        value.set_type(*mapped);
    }
    rewrite_signatures(module, &signatures);

    // Thunks + the frk_rt_alloc declaration are built against retyped
    // callee signatures, so this happens after the sweeps.
    // Sharing must be resolved BEFORE any rewriting: use counts key on
    // pre-lowering SSA values, and op replacement rewrites operands in
    // place (a mid-rewrite lookup would miss and misread transfer).
    let mut retain_shared: HashMap<(usize, usize), bool> = HashMap::new();
    for plan in &plans {
        let sites: &[(OperationRef, usize)] = match plan {
            Planned::ProductSnoc { op, .. } => &[(*op, 1usize)],
            Planned::BoxNew { op, .. } => &[(*op, 0)],
            Planned::BoxSet { op, .. } => &[(*op, 1)],
            Planned::ArraySet { op, .. } => &[(*op, 2)],
            Planned::DynWrap { op, .. } => &[(*op, 0)],
            // Table stores own both the key and the value (D-057).
            Planned::TableRawSet { op } => &[(*op, 1), (*op, 2)],
            Planned::TableSetMeta { op } => &[(*op, 1)],
            _ => continue,
        };
        for (op, index) in sites {
            if let Ok(value) = op.operand(*index) {
                let count = use_counts
                    .get(&(value.to_raw().ptr as usize))
                    .copied()
                    .unwrap_or(0);
                retain_shared.insert((op.to_raw().ptr as usize, *index), count > 1);
            }
        }
    }

    let needs_allocator = plans.iter().any(|plan| {
        matches!(
            plan,
            Planned::MakeClosure { .. } | Planned::BoxNew { .. } | Planned::ArrayNew { .. }
        ) || matches!(
            plan,
            Planned::DynWrap { payload_kind: SlotKind::Closure | SlotKind::Words { .. }, .. }
        ) || matches!(plan, Planned::TableNew { .. })
    });
    if needs_allocator {
        declare_runtime(context, module, strategy)?;
        synthesize_thunks(context, module, &plans)?;
    }
    declare_str_runtime(context, module, &plans)?;

    let rewriter = IrRewriter::new(context);
    let rewriter = rewriter.as_rewriter_base();
    let mut releases: Vec<(OperationRef, Value)> = Vec::new();
    for plan in plans {
        apply(context, &rewriter, plan, strategy, &retain_shared, &mut releases)?;
    }
    // GC step 1: the planned block-end releases. Terminators survive
    // the rewrite, so the anchors are still valid here.
    for (terminator, pointer) in releases {
        rewriter.set_insertion_point_before(terminator);
        rewriter.insert(direct_call_void(
            context,
            "frk_rt_rc_release",
            &[pointer],
            terminator.location(),
        )?);
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
    str_counter: &mut usize,
    use_counts: &mut HashMap<usize, usize>,
    use_sites: &mut HashMap<usize, (Vec<usize>, bool)>,
    op_blocks: &mut HashMap<usize, usize>,
) -> Result<(), String> {
    op_blocks.insert(
        op.to_raw().ptr as usize,
        op.block().map(|b| b.to_raw().ptr as usize).unwrap_or(0),
    );
    let name = op
        .name()
        .as_string_ref()
        .as_str()
        .map_err(|_| "non-UTF-8 op name".to_string())?
        .to_string();

    // SSA use counts feed the rc transfer-elision (D-041); use SITES
    // feed the block-local lifetime analysis (D-053 step 1). A use on
    // an op with successors (branches) or func.return escapes the
    // block/function — conservative: such values are never released.
    let escapes = op.successor_count() > 0
        || op
            .name()
            .as_string_ref()
            .as_str()
            .is_ok_and(|n| n == "func.return" || n == "func.call");
    for index in 0..op.operand_count() {
        if let Ok(operand) = op.operand(index) {
            let key = operand.to_raw().ptr as usize;
            *use_counts.entry(key).or_insert(0) += 1;
            let entry = use_sites.entry(key).or_insert((Vec::new(), false));
            entry.0.push(op.to_raw().ptr as usize);
            entry.1 |= escapes;
        }
    }

    if let Some(suffix) = name.strip_prefix("frk_adt.") {
        plans.push(plan_adt(context, suffix, op)?);
    } else if let Some(suffix) = name.strip_prefix("frk_closure.") {
        plans.push(plan_closure(context, suffix, op, thunk_counter)?);
    } else if let Some(suffix) = name.strip_prefix("frk_mem.") {
        plans.push(plan_mem(context, suffix, op)?);
    } else if let Some(suffix) = name.strip_prefix("frk_str.") {
        plans.push(plan_str(suffix, op, str_counter)?);
    } else if let Some(suffix) = name.strip_prefix("frk_dyn.") {
        plans.push(plan_dyn(context, suffix, op)?);
    } else if let Some(suffix) = name.strip_prefix("frk_bstr.") {
        plans.push(plan_bstr(suffix, op, str_counter)?);
    } else if let Some(suffix) = name.strip_prefix("frk_ctl.") {
        plans.push(plan_ctl(context, suffix, op)?);
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
                collect(
                    context, inner_op, plans, retypes, signatures, thunk_counter, str_counter,
                    use_counts, use_sites, op_blocks,
                )?;
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
            let old = kinds_of(context, &decode_product(context, operand_type()?)?)?;
            let appended = op
                .operand(1)
                .map_err(|_| "snoc without a value operand".to_string())?
                .r#type();
            Ok(Planned::ProductSnoc {
                op,
                container: map_type(context, result_type()?)?,
                old_slots: total_slots(&old),
                kind: slot_kind(context, appended)?,
                appended_retain: retain_kind(&slot_kind(context, appended)?, appended),
            })
        }
        "make_sum" => {
            let variants = decode_sum(context, result_type()?)?;
            let tag = index("variant")? as i64;
            kinds_of(
                context,
                variants
                    .get(tag as usize)
                    .ok_or_else(|| format!("variant {tag} out of range"))?,
            )?;
            let payload = kinds_of(context, &decode_product(context, operand_type()?)?)?;
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
                context,
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
                kind: kinds[field].clone(),
            })
        }
        "get" => {
            let kinds = kinds_of(context, &decode_product(context, operand_type()?)?)?;
            let field = index("field")?;
            if field >= kinds.len() {
                return Err(format!("field {field} out of range"));
            }
            Ok(Planned::Read {
                op,
                offset: total_slots(&kinds[..field]),
                kind: kinds[field].clone(),
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
            let env_fields = decode_product(
                context,
                op.operand(0)
                    .map_err(|_| "make without an env operand".to_string())?
                    .r#type(),
            )?;
            let env_kinds = kinds_of(context, &env_fields)?;
            let env_layout = kinds_layout(&env_kinds, &env_fields);
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
                env_layout,
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
                param_kinds: kinds_of(context, &params)?,
                result: map_type(context, *result)?,
            })
        }
        other => Err(format!("no lowering for frk_closure.{other}")),
    }
}

fn plan_mem<'c, 'a>(
    context: &'c Context,
    suffix: &str,
    op: OperationRef<'c, 'a>,
) -> Result<Planned<'c, 'a>, String> {
    match suffix {
        "box_new" => {
            let elem = crate::mem::decode_box(
                context,
                op.result(0)
                    .map_err(|_| "box_new without a result".to_string())?
                    .r#type(),
            )?;
            let kind = slot_kind(context, elem)?;
            let layout = kinds_layout(std::slice::from_ref(&kind), &[elem]);
            let payload_retain = retain_kind(&kind, elem);
            Ok(Planned::BoxNew {
                op,
                payload_bytes: (kind.slots().max(1)) * 8,
                payload_retain,
                die_at: None,
                layout,
            })
        }
        "box_get" => {
            let elem = crate::mem::decode_box(
                context,
                op.operand(0)
                    .map_err(|_| "box_get without an operand".to_string())?
                    .r#type(),
            )?;
            Ok(Planned::BoxGet { op, result: map_type(context, elem)? })
        }
        "box_set" => {
            let elem = crate::mem::decode_box(
                context,
                op.operand(0)
                    .map_err(|_| "box_set without an operand".to_string())?
                    .r#type(),
            )?;
            Ok(Planned::BoxSet {
                op,
                payload_retain: retain_kind(&slot_kind(context, elem)?, elem),
            })
        }
        "array_new" => {
            let elem = crate::mem::decode_arr(
                context,
                op.result(0)
                    .map_err(|_| "array_new without a result".to_string())?
                    .r#type(),
            )?;
            let kind = slot_kind(context, elem)?;
            let layout = match &kind {
                SlotKind::Ptr { managed: true } => LAYOUT_ARRAY_PTR,
                SlotKind::Words { .. } if elem.to_string() == "!frk_dyn.dyn" => {
                    LAYOUT_ARRAY_DYN
                }
                _ => LAYOUT_ARRAY_LEAF,
            };
            Ok(Planned::ArrayNew { op, die_at: None, layout, elem_slots: kind.slots() })
        }
        "array_get" => {
            let elem = crate::mem::decode_arr(
                context,
                op.operand(0)
                    .map_err(|_| "array_get without an operand".to_string())?
                    .r#type(),
            )?;
            let kind = slot_kind(context, elem)?;
            let is_dyn = elem.to_string() == "!frk_dyn.dyn";
            if kind.slots() != 1 && !is_dyn {
                return Err(format!(
                    "array elements must be one slot or dyn (D-049/D-058), got {elem}"
                ));
            }
            Ok(Planned::ArrayGet { op, elem_mapped: map_type(context, elem)?, elem_kind: kind })
        }
        "array_set" => {
            let elem = crate::mem::decode_arr(
                context,
                op.operand(0)
                    .map_err(|_| "array_set without an operand".to_string())?
                    .r#type(),
            )?;
            let kind = slot_kind(context, elem)?;
            let is_dyn = elem.to_string() == "!frk_dyn.dyn";
            if kind.slots() != 1 && !is_dyn {
                return Err(format!(
                    "array elements must be one slot or dyn (D-049/D-058), got {elem}"
                ));
            }
            let elem_retain = retain_kind(&kind, elem);
            Ok(Planned::ArraySet {
                op,
                elem_mapped: map_type(context, elem)?,
                elem_kind: kind,
                elem_retain,
            })
        }
        "array_len" => Ok(Planned::ArrayLen { op }),
        other => Err(format!("no lowering for frk_mem.{other}")),
    }
}

fn plan_ctl<'c, 'a>(
    context: &'c Context,
    suffix: &str,
    op: OperationRef<'c, 'a>,
) -> Result<Planned<'c, 'a>, String> {
    match suffix {
        "prompt" => {
            let result = map_type(
                context,
                op.result(0)
                    .map_err(|_| "prompt without a result".to_string())?
                    .r#type(),
            )?;
            Ok(Planned::CtlPrompt { op, result })
        }
        "abort" => Ok(Planned::CtlAbort { op }),
        "pending" => Ok(Planned::CtlPending { op }),
        other => Err(format!("no lowering for frk_ctl.{other}")),
    }
}

fn plan_dyn<'c, 'a>(
    context: &'c Context,
    suffix: &str,
    op: OperationRef<'c, 'a>,
) -> Result<Planned<'c, 'a>, String> {
    match suffix {
        "wrap" => {
            let tag = crate::dyn_dialect::tag_attr(op)?;
            let payload = op
                .operand(0)
                .map_err(|_| "wrap without an operand".to_string())?
                .r#type();
            let kind = slot_kind(context, payload)?;
            let boxed_layout = kinds_layout(std::slice::from_ref(&kind), &[payload]);
            let payload_retain = retain_kind(&kind, payload);
            Ok(Planned::DynWrap { op, tag, payload_kind: kind, payload_retain, boxed_layout })
        }
        "unwrap" => {
            let tag = crate::dyn_dialect::tag_attr(op)?;
            let result = op
                .result(0)
                .map_err(|_| "unwrap without a result".to_string())?
                .r#type();
            Ok(Planned::DynUnwrap {
                op,
                tag,
                result_kind: slot_kind(context, result)?,
                result_mapped: map_type(context, result)?,
            })
        }
        "tag_of" => Ok(Planned::DynTagOf { op }),
        "table_new" => Ok(Planned::TableNew { op }),
        "raw_get" => Ok(Planned::TableRawGet { op }),
        "raw_set" => Ok(Planned::TableRawSet { op }),
        "table_len" => Ok(Planned::TableLen { op }),
        "set_meta" => Ok(Planned::TableSetMeta { op }),
        "get_meta" => Ok(Planned::TableGetMeta { op }),
        "payload_word" => Ok(Planned::DynPayloadWord { op }),
        "table_next" => Ok(Planned::TableNext { op }),
        other => Err(format!("no lowering for frk_dyn.{other}")),
    }
}

fn plan_bstr<'c, 'a>(
    suffix: &str,
    op: OperationRef<'c, 'a>,
    str_counter: &mut usize,
) -> Result<Planned<'c, 'a>, String> {
    match suffix {
        "lit" => {
            let attribute = op
                .attribute("text")
                .map_err(|_| "frk_bstr.lit without text".to_string())?;
            let bytes = crate::attr_util::string_attr_bytes(attribute)?;
            let text = String::from_utf8(bytes)
                .map_err(|_| "non-UTF-8 bstr literal (ASCII fence, D-056)".to_string())?;
            let symbol = format!("__frk_bstr_{}", *str_counter);
            *str_counter += 1;
            Ok(Planned::BstrLit { op, text, symbol })
        }
        "concat" => Ok(Planned::BstrConcat { op }),
        "eq" => Ok(Planned::BstrEq { op }),
        "len" => Ok(Planned::BstrLen { op }),
        "sub" => Ok(Planned::BstrSub { op }),
        "rep" => Ok(Planned::BstrRep { op }),
        other => Err(format!("no lowering for frk_bstr.{other}")),
    }
}

fn plan_str<'c, 'a>(
    suffix: &str,
    op: OperationRef<'c, 'a>,
    str_counter: &mut usize,
) -> Result<Planned<'c, 'a>, String> {
    match suffix {
        "lit" => {
            let attribute = op
                .attribute("text")
                .map_err(|_| "frk_str.lit without text".to_string())?;
            let text = String::from_utf8(crate::attr_util::string_attr_bytes(attribute)?)
                .map_err(|_| "non-UTF-8 str literal".to_string())?;
            let symbol = format!("__frk_str_{}", *str_counter);
            *str_counter += 1;
            Ok(Planned::StrLit { op, text, symbol })
        }
        "concat" => Ok(Planned::StrCall { op, callee: "frk_rt_str_concat", trunc_to_i1: false }),
        "eq" => Ok(Planned::StrCall { op, callee: "frk_rt_str_eq", trunc_to_i1: true }),
        "len" => Ok(Planned::StrCall { op, callee: "frk_rt_str_len", trunc_to_i1: false }),
        other => Err(format!("no lowering for frk_str.{other}")),
    }
}

fn is_kernel_type(r#type: Type<'_>) -> bool {
    let printed = r#type.to_string();
    printed.starts_with("!frk_adt.")
        || printed.starts_with("!frk_closure.")
        || printed.starts_with("!frk_mem.")
        || printed.starts_with("!frk_str.")
        || printed.starts_with("!frk_dyn.")
        || printed.starts_with("!frk_bstr.")
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
            max_slots = max_slots.max(total_slots(&kinds_of(context, fields)?));
        }
        Ok(slots_struct(context, 1 + max_slots))
    } else if printed.starts_with("!frk_adt.product<") {
        let kinds = kinds_of(context, &decode_product(context, r#type)?)?;
        Ok(slots_struct(context, total_slots(&kinds)))
    } else if printed.starts_with("!frk_closure.fn<") {
        Ok(closure_struct(context))
    } else if printed.starts_with("!frk_mem.box<")
        || printed.starts_with("!frk_mem.arr<")
        || printed == "!frk_str.str"
        || printed == "!frk_bstr.str"
    {
        Ok(llvm::r#type::pointer(context, 0))
    } else if printed == "!frk_dyn.dyn" {
        Ok(slots_struct(context, 2))
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

/// Declares the string runtime's symbols when str plans exist
/// (strategy-independent: strings are rt-owned, D-049).
fn declare_str_runtime(
    context: &Context,
    module: OperationRef<'_, '_>,
    plans: &[Planned<'_, '_>],
) -> Result<(), String> {
    let mut needed: Vec<(&str, bool)> = Vec::new(); // (symbol, returns ptr)
    for plan in plans {
        match plan {
            Planned::StrLit { .. } => needed.push(("frk_rt_str_from_units", true)),
            Planned::StrCall { callee, .. } => {
                needed.push((callee, *callee == "frk_rt_str_concat"))
            }
            _ => {}
        }
    }
    for plan in plans {
        match plan {
            Planned::DynUnwrap { .. } => needed.push(("frk_rt_dyn_check", false)),
            Planned::BstrLit { .. } => needed.push(("frk_rt_bstr_intern", true)),
            Planned::BstrConcat { .. } => needed.push(("frk_rt_bstr_concat", true)),
            Planned::TableNew { .. } => needed.push(("frk_rt_table_init", false)),
            Planned::TableRawGet { .. } => needed.push(("frk_rt_table_raw_get", false)),
            Planned::TableRawSet { .. } => needed.push(("frk_rt_table_raw_set", false)),
            Planned::TableLen { .. } => needed.push(("frk_rt_table_len", false)),
            Planned::TableNext { .. } => needed.push(("frk_rt_table_next", false)),
            Planned::BstrSub { .. } => needed.push(("frk_rt_bstr_sub", true)),
            Planned::BstrRep { .. } => needed.push(("frk_rt_bstr_rep", true)),
            Planned::CtlPrompt { .. } => {
                needed.push(("frk_rt_ctl_prompt_enter", false));
                needed.push(("frk_rt_ctl_prompt_exit", false));
                needed.push(("frk_rt_ctl_resolve", false));
            }
            Planned::CtlAbort { .. } => needed.push(("frk_rt_ctl_abort", false)),
            Planned::CtlPending { .. } => needed.push(("frk_rt_ctl_pending", false)),
            _ => {}
        }
    }
    if needed.is_empty() {
        return Ok(());
    }
    needed.sort();
    needed.dedup();

    let location = module.location();
    let i64_type: Type = IntegerType::new(context, 64).into();
    let ptr = llvm::r#type::pointer(context, 0);
    let body = module
        .region(0)
        .map_err(|e| e.to_string())?
        .first_block()
        .ok_or_else(|| "module without a body".to_string())?;
    for (symbol, _) in needed {
        let function_type = match symbol {
            "frk_rt_str_from_units" => llvm::r#type::function(ptr, &[ptr, i64_type], false),
            "frk_rt_str_concat" => llvm::r#type::function(ptr, &[ptr, ptr], false),
            "frk_rt_str_eq" => llvm::r#type::function(i64_type, &[ptr, ptr], false),
            "frk_rt_str_len" => llvm::r#type::function(i64_type, &[ptr], false),
            "frk_rt_dyn_check" => {
                llvm::r#type::function(llvm::r#type::void(context), &[i64_type, i64_type], false)
            }
            "frk_rt_bstr_intern" => llvm::r#type::function(ptr, &[ptr, i64_type], false),
            "frk_rt_bstr_concat" => llvm::r#type::function(ptr, &[ptr, ptr], false),
            "frk_rt_table_init" => {
                llvm::r#type::function(llvm::r#type::void(context), &[i64_type], false)
            }
            "frk_rt_table_raw_get" => llvm::r#type::function(
                llvm::r#type::void(context),
                &[i64_type, i64_type, i64_type, ptr],
                false,
            ),
            "frk_rt_table_raw_set" => llvm::r#type::function(
                llvm::r#type::void(context),
                &[i64_type; 5],
                false,
            ),
            "frk_rt_table_len" => llvm::r#type::function(i64_type, &[i64_type], false),
            "frk_rt_table_next" => llvm::r#type::function(
                llvm::r#type::void(context),
                &[i64_type, i64_type, i64_type, ptr],
                false,
            ),
            "frk_rt_bstr_sub" => {
                llvm::r#type::function(ptr, &[ptr, i64_type, i64_type], false)
            }
            "frk_rt_bstr_rep" => llvm::r#type::function(ptr, &[ptr, i64_type], false),
            "frk_rt_ctl_prompt_enter" => llvm::r#type::function(i64_type, &[], false),
            "frk_rt_ctl_prompt_exit" => {
                llvm::r#type::function(llvm::r#type::void(context), &[i64_type], false)
            }
            "frk_rt_ctl_abort" => llvm::r#type::function(
                llvm::r#type::void(context),
                &[i64_type, i64_type, i64_type],
                false,
            ),
            "frk_rt_ctl_pending" => llvm::r#type::function(i64_type, &[], false),
            "frk_rt_ctl_resolve" => {
                llvm::r#type::function(i64_type, &[i64_type, ptr], false)
            }
            other => return Err(format!("unknown str runtime symbol {other}")),
        };
        let declaration = OperationBuilder::new("llvm.func", location)
            .add_attributes(&[
                (
                    melior::ir::Identifier::new(context, "sym_name"),
                    StringAttribute::new(context, symbol).into(),
                ),
                (
                    melior::ir::Identifier::new(context, "function_type"),
                    TypeAttribute::new(function_type).into(),
                ),
            ])
            .add_regions([Region::new()])
            .build()
            .map_err(|e| e.to_string())?;
        body.append_operation(declaration);
    }
    Ok(())
}

/// Declares the strategy's runtime symbols (resolved by the JIT's
/// registered symbols or by linking frk-rt): the allocator always, and
/// under Rc the retain hook too.
fn declare_runtime(
    context: &Context,
    module: OperationRef<'_, '_>,
    strategy: Strategy,
) -> Result<(), String> {
    let location = module.location();
    let i64_type: Type = IntegerType::new(context, 64).into();
    let ptr = llvm::r#type::pointer(context, 0);
    let body = module
        .region(0)
        .map_err(|e| e.to_string())?
        .first_block()
        .ok_or_else(|| "module without a body".to_string())?;

    let declare = |name: &str, function_type: Type| -> Result<(), String> {
        let declaration = OperationBuilder::new("llvm.func", location)
            .add_attributes(&[
                (
                    melior::ir::Identifier::new(context, "sym_name"),
                    StringAttribute::new(context, name).into(),
                ),
                (
                    melior::ir::Identifier::new(context, "function_type"),
                    TypeAttribute::new(function_type).into(),
                ),
            ])
            .add_regions([Region::new()])
            .build()
            .map_err(|e| e.to_string())?;
        body.append_operation(declaration);
        Ok(())
    };

    match strategy {
        Strategy::Arena => declare(
            strategy.alloc_symbol(),
            llvm::r#type::function(ptr, &[i64_type], false),
        )?,
        // D-057: rc allocations carry their layout descriptor.
        Strategy::Rc => declare(
            strategy.alloc_symbol(),
            llvm::r#type::function(ptr, &[i64_type, i64_type], false),
        )?,
    }
    if strategy == Strategy::Rc {
        declare(
            "frk_rt_rc_retain",
            llvm::r#type::function(llvm::r#type::void(context), &[ptr], false),
        )?;
        declare(
            "frk_rt_rc_release",
            llvm::r#type::function(llvm::r#type::void(context), &[ptr], false),
        )?;
    }
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
                SlotKind::Words { slots, mapped } => {
                    let mut rebuilt = {
                        let undef = block.append_operation(llvm::undef(*mapped, location));
                        let raw = undef.result(0).map_err(|e| e.to_string())?.to_raw();
                        unsafe { Value::from_raw(raw) }
                    };
                    for index in 0..*slots {
                        let word = load_slot(context, &block, env_ptr, offset + index, location)?;
                        let inserted = block.append_operation(llvm::insert_value(
                            context,
                            rebuilt,
                            DenseI64ArrayAttribute::new(context, &[index as i64]),
                            word,
                            location,
                        ));
                        let raw = inserted.result(0).map_err(|e| e.to_string())?.to_raw();
                        rebuilt = unsafe { Value::from_raw(raw) };
                    }
                    call_args.push(rebuilt);
                    offset += slots;
                }
                SlotKind::Ptr { .. } => {
                    let slot = load_slot(context, &block, env_ptr, offset, location)?;
                    let as_ptr = block.append_operation(cast_op(
                        "llvm.inttoptr",
                        slot,
                        llvm::r#type::pointer(context, 0),
                        location,
                    )?);
                    let raw = as_ptr.result(0).map_err(|e| e.to_string())?.to_raw();
                    call_args.push(unsafe { Value::from_raw(raw) });
                    offset += 1;
                }
                SlotKind::F64 => {
                    let slot = load_slot(context, &block, env_ptr, offset, location)?;
                    let f64_type = Type::parse(context, "f64").ok_or("f64 type")?;
                    let as_f64 =
                        block.append_operation(cast_op("arith.bitcast", slot, f64_type, location)?);
                    let raw = as_f64.result(0).map_err(|e| e.to_string())?.to_raw();
                    call_args.push(unsafe { Value::from_raw(raw) });
                    offset += 1;
                }
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

fn apply<'c, 'a>(
    context: &'c Context,
    rewriter: &RewriterBase<'c, '_>,
    plan: Planned<'c, 'a>,
    strategy: Strategy,
    retain_shared: &HashMap<(usize, usize), bool>,
    releases: &mut Vec<(OperationRef<'c, 'a>, Value<'c, 'c>)>,
) -> Result<(), String> {
    match plan {
        Planned::DynWrap { op, tag, payload_kind, payload_retain, boxed_layout } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let i64_type: Type = IntegerType::new(context, 64).into();
            let payload = operand(op, 0)?;
            // The payload WORD, by kind (D-051): scalars adapt in
            // place; multi-word payloads (closures, adt structs)
            // heap-box through the strategy allocator.
            let word = match &payload_kind {
                SlotKind::Int(width) => {
                    if *width < 64 {
                        result_value(rewriter.insert(cast_op(
                            "arith.extui", payload, i64_type, location,
                        )?))?
                    } else {
                        unsafe { Value::from_raw(payload.to_raw()) }
                    }
                }
                SlotKind::F64 => result_value(rewriter.insert(cast_op(
                    "arith.bitcast", payload, i64_type, location,
                )?))?,
                SlotKind::Ptr { .. } => result_value(rewriter.insert(cast_op(
                    "llvm.ptrtoint", payload, i64_type, location,
                )?))?,
                SlotKind::Closure | SlotKind::Words { .. } => {
                    let shared = retain_shared
                        .get(&(op.to_raw().ptr as usize, 0))
                        .copied()
                        .unwrap_or(false);
                    maybe_retain(
                        context, rewriter, strategy, payload_retain, payload, shared, location,
                    )?;
                    let slots = payload_kind.slots();
                    let size = result_value(rewriter.insert(
                        melior::dialect::arith::constant(
                            context,
                            IntegerAttribute::new(i64_type, (slots * 8) as i64).into(),
                            location,
                        ),
                    ))?;
                    let boxed = strategy_alloc(
                        context, rewriter, strategy, size, boxed_layout, location,
                    )?;
                    rewriter.insert(store_op(context, payload, boxed, location)?);
                    result_value(rewriter.insert(cast_op(
                        "llvm.ptrtoint", boxed, i64_type, location,
                    )?))?
                }
            };
            let dyn_struct = slots_struct(context, 2);
            let mut acc = result_value(rewriter.insert(llvm::undef(dyn_struct, location)))?;
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
            acc = result_value(rewriter.insert(llvm::insert_value(
                context,
                acc,
                DenseI64ArrayAttribute::new(context, &[1]),
                word,
                location,
            )))?;
            finish(rewriter, op, acc)
        }
        Planned::DynUnwrap { op, tag, result_kind, result_mapped } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let i64_type: Type = IntegerType::new(context, 64).into();
            let container = operand(op, 0)?;
            let actual = result_value(rewriter.insert(llvm::extract_value(
                context,
                container,
                DenseI64ArrayAttribute::new(context, &[0]),
                i64_type,
                location,
            )))?;
            let expected = result_value(rewriter.insert(melior::dialect::arith::constant(
                context,
                IntegerAttribute::new(i64_type, tag).into(),
                location,
            )))?;
            // Straight-line native tag check (D-054): the rt aborts on
            // mismatch — no CFG surgery mid-rewrite. In-process JIT
            // corpus law keeps mismatches out of jit goldens; the trap
            // contract is verified at interp (semantics) and AOT
            // (subprocess) levels.
            rewriter.insert(direct_call_void(
                context,
                "frk_rt_dyn_check",
                &[actual, expected],
                location,
            )?);
            let word = result_value(rewriter.insert(llvm::extract_value(
                context,
                container,
                DenseI64ArrayAttribute::new(context, &[1]),
                i64_type,
                location,
            )))?;
            let value = match &result_kind {
                SlotKind::Int(width) => {
                    if *width < 64 {
                        result_value(rewriter.insert(cast_op(
                            "arith.trunci", word, result_mapped, location,
                        )?))?
                    } else {
                        word
                    }
                }
                SlotKind::F64 => result_value(rewriter.insert(cast_op(
                    "arith.bitcast", word, result_mapped, location,
                )?))?,
                SlotKind::Ptr { .. } => result_value(rewriter.insert(cast_op(
                    "llvm.inttoptr",
                    word,
                    llvm::r#type::pointer(context, 0),
                    location,
                )?))?,
                SlotKind::Closure | SlotKind::Words { .. } => {
                    let ptr = llvm::r#type::pointer(context, 0);
                    let boxed = result_value(rewriter.insert(cast_op(
                        "llvm.inttoptr", word, ptr, location,
                    )?))?;
                    result_value(rewriter.insert(
                        OperationBuilder::new("llvm.load", location)
                            .add_attributes(&[(
                                melior::ir::Identifier::new(context, "ordering"),
                                Attribute::parse(context, "0 : i64").ok_or("ordering")?,
                            )])
                            .add_operands(&[boxed])
                            .add_results(&[result_mapped])
                            .build()
                            .map_err(|e| e.to_string())?,
                    ))?
                }
            };
            finish(rewriter, op, value)
        }
        Planned::DynTagOf { op } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let i64_type: Type = IntegerType::new(context, 64).into();
            let container = operand(op, 0)?;
            let tag = result_value(rewriter.insert(llvm::extract_value(
                context,
                container,
                DenseI64ArrayAttribute::new(context, &[0]),
                i64_type,
                location,
            )))?;
            finish(rewriter, op, tag)
        }
        Planned::ArrayNew { op, die_at, layout, elem_slots } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let i64_type: Type = IntegerType::new(context, 64).into();
            let len = operand(op, 0)?;
            let eight = result_value(rewriter.insert(melior::dialect::arith::constant(
                context,
                IntegerAttribute::new(i64_type, 8).into(),
                location,
            )))?;
            let stride = result_value(rewriter.insert(melior::dialect::arith::constant(
                context,
                IntegerAttribute::new(i64_type, (elem_slots * 8) as i64).into(),
                location,
            )))?;
            let data_bytes = result_value(rewriter.insert(
                OperationBuilder::new("arith.muli", location)
                    .add_operands(&[len, stride])
                    .add_results(&[i64_type])
                    .build()
                    .map_err(|e| e.to_string())?,
            ))?;
            let total = result_value(rewriter.insert(
                OperationBuilder::new("arith.addi", location)
                    .add_operands(&[data_bytes, eight])
                    .add_results(&[i64_type])
                    .build()
                    .map_err(|e| e.to_string())?,
            ))?;
            let base = strategy_alloc(context, rewriter, strategy, total, layout, location)?;
            rewriter.insert(store_op(context, len, base, location)?);
            if let Some(terminator) = die_at {
                releases.push((terminator, base));
            }
            finish(rewriter, op, base)
        }
        Planned::ArrayLen { op } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let i64_type: Type = IntegerType::new(context, 64).into();
            let base = operand(op, 0)?;
            let len = result_value(rewriter.insert(
                OperationBuilder::new("llvm.load", location)
                    .add_attributes(&[(
                        melior::ir::Identifier::new(context, "ordering"),
                        Attribute::parse(context, "0 : i64").ok_or("ordering")?,
                    )])
                    .add_operands(&[base])
                    .add_results(&[i64_type])
                    .build()
                    .map_err(|e| e.to_string())?,
            ))?;
            finish(rewriter, op, len)
        }
        Planned::ArrayGet { op, elem_kind, elem_mapped } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let slots = elem_kind.slots();
            let addr = element_address(context, rewriter, op, slots, location)?;
            if slots == 2 {
                // dyn element (D-058): load both words, rebuild.
                let i64_type: Type = IntegerType::new(context, 64).into();
                let _ = i64_type;
                let w0 = load_slot_at(context, rewriter, addr, 0, location)?;
                let w1 = load_slot_at(context, rewriter, addr, 1, location)?;
                let value = build_dyn_words(context, rewriter, w0, w1, location)?;
                finish(rewriter, op, value)
            } else {
                let loaded = result_value(rewriter.insert(
                    OperationBuilder::new("llvm.load", location)
                        .add_attributes(&[(
                            melior::ir::Identifier::new(context, "ordering"),
                            Attribute::parse(context, "0 : i64").ok_or("ordering")?,
                        )])
                        .add_operands(&[addr])
                        .add_results(&[elem_slot_type(context, &elem_kind, elem_mapped)])
                        .build()
                        .map_err(|e| e.to_string())?,
                ))?;
                let value =
                    elem_from_slot(context, rewriter, &elem_kind, elem_mapped, loaded, location)?;
                finish(rewriter, op, value)
            }
        }
        Planned::ArraySet { op, elem_kind, elem_mapped, elem_retain } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let slots = elem_kind.slots();
            let addr = element_address(context, rewriter, op, slots, location)?;
            let raw = operand(op, 2)?;
            let shared = retain_shared
                .get(&(op.to_raw().ptr as usize, 2))
                .copied()
                .unwrap_or(false);
            maybe_retain(context, rewriter, strategy, elem_retain, raw, shared, location)?;
            if slots == 2 {
                // dyn element: store tag word then payload word.
                let i64_type: Type = IntegerType::new(context, 64).into();
                let ptr = llvm::r#type::pointer(context, 0);
                let w0 = dyn_field(context, rewriter, raw, 0, location)?;
                let w1 = dyn_field(context, rewriter, raw, 1, location)?;
                rewriter.insert(store_op(context, w0, addr, location)?);
                let addr_int = result_value(rewriter.insert(cast_op(
                    "llvm.ptrtoint", addr, i64_type, location,
                )?))?;
                let eight = result_value(rewriter.insert(melior::dialect::arith::constant(
                    context,
                    IntegerAttribute::new(i64_type, 8).into(),
                    location,
                )))?;
                let addr2_int = result_value(rewriter.insert(
                    OperationBuilder::new("arith.addi", location)
                        .add_operands(&[addr_int, eight])
                        .add_results(&[i64_type])
                        .build()
                        .map_err(|e| e.to_string())?,
                ))?;
                let addr2 = result_value(rewriter.insert(cast_op(
                    "llvm.inttoptr", addr2_int, ptr, location,
                )?))?;
                rewriter.insert(store_op(context, w1, addr2, location)?);
            } else {
                let slot =
                    elem_to_slot(context, rewriter, &elem_kind, elem_mapped, raw, location)?;
                rewriter.insert(store_op(context, slot, addr, location)?);
            }
            rewriter.erase_op(op);
            Ok(())
        }
        Planned::TableNew { op } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let i64_type: Type = IntegerType::new(context, 64).into();
            let _ptr = llvm::r#type::pointer(context, 0);
            let size = result_value(rewriter.insert(melior::dialect::arith::constant(
                context,
                IntegerAttribute::new(i64_type, 32).into(),
                location,
            )))?;
            let shell = strategy_alloc(
                context, rewriter, strategy, size, LAYOUT_TABLE_SHELL, location,
            )?;
            let shell_word =
                result_value(rewriter.insert(cast_op("llvm.ptrtoint", shell, i64_type, location)?))?;
            rewriter.insert(direct_call_void(
                context,
                "frk_rt_table_init",
                &[shell_word],
                location,
            )?);
            let value = build_dyn(context, rewriter, 4, shell_word, location)?;
            finish(rewriter, op, value)
        }
        Planned::TableRawGet { op } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let i64_type: Type = IntegerType::new(context, 64).into();
            let ptr = llvm::r#type::pointer(context, 0);
            let table_pay = dyn_field(context, rewriter, operand(op, 0)?, 1, location)?;
            let key_tag = dyn_field(context, rewriter, operand(op, 1)?, 0, location)?;
            let key_pay = dyn_field(context, rewriter, operand(op, 1)?, 1, location)?;
            let two = result_value(rewriter.insert(melior::dialect::arith::constant(
                context,
                IntegerAttribute::new(i64_type, 2).into(),
                location,
            )))?;
            let out = result_value(rewriter.insert(
                OperationBuilder::new("llvm.alloca", location)
                    .add_attributes(&[(
                        melior::ir::Identifier::new(context, "elem_type"),
                        TypeAttribute::new(i64_type).into(),
                    )])
                    .add_operands(&[two])
                    .add_results(&[ptr])
                    .build()
                    .map_err(|e| e.to_string())?,
            ))?;
            rewriter.insert(direct_call_void(
                context,
                "frk_rt_table_raw_get",
                &[table_pay, key_tag, key_pay, out],
                location,
            )?);
            let out_tag = load_slot_at(context, rewriter, out, 0, location)?;
            let out_pay = load_slot_at(context, rewriter, out, 1, location)?;
            let value = build_dyn_words(context, rewriter, out_tag, out_pay, location)?;
            finish(rewriter, op, value)
        }
        Planned::TableRawSet { op } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let i64_type: Type = IntegerType::new(context, 64).into();
            let ptr = llvm::r#type::pointer(context, 0);
            let table_pay = dyn_field(context, rewriter, operand(op, 0)?, 1, location)?;
            let key_tag = dyn_field(context, rewriter, operand(op, 1)?, 0, location)?;
            let key_pay = dyn_field(context, rewriter, operand(op, 1)?, 1, location)?;
            let value_tag = dyn_field(context, rewriter, operand(op, 2)?, 0, location)?;
            let value_pay = dyn_field(context, rewriter, operand(op, 2)?, 1, location)?;

            // rc ownership discipline (D-057 frontier symmetry): the
            // table owns stored keys and values — retain the incoming
            // pair-payloads (masked, transfer-elided) BEFORE the set,
            // release the overwritten value AFTER. Deleted keys stay
            // leak-biased (documented).
            let (old_tag, old_pay) = if strategy == Strategy::Rc {
                let two = result_value(rewriter.insert(melior::dialect::arith::constant(
                    context,
                    IntegerAttribute::new(i64_type, 2).into(),
                    location,
                )))?;
                let out = result_value(rewriter.insert(
                    OperationBuilder::new("llvm.alloca", location)
                        .add_attributes(&[(
                            melior::ir::Identifier::new(context, "elem_type"),
                            TypeAttribute::new(i64_type).into(),
                        )])
                        .add_operands(&[two])
                        .add_results(&[ptr])
                        .build()
                        .map_err(|e| e.to_string())?,
                ))?;
                rewriter.insert(direct_call_void(
                    context,
                    "frk_rt_table_raw_get",
                    &[table_pay, key_tag, key_pay, out],
                    location,
                )?);
                let old_tag = load_slot_at(context, rewriter, out, 0, location)?;
                let old_pay = load_slot_at(context, rewriter, out, 1, location)?;
                for (tag, pay, index) in
                    [(key_tag, key_pay, 1usize), (value_tag, value_pay, 2usize)]
                {
                    let shared = retain_shared
                        .get(&(op.to_raw().ptr as usize, index))
                        .copied()
                        .unwrap_or(false);
                    if shared {
                        let masked =
                            masked_dyn_ptr(context, rewriter, tag, pay, location)?;
                        rewriter.insert(direct_call_void(
                            context,
                            "frk_rt_rc_retain",
                            &[masked],
                            location,
                        )?);
                    }
                }
                (Some(old_tag), Some(old_pay))
            } else {
                (None, None)
            };

            rewriter.insert(direct_call_void(
                context,
                "frk_rt_table_raw_set",
                &[table_pay, key_tag, key_pay, value_tag, value_pay],
                location,
            )?);

            if let (Some(old_tag), Some(old_pay)) = (old_tag, old_pay) {
                let masked = masked_dyn_ptr(context, rewriter, old_tag, old_pay, location)?;
                rewriter.insert(direct_call_void(
                    context,
                    "frk_rt_rc_release",
                    &[masked],
                    location,
                )?);
            }
            rewriter.erase_op(op);
            Ok(())
        }
        Planned::TableLen { op } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let i64_type: Type = IntegerType::new(context, 64).into();
            let table_pay = dyn_field(context, rewriter, operand(op, 0)?, 1, location)?;
            let len = result_value(rewriter.insert(direct_call(
                context,
                "frk_rt_table_len",
                &[table_pay],
                i64_type,
                location,
            )?))?;
            finish(rewriter, op, len)
        }
        Planned::TableSetMeta { op } => {
            // Meta = shell word 3, stored inline (D-056): nil's payload
            // is 0, so nil-meta and no-meta coincide by construction.
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let i64_type: Type = IntegerType::new(context, 64).into();
            let ptr = llvm::r#type::pointer(context, 0);
            let table_pay = dyn_field(context, rewriter, operand(op, 0)?, 1, location)?;
            let meta_tag = dyn_field(context, rewriter, operand(op, 1)?, 0, location)?;
            let meta_pay = dyn_field(context, rewriter, operand(op, 1)?, 1, location)?;
            let offset = result_value(rewriter.insert(melior::dialect::arith::constant(
                context,
                IntegerAttribute::new(i64_type, 24).into(),
                location,
            )))?;
            let addr_word = result_value(rewriter.insert(
                OperationBuilder::new("arith.addi", location)
                    .add_operands(&[table_pay, offset])
                    .add_results(&[i64_type])
                    .build()
                    .map_err(|e| e.to_string())?,
            ))?;
            let addr =
                result_value(rewriter.insert(cast_op("llvm.inttoptr", addr_word, ptr, location)?))?;
            if strategy == Strategy::Rc {
                // Retain new meta (masked; shared-elided), release old.
                let shared = retain_shared
                    .get(&(op.to_raw().ptr as usize, 1))
                    .copied()
                    .unwrap_or(false);
                if shared {
                    let masked =
                        masked_dyn_ptr(context, rewriter, meta_tag, meta_pay, location)?;
                    rewriter.insert(direct_call_void(
                        context,
                        "frk_rt_rc_retain",
                        &[masked],
                        location,
                    )?);
                }
                let old = load_slot_at(context, rewriter, addr, 0, location)?;
                let old_ptr =
                    result_value(rewriter.insert(cast_op("llvm.inttoptr", old, ptr, location)?))?;
                rewriter.insert(store_op(context, meta_pay, addr, location)?);
                rewriter.insert(direct_call_void(
                    context,
                    "frk_rt_rc_release",
                    &[old_ptr],
                    location,
                )?);
            } else {
                rewriter.insert(store_op(context, meta_pay, addr, location)?);
            }
            rewriter.erase_op(op);
            Ok(())
        }
        Planned::TableGetMeta { op } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let i64_type: Type = IntegerType::new(context, 64).into();
            let ptr = llvm::r#type::pointer(context, 0);
            let table_pay = dyn_field(context, rewriter, operand(op, 0)?, 1, location)?;
            let offset = result_value(rewriter.insert(melior::dialect::arith::constant(
                context,
                IntegerAttribute::new(i64_type, 24).into(),
                location,
            )))?;
            let addr_word = result_value(rewriter.insert(
                OperationBuilder::new("arith.addi", location)
                    .add_operands(&[table_pay, offset])
                    .add_results(&[i64_type])
                    .build()
                    .map_err(|e| e.to_string())?,
            ))?;
            let addr =
                result_value(rewriter.insert(cast_op("llvm.inttoptr", addr_word, ptr, location)?))?;
            let word = load_slot_at(context, rewriter, addr, 0, location)?;
            // tag = word == 0 ? nil : table.
            let zero = result_value(rewriter.insert(melior::dialect::arith::constant(
                context,
                IntegerAttribute::new(i64_type, 0).into(),
                location,
            )))?;
            let four = result_value(rewriter.insert(melior::dialect::arith::constant(
                context,
                IntegerAttribute::new(i64_type, 4).into(),
                location,
            )))?;
            let is_zero = result_value(rewriter.insert(
                OperationBuilder::new("arith.cmpi", location)
                    .add_attributes(&[(
                        melior::ir::Identifier::new(context, "predicate"),
                        IntegerAttribute::new(i64_type, 0).into(),
                    )])
                    .add_operands(&[word, zero])
                    .add_results(&[IntegerType::new(context, 1).into()])
                    .build()
                    .map_err(|e| e.to_string())?,
            ))?;
            let tag = result_value(rewriter.insert(
                OperationBuilder::new("arith.select", location)
                    .add_operands(&[is_zero, zero, four])
                    .add_results(&[i64_type])
                    .build()
                    .map_err(|e| e.to_string())?,
            ))?;
            let value = build_dyn_words(context, rewriter, tag, word, location)?;
            finish(rewriter, op, value)
        }
        Planned::TableNext { op } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let i64_type: Type = IntegerType::new(context, 64).into();
            let ptr = llvm::r#type::pointer(context, 0);
            let table_pay = dyn_field(context, rewriter, operand(op, 0)?, 1, location)?;
            let key_tag = dyn_field(context, rewriter, operand(op, 1)?, 0, location)?;
            let key_pay = dyn_field(context, rewriter, operand(op, 1)?, 1, location)?;
            let four = result_value(rewriter.insert(melior::dialect::arith::constant(
                context,
                IntegerAttribute::new(i64_type, 4).into(),
                location,
            )))?;
            let out = result_value(rewriter.insert(
                OperationBuilder::new("llvm.alloca", location)
                    .add_attributes(&[(
                        melior::ir::Identifier::new(context, "elem_type"),
                        TypeAttribute::new(i64_type).into(),
                    )])
                    .add_operands(&[four])
                    .add_results(&[ptr])
                    .build()
                    .map_err(|e| e.to_string())?,
            ))?;
            rewriter.insert(direct_call_void(
                context,
                "frk_rt_table_next",
                &[table_pay, key_tag, key_pay, out],
                location,
            )?);
            let kt = load_slot_at(context, rewriter, out, 0, location)?;
            let kp = load_slot_at(context, rewriter, out, 1, location)?;
            let vt = load_slot_at(context, rewriter, out, 2, location)?;
            let vp = load_slot_at(context, rewriter, out, 3, location)?;
            let key_dyn = build_dyn_words(context, rewriter, kt, kp, location)?;
            let value_dyn = build_dyn_words(context, rewriter, vt, vp, location)?;
            // Two results: replace each, then erase.
            let r0 = op.result(0).map_err(|e| e.to_string())?;
            let r1 = op.result(1).map_err(|e| e.to_string())?;
            rewriter.replace_all_uses_with(r0.into(), key_dyn);
            rewriter.replace_all_uses_with(r1.into(), value_dyn);
            rewriter.erase_op(op);
            Ok(())
        }
        Planned::BstrSub { op } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let ptr = llvm::r#type::pointer(context, 0);
            let value = result_value(rewriter.insert(direct_call(
                context,
                "frk_rt_bstr_sub",
                &[operand(op, 0)?, operand(op, 1)?, operand(op, 2)?],
                ptr,
                location,
            )?))?;
            finish(rewriter, op, value)
        }
        Planned::BstrRep { op } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let ptr = llvm::r#type::pointer(context, 0);
            let value = result_value(rewriter.insert(direct_call(
                context,
                "frk_rt_bstr_rep",
                &[operand(op, 0)?, operand(op, 1)?],
                ptr,
                location,
            )?))?;
            finish(rewriter, op, value)
        }
        Planned::DynPayloadWord { op } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let word = dyn_field(context, rewriter, operand(op, 0)?, 1, location)?;
            finish(rewriter, op, word)
        }
        Planned::CtlPrompt { op, result } => {
            // κ_frk §2 as a branchless sequence (D-060): enter → apply
            // body(token) → exit → resolve-overwrites → load. The body
            // apply carries `frk.ctl_body` so the guard pass leaves it
            // alone — its result is caught locally by `resolve`, never
            // propagated past `prompt`.
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let i64_type: Type = IntegerType::new(context, 64).into();
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
            let token = result_value(rewriter.insert(direct_call(
                context,
                "frk_rt_ctl_prompt_enter",
                &[],
                i64_type,
                location,
            )?))?;
            let body_call = rewriter.insert(
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
                            Attribute::parse(context, "array<i32: 3, 0>")
                                .ok_or("operandSegmentSizes")?,
                        ),
                        (
                            melior::ir::Identifier::new(context, "frk.ctl_body"),
                            Attribute::parse(context, "unit").ok_or("ctl_body")?,
                        ),
                    ])
                    .add_operands(&[fn_ptr, env_ptr, token])
                    .add_results(&[result])
                    .build()
                    .map_err(|e| e.to_string())?,
            );
            let body_result = result_value(body_call)?;
            rewriter.insert(direct_call_void(
                context,
                "frk_rt_ctl_prompt_exit",
                &[token],
                location,
            )?);
            let two = result_value(rewriter.insert(melior::dialect::arith::constant(
                context,
                IntegerAttribute::new(i64_type, 2).into(),
                location,
            )))?;
            let out = result_value(rewriter.insert(
                OperationBuilder::new("llvm.alloca", location)
                    .add_attributes(&[(
                        melior::ir::Identifier::new(context, "elem_type"),
                        TypeAttribute::new(i64_type).into(),
                    )])
                    .add_operands(&[two])
                    .add_results(&[ptr])
                    .build()
                    .map_err(|e| e.to_string())?,
            ))?;
            let body_tag = dyn_field(context, rewriter, body_result, 0, location)?;
            let body_pay = dyn_field(context, rewriter, body_result, 1, location)?;
            store_slot_at(context, rewriter, out, 0, body_tag, location)?;
            store_slot_at(context, rewriter, out, 1, body_pay, location)?;
            // resolve OVERWRITES out with the parked value iff this
            // prompt is the abort's target; its return is unused.
            rewriter.insert(direct_call(
                context,
                "frk_rt_ctl_resolve",
                &[token, out],
                i64_type,
                location,
            )?);
            let yield_tag = load_slot_at(context, rewriter, out, 0, location)?;
            let yield_pay = load_slot_at(context, rewriter, out, 1, location)?;
            let yielded = build_dyn_words(context, rewriter, yield_tag, yield_pay, location)?;
            finish(rewriter, op, yielded)
        }
        Planned::CtlAbort { op } => {
            // Park the value + set pending. The early return that
            // diverts control is the frk-ctl-guard pass's job.
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let token = operand(op, 0)?;
            let tag = dyn_field(context, rewriter, operand(op, 1)?, 0, location)?;
            let payload = dyn_field(context, rewriter, operand(op, 1)?, 1, location)?;
            rewriter.insert(direct_call_void(
                context,
                "frk_rt_ctl_abort",
                &[token, tag, payload],
                location,
            )?);
            rewriter.erase_op(op);
            Ok(())
        }
        Planned::CtlPending { op } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let i64_type: Type = IntegerType::new(context, 64).into();
            let pending = result_value(rewriter.insert(direct_call(
                context,
                "frk_rt_ctl_pending",
                &[],
                i64_type,
                location,
            )?))?;
            finish(rewriter, op, pending)
        }
        Planned::BstrLit { op, text, symbol } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let i64_type: Type = IntegerType::new(context, 64).into();
            let ptr = llvm::r#type::pointer(context, 0);
            let bytes = text.as_bytes();

            let module = root_module(op)?;
            let body = module
                .region(0)
                .map_err(|e| e.to_string())?
                .first_block()
                .ok_or_else(|| "module without a body".to_string())?;
            let count = bytes.len().max(1);
            let elements = if bytes.is_empty() {
                "0".to_string()
            } else {
                bytes
                    .iter()
                    .map(|byte| byte.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let dense = format!("dense<[{elements}]> : tensor<{count}xi8>");
            let array_type = Type::parse(context, &format!("!llvm.array<{count} x i8>"))
                .ok_or("byte array type")?;
            let global = OperationBuilder::new("llvm.mlir.global", location)
                .add_attributes(&[
                    (
                        melior::ir::Identifier::new(context, "sym_name"),
                        StringAttribute::new(context, &symbol).into(),
                    ),
                    (
                        melior::ir::Identifier::new(context, "global_type"),
                        TypeAttribute::new(array_type).into(),
                    ),
                    (
                        melior::ir::Identifier::new(context, "value"),
                        Attribute::parse(context, &dense)
                            .ok_or_else(|| format!("unparsable {dense}"))?,
                    ),
                    (
                        melior::ir::Identifier::new(context, "linkage"),
                        Attribute::parse(context, "#llvm.linkage<internal>").ok_or("linkage")?,
                    ),
                    (
                        melior::ir::Identifier::new(context, "constant"),
                        Attribute::unit(context),
                    ),
                    (
                        melior::ir::Identifier::new(context, "addr_space"),
                        Attribute::parse(context, "0 : i32").ok_or("addr_space")?,
                    ),
                    (
                        melior::ir::Identifier::new(context, "visibility_"),
                        Attribute::parse(context, "0 : i64").ok_or("visibility")?,
                    ),
                ])
                .add_regions([Region::new()])
                .build()
                .map_err(|e| e.to_string())?;
            body.insert_operation(0, global);

            let address = result_value(rewriter.insert(
                OperationBuilder::new("llvm.mlir.addressof", location)
                    .add_attributes(&[(
                        melior::ir::Identifier::new(context, "global_name"),
                        FlatSymbolRefAttribute::new(context, &symbol).into(),
                    )])
                    .add_results(&[ptr])
                    .build()
                    .map_err(|e| e.to_string())?,
            ))?;
            let len = result_value(rewriter.insert(melior::dialect::arith::constant(
                context,
                IntegerAttribute::new(i64_type, bytes.len() as i64).into(),
                location,
            )))?;
            let value = result_value(rewriter.insert(direct_call(
                context,
                "frk_rt_bstr_intern",
                &[address, len],
                ptr,
                location,
            )?))?;
            finish(rewriter, op, value)
        }
        Planned::BstrConcat { op } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let ptr = llvm::r#type::pointer(context, 0);
            let lhs = operand(op, 0)?;
            let rhs = operand(op, 1)?;
            let value = result_value(rewriter.insert(direct_call(
                context,
                "frk_rt_bstr_concat",
                &[lhs, rhs],
                ptr,
                location,
            )?))?;
            finish(rewriter, op, value)
        }
        Planned::BstrEq { op } => {
            // Interned identity (D-056): pointer comparison, inline.
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let i64_type: Type = IntegerType::new(context, 64).into();
            let lhs = result_value(rewriter.insert(cast_op(
                "llvm.ptrtoint",
                operand(op, 0)?,
                i64_type,
                location,
            )?))?;
            let rhs = result_value(rewriter.insert(cast_op(
                "llvm.ptrtoint",
                operand(op, 1)?,
                i64_type,
                location,
            )?))?;
            let equal = result_value(rewriter.insert(
                OperationBuilder::new("arith.cmpi", location)
                    .add_attributes(&[(
                        melior::ir::Identifier::new(context, "predicate"),
                        IntegerAttribute::new(i64_type, 0).into(),
                    )])
                    .add_operands(&[lhs, rhs])
                    .add_results(&[IntegerType::new(context, 1).into()])
                    .build()
                    .map_err(|e| e.to_string())?,
            ))?;
            finish(rewriter, op, equal)
        }
        Planned::BstrLen { op } => {
            // Inline header load (D-056).
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let i64_type: Type = IntegerType::new(context, 64).into();
            let len = result_value(rewriter.insert(
                OperationBuilder::new("llvm.load", location)
                    .add_attributes(&[(
                        melior::ir::Identifier::new(context, "ordering"),
                        Attribute::parse(context, "0 : i64").ok_or("ordering")?,
                    )])
                    .add_operands(&[operand(op, 0)?])
                    .add_results(&[i64_type])
                    .build()
                    .map_err(|e| e.to_string())?,
            ))?;
            finish(rewriter, op, len)
        }
        Planned::StrLit { op, text, symbol } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let i64_type: Type = IntegerType::new(context, 64).into();
            let ptr = llvm::r#type::pointer(context, 0);
            let units: Vec<u16> = text.encode_utf16().collect();

            // The units global (module level).
            let module = root_module(op)?;
            let body = module
                .region(0)
                .map_err(|e| e.to_string())?
                .first_block()
                .ok_or_else(|| "module without a body".to_string())?;
            let unit_count = units.len().max(1);
            let elements = if units.is_empty() {
                "0".to_string()
            } else {
                units
                    .iter()
                    .map(|unit| unit.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let dense = format!("dense<[{elements}]> : tensor<{unit_count}xi16>");
            let array_type = Type::parse(context, &format!("!llvm.array<{unit_count} x i16>"))
                .ok_or("array type")?;
            let global = OperationBuilder::new("llvm.mlir.global", location)
                .add_attributes(&[
                    (
                        melior::ir::Identifier::new(context, "sym_name"),
                        StringAttribute::new(context, &symbol).into(),
                    ),
                    (
                        melior::ir::Identifier::new(context, "global_type"),
                        TypeAttribute::new(array_type).into(),
                    ),
                    (
                        melior::ir::Identifier::new(context, "value"),
                        Attribute::parse(context, &dense)
                            .ok_or_else(|| format!("unparsable {dense}"))?,
                    ),
                    (
                        melior::ir::Identifier::new(context, "linkage"),
                        Attribute::parse(context, "#llvm.linkage<internal>").ok_or("linkage")?,
                    ),
                    (
                        melior::ir::Identifier::new(context, "constant"),
                        Attribute::unit(context),
                    ),
                    (
                        melior::ir::Identifier::new(context, "addr_space"),
                        Attribute::parse(context, "0 : i32").ok_or("addr_space")?,
                    ),
                    (
                        melior::ir::Identifier::new(context, "visibility_"),
                        Attribute::parse(context, "0 : i64").ok_or("visibility")?,
                    ),
                ])
                .add_regions([Region::new()])
                .build()
                .map_err(|e| e.to_string())?;
            body.insert_operation(0, global);

            let address = result_value(rewriter.insert(
                OperationBuilder::new("llvm.mlir.addressof", location)
                    .add_attributes(&[(
                        melior::ir::Identifier::new(context, "global_name"),
                        FlatSymbolRefAttribute::new(context, &symbol).into(),
                    )])
                    .add_results(&[ptr])
                    .build()
                    .map_err(|e| e.to_string())?,
            ))?;
            let len = result_value(rewriter.insert(melior::dialect::arith::constant(
                context,
                IntegerAttribute::new(i64_type, units.len() as i64).into(),
                location,
            )))?;
            let value = result_value(rewriter.insert(direct_call(
                context,
                "frk_rt_str_from_units",
                &[address, len],
                ptr,
                location,
            )?))?;
            finish(rewriter, op, value)
        }
        Planned::StrCall { op, callee, trunc_to_i1 } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let i64_type: Type = IntegerType::new(context, 64).into();
            let ptr = llvm::r#type::pointer(context, 0);
            let mut arguments = Vec::new();
            for index in 0..op.operand_count() {
                arguments.push(operand(op, index)?);
            }
            let result_type = if callee == "frk_rt_str_concat" { ptr } else { i64_type };
            let raw = result_value(rewriter.insert(direct_call(
                context, callee, &arguments, result_type, location,
            )?))?;
            let value = if trunc_to_i1 {
                result_value(rewriter.insert(cast_op(
                    "arith.trunci",
                    raw,
                    IntegerType::new(context, 1).into(),
                    location,
                )?))?
            } else {
                raw
            };
            finish(rewriter, op, value)
        }
        Planned::BoxNew { op, payload_bytes, payload_retain, die_at, layout } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let i64_type: Type = IntegerType::new(context, 64).into();
            let _ptr = llvm::r#type::pointer(context, 0);
            let size = result_value(rewriter.insert(melior::dialect::arith::constant(
                context,
                IntegerAttribute::new(i64_type, payload_bytes as i64).into(),
                location,
            )))?;
            let payload_ptr = strategy_alloc(context, rewriter, strategy, size, layout, location)?;
            let payload = operand(op, 0)?;
            let shared = retain_shared
                .get(&(op.to_raw().ptr as usize, 0))
                .copied()
                .unwrap_or(false);
            maybe_retain(context, rewriter, strategy, payload_retain, payload, shared, location)?;
            rewriter.insert(store_op(context, payload, payload_ptr, location)?);
            if let Some(terminator) = die_at {
                releases.push((terminator, payload_ptr));
            }
            finish(rewriter, op, payload_ptr)
        }
        Planned::BoxGet { op, result } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let boxed = operand(op, 0)?;
            let loaded = result_value(rewriter.insert(
                OperationBuilder::new("llvm.load", location)
                    .add_attributes(&[(
                        melior::ir::Identifier::new(context, "ordering"),
                        Attribute::parse(context, "0 : i64").ok_or("ordering attr")?,
                    )])
                    .add_operands(&[boxed])
                    .add_results(&[result])
                    .build()
                    .map_err(|e| e.to_string())?,
            ))?;
            finish(rewriter, op, loaded)
        }
        Planned::BoxSet { op, payload_retain } => {
            rewriter.set_insertion_point_before(op);
            let location = op.location();
            let boxed = operand(op, 0)?;
            let payload = operand(op, 1)?;
            let shared = retain_shared
                .get(&(op.to_raw().ptr as usize, 1))
                .copied()
                .unwrap_or(false);
            maybe_retain(context, rewriter, strategy, payload_retain, payload, shared, location)?;
            rewriter.insert(store_op(context, payload, boxed, location)?);
            rewriter.erase_op(op);
            Ok(())
        }
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
            appended_retain,
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
            let shared = retain_shared
                .get(&(op.to_raw().ptr as usize, 1))
                .copied()
                .unwrap_or(false);
            maybe_retain(context, rewriter, strategy, appended_retain, appended, shared, location)?;
            write_slots(
                context, rewriter, &mut acc, old_slots, kind.clone(), appended, location,
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
            env_layout,
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
            let env_ptr =
                strategy_alloc(context, rewriter, strategy, size, env_layout, location)?;
            let env_value = operand(op, 0)?;
            // No retains here by design: managed pointers were retained
            // (or transfer-elided) when they entered the env product at
            // snoc time — the product-to-heap copy is not a new owner
            // acquisition (D-041 ownership model).
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
                    read_slots(context, rewriter, arg_pack, offset, kind.clone(), location)?;
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

/// The last operation of `op`'s block — its terminator. Terminators
/// survive the kernel lowering untouched (they are cf/func ops), so a
/// reference captured now is a valid insertion anchor after rewriting.
fn block_terminator<'c, 'a>(op: OperationRef<'c, 'a>) -> Option<OperationRef<'c, 'a>> {
    let block = op.block()?;
    let mut current = block.first_operation()?;
    loop {
        match current.next_in_block() {
            Some(next) => current = next,
            None => return Some(current),
        }
    }
}

/// The enclosing builtin.module of any op.
fn root_module<'c, 'a>(op: OperationRef<'c, 'a>) -> Result<OperationRef<'c, 'a>, String> {
    let mut current = op;
    loop {
        let Some(parent) = current.parent_operation() else {
            return Ok(current);
        };
        current = parent;
    }
}

/// &arr[i]: base + 8 (len header) + i*stride, via ptrtoint
/// arithmetic — portable across every grid triple (D-049/D-058).
fn element_address<'c>(
    context: &'c Context,
    rewriter: &RewriterBase<'c, '_>,
    op: OperationRef<'c, '_>,
    elem_slots: usize,
    location: Location<'c>,
) -> Result<Value<'c, 'c>, String> {
    let i64_type: Type = IntegerType::new(context, 64).into();
    let ptr = llvm::r#type::pointer(context, 0);
    let base = operand(op, 0)?;
    let index = operand(op, 1)?;
    let base_int = result_value(rewriter.insert(cast_op("llvm.ptrtoint", base, i64_type, location)?))?;
    let eight = result_value(rewriter.insert(melior::dialect::arith::constant(
        context,
        IntegerAttribute::new(i64_type, 8).into(),
        location,
    )))?;
    let stride = result_value(rewriter.insert(melior::dialect::arith::constant(
        context,
        IntegerAttribute::new(i64_type, (elem_slots * 8) as i64).into(),
        location,
    )))?;
    let scaled = result_value(rewriter.insert(
        OperationBuilder::new("arith.muli", location)
            .add_operands(&[index, stride])
            .add_results(&[i64_type])
            .build()
            .map_err(|e| e.to_string())?,
    ))?;
    let skip_len = result_value(rewriter.insert(
        OperationBuilder::new("arith.addi", location)
            .add_operands(&[base_int, eight])
            .add_results(&[i64_type])
            .build()
            .map_err(|e| e.to_string())?,
    ))?;
    let addr_int = result_value(rewriter.insert(
        OperationBuilder::new("arith.addi", location)
            .add_operands(&[skip_len, scaled])
            .add_results(&[i64_type])
            .build()
            .map_err(|e| e.to_string())?,
    ))?;
    result_value(rewriter.insert(cast_op("llvm.inttoptr", addr_int, ptr, location)?))
}

/// The in-memory type one element slot stores as.
fn elem_slot_type<'c>(context: &'c Context, kind: &SlotKind<'c>, mapped: Type<'c>) -> Type<'c> {
    match kind {
        SlotKind::Int(width) if *width < 64 => IntegerType::new(context, 64).into(),
        SlotKind::Ptr { .. } => llvm::r#type::pointer(context, 0),
        _ => mapped,
    }
}

/// Loaded slot → the element's mapped value.
fn elem_from_slot<'c>(
    _context: &'c Context,
    rewriter: &RewriterBase<'c, '_>,
    kind: &SlotKind<'c>,
    mapped: Type<'c>,
    loaded: Value<'c, '_>,
    location: Location<'c>,
) -> Result<Value<'c, 'c>, String> {
    match kind {
        SlotKind::Int(width) if *width < 64 => {
            result_value(rewriter.insert(cast_op("arith.trunci", loaded, mapped, location)?))
        }
        _ => Ok(unsafe { Value::from_raw(loaded.to_raw()) }),
    }
}

/// The element's value → what the slot stores.
fn elem_to_slot<'c>(
    _context: &'c Context,
    rewriter: &RewriterBase<'c, '_>,
    kind: &SlotKind<'c>,
    _mapped: Type<'c>,
    value: Value<'c, '_>,
    location: Location<'c>,
) -> Result<Value<'c, 'c>, String> {
    match kind {
        SlotKind::Int(width) if *width < 64 => {
            let i64_type: Type = IntegerType::new(_context, 64).into();
            result_value(rewriter.insert(cast_op("arith.extui", value, i64_type, location)?))
        }
        _ => Ok(unsafe { Value::from_raw(value.to_raw()) }),
    }
}

/// Typed llvm.store of `value` at `address`.
fn store_op<'c>(
    context: &'c Context,
    value: Value<'c, '_>,
    address: Value<'c, '_>,
    location: Location<'c>,
) -> Result<Operation<'c>, String> {
    OperationBuilder::new("llvm.store", location)
        .add_attributes(&[(
            melior::ir::Identifier::new(context, "ordering"),
            Attribute::parse(context, "0 : i64").ok_or("ordering attr")?,
        )])
        .add_operands(&[value, address])
        .build()
        .map_err(|e| e.to_string())
}

/// Under Rc, an owning store retains what it stores — coverage EQUAL
/// to the tracer's (D-057 frontier symmetry). Elided when the store
/// is the value's only use (ownership transfer, D-041). The DynPair
/// arm is branch-free: the payload pointer is masked to null unless
/// the tag is table/fun, and retain(null) is a no-op.
#[allow(clippy::too_many_arguments)]
fn maybe_retain<'c>(
    context: &'c Context,
    rewriter: &RewriterBase<'c, '_>,
    strategy: Strategy,
    kind: RetainKind,
    value: Value<'c, '_>,
    shared: bool,
    location: Location<'c>,
) -> Result<(), String> {
    if strategy != Strategy::Rc || !shared || kind == RetainKind::None {
        return Ok(());
    }
    let managed = managed_ptr_of(context, rewriter, kind, value, location)?;
    let Some(managed) = managed else {
        return Ok(());
    };
    rewriter.insert(direct_call_void(
        context,
        "frk_rt_rc_retain",
        &[managed],
        location,
    )?);
    Ok(())
}

/// Projects the managed pointer a store owns, per RetainKind.
fn managed_ptr_of<'c>(
    context: &'c Context,
    rewriter: &RewriterBase<'c, '_>,
    kind: RetainKind,
    value: Value<'c, '_>,
    location: Location<'c>,
) -> Result<Option<Value<'c, 'c>>, String> {
    let i64_type: Type = IntegerType::new(context, 64).into();
    let ptr = llvm::r#type::pointer(context, 0);
    match kind {
        RetainKind::None => Ok(None),
        RetainKind::Ptr => Ok(Some(unsafe { Value::from_raw(value.to_raw()) })),
        RetainKind::ClosureEnv => Ok(Some(result_value(rewriter.insert(
            llvm::extract_value(
                context,
                value,
                DenseI64ArrayAttribute::new(context, &[1]),
                ptr,
                location,
            ),
        ))?)),
        RetainKind::DynPair => {
            let tag = result_value(rewriter.insert(llvm::extract_value(
                context,
                value,
                DenseI64ArrayAttribute::new(context, &[0]),
                i64_type,
                location,
            )))?;
            let pay = result_value(rewriter.insert(llvm::extract_value(
                context,
                value,
                DenseI64ArrayAttribute::new(context, &[1]),
                i64_type,
                location,
            )))?;
            Ok(Some(masked_dyn_ptr(context, rewriter, tag, pay, location)?))
        }
    }
}

/// tag ∈ {table, fun} ? payload-as-ptr : null (branch-free).
fn masked_dyn_ptr<'c>(
    context: &'c Context,
    rewriter: &RewriterBase<'c, '_>,
    tag: Value<'c, '_>,
    pay: Value<'c, '_>,
    location: Location<'c>,
) -> Result<Value<'c, 'c>, String> {
    let i64_type: Type = IntegerType::new(context, 64).into();
    let ptr = llvm::r#type::pointer(context, 0);
    let four = result_value(rewriter.insert(melior::dialect::arith::constant(
        context,
        IntegerAttribute::new(i64_type, 4).into(),
        location,
    )))?;
    let five = result_value(rewriter.insert(melior::dialect::arith::constant(
        context,
        IntegerAttribute::new(i64_type, 5).into(),
        location,
    )))?;
    let zero = result_value(rewriter.insert(melior::dialect::arith::constant(
        context,
        IntegerAttribute::new(i64_type, 0).into(),
        location,
    )))?;
    let tag_v = unsafe { Value::from_raw(tag.to_raw()) };
    let pay_v = unsafe { Value::from_raw(pay.to_raw()) };
    let cmpi = |predicate: i64, a: Value<'c, 'c>, b: Value<'c, 'c>| {
        rewriter
            .insert(
                OperationBuilder::new("arith.cmpi", location)
                    .add_attributes(&[(
                        melior::ir::Identifier::new(context, "predicate"),
                        IntegerAttribute::new(i64_type, predicate).into(),
                    )])
                    .add_operands(&[a, b])
                    .add_results(&[IntegerType::new(context, 1).into()])
                    .build()
                    .map_err(|e| e.to_string())
                    .map(|op| op)
                    .unwrap(),
            )
            .result(0)
            .map(|r| unsafe { Value::from_raw(r.to_raw()) })
            .map_err(|e| e.to_string())
    };
    let is4 = cmpi(0, tag_v, four)?;
    let is5 = cmpi(0, tag_v, five)?;
    let either = result_value(rewriter.insert(
        OperationBuilder::new("arith.ori", location)
            .add_operands(&[is4, is5])
            .add_results(&[IntegerType::new(context, 1).into()])
            .build()
            .map_err(|e| e.to_string())?,
    ))?;
    let masked = result_value(rewriter.insert(
        OperationBuilder::new("arith.select", location)
            .add_operands(&[either, pay_v, zero])
            .add_results(&[i64_type])
            .build()
            .map_err(|e| e.to_string())?,
    ))?;
    result_value(rewriter.insert(cast_op("llvm.inttoptr", masked, ptr, location)?))
}

/// Strategy allocation call: arena takes bytes; rc takes
/// (bytes, layout) — the D-057 descriptor rides every allocation.
fn strategy_alloc<'c>(
    context: &'c Context,
    rewriter: &RewriterBase<'c, '_>,
    strategy: Strategy,
    size: Value<'c, '_>,
    layout: u64,
    location: Location<'c>,
) -> Result<Value<'c, 'c>, String> {
    let i64_type: Type = IntegerType::new(context, 64).into();
    let ptr = llvm::r#type::pointer(context, 0);
    let size = unsafe { Value::from_raw(size.to_raw()) };
    match strategy {
        Strategy::Arena => result_value(rewriter.insert(direct_call(
            context,
            strategy.alloc_symbol(),
            &[size],
            ptr,
            location,
        )?)),
        Strategy::Rc => {
            let layout_value = result_value(rewriter.insert(
                melior::dialect::arith::constant(
                    context,
                    IntegerAttribute::new(i64_type, layout as i64).into(),
                    location,
                ),
            ))?;
            result_value(rewriter.insert(direct_call(
                context,
                strategy.alloc_symbol(),
                &[size, layout_value],
                ptr,
                location,
            )?))
        }
    }
}

/// One field (0 = tag, 1 = payload) of a lowered dyn struct value.
fn dyn_field<'c>(
    context: &'c Context,
    rewriter: &RewriterBase<'c, '_>,
    value: Value<'c, '_>,
    index: i64,
    location: Location<'c>,
) -> Result<Value<'c, 'c>, String> {
    let i64_type: Type = IntegerType::new(context, 64).into();
    result_value(rewriter.insert(llvm::extract_value(
        context,
        value,
        DenseI64ArrayAttribute::new(context, &[index]),
        i64_type,
        location,
    )))
}

/// Builds a dyn struct from a constant tag + payload word.
fn build_dyn<'c>(
    context: &'c Context,
    rewriter: &RewriterBase<'c, '_>,
    tag: i64,
    payload: Value<'c, '_>,
    location: Location<'c>,
) -> Result<Value<'c, 'c>, String> {
    let i64_type: Type = IntegerType::new(context, 64).into();
    let tag_value = result_value(rewriter.insert(melior::dialect::arith::constant(
        context,
        IntegerAttribute::new(i64_type, tag).into(),
        location,
    )))?;
    build_dyn_words(context, rewriter, tag_value, unsafe {
        Value::from_raw(payload.to_raw())
    }, location)
}

/// Builds a dyn struct from tag + payload SSA words.
fn build_dyn_words<'c>(
    context: &'c Context,
    rewriter: &RewriterBase<'c, '_>,
    tag: Value<'c, '_>,
    payload: Value<'c, '_>,
    location: Location<'c>,
) -> Result<Value<'c, 'c>, String> {
    let dyn_struct = slots_struct(context, 2);
    let mut acc = result_value(rewriter.insert(llvm::undef(dyn_struct, location)))?;
    acc = result_value(rewriter.insert(llvm::insert_value(
        context,
        acc,
        DenseI64ArrayAttribute::new(context, &[0]),
        unsafe { Value::from_raw(tag.to_raw()) },
        location,
    )))?;
    result_value(rewriter.insert(llvm::insert_value(
        context,
        acc,
        DenseI64ArrayAttribute::new(context, &[1]),
        unsafe { Value::from_raw(payload.to_raw()) },
        location,
    )))
}

/// Loads slot `index` (i64) from a base pointer.
fn load_slot_at<'c>(
    context: &'c Context,
    rewriter: &RewriterBase<'c, '_>,
    base: Value<'c, '_>,
    index: usize,
    location: Location<'c>,
) -> Result<Value<'c, 'c>, String> {
    let i64_type: Type = IntegerType::new(context, 64).into();
    let ptr = llvm::r#type::pointer(context, 0);
    let address = if index == 0 {
        unsafe { Value::from_raw(base.to_raw()) }
    } else {
        let base_word =
            result_value(rewriter.insert(cast_op("llvm.ptrtoint", base, i64_type, location)?))?;
        let offset = result_value(rewriter.insert(melior::dialect::arith::constant(
            context,
            IntegerAttribute::new(i64_type, (index * 8) as i64).into(),
            location,
        )))?;
        let sum = result_value(rewriter.insert(
            OperationBuilder::new("arith.addi", location)
                .add_operands(&[base_word, offset])
                .add_results(&[i64_type])
                .build()
                .map_err(|e| e.to_string())?,
        ))?;
        result_value(rewriter.insert(cast_op("llvm.inttoptr", sum, ptr, location)?))?
    };
    result_value(rewriter.insert(
        OperationBuilder::new("llvm.load", location)
            .add_attributes(&[(
                melior::ir::Identifier::new(context, "ordering"),
                Attribute::parse(context, "0 : i64").ok_or("ordering")?,
            )])
            .add_operands(&[address])
            .add_results(&[i64_type])
            .build()
            .map_err(|e| e.to_string())?,
    ))
}

/// Stores i64 `value` into slot `index` at `base` (mirror of
/// [`load_slot_at`]).
fn store_slot_at<'c>(
    context: &'c Context,
    rewriter: &RewriterBase<'c, '_>,
    base: Value<'c, '_>,
    index: usize,
    value: Value<'c, '_>,
    location: Location<'c>,
) -> Result<(), String> {
    let i64_type: Type = IntegerType::new(context, 64).into();
    let ptr = llvm::r#type::pointer(context, 0);
    let address = if index == 0 {
        unsafe { Value::from_raw(base.to_raw()) }
    } else {
        let base_word =
            result_value(rewriter.insert(cast_op("llvm.ptrtoint", base, i64_type, location)?))?;
        let offset = result_value(rewriter.insert(melior::dialect::arith::constant(
            context,
            IntegerAttribute::new(i64_type, (index * 8) as i64).into(),
            location,
        )))?;
        let sum = result_value(rewriter.insert(
            OperationBuilder::new("arith.addi", location)
                .add_operands(&[base_word, offset])
                .add_results(&[i64_type])
                .build()
                .map_err(|e| e.to_string())?,
        ))?;
        result_value(rewriter.insert(cast_op("llvm.inttoptr", sum, ptr, location)?))?
    };
    rewriter.insert(store_op(context, value, address, location)?);
    Ok(())
}

/// Void direct llvm.call by symbol.
fn direct_call_void<'c>(
    context: &'c Context,
    callee: &str,
    arguments: &[Value<'c, '_>],
    location: Location<'c>,
) -> Result<Operation<'c>, String> {
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
                Attribute::parse(
                    context,
                    &format!("array<i32: {}, 0>", arguments.len()),
                )
                .ok_or("operandSegmentSizes")?,
            ),
        ])
        .add_operands(arguments)
        .build()
        .map_err(|e| e.to_string())
}

/// Direct llvm.call by symbol (the allocator)./// Direct llvm.call by symbol (the allocator).
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
    kind: SlotKind<'c>,
    location: Location<'c>,
) -> Result<Value<'c, 'c>, String> {
    let i64_type: Type = IntegerType::new(context, 64).into();
    let ptr = llvm::r#type::pointer(context, 0);
    match kind {
        SlotKind::Words { slots, mapped } => {
            // Rebuild the nested adt struct from verbatim word slots.
            let mut rebuilt = result_value(rewriter.insert(llvm::undef(mapped, location)))?;
            for index in 0..slots {
                let word = result_value(rewriter.insert(llvm::extract_value(
                    context,
                    container,
                    DenseI64ArrayAttribute::new(context, &[(offset + index) as i64]),
                    i64_type,
                    location,
                )))?;
                rebuilt = result_value(rewriter.insert(llvm::insert_value(
                    context,
                    rebuilt,
                    DenseI64ArrayAttribute::new(context, &[index as i64]),
                    word,
                    location,
                )))?;
            }
            Ok(rebuilt)
        }
        SlotKind::Ptr { .. } => {
            let slot = result_value(rewriter.insert(llvm::extract_value(
                context,
                container,
                DenseI64ArrayAttribute::new(context, &[offset as i64]),
                i64_type,
                location,
            )))?;
            result_value(rewriter.insert(cast_op("llvm.inttoptr", slot, ptr, location)?))
        }
        SlotKind::F64 => {
            let slot = result_value(rewriter.insert(llvm::extract_value(
                context,
                container,
                DenseI64ArrayAttribute::new(context, &[offset as i64]),
                i64_type,
                location,
            )))?;
            let f64_type = Type::parse(context, "f64").ok_or("f64 type")?;
            result_value(rewriter.insert(cast_op("arith.bitcast", slot, f64_type, location)?))
        }
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
    kind: SlotKind<'c>,
    value: Value<'c, '_>,
    location: Location<'c>,
) -> Result<(), String> {
    let i64_type: Type = IntegerType::new(context, 64).into();
    match kind {
        SlotKind::Words { slots, .. } => {
            for index in 0..slots {
                let word = result_value(rewriter.insert(llvm::extract_value(
                    context,
                    value,
                    DenseI64ArrayAttribute::new(context, &[index as i64]),
                    i64_type,
                    location,
                )))?;
                *accumulator = result_value(rewriter.insert(llvm::insert_value(
                    context,
                    *accumulator,
                    DenseI64ArrayAttribute::new(context, &[(offset + index) as i64]),
                    word,
                    location,
                )))?;
            }
        }
        SlotKind::Ptr { .. } => {
            let word =
                result_value(rewriter.insert(cast_op("llvm.ptrtoint", value, i64_type, location)?))?;
            *accumulator = result_value(rewriter.insert(llvm::insert_value(
                context,
                *accumulator,
                DenseI64ArrayAttribute::new(context, &[offset as i64]),
                word,
                location,
            )))?;
        }
        SlotKind::F64 => {
            let word =
                result_value(rewriter.insert(cast_op("arith.bitcast", value, i64_type, location)?))?;
            *accumulator = result_value(rewriter.insert(llvm::insert_value(
                context,
                *accumulator,
                DenseI64ArrayAttribute::new(context, &[offset as i64]),
                word,
                location,
            )))?;
        }
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
