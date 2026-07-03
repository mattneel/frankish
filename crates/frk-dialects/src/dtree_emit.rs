//! Decision-tree → dispatch-IR emission (promoted out of the ml_core
//! frontend at M6 — the extraction loop working as designed: the
//! specimen built it, the promotion pass moved it down to where every
//! match-bearing frontend can reach it).
//!
//! The component is frontend-agnostic because the kernel types carry
//! everything: occurrence typing walks the scrutinee's `!frk_adt` type
//! through [`crate::adt::decode_sum`]/[`decode_product`], so the ONLY
//! thing a frontend supplies is arm-body emission (a callback receiving
//! the arm index and the pattern bindings as SSA values).
//!
//! Dispatch shapes (all D-031-honest — no region ops):
//! - `SwitchTag` on a sum occurrence: `frk_adt.tag_of` + `cf.switch`;
//!   on an `i1` occurrence (frontends encode bool as a two-variant sum
//!   in the matrix): `arith.select` to i64 + `cf.switch`.
//! - `SwitchInt`: `cf.switch` on the value directly.
//! - Zero-explicit-case switches (single-variant dispatch) emit the
//!   sole subtree inline — `cf.switch` cannot carry an empty case list.
//! - Leaves bind occurrence values (`extract`/`get` chains, recomputed
//!   per branch — pure ops, no sharing needed) and branch to the merge
//!   block with the arm's value.
//!
//! `Fail` nodes are a caller bug: check the compiler's diagnostics for
//! exhaustiveness BEFORE emitting (the ml frontend rejects with a
//! witness); this layer errors out loudly if one survives.

use melior::Context;
use melior::ir::attribute::{Attribute, DenseI32ArrayAttribute, IntegerAttribute};
use melior::ir::operation::OperationBuilder;
use melior::ir::r#type::IntegerType;
use melior::ir::{Block, BlockLike, BlockRef, Identifier, Location, Region, RegionLike, Type, Value, ValueLike};

use crate::adt::{decode_product, decode_sum};
use crate::adt_dtree::{Access, DecisionTree, Occurrence};

/// Emits `tree` as dispatch IR starting in `entry`; every leaf branches
/// to `merge` (whose single block argument is the match result).
///
/// `emit_arm(arm_entry, arm_index, bindings)` emits one arm's body
/// starting in `arm_entry` and returns the arm's value plus the block
/// its code ended in (arm bodies may split blocks themselves).
#[allow(clippy::too_many_arguments)]
pub fn emit_dispatch<'c, 'r>(
    context: &'c Context,
    region: &'r Region<'c>,
    entry: BlockRef<'c, 'r>,
    tree: &DecisionTree,
    scrutinee: Value<'c, 'r>,
    scrutinee_type: Type<'c>,
    merge: BlockRef<'c, 'r>,
    emit_arm: &mut dyn FnMut(
        BlockRef<'c, 'r>,
        usize,
        &[(String, Value<'c, 'r>)],
    ) -> Result<(Value<'c, 'r>, BlockRef<'c, 'r>), String>,
) -> Result<(), String> {
    let mut cx = Cx { context, region, merge };
    cx.tree(entry, tree, scrutinee, scrutinee_type, emit_arm)
}

struct Cx<'c, 'r> {
    context: &'c Context,
    region: &'r Region<'c>,
    merge: BlockRef<'c, 'r>,
}

type ArmEmitter<'a, 'c, 'r> = &'a mut dyn FnMut(
    BlockRef<'c, 'r>,
    usize,
    &[(String, Value<'c, 'r>)],
) -> Result<(Value<'c, 'r>, BlockRef<'c, 'r>), String>;

