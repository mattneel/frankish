//! Emission: typed ml_core AST → kernel-dialect module (M5). This is
//! where D-034's deferred piece meets its first consumer: `match`
//! compiles through the Maranget decision-tree pass into the
//! de-regioned dispatch shape (tag switches + guarded extracts).
//!
//! Shape decisions:
//! - Pure cf-style CFG (no scf): match dispatch needs multi-block
//!   regions, scf.if regions are single-block, so `if` uses cond_br
//!   diamonds too — one uniform emission mode.
//! - Everything callable is a closure. Every `fun` lambda-lifts to a
//!   top-level `func.func @__ml_fn_<id>(captures..., param) -> result`
//!   and the expression site emits `closure.make` over a packed env.
//!   `let rec` groups share one capture list; each lifted body's
//!   prologue re-makes every group member's closure from its own
//!   capture params — by-value capture can't tie the knot, re-making
//!   ties it per call (the D-035 spin pattern).
//! - Branch ops are built generically with the attribute names MLIR 22
//!   actually uses (`operandSegmentSizes` — melior's cf helpers still
//!   spell the pre-22 names).
//! - Locations are all unknown (§6.5 span threading is ledgered debt).

use std::collections::{BTreeMap, HashMap, HashSet};

use melior::Context;
use melior::ir::attribute::{
    Attribute, DenseI32ArrayAttribute, FlatSymbolRefAttribute, IntegerAttribute, StringAttribute,
    TypeAttribute,
};
use melior::ir::operation::{OperationBuilder, OperationLike};
use melior::ir::r#type::{FunctionType, IntegerType};
use melior::ir::{
    Block, BlockLike, BlockRef, Identifier, Location, Module, Region, RegionLike, Type, Value,
    ValueLike,
};

use crate::ast::Pattern;
use crate::infer::{TBinding, TExpr, TKind, TypedProgram};
use crate::types::Ty;
use frk_dialects::adt_dtree::{self, CompiledMatch, Matrix, ValueType};

pub fn emit<'c>(context: &'c Context, program: &TypedProgram) -> Result<Module<'c>, String> {
    let location = Location::unknown(context);
    let module = Module::new(location);

    let mut emitter = Emitter {
        context,
        program,
        lift_queue: Vec::new(),
        lifted_done: HashSet::new(),
    };

    // @main() -> lowered(τ): top-level decls behave as a let-chain,
    // then the `main` closure is applied to unit. τ = int for batch
    // compilation; the REPL path allows any concrete τ (D-043).
    {
        let result_ty = program.main_result.clone().unwrap_or(Ty::Int);
        let region = Region::new();
        let entry = region.append_block(Block::new(&[]));
        let mut fcx = FnCtx { region: &region, block: entry, env: HashMap::new() };

        for (recursive, bindings) in &program.decls {
            emitter.emit_binding_group(&mut fcx, *recursive, bindings)?;
        }
        let main_closure = fcx
            .env
            .get("main")
            .copied()
            .ok_or_else(|| "no main closure after decls".to_string())?;
        let unit = emitter.emit_unit(&fcx)?;
        let result = emitter.emit_apply_raw(
            &fcx,
            main_closure,
            unit,
            &Ty::Unit,
            &result_ty,
        )?;
        fcx.block.append_operation(
            OperationBuilder::new("func.return", location)
                .add_operands(&[result])
                .build()
                .map_err(|e| e.to_string())?,
        );

        let mlir_result = emitter.mlir_type(&result_ty)?;
        let function = melior::dialect::func::func(
            context,
            StringAttribute::new(context, "main"),
            TypeAttribute::new(FunctionType::new(context, &[], &[mlir_result]).into()),
            region,
            &[(
                Identifier::new(context, "llvm.emit_c_interface"),
                Attribute::unit(context),
            )],
            location,
        );
        module.body().append_operation(function);
    }

    // Drain the lift queue (lifting may enqueue more lifts).
    while let Some(job) = emitter.lift_queue.pop() {
        emitter.emit_lifted(&module, job)?;
    }

    if !module.as_operation().verify() {
        return Err(format!(
            "emitted module failed MLIR verification:\n{}",
            module.as_operation()
        ));
    }
    Ok(module)
}

