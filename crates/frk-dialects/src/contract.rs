//! frk.contract — checked casts with blame (SPEC §4.6; D-015 executed
//! by D-072). The dialect namespace is `frk_contract`; SPEC prose
//! writes "frk.contract" for the same thing.
//!
//! v0 surface (TS-1, D-072):
//! - `narrow(sum) {variant, blame} -> sum` — a CHECKED CAST asserting
//!   the operand's tag is `variant`. Identity on success; refutation
//!   is a deterministic blame trap. The result type equals the operand
//!   type: narrowing is a *fact*, not a representation change — the
//!   claimed variant rides as the attribute that downstream
//!   `frk_adt.extract` sites use.
//!
//! Trust-but-verify (the D-072 architecture): frontends emit a narrow
//! for every IMPORTED flow fact (e.g. tsc's control-flow narrowing) —
//! the fact is untrusted input. The interpreter executes every check
//! (reference semantics is maximal checking). Native paths first run
//! [`promote_narrows`], a forward must-dataflow pass that re-derives
//! the facts from `frk_adt.tag_of` tests on `cf.cond_br` edges and
//! DELETES every narrow it can prove; the rest lower to runtime
//! checks (`frk_rt_contract_check`, blame attached). A wrong promotion
//! is a divergence against the always-checking interpreter (L3).

use std::collections::{HashMap, VecDeque};

use melior::ir::attribute::StringAttribute;
use melior::ir::operation::OperationLike;
use melior::ir::{BlockLike, BlockRef, OperationRef, RegionLike, ValueLike};
use melior::{Context, IrRewriter};

use crate::adt::{decode_sum, index_attr};

/// The dialect definition, loaded with the combined kernel module by
/// [`crate::register`] (the `@frk_adt::@sum` reference resolves there).
pub const IRDL: &str = r##"
irdl.dialect @frk_contract {
  irdl.operation @narrow {
    %sum_in = irdl.base @frk_adt::@sum
    %sum_out = irdl.base @frk_adt::@sum
    %vidx = irdl.base "#builtin.integer"
    %blame = irdl.base "#builtin.string"
    irdl.operands(sum: %sum_in)
    irdl.results(narrowed: %sum_out)
    irdl.attributes { "variant" = %vidx, "blame" = %blame }
  }
}
"##;

// ---- semantic verification (K1 second half; driven by crate::verify) ----

pub(crate) fn verify_op<'c>(
    context: &'c Context,
    name: &str,
    op: OperationRef<'c, '_>,
) -> Result<(), String> {
    match name {
        "narrow" => {
            let operand = op
                .operand(0)
                .map_err(|_| "narrow without an operand".to_string())?
                .r#type();
            let result = op
                .result(0)
                .map_err(|_| "narrow without a result".to_string())?
                .r#type();
            if operand != result {
                return Err(format!(
                    "narrow is identity-on-success: operand is {operand}, result declares {result}"
                ));
            }
            let variants = decode_sum(context, operand)?;
            let variant = index_attr(op, "variant")?;
            if variant >= variants.len() {
                return Err(format!(
                    "variant {variant} out of range: the sum has {} variant(s)",
                    variants.len()
                ));
            }
            blame_of(op)?;
            Ok(())
        }
        other => Err(format!("unknown frk_contract op {other:?}")),
    }
}

/// The blame string attribute — carried verbatim into trap messages.
pub(crate) fn blame_of(op: OperationRef<'_, '_>) -> Result<String, String> {
    let attribute = op
        .attribute("blame")
        .map_err(|_| "narrow without a blame attribute".to_string())?;
    let blame = StringAttribute::try_from(attribute)
        .map_err(|_| "blame must be a string attribute".to_string())?;
    Ok(blame.value().to_string())
}

// ---- the promotion pass (D-072 §5) ----

/// Raw-pointer identity for SSA values (the dataflow state key).
fn value_key(value: melior::ir::Value<'_, '_>) -> usize {
    value.to_raw().ptr as usize
}

fn block_key(block: BlockRef<'_, '_>) -> usize {
    block.to_raw().ptr as usize
}

/// The defining op of `value`, if it is an op result.
fn defining_op<'c, 'a>(
    value: melior::ir::Value<'c, 'a>,
) -> Option<OperationRef<'c, 'a>> {
    unsafe {
        let raw = value.to_raw();
        if !mlir_sys::mlirValueIsAOpResult(raw) {
            return None;
        }
        Some(OperationRef::from_raw(mlir_sys::mlirOpResultGetOwner(raw)))
    }
}

fn op_name(op: OperationRef<'_, '_>) -> String {
    op.name()
        .as_string_ref()
        .as_str()
        .map(str::to_string)
        .unwrap_or_default()
}

/// Resolves a value through `frk_contract.narrow` results to the
/// underlying sum SSA value — facts key on the root.
fn resolve_root<'c, 'a>(
    mut value: melior::ir::Value<'c, 'a>,
) -> melior::ir::Value<'c, 'a> {
    loop {
        let Some(owner) = defining_op(value) else {
            return value;
        };
        if op_name(owner) != "frk_contract.narrow" {
            return value;
        }
        let Ok(operand) = owner.operand(0) else {
            return value;
        };
        value = unsafe { melior::ir::Value::from_raw(operand.to_raw()) };
    }
}