impl<'c, 'r> Cx<'c, 'r> {
    fn loc(&self) -> Location<'c> {
        Location::unknown(self.context)
    }

    fn i64_type(&self) -> Type<'c> {
        IntegerType::new(self.context, 64).into()
    }

    fn op_result(
        &self,
        block: BlockRef<'c, 'r>,
        op: melior::ir::Operation<'c>,
    ) -> Result<Value<'c, 'r>, String> {
        let inserted = block.append_operation(op);
        let raw = inserted
            .result(0)
            .map_err(|_| "op has no result".to_string())?
            .to_raw();
        Ok(unsafe { Value::from_raw(raw) })
    }

    fn tree(
        &mut self,
        block: BlockRef<'c, 'r>,
        tree: &DecisionTree,
        scrutinee: Value<'c, 'r>,
        scrutinee_type: Type<'c>,
        emit_arm: ArmEmitter<'_, 'c, 'r>,
    ) -> Result<(), String> {
        match tree {
            DecisionTree::Fail => Err(
                "FAIL reached dispatch emission — the caller must reject \
                 inexhaustive matches before emitting"
                    .to_string(),
            ),
            DecisionTree::Leaf { arm, bindings } => {
                let mut bound = Vec::with_capacity(bindings.len());
                for (name, occurrence) in bindings {
                    let (value, _) =
                        self.occurrence(block, scrutinee, scrutinee_type, occurrence)?;
                    bound.push((name.clone(), value));
                }
                let (value, exit_block) = emit_arm(block, *arm, &bound)?;
                exit_block.append_operation(self.br(self.merge, Some(value))?);
                Ok(())
            }
            DecisionTree::SwitchTag { occurrence, cases, default } => {
                let (value, value_type) =
                    self.occurrence(block, scrutinee, scrutinee_type, occurrence)?;
                let flag = if value_type.to_string() == "i1" {
                    // Bool scrutinee, encoded as a two-variant sum in
                    // the matrix: false=0, true=1 via select.
                    let one = self.const_i64(block, 1)?;
                    let zero = self.const_i64(block, 0)?;
                    self.op_result(
                        block,
                        OperationBuilder::new("arith.select", self.loc())
                            .add_operands(&[value, one, zero])
                            .add_results(&[self.i64_type()])
                            .build()
                            .map_err(|e| e.to_string())?,
                    )?
                } else {
                    self.op_result(
                        block,
                        OperationBuilder::new("frk_adt.tag_of", self.loc())
                            .add_operands(&[value])
                            .add_results(&[self.i64_type()])
                            .build()
                            .map_err(|e| e.to_string())?,
                    )?
                };
                let case_values: Vec<i64> = cases.iter().map(|(tag, _)| *tag as i64).collect();
                let subtrees: Vec<&DecisionTree> =
                    cases.iter().map(|(_, subtree)| subtree).collect();
                self.switch(
                    block,
                    flag,
                    &case_values,
                    &subtrees,
                    default.as_deref(),
                    scrutinee,
                    scrutinee_type,
                    emit_arm,
                )
            }
            DecisionTree::SwitchInt { occurrence, cases, default } => {
                let (value, _) =
                    self.occurrence(block, scrutinee, scrutinee_type, occurrence)?;
                let case_values: Vec<i64> = cases.iter().map(|(literal, _)| *literal).collect();
                let subtrees: Vec<&DecisionTree> =
                    cases.iter().map(|(_, subtree)| subtree).collect();
                self.switch(
                    block,
                    value,
                    &case_values,
                    &subtrees,
                    Some(default),
                    scrutinee,
                    scrutinee_type,
                    emit_arm,
                )
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn switch(
        &mut self,
        block: BlockRef<'c, 'r>,
        flag: Value<'c, 'r>,
        case_values: &[i64],
        subtrees: &[&DecisionTree],
        default: Option<&DecisionTree>,
        scrutinee: Value<'c, 'r>,
        scrutinee_type: Type<'c>,
        emit_arm: ArmEmitter<'_, 'c, 'r>,
    ) -> Result<(), String> {
        // cf.switch always needs a default successor; with complete
        // coverage the last case doubles as it.
        let (explicit, default_tree): (Vec<(i64, &DecisionTree)>, &DecisionTree) = match default
        {
            Some(tree) => (
                case_values.iter().copied().zip(subtrees.iter().copied()).collect(),
                tree,
            ),
            None => {
                let last = subtrees.len() - 1;
                (
                    case_values[..last]
                        .iter()
                        .copied()
                        .zip(subtrees[..last].iter().copied())
                        .collect(),
                    subtrees[last],
                )
            }
        };

        if explicit.is_empty() {
            // Single-variant dispatch: no branch at all.
            return self.tree(block, default_tree, scrutinee, scrutinee_type, emit_arm);
        }

        let default_block = self.region.append_block(Block::new(&[]));
        let case_blocks: Vec<BlockRef<'c, 'r>> = explicit
            .iter()
            .map(|_| self.region.append_block(Block::new(&[])))
            .collect();

        let case_values_text = explicit
            .iter()
            .map(|(value, _)| value.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let dense = format!(
            "dense<[{case_values_text}]> : vector<{}xi64>",
            explicit.len()
        );
        let mut successors: Vec<&Block<'c>> = vec![&default_block];
        for case_block in &case_blocks {
            successors.push(case_block);
        }
        let segments = vec![0i32; explicit.len()];
        block.append_operation(
            OperationBuilder::new("cf.switch", self.loc())
                .add_attributes(&[
                    (
                        Identifier::new(self.context, "case_values"),
                        Attribute::parse(self.context, &dense)
                            .ok_or_else(|| format!("unparsable {dense}"))?,
                    ),
                    (
                        Identifier::new(self.context, "case_operand_segments"),
                        DenseI32ArrayAttribute::new(self.context, &segments).into(),
                    ),
                    (
                        Identifier::new(self.context, "operandSegmentSizes"),
                        DenseI32ArrayAttribute::new(self.context, &[1, 0, 0]).into(),
                    ),
                ])
                .add_operands(&[flag])
                .add_successors(&successors)
                .build()
                .map_err(|e| e.to_string())?,
        );

        for ((_, subtree), case_block) in explicit.iter().zip(&case_blocks) {
            self.tree(*case_block, subtree, scrutinee, scrutinee_type, emit_arm)?;
        }
        self.tree(default_block, default_tree, scrutinee, scrutinee_type, emit_arm)
    }

    /// Emits the access chain for an occurrence; returns (value, type).
    /// Types come from the kernel type parameters themselves.
    fn occurrence(
        &self,
        block: BlockRef<'c, 'r>,
        scrutinee: Value<'c, 'r>,
        scrutinee_type: Type<'c>,
        occurrence: &Occurrence,
    ) -> Result<(Value<'c, 'r>, Type<'c>), String> {
        let i64_type = self.i64_type();
        let mut value = scrutinee;
        let mut current_type = scrutinee_type;
        for access in occurrence {
            match access {
                Access::SumField { variant, field } => {
                    let variants = decode_sum(self.context, current_type)?;
                    let field_type = *variants
                        .get(*variant)
                        .and_then(|fields| fields.get(*field))
                        .ok_or_else(|| {
                            format!("occurrence out of range: v{variant} f{field}")
                        })?;
                    value = self.op_result(
                        block,
                        OperationBuilder::new("frk_adt.extract", self.loc())
                            .add_attributes(&[
                                (
                                    Identifier::new(self.context, "variant"),
                                    IntegerAttribute::new(i64_type, *variant as i64).into(),
                                ),
                                (
                                    Identifier::new(self.context, "field"),
                                    IntegerAttribute::new(i64_type, *field as i64).into(),
                                ),
                            ])
                            .add_operands(&[value])
                            .add_results(&[field_type])
                            .build()
                            .map_err(|e| e.to_string())?,
                    )?;
                    current_type = field_type;
                }
                Access::ProductField { field } => {
                    let fields = decode_product(self.context, current_type)?;
                    let field_type = *fields.get(*field).ok_or_else(|| {
                        format!("occurrence out of range: p{field}")
                    })?;
                    value = self.op_result(
                        block,
                        OperationBuilder::new("frk_adt.get", self.loc())
                            .add_attributes(&[(
                                Identifier::new(self.context, "field"),
                                IntegerAttribute::new(i64_type, *field as i64).into(),
                            )])
                            .add_operands(&[value])
                            .add_results(&[field_type])
                            .build()
                            .map_err(|e| e.to_string())?,
                    )?;
                    current_type = field_type;
                }
            }
        }
        Ok((value, current_type))
    }

    fn const_i64(
        &self,
        block: BlockRef<'c, 'r>,
        value: i64,
    ) -> Result<Value<'c, 'r>, String> {
        self.op_result(
            block,
            melior::dialect::arith::constant(
                self.context,
                IntegerAttribute::new(self.i64_type(), value).into(),
                self.loc(),
            ),
        )
    }

    fn br(
        &self,
        target: BlockRef<'c, 'r>,
        value: Option<Value<'c, 'r>>,
    ) -> Result<melior::ir::Operation<'c>, String> {
        let operands: Vec<Value<'c, 'r>> = value.into_iter().collect();
        OperationBuilder::new("cf.br", self.loc())
            .add_operands(&operands)
            .add_successors(&[&target])
            .build()
            .map_err(|e| e.to_string())
    }
}