/// One lambda to lift. For `let rec` groups every member carries the
/// whole group's info so its prologue can re-make all of them.
struct LiftJob {
    symbol: String,
    captures: Vec<(String, Ty)>,
    param: Option<String>,
    param_ty: Ty,
    result_ty: Ty,
    body: TExpr,
    /// (name, lifted symbol, fn type) for every member of the rec
    /// group, when this lambda came from one.
    rec_group: Vec<(String, String, Ty)>,
}

struct Emitter<'c, 'p> {
    context: &'c Context,
    program: &'p TypedProgram,
    lift_queue: Vec<LiftJob>,
    lifted_done: HashSet<String>,
}

struct FnCtx<'c, 'r> {
    region: &'r Region<'c>,
    block: BlockRef<'c, 'r>,
    env: HashMap<String, Value<'c, 'r>>,
}

impl<'c, 'p> Emitter<'c, 'p> {
    fn loc(&self) -> Location<'c> {
        Location::unknown(self.context)
    }

    // ---- types ----

    fn spell(&self, ty: &Ty) -> Result<String, String> {
        Ok(match ty {
            Ty::Unit => "!frk_adt.product<[]>".to_string(),
            Ty::Bool => "i1".to_string(),
            Ty::Int => "i64".to_string(),
            Ty::Tuple(items) => {
                let parts: Vec<String> =
                    items.iter().map(|t| self.spell(t)).collect::<Result<_, _>>()?;
                format!("!frk_adt.product<[{}]>", parts.join(", "))
            }
            Ty::Adt(name) => {
                let info = self
                    .program
                    .adts
                    .get(name)
                    .ok_or_else(|| format!("unknown adt {name}"))?;
                let variants: Vec<String> = info
                    .ctors
                    .iter()
                    .map(|(_, payload)| {
                        let fields: Vec<String> =
                            payload.iter().map(|t| self.spell(t)).collect::<Result<_, _>>()?;
                        Ok(format!("[{}]", fields.join(", ")))
                    })
                    .collect::<Result<_, String>>()?;
                format!("!frk_adt.sum<[{}]>", variants.join(", "))
            }
            Ty::Fun(a, b) => {
                format!("!frk_closure.fn<[{}], [{}]>", self.spell(a)?, self.spell(b)?)
            }
            Ty::Var(vid) => return Err(format!("unzonked type variable 't{}", vid.0)),
        })
    }

    fn mlir_type(&self, ty: &Ty) -> Result<Type<'c>, String> {
        let spelling = self.spell(ty)?;
        Type::parse(self.context, &spelling)
            .ok_or_else(|| format!("unparsable type {spelling}"))
    }

    // ---- small op helpers (all insert into fcx.block) ----

    fn op_result<'r>(
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

    fn const_i64<'r>(&self, fcx: &FnCtx<'c, 'r>, value: i64) -> Result<Value<'c, 'r>, String> {
        let i64_type: Type = IntegerType::new(self.context, 64).into();
        self.op_result(
            fcx.block,
            melior::dialect::arith::constant(
                self.context,
                IntegerAttribute::new(i64_type, value).into(),
                self.loc(),
            ),
        )
    }

    fn const_bool<'r>(&self, fcx: &FnCtx<'c, 'r>, value: bool) -> Result<Value<'c, 'r>, String> {
        let i1_type: Type = IntegerType::new(self.context, 1).into();
        self.op_result(
            fcx.block,
            melior::dialect::arith::constant(
                self.context,
                IntegerAttribute::new(i1_type, value as i64).into(),
                self.loc(),
            ),
        )
    }

    fn emit_unit<'r>(&self, fcx: &FnCtx<'c, 'r>) -> Result<Value<'c, 'r>, String> {
        let empty = self.mlir_type(&Ty::Unit)?;
        self.op_result(
            fcx.block,
            OperationBuilder::new("frk_adt.product_new", self.loc())
                .add_results(&[empty])
                .build()
                .map_err(|e| e.to_string())?,
        )
    }

    /// Packs values into a product via new + snoc chain.
    fn emit_pack<'r>(
        &self,
        fcx: &FnCtx<'c, 'r>,
        values: &[(Value<'c, 'r>, Ty)],
    ) -> Result<Value<'c, 'r>, String> {
        let mut acc = self.emit_unit(fcx)?;
        let mut types: Vec<Ty> = Vec::new();
        for (value, ty) in values {
            types.push(ty.clone());
            let result_ty = self.mlir_type(&Ty::Tuple(types.clone()))?;
            acc = self.op_result(
                fcx.block,
                OperationBuilder::new("frk_adt.product_snoc", self.loc())
                    .add_operands(&[acc, *value])
                    .add_results(&[result_ty])
                    .build()
                    .map_err(|e| e.to_string())?,
            )?;
        }
        Ok(acc)
    }

    fn emit_apply_raw<'r>(
        &self,
        fcx: &FnCtx<'c, 'r>,
        closure: Value<'c, 'r>,
        arg: Value<'c, 'r>,
        arg_ty: &Ty,
        result_ty: &Ty,
    ) -> Result<Value<'c, 'r>, String> {
        let pack = self.emit_pack(fcx, &[(arg, arg_ty.clone())])?;
        let result = self.mlir_type(result_ty)?;
        self.op_result(
            fcx.block,
            OperationBuilder::new("frk_closure.apply", self.loc())
                .add_operands(&[closure, pack])
                .add_results(&[result])
                .build()
                .map_err(|e| e.to_string())?,
        )
    }

    // ---- expressions ----

    fn emit_expr<'r>(
        &mut self,
        fcx: &mut FnCtx<'c, 'r>,
        expr: &TExpr,
    ) -> Result<Value<'c, 'r>, String> {
        match &expr.kind {
            TKind::Unit => self.emit_unit(fcx),
            TKind::Int(value) => self.const_i64(fcx, *value),
            TKind::Bool(value) => self.const_bool(fcx, *value),
            TKind::Var(name) => fcx
                .env
                .get(name)
                .copied()
                .ok_or_else(|| format!("unbound at emission: {name}")),
            TKind::Neg(inner) => {
                let value = self.emit_expr(fcx, inner)?;
                let zero = self.const_i64(fcx, 0)?;
                let i64_type: Type = IntegerType::new(self.context, 64).into();
                self.op_result(
                    fcx.block,
                    OperationBuilder::new("arith.subi", self.loc())
                        .add_operands(&[zero, value])
                        .add_results(&[i64_type])
                        .build()
                        .map_err(|e| e.to_string())?,
                )
            }
            TKind::Bin { op, lhs, rhs } => {
                use crate::ast::BinOp::*;
                let lhs_ty = lhs.ty.clone();
                let lhs = self.emit_expr(fcx, lhs)?;
                let rhs = self.emit_expr(fcx, rhs)?;
                let i64_type: Type = IntegerType::new(self.context, 64).into();
                let i1_type: Type = IntegerType::new(self.context, 1).into();
                let arith = |name: &str, result: Type<'c>| {
                    OperationBuilder::new(name, self.loc())
                        .add_operands(&[lhs, rhs])
                        .add_results(&[result])
                        .build()
                        .map_err(|e| e.to_string())
                };
                let cmpi = |predicate: i64| {
                    OperationBuilder::new("arith.cmpi", self.loc())
                        .add_attributes(&[(
                            Identifier::new(self.context, "predicate"),
                            IntegerAttribute::new(i64_type, predicate).into(),
                        )])
                        .add_operands(&[lhs, rhs])
                        .add_results(&[i1_type])
                        .build()
                        .map_err(|e| e.to_string())
                };
                match op {
                    Add => self.op_result(fcx.block, arith("arith.addi", i64_type)?),
                    Sub => self.op_result(fcx.block, arith("arith.subi", i64_type)?),
                    Mul => self.op_result(fcx.block, arith("arith.muli", i64_type)?),
                    Div => self.op_result(fcx.block, arith("arith.divsi", i64_type)?),
                    Eq | Ne => {
                        // eq=0, ne=1; i1 operands compare fine too.
                        let predicate = if matches!(op, Eq) { 0 } else { 1 };
                        let _ = lhs_ty;
                        self.op_result(fcx.block, cmpi(predicate)?)
                    }
                    Lt => self.op_result(fcx.block, cmpi(2)?),
                    Le => self.op_result(fcx.block, cmpi(3)?),
                    Gt => self.op_result(fcx.block, cmpi(4)?),
                    Ge => self.op_result(fcx.block, cmpi(5)?),
                    AndAlso => {
                        // Pure subset: a && b ≡ select(a, b, false).
                        let f = self.const_bool(fcx, false)?;
                        self.op_result(
                            fcx.block,
                            OperationBuilder::new("arith.select", self.loc())
                                .add_operands(&[lhs, rhs, f])
                                .add_results(&[i1_type])
                                .build()
                                .map_err(|e| e.to_string())?,
                        )
                    }
                    OrElse => {
                        let t = self.const_bool(fcx, true)?;
                        self.op_result(
                            fcx.block,
                            OperationBuilder::new("arith.select", self.loc())
                                .add_operands(&[lhs, t, rhs])
                                .add_results(&[i1_type])
                                .build()
                                .map_err(|e| e.to_string())?,
                        )
                    }
                }
            }
            TKind::Tuple(items) => {
                let mut values = Vec::new();
                for item in items {
                    let value = self.emit_expr(fcx, item)?;
                    values.push((value, item.ty.clone()));
                }
                self.emit_pack(fcx, &values)
            }
            TKind::MakeCtor { adt, tag, payload } => {
                let mut values = Vec::new();
                for field in payload {
                    let value = self.emit_expr(fcx, field)?;
                    values.push((value, field.ty.clone()));
                }
                let pack = self.emit_pack(fcx, &values)?;
                let sum_ty = self.mlir_type(&Ty::Adt(adt.clone()))?;
                let i64_type: Type = IntegerType::new(self.context, 64).into();
                self.op_result(
                    fcx.block,
                    OperationBuilder::new("frk_adt.make_sum", self.loc())
                        .add_attributes(&[(
                            Identifier::new(self.context, "variant"),
                            IntegerAttribute::new(i64_type, *tag as i64).into(),
                        )])
                        .add_operands(&[pack])
                        .add_results(&[sum_ty])
                        .build()
                        .map_err(|e| e.to_string())?,
                )
            }
            TKind::If { cond, then, els } => {
                let cond_value = self.emit_expr(fcx, cond)?;
                let result_ty = self.mlir_type(&expr.ty)?;

                let then_block = fcx.region.append_block(Block::new(&[]));
                let else_block = fcx.region.append_block(Block::new(&[]));
                let merge_block = fcx
                    .region
                    .append_block(Block::new(&[(result_ty, self.loc())]));

                fcx.block.append_operation(self.cond_br(
                    cond_value,
                    then_block,
                    else_block,
                )?);

                fcx.block = then_block;
                let then_value = self.emit_expr(fcx, then)?;
                fcx.block
                    .append_operation(melior::dialect::cf::br(&merge_block, &[then_value], self.loc()));

                fcx.block = else_block;
                let else_value = self.emit_expr(fcx, els)?;
                fcx.block
                    .append_operation(melior::dialect::cf::br(&merge_block, &[else_value], self.loc()));

                fcx.block = merge_block;
                let raw = merge_block
                    .argument(0)
                    .map_err(|e| e.to_string())?
                    .to_raw();
                Ok(unsafe { Value::from_raw(raw) })
            }
            TKind::Fun { .. } => self.emit_closure(fcx, expr, &[]),
            TKind::App { func, arg } => {
                let closure = self.emit_expr(fcx, func)?;
                let arg_value = self.emit_expr(fcx, arg)?;
                let result_ty = expr.ty.clone();
                self.emit_apply_raw(fcx, closure, arg_value, &arg.ty, &result_ty)
            }
            TKind::Let { rec, bindings, body } => {
                self.emit_binding_group(fcx, *rec, bindings)?;
                self.emit_expr(fcx, body)
            }
            TKind::Match { scrutinee, arms } => self.emit_match(fcx, expr, scrutinee, arms),
        }
    }

    // ---- closures & lifting ----

    /// Emits a closure value for a Fun expression. `rec_group` is the
    /// group metadata when this lambda is a `let rec` right-hand side.
    fn emit_closure<'r>(
        &mut self,
        fcx: &mut FnCtx<'c, 'r>,
        fun: &TExpr,
        rec_group: &[(String, String, Ty)],
    ) -> Result<Value<'c, 'r>, String> {
        let TKind::Fun { id, param, param_ty, body } = &fun.kind else {
            return Err("emit_closure on a non-fun".to_string());
        };
        let Ty::Fun(_, result_ty) = &fun.ty else {
            return Err("fun with a non-arrow type".to_string());
        };

        let symbol = format!("__ml_fn_{id}");
        let captures = self.captures_of(fun, rec_group)?;

        if !self.lifted_done.contains(&symbol) {
            self.lifted_done.insert(symbol.clone());
            self.lift_queue.push(LiftJob {
                symbol: symbol.clone(),
                captures: captures.clone(),
                param: param.clone(),
                param_ty: param_ty.clone(),
                result_ty: (**result_ty).clone(),
                body: (**body).clone(),
                rec_group: rec_group.to_vec(),
            });
        }

        let mut capture_values = Vec::new();
        for (name, ty) in &captures {
            let value = fcx
                .env
                .get(name)
                .copied()
                .ok_or_else(|| format!("capture {name} unbound at emission"))?;
            capture_values.push((value, ty.clone()));
        }
        let env_pack = self.emit_pack(fcx, &capture_values)?;
        let fn_ty = self.mlir_type(&fun.ty)?;
        self.op_result(
            fcx.block,
            OperationBuilder::new("frk_closure.make", self.loc())
                .add_attributes(&[(
                    Identifier::new(self.context, "callee"),
                    FlatSymbolRefAttribute::new(self.context, &symbol).into(),
                )])
                .add_operands(&[env_pack])
                .add_results(&[fn_ty])
                .build()
                .map_err(|e| e.to_string())?,
        )
    }

    /// Free variables of a lambda (sorted for determinism), excluding
    /// its own param and the rec-group names it can re-make.
    fn captures_of(
        &self,
        fun: &TExpr,
        rec_group: &[(String, String, Ty)],
    ) -> Result<Vec<(String, Ty)>, String> {
        let TKind::Fun { param, body, .. } = &fun.kind else {
            return Err("captures_of on a non-fun".to_string());
        };
        let mut bound: HashSet<String> = rec_group.iter().map(|(n, _, _)| n.clone()).collect();
        if let Some(name) = param {
            bound.insert(name.clone());
        }
        let mut free = BTreeMap::new();
        free_vars(body, &bound, &mut free);
        Ok(free.into_iter().collect())
    }

    fn emit_binding_group<'r>(
        &mut self,
        fcx: &mut FnCtx<'c, 'r>,
        recursive: bool,
        bindings: &[TBinding],
    ) -> Result<(), String> {
        if !recursive {
            for binding in bindings {
                if binding.dead {
                    continue;
                }
                let value = self.emit_expr(fcx, &binding.expr)?;
                fcx.env.insert(binding.name.clone(), value);
            }
            return Ok(());
        }

        // let rec group: shared metadata so every member can re-make all.
        let group: Vec<(String, String, Ty)> = bindings
            .iter()
            .map(|binding| {
                let TKind::Fun { id, .. } = &binding.expr.kind else {
                    return Err(format!("`let rec {}` must bind a function", binding.name));
                };
                Ok((
                    binding.name.clone(),
                    format!("__ml_fn_{id}"),
                    binding.expr.ty.clone(),
                ))
            })
            .collect::<Result<_, String>>()?;

        for binding in bindings {
            let value = self.emit_closure(fcx, &binding.expr, &group)?;
            fcx.env.insert(binding.name.clone(), value);
        }
        Ok(())
    }

    fn emit_lifted(&mut self, module: &Module<'c>, job: LiftJob) -> Result<(), String> {
        let mut input_tys = Vec::new();
        for (_, ty) in &job.captures {
            input_tys.push(self.mlir_type(ty)?);
        }
        input_tys.push(self.mlir_type(&job.param_ty)?);
        let result_ty = self.mlir_type(&job.result_ty)?;

        let region = Region::new();
        let entry = region.append_block(Block::new(
            &input_tys
                .iter()
                .map(|ty| (*ty, self.loc()))
                .collect::<Vec<_>>(),
        ));

        let mut fcx = FnCtx { region: &region, block: entry, env: HashMap::new() };
        for (index, (name, _)) in job.captures.iter().enumerate() {
            let raw = entry.argument(index).map_err(|e| e.to_string())?.to_raw();
            fcx.env
                .insert(name.clone(), unsafe { Value::from_raw(raw) });
        }
        if let Some(name) = &job.param {
            let raw = entry
                .argument(job.captures.len())
                .map_err(|e| e.to_string())?
                .to_raw();
            fcx.env
                .insert(name.clone(), unsafe { Value::from_raw(raw) });
        }

        // Rec prologue: re-make every group member from own captures.
        if !job.rec_group.is_empty() {
            let mut capture_values = Vec::new();
            for (index, (name, ty)) in job.captures.iter().enumerate() {
                let raw = entry.argument(index).map_err(|e| e.to_string())?.to_raw();
                let value = unsafe { Value::from_raw(raw) };
                capture_values.push((value, ty.clone()));
                let _ = name;
            }
            for (name, symbol, fn_ty) in &job.rec_group {
                let env_pack = self.emit_pack(&fcx, &capture_values)?;
                let fn_mlir_ty = self.mlir_type(fn_ty)?;
                let closure = self.op_result(
                    fcx.block,
                    OperationBuilder::new("frk_closure.make", self.loc())
                        .add_attributes(&[(
                            Identifier::new(self.context, "callee"),
                            FlatSymbolRefAttribute::new(self.context, symbol).into(),
                        )])
                        .add_operands(&[env_pack])
                        .add_results(&[fn_mlir_ty])
                        .build()
                        .map_err(|e| e.to_string())?,
                )?;
                fcx.env.insert(name.clone(), closure);
            }
        }

        let result = self.emit_expr(&mut fcx, &job.body)?;
        fcx.block.append_operation(
            OperationBuilder::new("func.return", self.loc())
                .add_operands(&[result])
                .build()
                .map_err(|e| e.to_string())?,
        );

        let function = melior::dialect::func::func(
            self.context,
            StringAttribute::new(self.context, &job.symbol),
            TypeAttribute::new(FunctionType::new(self.context, &input_tys, &[result_ty]).into()),
            region,
            &[],
            self.loc(),
        );
        module.body().append_operation(function);
        Ok(())
    }

    // ---- match ----

    fn emit_match<'r>(
        &mut self,
        fcx: &mut FnCtx<'c, 'r>,
        whole: &TExpr,
        scrutinee: &TExpr,
        arms: &[(Pattern, TExpr)],
    ) -> Result<Value<'c, 'r>, String> {
        let scrutinee_value = self.emit_expr(fcx, scrutinee)?;
        let scrutinee_ty = scrutinee.ty.clone();

        let dtree_patterns: Vec<adt_dtree::Pattern> = arms
            .iter()
            .map(|(pattern, _)| self.to_dtree_pattern(pattern, &scrutinee_ty))
            .collect::<Result<_, _>>()?;
        let matrix = Matrix::over_scrutinee(self.value_type(&scrutinee_ty)?, dtree_patterns);
        let CompiledMatch { tree, diagnostics } =
            adt_dtree::compile(matrix).map_err(|e| e.to_string())?;
        if let Some(witness) = &diagnostics.inexhaustive {
            return Err(format!("non-exhaustive match: missing case where {witness}"));
        }
        if !diagnostics.redundant_arms.is_empty() {
            return Err(format!(
                "redundant match arm(s): {:?} (v0 treats redundancy as an error, D-038)",
                diagnostics.redundant_arms
            ));
        }

        let result_ty = self.mlir_type(&whole.ty)?;
        let merge_block = fcx
            .region
            .append_block(Block::new(&[(result_ty, self.loc())]));

        // The promoted dispatch emitter (M6): occurrence typing walks
        // the kernel types; we only supply arm bodies.
        let scrutinee_mlir_ty = self.mlir_type(&scrutinee_ty)?;
        let region = fcx.region;
        let base_env = fcx.env.clone();
        let entry = fcx.block;
        frk_dialects::dtree_emit::emit_dispatch(
            self.context,
            region,
            entry,
            &tree,
            scrutinee_value,
            scrutinee_mlir_ty,
            merge_block,
            &mut |arm_entry, arm_index, bindings| {
                let mut env = base_env.clone();
                for (name, value) in bindings {
                    env.insert(name.clone(), *value);
                }
                let mut arm_fcx = FnCtx { region, block: arm_entry, env };
                let value = self.emit_expr(&mut arm_fcx, &arms[arm_index].1)?;
                Ok((value, arm_fcx.block))
            },
        )?;

        fcx.block = merge_block;
        let raw = merge_block
            .argument(0)
            .map_err(|e| e.to_string())?
            .to_raw();
        Ok(unsafe { Value::from_raw(raw) })
    }


    fn to_dtree_pattern(
        &self,
        pattern: &Pattern,
        ty: &Ty,
    ) -> Result<adt_dtree::Pattern, String> {
        Ok(match (pattern, ty) {
            (Pattern::Wild, _) | (Pattern::Unit, _) => adt_dtree::Pattern::Wildcard,
            (Pattern::Var(name), _) => adt_dtree::Pattern::Binding(name.clone()),
            (Pattern::Int(value), _) => adt_dtree::Pattern::Int(*value),
            (Pattern::Bool(value), _) => adt_dtree::Pattern::Variant {
                tag: *value as usize,
                fields: Vec::new(),
            },
            (Pattern::Tuple(items), Ty::Tuple(item_tys)) => adt_dtree::Pattern::Product(
                items
                    .iter()
                    .zip(item_tys)
                    .map(|(item, item_ty)| self.to_dtree_pattern(item, item_ty))
                    .collect::<Result<_, _>>()?,
            ),
            (Pattern::Tuple(_), other) => {
                return Err(format!("tuple pattern against {other}"));
            }
            (Pattern::Ctor { name, arg }, _) => {
                let info = self
                    .program
                    .ctors
                    .get(name)
                    .ok_or_else(|| format!("unknown constructor {name}"))?;
                let fields = match (info.payload.len(), arg) {
                    (0, None) => Vec::new(),
                    (1, Some(inner)) => vec![self.to_dtree_pattern(inner, &info.payload[0])?],
                    (n, Some(inner)) => match inner.as_ref() {
                        Pattern::Tuple(items) if items.len() == n => items
                            .iter()
                            .zip(&info.payload)
                            .map(|(item, field_ty)| self.to_dtree_pattern(item, field_ty))
                            .collect::<Result<_, _>>()?,
                        Pattern::Wild => vec![adt_dtree::Pattern::Wildcard; n],
                        _ => return Err(format!("{name} pattern payload shape")),
                    },
                    _ => return Err(format!("{name} pattern arity")),
                };
                adt_dtree::Pattern::Variant { tag: info.tag, fields }
            }
        })
    }

    fn value_type(&self, ty: &Ty) -> Result<ValueType, String> {
        Ok(match ty {
            Ty::Int => ValueType::Int,
            // Bool = a two-variant "sum" for finite-signature dispatch.
            Ty::Bool => ValueType::Sum(vec![vec![], vec![]]),
            // Only irrefutable patterns can sit on these columns; the
            // stand-in is never specialized.
            Ty::Unit | Ty::Fun(..) => ValueType::Int,
            Ty::Tuple(items) => ValueType::Product(
                items.iter().map(|t| self.value_type(t)).collect::<Result<_, _>>()?,
            ),
            Ty::Adt(name) => {
                let info = self
                    .program
                    .adts
                    .get(name)
                    .ok_or_else(|| format!("unknown adt {name}"))?;
                ValueType::Sum(
                    info.ctors
                        .iter()
                        .map(|(_, payload)| {
                            payload.iter().map(|t| self.value_type(t)).collect::<Result<_, _>>()
                        })
                        .collect::<Result<_, _>>()?,
                )
            }
            Ty::Var(vid) => return Err(format!("unzonked type variable 't{}", vid.0)),
        })
    }

    // ---- generic branch builders (MLIR-22 attribute names) ----

    fn cond_br<'r>(
        &self,
        condition: Value<'c, 'r>,
        on_true: BlockRef<'c, 'r>,
        on_false: BlockRef<'c, 'r>,
    ) -> Result<melior::ir::Operation<'c>, String> {
        OperationBuilder::new("cf.cond_br", self.loc())
            .add_attributes(&[(
                Identifier::new(self.context, "operandSegmentSizes"),
                DenseI32ArrayAttribute::new(self.context, &[1, 0, 0]).into(),
            )])
            .add_operands(&[condition])
            .add_successors(&[&on_true, &on_false])
            .build()
            .map_err(|e| e.to_string())
    }

}