/// An i64 `arith.constant`'s value, if that is what `value` is.
fn constant_i64(value: melior::ir::Value<'_, '_>) -> Option<i64> {
    let op = defining_op(value)?;
    if op_name(op) != "arith.constant" {
        return None;
    }
    let attribute = op.attribute("value").ok()?;
    melior::ir::attribute::IntegerAttribute::try_from(attribute)
        .ok()
        .map(|a| a.value())
}

/// Decodes a branch condition of the shape
/// `arith.cmpi eq/ne (frk_adt.tag_of(root), const K)` (either operand
/// order). Returns (root value, K, is_eq).
fn tag_test<'c, 'a>(
    condition: melior::ir::Value<'c, 'a>,
) -> Option<(melior::ir::Value<'c, 'a>, i64, bool)> {
    let cmp = defining_op(condition)?;
    if op_name(cmp) != "arith.cmpi" {
        return None;
    }
    let predicate = melior::ir::attribute::IntegerAttribute::try_from(
        cmp.attribute("predicate").ok()?,
    )
    .ok()?
    .value();
    // arith CmpIPredicate: eq = 0, ne = 1.
    let is_eq = match predicate {
        0 => true,
        1 => false,
        _ => return None,
    };
    let lhs = unsafe { melior::ir::Value::from_raw(cmp.operand(0).ok()?.to_raw()) };
    let rhs = unsafe { melior::ir::Value::from_raw(cmp.operand(1).ok()?.to_raw()) };
    for (side, other) in [(lhs, rhs), (rhs, lhs)] {
        let Some(owner) = defining_op(side) else {
            continue;
        };
        if op_name(owner) != "frk_adt.tag_of" {
            continue;
        }
        let Some(constant) = constant_i64(other) else {
            continue;
        };
        let sum = unsafe { melior::ir::Value::from_raw(owner.operand(0).ok()?.to_raw()) };
        return Some((resolve_root(sum), constant, is_eq));
    }
    None
}

/// Deletes every `frk_contract.narrow` the dataflow can prove, module
/// wide. Returns (promoted, surviving) counts.
///
/// Per function region: state = possible-tag bitmask per sum root at
/// each block's entry. The entry block starts every root at the full
/// mask; edges constrain (a `cf.cond_br` on a tag test intersects the
/// tested tag on its true edge and subtracts it on the false edge, for
/// `eq` — `ne` mirrored); block entry state is the union over
/// predecessor contributions; iterate to fixpoint. Sums are pure
/// values, so facts never invalidate — the transfer is monotone. A
/// narrow claiming variant v of root r promotes iff its block-entry
/// mask for r is a subset of {v}.
pub fn promote_narrows(module: OperationRef<'_, '_>) -> Result<(usize, usize), String> {
    // Sound: the context strictly outlives every IR object walked here.
    let context = unsafe { module.context().to_ref() };
    let mut promoted = 0usize;
    let mut surviving = 0usize;

    let body_region = module.region(0).map_err(|e| e.to_string())?;
    let Some(body) = body_region.first_block() else {
        return Ok((0, 0));
    };
    let mut next_function = body.first_operation();
    while let Some(function) = next_function {
        next_function = function.next_in_block();
        if op_name(function) != "func.func" {
            continue;
        }
        let Ok(region) = function.region(0) else {
            continue;
        };
        let (a, b) = promote_region(context, &region)?;
        promoted += a;
        surviving += b;
    }
    Ok((promoted, surviving))
}

fn promote_region<'c>(
    context: &'c Context,
    region: &melior::ir::RegionRef<'c, '_>,
) -> Result<(usize, usize), String> {
    // Collect blocks (order = region order; entry first) and the
    // narrow ops per block.
    let mut blocks = Vec::new();
    let mut block_index: HashMap<usize, usize> = HashMap::new();
    let mut cursor = region.first_block();
    while let Some(block) = cursor {
        block_index.insert(block_key(block), blocks.len());
        blocks.push(block);
        cursor = block.next_in_region();
    }
    if blocks.is_empty() {
        return Ok((0, 0));
    }

    // Narrows: (block index, op, root key, claimed variant). Roots:
    // key -> variant count (from the sum type).
    let mut narrows = Vec::new();
    let mut root_widths: HashMap<usize, usize> = HashMap::new();
    for (index, block) in blocks.iter().enumerate() {
        let mut next = block.first_operation();
        while let Some(op) = next {
            next = op.next_in_block();
            if op_name(op) != "frk_contract.narrow" {
                continue;
            }
            let operand = op.operand(0).map_err(|e| e.to_string())?;
            let root =
                resolve_root(unsafe { melior::ir::Value::from_raw(operand.to_raw()) });
            let variants = decode_sum(context, root.r#type())?.len();
            let claimed = index_attr(op, "variant")?;
            // >64 variants: the mask cannot express the state — such a
            // narrow honestly never promotes (D-072 fence).
            if variants <= 64 {
                root_widths.insert(value_key(root), variants);
                narrows.push((index, op, value_key(root), claimed));
            } else {
                narrows.push((index, op, usize::MAX, claimed));
            }
        }
    }
    if narrows.is_empty() {
        return Ok((0, 0));
    }

    let full = |width: usize| -> u64 {
        if width >= 64 { u64::MAX } else { (1u64 << width) - 1 }
    };

    // Forward must-dataflow. state[b]: root key -> possible-tag mask.
    // Absent block state = unreachable-so-far; absent root in a
    // reachable state = full mask (facts only ever shrink from full).
    let mut states: Vec<Option<HashMap<usize, u64>>> = vec![None; blocks.len()];
    states[0] = Some(HashMap::new());
    let mut worklist: VecDeque<usize> = VecDeque::from([0]);
    while let Some(index) = worklist.pop_front() {
        let out = states[index].clone().expect("worklist holds reachable blocks");
        let block = blocks[index];
        let Some(terminator) = last_op(block) else {
            continue;
        };
        let successors: Vec<BlockRef> = terminator.successors().collect();
        // Edge constraint: only cf.cond_br over a tag test constrains.
        let test = if op_name(terminator) == "cf.cond_br" {
            terminator
                .operand(0)
                .ok()
                .and_then(|c| tag_test(unsafe { melior::ir::Value::from_raw(c.to_raw()) }))
        } else {
            None
        };
        for (position, successor) in successors.iter().enumerate() {
            let mut contribution = out.clone();
            if let Some((root, tag, is_eq)) = &test {
                let key = value_key(*root);
                if let Some(width) = root_widths.get(&key) {
                    let all = full(*width);
                    let bit = if *tag >= 0 && (*tag as usize) < *width {
                        1u64 << *tag
                    } else {
                        0
                    };
                    let entry = contribution.entry(key).or_insert(all);
                    // cond_br successor 0 is the true edge, 1 the false
                    // edge. eq/true and ne/false intersect; the other
                    // two subtract.
                    let intersect = (position == 0) == *is_eq;
                    if intersect {
                        *entry &= bit;
                    } else {
                        *entry &= !bit;
                    }
                }
            }
            let Some(target) = block_index.get(&block_key(*successor)) else {
                continue;
            };
            let changed = match &mut states[*target] {
                None => {
                    states[*target] = Some(contribution);
                    true
                }
                Some(existing) => {
                    let mut changed = false;
                    // Union: a root missing on either side is full there.
                    for (key, mask) in &contribution {
                        let width = root_widths.get(key).copied().unwrap_or(64);
                        let entry = entry_or_full(existing, *key, full(width));
                        let merged = *entry | *mask;
                        if merged != *entry {
                            *entry = merged;
                            changed = true;
                        }
                    }
                    // A root constrained in `existing` but absent in
                    // `contribution` widens back to full.
                    let keys: Vec<usize> = existing.keys().copied().collect();
                    for key in keys {
                        if !contribution.contains_key(&key) {
                            let width = root_widths.get(&key).copied().unwrap_or(64);
                            let all = full(width);
                            let entry = existing.get_mut(&key).expect("key just listed");
                            if *entry != all {
                                *entry = all;
                                changed = true;
                            }
                        }
                    }
                    changed
                }
            };
            if changed {
                worklist.push_back(*target);
            }
        }
    }

    // Promote what the fixpoint proves.
    let rewriter = IrRewriter::new(context);
    let rewriter = rewriter.as_rewriter_base();
    let mut promoted = 0usize;
    let mut surviving = 0usize;
    for (index, op, root, claimed) in narrows {
        let proven = root != usize::MAX
            && match &states[index] {
                // Unreachable block: any claim holds vacuously.
                None => true,
                Some(state) => {
                    let width = root_widths.get(&root).copied().unwrap_or(64);
                    let mask = state.get(&root).copied().unwrap_or(full(width));
                    mask & !(1u64 << claimed) == 0
                }
            };
        if proven {
            let result = op.result(0).map_err(|e| e.to_string())?;
            let operand = op.operand(0).map_err(|e| e.to_string())?;
            rewriter.replace_all_uses_with(result.into(), unsafe {
                melior::ir::Value::from_raw(operand.to_raw())
            });
            rewriter.erase_op(op);
            promoted += 1;
        } else {
            surviving += 1;
        }
    }
    Ok((promoted, surviving))
}

fn entry_or_full<'m>(
    state: &'m mut HashMap<usize, u64>,
    key: usize,
    full: u64,
) -> &'m mut u64 {
    state.entry(key).or_insert(full)
}

fn last_op<'c, 'a>(block: BlockRef<'c, 'a>) -> Option<OperationRef<'c, 'a>> {
    let mut current = block.first_operation()?;
    while let Some(next) = current.next_in_block() {
        current = next;
    }
    Some(current)
}