/// Free variables of a typed expression, accumulated with their types.
fn free_vars(expr: &TExpr, bound: &HashSet<String>, out: &mut BTreeMap<String, Ty>) {
    match &expr.kind {
        TKind::Unit | TKind::Int(_) | TKind::Bool(_) => {}
        TKind::Var(name) => {
            if !bound.contains(name) && !out.contains_key(name) {
                out.insert(name.clone(), expr.ty.clone());
            }
        }
        TKind::MakeCtor { payload, .. } => {
            for field in payload {
                free_vars(field, bound, out);
            }
        }
        TKind::Tuple(items) => {
            for item in items {
                free_vars(item, bound, out);
            }
        }
        TKind::Neg(inner) => free_vars(inner, bound, out),
        TKind::Bin { lhs, rhs, .. } => {
            free_vars(lhs, bound, out);
            free_vars(rhs, bound, out);
        }
        TKind::If { cond, then, els } => {
            free_vars(cond, bound, out);
            free_vars(then, bound, out);
            free_vars(els, bound, out);
        }
        TKind::Fun { param, body, .. } => {
            let mut inner = bound.clone();
            if let Some(name) = param {
                inner.insert(name.clone());
            }
            free_vars(body, &inner, out);
        }
        TKind::App { func, arg } => {
            free_vars(func, bound, out);
            free_vars(arg, bound, out);
        }
        TKind::Let { rec, bindings, body } => {
            if *rec {
                // Rec: every rhs sees the whole group.
                let mut inner = bound.clone();
                for binding in bindings {
                    inner.insert(binding.name.clone());
                }
                for binding in bindings {
                    if !binding.dead {
                        free_vars(&binding.expr, &inner, out);
                    }
                }
                free_vars(body, &inner, out);
            } else {
                // Non-rec: each rhs sees only PRIOR names — a shadowing
                // rhs still refers to (and captures) the outer binding.
                let mut inner = bound.clone();
                for binding in bindings {
                    if !binding.dead {
                        free_vars(&binding.expr, &inner, out);
                    }
                    inner.insert(binding.name.clone());
                }
                free_vars(body, &inner, out);
            }
        }
        TKind::Match { scrutinee, arms } => {
            free_vars(scrutinee, bound, out);
            for (pattern, body) in arms {
                let mut inner = bound.clone();
                pattern_names(pattern, &mut inner);
                free_vars(body, &inner, out);
            }
        }
    }
}

fn pattern_names(pattern: &Pattern, out: &mut HashSet<String>) {
    match pattern {
        Pattern::Wild | Pattern::Int(_) | Pattern::Bool(_) | Pattern::Unit => {}
        Pattern::Var(name) => {
            out.insert(name.clone());
        }
        Pattern::Tuple(items) => {
            for item in items {
                pattern_names(item, out);
            }
        }
        Pattern::Ctor { arg, .. } => {
            if let Some(inner) = arg {
                pattern_names(inner, out);
            }
        }
    }
}
