//! femto_lua emission (M11 bar 3; D-052/D-054/D-056): the whole
//! value world is `!frk_dyn.dyn` (fat values, D-051); every local is
//! a `frk_mem.box<dyn>` (Lua locals are mutable; upvalue capture by
//! reference falls out of box identity); functions lambda-lift to
//! `@__lua_fn_N(_G: dyn, capture-boxes..., params: dyn...) -> dyn`.
//!
//! The Lua PROTOCOLS are synthesized IR helpers, not kernel ops or rt
//! callbacks (D-056.2): truthiness, tostring, print, equality, concat
//! coercion, length, and the __index metatable walk (table AND
//! function forms — the function form dispatches through
//! frk_closure.apply) are ordinary functions emitted once per module,
//! running identically on the interpreter, the JIT, and every AOT
//! triple.
//!
//! Lua-vs-C semantics handled here: % is FLOOR-mod (a − ⌊a/b⌋·b),
//! built from trunc + fix-up (fptosi/sitofp + select) — no math
//! dialect, no new pipeline passes; and/or are VALUE-returning
//! short-circuits through truthiness; # dispatches str/table.
//!
//! Spans thread token offsets → FileLineColLoc (§6.5 discipline).

use std::collections::{BTreeSet, HashMap, HashSet};

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

use super::ast::{BinOp, Block as LBlock, Expr, Field, Stat, UnOp};

const TAG_NIL: i64 = 0;
const TAG_BOOL: i64 = 1;
const TAG_NUM: i64 = 2;
const TAG_STR: i64 = 3;
const TAG_FUN: i64 = 5;

type Result<T> = std::result::Result<T, String>;

pub fn emit<'c>(
    context: &'c Context,
    file: &str,
    source: &str,
    chunk: &LBlock,
) -> Result<Module<'c>> {
    // The seed module (M17, D-062): the plain-dyn protocol helpers are
    // kernel IR in intrinsics.mlir; the emitter appends around them.
    let module = crate::intrinsics::seed_module(
        context,
        "femto_lua",
        include_str!("intrinsics.mlir"),
    )?;
    let mut line_starts = vec![0usize];
    for (offset, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            line_starts.push(offset + 1);
        }
    }
    let mut emitter = Emitter {
        context,
        file: file.to_string(),
        line_starts,
        lift_queue: Vec::new(),
        next_fn: 0,
        // D-084.4 module license: only chunks mentioning `coroutine`
        // transform (per-call licensing is unsound — mutable _G, no
        // static call graph). Unlicensed modules emit byte-identical
        // IR to pre-M35.
        licensed: source.contains("coroutine"),
    };

    emitter.emit_main(&module, chunk)?;
    while let Some(job) = emitter.lift_queue.pop() {
        emitter.emit_lifted(&module, job)?;
    }

    if !module.as_operation().verify() {
        return Err(format!(
            "emitted lua module failed MLIR verification:\n{}",
            module.as_operation()
        ));
    }
    Ok(module)
}

struct LiftJob {
    symbol: String,
    captures: Vec<String>,
    params: Vec<String>,
    is_vararg: bool,
    body: LBlock,
}

struct Emitter<'c> {
    context: &'c Context,
    file: String,
    line_starts: Vec<usize>,
    lift_queue: Vec<LiftJob>,
    next_fn: usize,
    /// D-084.4: the resumable-frame transform's module license.
    licensed: bool,
}

/// Per-function suspension context (D-084; licensed lifted fns only):
/// each guarded call site becomes a resume STATE — the frame closure
/// re-enters this function with env[0] = the state id and the
/// continuation block's live-ins reloaded from the frame env.
struct SuspendCtx<'c, 'r> {
    symbol: String,
    stubs: Vec<StateStub<'c, 'r>>,
}

/// One resume state: the continuation block (whose args carry the
/// call result + every live-in) and the frame-field spec.
struct StateStub<'c, 'r> {
    state: i64,
    target: BlockRef<'c, 'r>,
    /// Live-in local names, in B_k arg order (sorted for determinism).
    names: Vec<String>,
    temps: usize,
    has_varargs: bool,
}

struct Fcx<'c, 'r> {
    region: &'r Region<'c>,
    block: BlockRef<'c, 'r>,
    /// name → its box<dyn> value.
    env: HashMap<String, Value<'c, 'r>>,
    /// The _G table (a dyn), threaded everywhere.
    globals: Value<'c, 'r>,
    terminated: bool,
    /// Enclosing loop exits, innermost last (`break`, D-058).
    break_targets: Vec<(BlockRef<'c, 'r>, Option<(Vec<String>, bool)>)>,
    /// The vararg tail (D-068): a PRIVATE arr copied from
    /// pack[nparams..] at the prologue, before the D-067 dispose.
    /// None in non-vararg functions and in main.
    varargs: Option<Value<'c, 'r>>,
    /// D-084: Some inside a licensed lifted function — the guard
    /// machinery lives here. None in main and unlicensed modules.
    suspend: Option<SuspendCtx<'c, 'r>>,
    /// SSA temporaries live across a potentially-suspending call
    /// (explist prefixes, binary-left values): pushed by the holder,
    /// REBOUND by the guard, read back by the holder (D-084.4's
    /// sibling-temps residue).
    live_temps: Vec<Value<'c, 'r>>,
    /// The incoming pack block-arg of the lifted fn (the resume pack
    /// on re-entry) — the stubs' walk argument.
    entry_pack: Option<Value<'c, 'r>>,
    /// Lexical scoping via SHADOW FRAMES (M35): boxes are stable heap
    /// pointers, so a name's VALUE may be legitimately rebound by a
    /// guard (same box, new SSA name) — wholesale env restore would
    /// resurrect pre-guard SSA (a dominance violation on resume
    /// paths). Only `local` bindings genuinely change a name's box;
    /// they record their shadowed predecessor here and scope exit
    /// undoes exactly those.
    scope_shadows: Vec<Vec<(String, Option<Value<'c, 'r>>)>>,
}

impl<'c> Emitter<'c> {
    // ---- types & locations ----

    fn envref_ty(&self) -> Type<'c> {
        Type::parse(self.context, "!frk_closure.envref").expect("envref type")
    }

    /// The env product type a lifted function's env_loads carry
    /// (D-063): [_G, capture boxes...].
    fn env_product_ty(&self, capture_count: usize) -> Type<'c> {
        let mut fields = vec!["!frk_dyn.dyn".to_string()];
        fields.extend(
            std::iter::repeat_n("!frk_mem.box<!frk_dyn.dyn>".to_string(), capture_count),
        );
        Type::parse(
            self.context,
            &format!("!frk_adt.product<[{}]>", fields.join(", ")),
        )
        .expect("env product type")
    }

    /// closure.env_load %env {index, env = <product>} -> T (D-063).
    fn env_load<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        env: Value<'c, 'r>,
        index: i64,
        env_ty: Type<'c>,
        result: Type<'c>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        self.build(
            block,
            "frk_closure.env_load",
            &[env],
            &[result],
            &[
                ("index", IntegerAttribute::new(self.i64_ty(), index).into()),
                ("env", TypeAttribute::new(env_ty).into()),
            ],
            location,
        )
    }

    /// The LICENSED env spelling (D-084.1): [state i64, chain-next
    /// dyn, _G dyn, live boxes…, varargs?, temps…]. Fresh closures are
    /// state 0 with live boxes = the captures; frame closures are
    /// state k with live boxes = every local in scope at the site.
    fn licensed_env_spelling(
        &self,
        boxes: usize,
        has_varargs: bool,
        temps: usize,
    ) -> String {
        let mut fields =
            vec!["i64".to_string(), "!frk_dyn.dyn".to_string(), "!frk_dyn.dyn".to_string()];
        fields.extend(std::iter::repeat_n(
            "!frk_mem.box<!frk_dyn.dyn>".to_string(),
            boxes,
        ));
        if has_varargs {
            fields.push("!frk_mem.arr<!frk_dyn.dyn>".to_string());
        }
        fields.extend(std::iter::repeat_n("!frk_dyn.dyn".to_string(), temps));
        format!("!frk_adt.product<[{}]>", fields.join(", "))
    }

    fn licensed_env_ty(&self, boxes: usize, has_varargs: bool, temps: usize) -> Type<'c> {
        Type::parse(self.context, &self.licensed_env_spelling(boxes, has_varargs, temps))
            .expect("licensed env type")
    }

    /// cf.cond_br where BOTH successors take (identical) arguments —
    /// the live-threaded loop-head shape (D-084).
    #[allow(clippy::too_many_arguments)]
    fn cond_br_two_args<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        condition: Value<'c, 'r>,
        on_true: BlockRef<'c, 'r>,
        true_args: &[Value<'c, 'r>],
        on_false: BlockRef<'c, 'r>,
        false_args: &[Value<'c, 'r>],
        location: Location<'c>,
    ) -> Result<()> {
        let mut operands = vec![condition];
        operands.extend_from_slice(true_args);
        operands.extend_from_slice(false_args);
        block.append_operation(
            OperationBuilder::new("cf.cond_br", location)
                .add_attributes(&[(
                    Identifier::new(self.context, "operandSegmentSizes"),
                    DenseI32ArrayAttribute::new(
                        self.context,
                        &[1, true_args.len() as i32, false_args.len() as i32],
                    )
                    .into(),
                )])
                .add_operands(&operands)
                .add_successors(&[&on_true, &on_false])
                .build()
                .map_err(|e| e.to_string())?,
        );
        Ok(())
    }

    /// cf.cond_br whose FALSE successor takes arguments (the guard's
    /// continuation-block shape).
    fn cond_br_false_args<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        condition: Value<'c, 'r>,
        on_true: BlockRef<'c, 'r>,
        on_false: BlockRef<'c, 'r>,
        false_args: &[Value<'c, 'r>],
        location: Location<'c>,
    ) -> Result<()> {
        let mut operands = vec![condition];
        operands.extend_from_slice(false_args);
        block.append_operation(
            OperationBuilder::new("cf.cond_br", location)
                .add_attributes(&[(
                    Identifier::new(self.context, "operandSegmentSizes"),
                    DenseI32ArrayAttribute::new(
                        self.context,
                        &[1, 0, false_args.len() as i32],
                    )
                    .into(),
                )])
                .add_operands(&operands)
                .add_successors(&[&on_true, &on_false])
                .build()
                .map_err(|e| e.to_string())?,
        );
        Ok(())
    }

    /// The guard COLD path (D-084.4): park this frame — env
    /// [state, chain-next (the cell's current head), _G, boxes…,
    /// varargs?, temps…] over the SAME symbol, every snoc carrying
    /// frk.capture (planner-invisible, always-retained); the cell's
    /// head becomes this frame; return the suspended dummy.
    #[allow(clippy::too_many_arguments)]
    fn emit_frame_capture<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        symbol: &str,
        state: i64,
        boxes: &[Value<'c, 'r>],
        globals: Value<'c, 'r>,
        varargs: Option<Value<'c, 'r>>,
        temps: &[Value<'c, 'r>],
        location: Location<'c>,
    ) -> Result<()> {
        let cells = self
            .call(block, "__lua_coro_cells", &[], &[self.pack_ty()], location)?
            .expect("cells");
        let zero = self.const_i64(block, 0, location)?;
        let chain = self.build(
            block,
            "frk_mem.array_get",
            &[cells, zero],
            &[self.dyn_ty()],
            &[],
            location,
        )?;
        let state_value = self.const_i64(block, state, location)?;

        let mut parts: Vec<String> = Vec::new();
        let empty = Type::parse(self.context, "!frk_adt.product<[]>").ok_or("empty")?;
        let mut acc = self.build(block, "frk_adt.product_new", &[], &[empty], &[], location)?;
        let capture_attr: Attribute = Attribute::unit(self.context);
        let mut snoc = |emitter: &Self,
                        acc: Value<'c, 'r>,
                        value: Value<'c, 'r>,
                        spelled: &str,
                        parts: &mut Vec<String>|
         -> Result<Value<'c, 'r>> {
            parts.push(spelled.to_string());
            let ty = Type::parse(
                emitter.context,
                &format!("!frk_adt.product<[{}]>", parts.join(", ")),
            )
            .ok_or("frame product type")?;
            emitter.build(
                block,
                "frk_adt.product_snoc",
                &[acc, value],
                &[ty],
                &[("frk.capture", capture_attr)],
                location,
            )
        };
        acc = snoc(self, acc, state_value, "i64", &mut parts)?;
        acc = snoc(self, acc, chain, "!frk_dyn.dyn", &mut parts)?;
        acc = snoc(self, acc, globals, "!frk_dyn.dyn", &mut parts)?;
        for value in boxes {
            acc = snoc(self, acc, *value, "!frk_mem.box<!frk_dyn.dyn>", &mut parts)?;
        }
        if let Some(va) = varargs {
            acc = snoc(self, acc, va, "!frk_mem.arr<!frk_dyn.dyn>", &mut parts)?;
        }
        for value in temps {
            acc = snoc(self, acc, *value, "!frk_dyn.dyn", &mut parts)?;
        }

        let closure = self.build(
            block,
            "frk_closure.make",
            &[acc],
            &[self.lua_fn_ty()],
            &[("callee", FlatSymbolRefAttribute::new(self.context, symbol).into())],
            location,
        )?;
        let frame = self.wrap(block, TAG_FUN, closure, location)?;
        self.build0(block, "frk_mem.array_set", &[cells, zero, frame], &[], location)?;
        let none = self.const_i64(block, 0, location)?;
        let dummy = self.build(
            block,
            "frk_mem.array_new",
            &[none],
            &[self.pack_ty()],
            &[],
            location,
        )?;
        ret(block, &[dummy], location)?;
        Ok(())
    }

    /// Reads the suspend flag as an i1.
    fn read_susp_flag<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        let cell_ty = Type::parse(self.context, "!frk_mem.box<f64>").ok_or("flag cell")?;
        let cell = self.build(
            block,
            "frk_mem.global_get",
            &[],
            &[cell_ty],
            &[("sym", StringAttribute::new(self.context, "lua_susp").into())],
            location,
        )?;
        let flag = self.build(block, "frk_mem.box_get", &[cell], &[self.f64_ty()], &[], location)?;
        let zero = self.const_f64(block, 0.0, location)?;
        self.cmpf(block, 6, flag, zero, location) // one (ordered !=)
    }

    /// The D-084 guard after a potentially-suspending call: hot path
    /// falls through to the continuation block (result + every
    /// live-in as block args); cold path parks the frame. Returns the
    /// rebound result; REBINDS fcx.env/globals/varargs/live_temps.
    fn guarded_result<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        result: Value<'c, 'r>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        let Some(suspend) = fcx.suspend.as_ref() else {
            return Ok(result);
        };
        let symbol = suspend.symbol.clone();
        let state = suspend.stubs.len() as i64 + 1;

        let mut names: Vec<String> = fcx.env.keys().cloned().collect();
        names.sort();
        let has_varargs = fcx.varargs.is_some();
        let temps = fcx.live_temps.len();

        let mut arg_specs: Vec<(Type<'c>, Location<'c>)> = vec![(self.pack_ty(), location)];
        arg_specs.extend(
            std::iter::repeat_n((self.box_ty(), location), names.len()),
        );
        arg_specs.push((self.dyn_ty(), location));
        if has_varargs {
            arg_specs.push((self.pack_ty(), location));
        }
        arg_specs.extend(std::iter::repeat_n((self.dyn_ty(), location), temps));

        let continuation = fcx.region.append_block(Block::new(&arg_specs));
        let cold = fcx.region.append_block(Block::new(&[]));

        let mut branch_args = vec![result];
        for name in &names {
            branch_args.push(fcx.env[name]);
        }
        branch_args.push(fcx.globals);
        if let Some(va) = fcx.varargs {
            branch_args.push(va);
        }
        branch_args.extend(fcx.live_temps.iter().copied());

        let flag = self.read_susp_flag(fcx.block, location)?;
        self.cond_br_false_args(fcx.block, flag, cold, continuation, &branch_args, location)?;

        let boxes: Vec<Value> = names.iter().map(|n| fcx.env[n]).collect();
        self.emit_frame_capture(
            cold,
            &symbol,
            state,
            &boxes,
            fcx.globals,
            fcx.varargs,
            &fcx.live_temps.clone(),
            location,
        )?;

        fcx.block = continuation;
        let rebound = block_arg(continuation, 0)?;
        let mut index = 1;
        for name in &names {
            fcx.env.insert(name.clone(), block_arg(continuation, index)?);
            index += 1;
        }
        fcx.globals = block_arg(continuation, index)?;
        index += 1;
        if has_varargs {
            fcx.varargs = Some(block_arg(continuation, index)?);
            index += 1;
        }
        for t in 0..temps {
            fcx.live_temps[t] = block_arg(continuation, index)?;
            index += 1;
        }

        fcx.suspend.as_mut().expect("suspend").stubs.push(StateStub {
            state,
            target: continuation,
            names,
            temps,
            has_varargs,
        });
        Ok(rebound)
    }

    /// D-084's dominance law: in licensed lifted fns, every join/head
    /// block carries the live-ins (env boxes, _G, varargs) as block
    /// args — resume paths (entry → stub → continuation) never
    /// executed the prologue, so nothing prologue-defined dominates a
    /// resume-reachable join. Returns None when not in a licensed
    /// lifted fn (blocks stay arg-free, IR byte-identical to
    /// pre-M35).
    fn live_spec(&self, fcx: &Fcx<'c, '_>) -> Option<(Vec<String>, bool)> {
        fcx.suspend.as_ref()?;
        let mut names: Vec<String> = fcx.env.keys().cloned().collect();
        names.sort();
        Some((names, fcx.varargs.is_some()))
    }

    fn live_arg_types(
        &self,
        spec: &Option<(Vec<String>, bool)>,
        location: Location<'c>,
    ) -> Vec<(Type<'c>, Location<'c>)> {
        let Some((names, has_varargs)) = spec else { return Vec::new() };
        let mut types = Vec::new();
        types.extend(std::iter::repeat_n((self.box_ty(), location), names.len()));
        types.push((self.dyn_ty(), location));
        if *has_varargs {
            types.push((self.pack_ty(), location));
        }
        types
    }

    fn live_arg_values<'r>(
        &self,
        fcx: &Fcx<'c, 'r>,
        spec: &Option<(Vec<String>, bool)>,
    ) -> Result<Vec<Value<'c, 'r>>> {
        let Some((names, has_varargs)) = spec else { return Ok(Vec::new()) };
        let mut values = Vec::new();
        for name in names {
            values.push(*fcx.env.get(name).ok_or_else(|| {
                format!("live-in `{name}` out of scope at branch (D-084)")
            })?);
        }
        values.push(fcx.globals);
        if *has_varargs {
            values.push(fcx.varargs.ok_or("varargs live-in missing")?);
        }
        Ok(values)
    }

    /// Rebinds fcx from a live-arg'd block's args, starting at `skip`
    /// (the block's extra leading args).
    fn live_rebind<'r>(
        &self,
        fcx: &mut Fcx<'c, 'r>,
        block: BlockRef<'c, 'r>,
        skip: usize,
        spec: &Option<(Vec<String>, bool)>,
    ) -> Result<()> {
        let Some((names, has_varargs)) = spec else { return Ok(()) };
        let mut index = skip;
        for name in names {
            fcx.env.insert(name.clone(), block_arg(block, index)?);
            index += 1;
        }
        fcx.globals = block_arg(block, index)?;
        index += 1;
        if *has_varargs {
            fcx.varargs = Some(block_arg(block, index)?);
        }
        Ok(())
    }

    fn dyn_ty(&self) -> Type<'c> {
        Type::parse(self.context, "!frk_dyn.dyn").expect("dyn type")
    }
    fn box_ty(&self) -> Type<'c> {
        Type::parse(self.context, "!frk_mem.box<!frk_dyn.dyn>").expect("box type")
    }
    fn bstr_ty(&self) -> Type<'c> {
        Type::parse(self.context, "!frk_bstr.str").expect("bstr type")
    }
    /// The D-058 pack: one argument/values array per call.
    fn pack_ty(&self) -> Type<'c> {
        Type::parse(self.context, "!frk_mem.arr<!frk_dyn.dyn>").expect("pack type")
    }
    /// THE Lua function type — every function, every arity (D-058).
    fn lua_fn_ty(&self) -> Type<'c> {
        Type::parse(
            self.context,
            "!frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>",
        )
        .expect("lua fn type")
    }
    fn i64_ty(&self) -> Type<'c> {
        IntegerType::new(self.context, 64).into()
    }
    fn i1_ty(&self) -> Type<'c> {
        IntegerType::new(self.context, 1).into()
    }
    fn f64_ty(&self) -> Type<'c> {
        Type::parse(self.context, "f64").expect("f64 type")
    }

    fn loc_at(&self, offset: usize) -> Location<'c> {
        let line = match self.line_starts.binary_search(&offset) {
            Ok(exact) => exact,
            Err(insert) => insert - 1,
        };
        Location::new(self.context, &self.file, line + 1, offset - self.line_starts[line] + 1)
    }

    // ---- op toolkit ----

    fn op1<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        op: melior::ir::Operation<'c>,
    ) -> Result<Value<'c, 'r>> {
        let inserted = block.append_operation(op);
        let raw = inserted
            .result(0)
            .map_err(|_| "op has no result".to_string())?
            .to_raw();
        Ok(unsafe { Value::from_raw(raw) })
    }

    fn build<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        name: &str,
        operands: &[Value<'c, 'r>],
        results: &[Type<'c>],
        attributes: &[(&str, Attribute<'c>)],
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        let mut builder = OperationBuilder::new(name, location)
            .add_operands(operands)
            .add_results(results);
        for (key, attribute) in attributes {
            builder = builder.add_attributes(&[(Identifier::new(self.context, key), *attribute)]);
        }
        self.op1(block, builder.build().map_err(|e| e.to_string())?)
    }

    fn build0<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        name: &str,
        operands: &[Value<'c, 'r>],
        attributes: &[(&str, Attribute<'c>)],
        location: Location<'c>,
    ) -> Result<()> {
        let mut builder = OperationBuilder::new(name, location).add_operands(operands);
        for (key, attribute) in attributes {
            builder = builder.add_attributes(&[(Identifier::new(self.context, key), *attribute)]);
        }
        block.append_operation(builder.build().map_err(|e| e.to_string())?);
        Ok(())
    }

    fn const_i64<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        value: i64,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        self.op1(
            block,
            melior::dialect::arith::constant(
                self.context,
                IntegerAttribute::new(self.i64_ty(), value).into(),
                location,
            ),
        )
    }

    fn const_bool<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        value: bool,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        self.op1(
            block,
            melior::dialect::arith::constant(
                self.context,
                IntegerAttribute::new(self.i1_ty(), value as i64).into(),
                location,
            ),
        )
    }

    fn const_f64<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        value: f64,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        let attribute = Attribute::parse(self.context, &format!("{value:?} : f64"))
            .ok_or_else(|| format!("unparsable f64 {value:?}"))?;
        self.build(block, "arith.constant", &[], &[self.f64_ty()], &[("value", attribute)], location)
    }

    fn wrap<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        tag: i64,
        value: Value<'c, 'r>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        self.build(
            block,
            "frk_dyn.wrap",
            &[value],
            &[self.dyn_ty()],
            &[("tag", IntegerAttribute::new(self.i64_ty(), tag).into())],
            location,
        )
    }

    fn unwrap<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        tag: i64,
        result: Type<'c>,
        value: Value<'c, 'r>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        self.build(
            block,
            "frk_dyn.unwrap",
            &[value],
            &[result],
            &[("tag", IntegerAttribute::new(self.i64_ty(), tag).into())],
            location,
        )
    }

    fn tag_of<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        value: Value<'c, 'r>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        self.build(block, "frk_dyn.tag_of", &[value], &[self.i64_ty()], &[], location)
    }

    fn nil_dyn<'r>(&self, block: BlockRef<'c, 'r>, location: Location<'c>) -> Result<Value<'c, 'r>> {
        let zero = self.const_i64(block, 0, location)?;
        self.wrap(block, TAG_NIL, zero, location)
    }

    fn str_lit<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        text: &str,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        self.build(
            block,
            "frk_bstr.lit",
            &[],
            &[self.bstr_ty()],
            &[("text", StringAttribute::new(self.context, text).into())],
            location,
        )
    }

    fn call<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        callee: &str,
        operands: &[Value<'c, 'r>],
        results: &[Type<'c>],
        location: Location<'c>,
    ) -> Result<Option<Value<'c, 'r>>> {
        let attribute: Attribute =
            FlatSymbolRefAttribute::new(self.context, callee).into();
        if results.is_empty() {
            self.build0(block, "func.call", operands, &[("callee", attribute)], location)?;
            Ok(None)
        } else {
            Ok(Some(self.build(
                block,
                "func.call",
                operands,
                results,
                &[("callee", attribute)],
                location,
            )?))
        }
    }

    fn br<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        target: BlockRef<'c, 'r>,
        values: &[Value<'c, 'r>],
        location: Location<'c>,
    ) -> Result<()> {
        block.append_operation(
            OperationBuilder::new("cf.br", location)
                .add_operands(values)
                .add_successors(&[&target])
                .build()
                .map_err(|e| e.to_string())?,
        );
        Ok(())
    }

    fn cond_br<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        condition: Value<'c, 'r>,
        on_true: BlockRef<'c, 'r>,
        on_false: BlockRef<'c, 'r>,
        location: Location<'c>,
    ) -> Result<()> {
        block.append_operation(
            OperationBuilder::new("cf.cond_br", location)
                .add_attributes(&[(
                    Identifier::new(self.context, "operandSegmentSizes"),
                    DenseI32ArrayAttribute::new(self.context, &[1, 0, 0]).into(),
                )])
                .add_operands(&[condition])
                .add_successors(&[&on_true, &on_false])
                .build()
                .map_err(|e| e.to_string())?,
        );
        Ok(())
    }


    fn func(
        &self,
        module: &Module<'c>,
        name: &str,
        inputs: &[Type<'c>],
        outputs: &[Type<'c>],
        region: Region<'c>,
        entry_attrs: bool,
    ) {
        let mut attributes = Vec::new();
        if entry_attrs {
            attributes.push((
                Identifier::new(self.context, "llvm.emit_c_interface"),
                Attribute::unit(self.context),
            ));
        }
        let function = melior::dialect::func::func(
            self.context,
            StringAttribute::new(self.context, name),
            TypeAttribute::new(FunctionType::new(self.context, inputs, outputs).into()),
            region,
            &attributes,
            Location::unknown(self.context),
        );
        module.body().append_operation(function);
    }

    // The protocol helpers — including the _v wrappers and iterator
    // protocol — live in intrinsics.mlir (M17 + M20, D-062/D-065).
    // The emitter builds no helper IR.

    fn cmpi<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        predicate: i64,
        lhs: Value<'c, 'r>,
        rhs: Value<'c, 'r>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        self.build(
            block,
            "arith.cmpi",
            &[lhs, rhs],
            &[self.i1_ty()],
            &[("predicate", IntegerAttribute::new(self.i64_ty(), predicate).into())],
            location,
        )
    }

    fn cmpf<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        predicate: i64,
        lhs: Value<'c, 'r>,
        rhs: Value<'c, 'r>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        self.build(
            block,
            "arith.cmpf",
            &[lhs, rhs],
            &[self.i1_ty()],
            &[("predicate", IntegerAttribute::new(self.i64_ty(), predicate).into())],
            location,
        )
    }

    fn helper_fun<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        symbol: &str,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        let empty = Type::parse(self.context, "!frk_adt.product<[]>").ok_or("empty")?;
        let env = self.build(block, "frk_adt.product_new", &[], &[empty], &[], location)?;
        let closure = self.build(
            block,
            "frk_closure.make",
            &[env],
            &[self.lua_fn_ty()],
            &[("callee", FlatSymbolRefAttribute::new(self.context, symbol).into())],
            location,
        )?;
        self.wrap(block, TAG_FUN, closure, location)
    }

    /// Builds a values/arguments pack (arr<dyn>) — D-058.
    fn make_pack<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        values: &[Value<'c, 'r>],
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        let len = self.const_i64(block, values.len() as i64, location)?;
        let pack = self.build(
            block,
            "frk_mem.array_new",
            &[len],
            &[self.pack_ty()],
            &[],
            location,
        )?;
        for (index, value) in values.iter().enumerate() {
            let index_value = self.const_i64(block, index as i64, location)?;
            self.build0(
                block,
                "frk_mem.array_set",
                &[pack, index_value, *value],
                &[],
                location,
            )?;
        }
        Ok(pack)
    }

    /// Calls a Lua function value: unwrap at THE fn type, one pack in,
    /// one pack out (D-058).
    fn call_lua<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        callee_dyn: Value<'c, 'r>,
        arguments: &[Value<'c, 'r>],
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        let pack = self.make_pack(block, arguments, location)?;
        self.call_lua_pack(block, callee_dyn, pack, location)
    }

    /// The apply half of call_lua, for a pre-built argument pack.
    fn call_lua_pack<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        callee_dyn: Value<'c, 'r>,
        pack: Value<'c, 'r>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        let function = self.unwrap(block, TAG_FUN, self.lua_fn_ty(), callee_dyn, location)?;
        let empty = Type::parse(self.context, "!frk_adt.product<[]>").ok_or("empty product")?;
        let product = self.build(block, "frk_adt.product_new", &[], &[empty], &[], location)?;
        let wrapped_ty = Type::parse(
            self.context,
            "!frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>",
        )
        .ok_or("arg product")?;
        let arg_product = self.build(
            block,
            "frk_adt.product_snoc",
            &[product, pack],
            &[wrapped_ty],
            &[],
            location,
        )?;
        self.build(
            block,
            "frk_closure.apply",
            &[function, arg_product],
            &[self.pack_ty()],
            &[],
            location,
        )
    }

    /// The values pack of an explist under Lua ADJUSTMENT (D-068):
    /// every non-final expression truncates to one value; a final
    /// Call forwards/appends its whole pack; a final `...` appends
    /// the vararg tail (always copied — the private arr stays owned
    /// by this frame). Empty explist = empty pack.
    fn emit_explist_pack<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        exprs: &[Expr],
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        let Some((last, init)) = exprs.split_last() else {
            return self.make_pack(fcx.block, &[], location);
        };
        // Prefix values ride live_temps (D-084.4): any later element
        // — or the final call — may suspend, and the guard rebinds
        // everything it carried across.
        let base = fcx.live_temps.len();
        for expression in init {
            let value = self.emit_expr(fcx, expression)?;
            fcx.live_temps.push(value);
        }
        let result = match last {
            Expr::Call(callee, arguments, _) => {
                let tail = self.emit_call_pack(fcx, callee, arguments, location)?;
                let prefix: Vec<Value> = fcx.live_temps[base..].to_vec();
                if prefix.is_empty() {
                    // Whole-pack forwarding: the no-copy fast path the
                    // D-063 tail-call law rides.
                    fcx.live_temps.truncate(base);
                    return Ok(tail);
                }
                self.pack_with_tail(fcx, &prefix, tail, location)
            }
            Expr::Vararg(_) => {
                let tail = fcx.varargs.ok_or_else(|| {
                    "internal: `...` survived parsing outside a vararg function".to_string()
                })?;
                let prefix: Vec<Value> = fcx.live_temps[base..].to_vec();
                self.pack_with_tail(fcx, &prefix, tail, location)
            }
            other => {
                let value = self.emit_expr(fcx, other)?;
                fcx.live_temps.push(value);
                let prefix: Vec<Value> = fcx.live_temps[base..].to_vec();
                self.make_pack(fcx.block, &prefix, location)
            }
        };
        fcx.live_temps.truncate(base);
        result
    }

    /// prefix values + every element of `tail`, as a fresh pack of
    /// runtime length (the copy loop is the __lua_pack_copy_into
    /// intrinsic — IR-authored, so the rc retain discipline is the
    /// kernel lowering's, not hand-written).
    fn pack_with_tail<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        prefix: &[Value<'c, 'r>],
        tail: Value<'c, 'r>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        let tail_len = self.build(
            fcx.block,
            "frk_mem.array_len",
            &[tail],
            &[self.i64_ty()],
            &[],
            location,
        )?;
        let prefix_len = self.const_i64(fcx.block, prefix.len() as i64, location)?;
        let total = self.build(
            fcx.block,
            "arith.addi",
            &[prefix_len, tail_len],
            &[self.i64_ty()],
            &[],
            location,
        )?;
        let pack = self.build(
            fcx.block,
            "frk_mem.array_new",
            &[total],
            &[self.pack_ty()],
            &[],
            location,
        )?;
        for (index, value) in prefix.iter().enumerate() {
            let index_value = self.const_i64(fcx.block, index as i64, location)?;
            self.build0(
                fcx.block,
                "frk_mem.array_set",
                &[pack, index_value, *value],
                &[],
                location,
            )?;
        }
        self.call(
            fcx.block,
            "__lua_pack_copy_into",
            &[pack, prefix_len, tail],
            &[],
            location,
        )?;
        Ok(pack)
    }

    /// pack[i] with nil-fill (the __lua_arg helper).
    fn pack_get<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        pack: Value<'c, 'r>,
        index: i64,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        let index_value = self.const_i64(block, index, location)?;
        self.call(block, "__lua_arg", &[pack, index_value], &[self.dyn_ty()], location)
            .map(|value| value.expect("result"))
    }


    // ---- main + lifted functions ----

    fn emit_main(&mut self, module: &Module<'c>, chunk: &LBlock) -> Result<()> {
        let location = Location::unknown(self.context);
        let region = Region::new();
        let entry = region.append_block(Block::new(&[]));
        let globals =
            self.build(entry, "frk_dyn.table_new", &[], &[self.dyn_ty()], &[], location)?;

        // Seed the stdlib subset (D-052/D-058): pack-convention
        // wrappers, all at THE one Lua fn type.
        for (name, helper) in [
            ("print", "__lua_print_v"),
            ("tostring", "__lua_tostring_v"),
            ("setmetatable", "__lua_setmetatable_v"),
            ("getmetatable", "__lua_getmetatable_v"),
            ("next", "__lua_next_v"),
            ("pairs", "__lua_pairs_v"),
            ("ipairs", "__lua_ipairs_v"),
            // M35 (D-084.5): type() joins the seeded stdlib —
            // type(co) == "thread" is corpus-load-bearing.
            ("type", "__lua_type_v"),
        ] {
            let wrapped = self.helper_fun(entry, helper, location)?;
            let key_lit = self.str_lit(entry, name, location)?;
            let key = self.wrap(entry, TAG_STR, key_lit, location)?;
            self.build0(
                entry,
                "frk_dyn.raw_set",
                &[globals, key, wrapped],
                &[],
                location,
            )?;
        }

        // The string module (D-058): a table of pack-convention funs.
        {
            let string_table =
                self.build(entry, "frk_dyn.table_new", &[], &[self.dyn_ty()], &[], location)?;
            for (field, helper) in
                [("sub", "__lua_string_sub_v"), ("rep", "__lua_string_rep_v")]
            {
                let fun = self.helper_fun(entry, helper, location)?;
                let key_lit = self.str_lit(entry, field, location)?;
                let key = self.wrap(entry, TAG_STR, key_lit, location)?;
                self.build0(entry, "frk_dyn.raw_set", &[string_table, key, fun], &[], location)?;
            }
            let key_lit = self.str_lit(entry, "string", location)?;
            let key = self.wrap(entry, TAG_STR, key_lit, location)?;
            self.build0(
                entry,
                "frk_dyn.raw_set",
                &[globals, key, string_table],
                &[],
                location,
            )?;
        }

        // M35 (D-084): the coroutine table + the suspend-channel cells,
        // licensed modules only.
        if self.licensed {
            self.call(entry, "__lua_coro_init", &[], &[], location)?;
            let coro_table =
                self.build(entry, "frk_dyn.table_new", &[], &[self.dyn_ty()], &[], location)?;
            for (field, helper) in [
                ("create", "__lua_coro_create_v"),
                ("resume", "__lua_coro_resume_v"),
                ("yield", "__lua_coro_yield_v"),
                ("status", "__lua_coro_status_v"),
                ("wrap", "__lua_coro_wrap_v"),
            ] {
                let fun = self.helper_fun(entry, helper, location)?;
                let key_lit = self.str_lit(entry, field, location)?;
                let key = self.wrap(entry, TAG_STR, key_lit, location)?;
                self.build0(entry, "frk_dyn.raw_set", &[coro_table, key, fun], &[], location)?;
            }
            let key_lit = self.str_lit(entry, "coroutine", location)?;
            let key = self.wrap(entry, TAG_STR, key_lit, location)?;
            self.build0(
                entry,
                "frk_dyn.raw_set",
                &[globals, key, coro_table],
                &[],
                location,
            )?;
        }

        let mut fcx = Fcx {
            region: &region,
            block: entry,
            env: HashMap::new(),
            globals,
            terminated: false,
            break_targets: Vec::new(),
            varargs: None,
            suspend: None,
            live_temps: Vec::new(),
            entry_pack: None,
            scope_shadows: Vec::new(),
        };
        self.emit_block(&mut fcx, chunk)?;
        if !fcx.terminated {
            fcx.block.append_operation(
                OperationBuilder::new("func.return", location)
                    .build()
                    .map_err(|e| e.to_string())?,
            );
        }
        self.func(module, "main", &[], &[], region, true);
        Ok(())
    }

    fn emit_lifted(&mut self, module: &Module<'c>, job: LiftJob) -> Result<()> {
        if self.licensed {
            return self.emit_lifted_resumable(module, job);
        }
        let location = Location::unknown(self.context);
        // D-063 uniform convention over the D-058 packs: EVERY lua
        // function is (envref, args-pack) -> values-pack. The env
        // product is [_G, capture boxes...], read via env_load — so
        // every lua function shares ONE native signature and tail
        // applies musttail by construction.
        let inputs = vec![self.envref_ty(), self.pack_ty()];

        let region = Region::new();
        let entry = region.append_block(Block::new(
            &inputs.iter().map(|ty| (*ty, location)).collect::<Vec<_>>(),
        ));
        let envref = block_arg(entry, 0)?;
        let env_ty = self.env_product_ty(job.captures.len());
        let globals =
            self.env_load(entry, envref, 0, env_ty, self.dyn_ty(), location)?;
        let mut env = HashMap::new();
        for (index, name) in job.captures.iter().enumerate() {
            let capture = self.env_load(
                entry,
                envref,
                1 + index as i64,
                env_ty,
                self.box_ty(),
                location,
            )?;
            env.insert(name.clone(), capture);
        }
        // Params: nil-filled reads from the pack (extras drop by
        // never being read) — Lua's arity adjustment, for free.
        let pack = block_arg(entry, 1)?;
        for (index, name) in job.params.iter().enumerate() {
            let value = self.pack_get(entry, pack, index as i64, location)?;
            let boxed = self.build(
                entry,
                "frk_mem.box_new",
                &[value],
                &[self.box_ty()],
                &[],
                location,
            )?;
            env.insert(name.clone(), boxed);
        }
        // Varargs (D-068): the `...` tail is pack[nparams..], COPIED
        // into a private arr BEFORE the dispose below — the callee-
        // owned-pack invariant (D-067) is untouched, and `...` reads
        // the private copy thereafter.
        let varargs = if job.is_vararg {
            let start = self.const_i64(entry, job.params.len() as i64, location)?;
            Some(
                self.call(
                    entry,
                    "__lua_pack_tail",
                    &[pack, start],
                    &[self.pack_ty()],
                    location,
                )?
                .expect("result"),
            )
        } else {
            None
        };
        // Packs are CALLEE-OWNED (D-067): all params are boxed above,
        // so the incoming pack's ownership ends here — long before any
        // tail call, so the D-064 tail shape is never disturbed.
        self.build0(entry, "frk_mem.dispose", &[pack], &[], location)?;

        let mut fcx = Fcx {
            region: &region,
            block: entry,
            env,
            globals,
            terminated: false,
            break_targets: Vec::new(),
            varargs,
            suspend: None,
            live_temps: Vec::new(),
            entry_pack: None,
            scope_shadows: Vec::new(),
        };
        self.emit_block(&mut fcx, &job.body)?;
        if !fcx.terminated {
            // Fall-off returns NO values (an empty pack).
            let empty = self.make_pack(fcx.block, &[], location)?;
            ret(fcx.block, &[empty], location)?;
        }
        self.func(module, &job.symbol, &inputs, &[self.pack_ty()], region, false);
        Ok(())
    }

    /// The LICENSED lift (D-084): same signature, but the entry
    /// dispatches on env[0] (the resume state) — state 0 is the
    /// normal prologue; state k re-enters at the continuation block
    /// of guarded call site k, live-ins reloaded from the frame env
    /// and the suspended call's result delivered by the chain walk.
    fn emit_lifted_resumable(&mut self, module: &Module<'c>, job: LiftJob) -> Result<()> {
        let location = Location::unknown(self.context);
        let inputs = vec![self.envref_ty(), self.pack_ty()];
        let region = Region::new();
        let entry = region.append_block(Block::new(
            &inputs.iter().map(|ty| (*ty, location)).collect::<Vec<_>>(),
        ));
        let envref = block_arg(entry, 0)?;
        let pack = block_arg(entry, 1)?;

        // State-0: the normal prologue, in its own block.
        let start = region.append_block(Block::new(&[]));
        let state0_ty = self.licensed_env_ty(job.captures.len(), false, 0);
        let globals = self.env_load(start, envref, 2, state0_ty, self.dyn_ty(), location)?;
        let mut env = HashMap::new();
        for (index, name) in job.captures.iter().enumerate() {
            let capture = self.env_load(
                start,
                envref,
                3 + index as i64,
                state0_ty,
                self.box_ty(),
                location,
            )?;
            env.insert(name.clone(), capture);
        }
        for (index, name) in job.params.iter().enumerate() {
            let value = self.pack_get(start, pack, index as i64, location)?;
            let boxed = self.build(
                start,
                "frk_mem.box_new",
                &[value],
                &[self.box_ty()],
                &[],
                location,
            )?;
            env.insert(name.clone(), boxed);
        }
        let varargs = if job.is_vararg {
            let from = self.const_i64(start, job.params.len() as i64, location)?;
            Some(
                self.call(start, "__lua_pack_tail", &[pack, from], &[self.pack_ty()], location)?
                    .expect("result"),
            )
        } else {
            None
        };
        self.build0(start, "frk_mem.dispose", &[pack], &[], location)?;

        let mut fcx = Fcx {
            region: &region,
            block: start,
            env,
            globals,
            terminated: false,
            break_targets: Vec::new(),
            varargs,
            suspend: Some(SuspendCtx { symbol: job.symbol.clone(), stubs: Vec::new() }),
            live_temps: Vec::new(),
            entry_pack: Some(pack),
            scope_shadows: Vec::new(),
        };
        self.emit_block(&mut fcx, &job.body)?;
        if !fcx.terminated {
            let empty = self.make_pack(fcx.block, &[], location)?;
            ret(fcx.block, &[empty], location)?;
        }

        // The resume stubs: state k reloads the frame fields, walks
        // the chain with the resume pack (nil chain = deliver), and
        // — because the walk can itself re-suspend — re-guards.
        let ctx = fcx.suspend.take().expect("suspend ctx");
        let mut stub_blocks: Vec<(i64, BlockRef)> = Vec::new();
        for stub in &ctx.stubs {
            let sk = region.append_block(Block::new(&[]));
            let env_ty =
                self.licensed_env_ty(stub.names.len(), stub.has_varargs, stub.temps);
            let chain = self.env_load(sk, envref, 1, env_ty, self.dyn_ty(), location)?;
            let sg = self.env_load(sk, envref, 2, env_ty, self.dyn_ty(), location)?;
            let mut boxes = Vec::new();
            for index in 0..stub.names.len() {
                boxes.push(self.env_load(
                    sk,
                    envref,
                    3 + index as i64,
                    env_ty,
                    self.box_ty(),
                    location,
                )?);
            }
            let mut field = 3 + stub.names.len() as i64;
            let sva = if stub.has_varargs {
                let va = self.env_load(sk, envref, field, env_ty, self.pack_ty(), location)?;
                field += 1;
                Some(va)
            } else {
                None
            };
            let mut temps = Vec::new();
            for _ in 0..stub.temps {
                temps.push(self.env_load(sk, envref, field, env_ty, self.dyn_ty(), location)?);
                field += 1;
            }
            let walked = self
                .call(
                    sk,
                    "__lua_coro_walk",
                    &[chain, pack],
                    &[self.pack_ty()],
                    location,
                )?
                .expect("walk result");
            let flag = self.read_susp_flag(sk, location)?;
            let cold = region.append_block(Block::new(&[]));
            let mut branch_args = vec![walked];
            branch_args.extend(boxes.iter().copied());
            branch_args.push(sg);
            if let Some(va) = sva {
                branch_args.push(va);
            }
            branch_args.extend(temps.iter().copied());
            self.cond_br_false_args(sk, flag, cold, stub.target, &branch_args, location)?;
            self.emit_frame_capture(
                cold,
                &ctx.symbol,
                stub.state,
                &boxes,
                sg,
                sva,
                &temps,
                location,
            )?;
            stub_blocks.push((stub.state, sk));
        }

        // The entry dispatch — built LAST, once every state exists.
        let state = self.env_load(entry, envref, 0, state0_ty, self.i64_ty(), location)?;
        if stub_blocks.is_empty() {
            self.br(entry, start, &[], location)?;
        } else {
            let values: Vec<String> =
                stub_blocks.iter().map(|(k, _)| k.to_string()).collect();
            let dense = format!(
                "dense<[{}]> : tensor<{}xi64>",
                values.join(", "),
                stub_blocks.len()
            );
            let segments = vec![0i32; stub_blocks.len()];
            let mut successors: Vec<&Block> = vec![&start];
            for (_, block) in &stub_blocks {
                successors.push(block);
            }
            entry.append_operation(
                OperationBuilder::new("cf.switch", location)
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
                    .add_operands(&[state])
                    .add_successors(&successors)
                    .build()
                    .map_err(|e| e.to_string())?,
            );
        }

        self.func(module, &job.symbol, &inputs, &[self.pack_ty()], region, false);
        Ok(())
    }

    /// Emits a closure value (a fun dyn) for a function body.
    fn emit_closure<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        params: &[String],
        is_vararg: bool,
        body: &LBlock,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        let mut bound: HashSet<String> = params.iter().cloned().collect();
        let mut free = BTreeSet::new();
        free_names_block(body, &mut bound, &mut free);
        // Captures = free names bound as LOCALS here; the rest are
        // globals and resolve through _G at use sites inside.
        let captures: Vec<String> =
            free.into_iter().filter(|name| fcx.env.contains_key(name)).collect();

        let symbol = format!("__lua_fn_{}", self.next_fn);
        self.next_fn += 1;
        self.lift_queue.push(LiftJob {
            symbol: symbol.clone(),
            captures: captures.clone(),
            params: params.to_vec(),
            is_vararg,
            body: body.clone(),
        });

        // Env pack: [_G, capture boxes...] — or, LICENSED (D-084.1),
        // [state 0, nil chain, _G, capture boxes...].
        let mut spelling_parts = Vec::new();
        let mut values: Vec<Value> = Vec::new();
        if self.licensed {
            spelling_parts.push("i64".to_string());
            spelling_parts.push("!frk_dyn.dyn".to_string());
            values.push(self.const_i64(fcx.block, 0, location)?);
            values.push(self.nil_dyn(fcx.block, location)?);
        }
        spelling_parts.push("!frk_dyn.dyn".to_string());
        spelling_parts
            .extend(std::iter::repeat_n("!frk_mem.box<!frk_dyn.dyn>".to_string(), captures.len()));
        let empty = Type::parse(self.context, "!frk_adt.product<[]>").ok_or("empty")?;
        let mut acc = self.build(fcx.block, "frk_adt.product_new", &[], &[empty], &[], location)?;
        values.push(fcx.globals);
        for name in &captures {
            values.push(fcx.env[name]);
        }
        for (index, value) in values.iter().enumerate() {
            let ty = Type::parse(
                self.context,
                &format!("!frk_adt.product<[{}]>", spelling_parts[..=index].join(", ")),
            )
            .ok_or("product type")?;
            acc =
                self.build(fcx.block, "frk_adt.product_snoc", &[acc, *value], &[ty], &[], location)?;
        }

        let closure = self.build(
            fcx.block,
            "frk_closure.make",
            &[acc],
            &[self.lua_fn_ty()],
            &[("callee", FlatSymbolRefAttribute::new(self.context, &symbol).into())],
            location,
        )?;
        self.wrap(fcx.block, TAG_FUN, closure, location)
    }

    // ---- statements ----

    fn emit_block<'r>(&mut self, fcx: &mut Fcx<'c, 'r>, block: &LBlock) -> Result<()> {
        // Lua blocks scope locals: `local` bindings introduced inside
        // are undone at exit — but names REBOUND by a guard (same box,
        // fresh SSA) keep their current values (M35; see
        // scope_shadows).
        fcx.scope_shadows.push(Vec::new());
        for statement in block {
            if fcx.terminated {
                break;
            }
            self.emit_stat(fcx, statement)?;
        }
        let frame = fcx.scope_shadows.pop().expect("scope frame");
        for (name, previous) in frame.into_iter().rev() {
            match previous {
                Some(value) => {
                    fcx.env.insert(name, value);
                }
                None => {
                    fcx.env.remove(&name);
                }
            }
        }
        Ok(())
    }

    /// Binds a `local` name, recording the shadowed predecessor in the
    /// innermost scope frame (function-lifetime bindings — params —
    /// have no frame and record nothing).
    fn bind_local<'r>(
        &self,
        fcx: &mut Fcx<'c, 'r>,
        name: &str,
        value: Value<'c, 'r>,
    ) {
        let previous = fcx.env.insert(name.to_string(), value);
        if let Some(frame) = fcx.scope_shadows.last_mut() {
            frame.push((name.to_string(), previous));
        }
    }

    fn emit_stat<'r>(&mut self, fcx: &mut Fcx<'c, 'r>, statement: &Stat) -> Result<()> {
        match statement {
            Stat::Local(name, value, span) => {
                let location = self.loc_at(*span);
                let value = self.emit_expr(fcx, value)?;
                let boxed = self.build(
                    fcx.block,
                    "frk_mem.box_new",
                    &[value],
                    &[self.box_ty()],
                    &[],
                    location,
                )?;
                self.bind_local(fcx, name, boxed);
                Ok(())
            }
            Stat::LocalFunction(name, params, is_vararg, body, span) => {
                let location = self.loc_at(*span);
                // Box first, bind, then build: recursion through the box.
                let nil = self.nil_dyn(fcx.block, location)?;
                let boxed = self.build(
                    fcx.block,
                    "frk_mem.box_new",
                    &[nil],
                    &[self.box_ty()],
                    &[],
                    location,
                )?;
                self.bind_local(fcx, name, boxed);
                let closure = self.emit_closure(fcx, params, *is_vararg, body, location)?;
                self.build0(fcx.block, "frk_mem.box_set", &[boxed, closure], &[], location)?;
                Ok(())
            }
            Stat::GlobalFunction(name, params, is_vararg, body, span) => {
                let location = self.loc_at(*span);
                let closure = self.emit_closure(fcx, params, *is_vararg, body, location)?;
                let key_lit = self.str_lit(fcx.block, name, location)?;
                let key = self.wrap(fcx.block, TAG_STR, key_lit, location)?;
                let globals = fcx.globals;
                self.build0(
                    fcx.block,
                    "frk_dyn.raw_set",
                    &[globals, key, closure],
                    &[],
                    location,
                )?;
                Ok(())
            }
            Stat::AssignName(name, value, span) => {
                let location = self.loc_at(*span);
                let value = self.emit_expr(fcx, value)?;
                match fcx.env.get(name) {
                    Some(boxed) => {
                        let boxed = *boxed;
                        self.build0(fcx.block, "frk_mem.box_set", &[boxed, value], &[], location)?;
                    }
                    None => {
                        let key_lit = self.str_lit(fcx.block, name, location)?;
                        let key = self.wrap(fcx.block, TAG_STR, key_lit, location)?;
                        let globals = fcx.globals;
                        self.build0(
                            fcx.block,
                            "frk_dyn.raw_set",
                            &[globals, key, value],
                            &[],
                            location,
                        )?;
                    }
                }
                Ok(())
            }
            Stat::AssignIndex(table, key, value, span) => {
                // v0.3 (D-068): writes go through the __newindex
                // protocol — existing keys raw-assign, absent keys
                // walk the metamethod (function and table forms).
                let location = self.loc_at(*span);
                let table = self.emit_expr(fcx, table)?;
                let key = self.emit_expr(fcx, key)?;
                let value = self.emit_expr(fcx, value)?;
                self.call(fcx.block, "__lua_setindex", &[table, key, value], &[], location)?;
                Ok(())
            }
            Stat::Call(expression, _) => {
                let _ = self.emit_expr(fcx, expression)?;
                Ok(())
            }
            Stat::Do(body, _) => self.emit_block(fcx, body),
            Stat::Break(span) => {
                let location = self.loc_at(*span);
                let (target, spec) = fcx
                    .break_targets
                    .last()
                    .cloned()
                    .ok_or_else(|| "break outside a loop".to_string())?;
                let args = self.live_arg_values(fcx, &spec)?;
                self.br(fcx.block, target, &args, location)?;
                fcx.terminated = true;
                Ok(())
            }
            Stat::Repeat(body, condition, span) => {
                let location = self.loc_at(*span);
                let head = fcx.region.append_block(Block::new(&[]));
                let done = fcx.region.append_block(Block::new(&[]));
                self.br(fcx.block, head, &[], location)?;
                fcx.block = head;
                fcx.terminated = false;
                // Lua scoping: `until` sees the body's locals — the
                // env restores AFTER the condition.
                let saved = fcx.env.clone();
                fcx.break_targets.push((done, None));
                for statement in body {
                    if fcx.terminated {
                        break;
                    }
                    self.emit_stat(fcx, statement)?;
                }
                fcx.break_targets.pop();
                if !fcx.terminated {
                    let condition_value = self.emit_expr(fcx, condition)?;
                    let truthy = self
                        .call(fcx.block, "__lua_truthy", &[condition_value], &[self.i1_ty()], location)?
                        .expect("result");
                    self.cond_br(fcx.block, truthy, done, head, location)?;
                }
                fcx.env = saved;
                fcx.block = done;
                fcx.terminated = false;
                Ok(())
            }
            Stat::LocalMulti(names, value, span) => {
                let location = self.loc_at(*span);
                let pack = self.emit_explist_pack(fcx, value, location)?;
                for (index, name) in names.iter().enumerate() {
                    let value = self.pack_get(fcx.block, pack, index as i64, location)?;
                    let boxed = self.build(
                        fcx.block,
                        "frk_mem.box_new",
                        &[value],
                        &[self.box_ty()],
                        &[],
                        location,
                    )?;
                    self.bind_local(fcx, name, boxed);
                }
                Ok(())
            }
            Stat::AssignMulti(names, value, span) => {
                let location = self.loc_at(*span);
                let pack = self.emit_explist_pack(fcx, value, location)?;
                for (index, name) in names.iter().enumerate() {
                    let value = self.pack_get(fcx.block, pack, index as i64, location)?;
                    match fcx.env.get(name) {
                        Some(boxed) => {
                            let boxed = *boxed;
                            self.build0(
                                fcx.block,
                                "frk_mem.box_set",
                                &[boxed, value],
                                &[],
                                location,
                            )?;
                        }
                        None => {
                            let key_lit = self.str_lit(fcx.block, name, location)?;
                            let key = self.wrap(fcx.block, TAG_STR, key_lit, location)?;
                            let globals = fcx.globals;
                            self.build0(
                                fcx.block,
                                "frk_dyn.raw_set",
                                &[globals, key, value],
                                &[],
                                location,
                            )?;
                        }
                    }
                }
                Ok(())
            }
            Stat::GenFor(names, iterator, body, span) => {
                let location = self.loc_at(*span);
                // for n1, n2 in EXPLIST do: adjusts to (f, s, ctrl) —
                // explicit triples are just an explist (D-068).
                let triple = self.emit_explist_pack(fcx, iterator, location)?;
                let iter_fn = self.pack_get(fcx.block, triple, 0, location)?;
                let state = self.pack_get(fcx.block, triple, 1, location)?;
                let control0 = self.pack_get(fcx.block, triple, 2, location)?;

                let head = fcx
                    .region
                    .append_block(Block::new(&[(self.dyn_ty(), location)]));
                let bbody = fcx.region.append_block(Block::new(&[]));
                let done = fcx.region.append_block(Block::new(&[]));
                self.br(fcx.block, head, &[control0], location)?;

                let control = block_arg(head, 0)?;
                fcx.block = head;
                let rpack = self.call_lua(fcx.block, iter_fn, &[state, control], location)?;
                let next_control = self.pack_get(fcx.block, rpack, 0, location)?;
                let tag = self.tag_of(fcx.block, next_control, location)?;
                let zero = self.const_i64(fcx.block, 0, location)?;
                let is_nil = self.cmpi(fcx.block, 0, tag, zero, location)?;
                self.cond_br(fcx.block, is_nil, done, bbody, location)?;

                fcx.block = bbody;
                fcx.terminated = false;
                let saved = fcx.env.clone();
                for (index, name) in names.iter().enumerate() {
                    let value = self.pack_get(fcx.block, rpack, index as i64, location)?;
                    let boxed = self.build(
                        fcx.block,
                        "frk_mem.box_new",
                        &[value],
                        &[self.box_ty()],
                        &[],
                        location,
                    )?;
                    self.bind_local(fcx, name, boxed);
                }
                fcx.break_targets.push((done, None));
                self.emit_block(fcx, body)?;
                fcx.break_targets.pop();
                fcx.env = saved;
                if !fcx.terminated {
                    self.br(fcx.block, head, &[next_control], location)?;
                }
                fcx.block = done;
                fcx.terminated = false;
                Ok(())
            }
            Stat::Return(values, span) => {
                let location = self.loc_at(*span);
                // The engine keeps the single-call no-copy forwarding
                // (the D-063 tail-call fast path) and adds `...` and
                // mixed explists (D-068).
                let pack = self.emit_explist_pack(fcx, values, location)?;
                ret(fcx.block, &[pack], location)?;
                fcx.terminated = true;
                Ok(())
            }
            Stat::If(arms, otherwise, span) => {
                let location = self.loc_at(*span);
                let join = fcx.region.append_block(Block::new(&[]));
                for (condition, body) in arms {
                    let condition_value = self.emit_expr(fcx, condition)?;
                    let truthy = self
                        .call(fcx.block, "__lua_truthy", &[condition_value], &[self.i1_ty()], location)?
                        .expect("result");
                    let bthen = fcx.region.append_block(Block::new(&[]));
                    let belse = fcx.region.append_block(Block::new(&[]));
                    self.cond_br(fcx.block, truthy, bthen, belse, location)?;
                    fcx.block = bthen;
                    fcx.terminated = false;
                    self.emit_block(fcx, body)?;
                    if !fcx.terminated {
                        self.br(fcx.block, join, &[], location)?;
                    }
                    fcx.block = belse;
                    fcx.terminated = false;
                }
                if let Some(body) = otherwise {
                    self.emit_block(fcx, body)?;
                }
                if !fcx.terminated {
                    self.br(fcx.block, join, &[], location)?;
                }
                fcx.block = join;
                fcx.terminated = false;
                Ok(())
            }
            Stat::While(condition, body, span) => {
                let location = self.loc_at(*span);
                // Licensed fns thread the live-ins through head/body/
                // done (D-084's dominance law); unlicensed emission is
                // byte-identical to pre-M35 (empty spec = no args).
                let spec = self.live_spec(fcx);
                let arg_types = self.live_arg_types(&spec, location);
                let head = fcx.region.append_block(Block::new(&arg_types));
                let bbody = fcx.region.append_block(Block::new(&arg_types));
                let done = fcx.region.append_block(Block::new(&arg_types));
                let entry_args = self.live_arg_values(fcx, &spec)?;
                self.br(fcx.block, head, &entry_args, location)?;
                fcx.block = head;
                self.live_rebind(fcx, head, 0, &spec)?;
                let condition_value = self.emit_expr(fcx, condition)?;
                let truthy = self
                    .call(fcx.block, "__lua_truthy", &[condition_value], &[self.i1_ty()], location)?
                    .expect("result");
                let branch_args = self.live_arg_values(fcx, &spec)?;
                self.cond_br_two_args(
                    fcx.block, truthy, bbody, &branch_args, done, &branch_args, location,
                )?;
                fcx.block = bbody;
                self.live_rebind(fcx, bbody, 0, &spec)?;
                fcx.terminated = false;
                fcx.break_targets.push((done, spec.clone()));
                self.emit_block(fcx, body)?;
                fcx.break_targets.pop();
                if !fcx.terminated {
                    let back_args = self.live_arg_values(fcx, &spec)?;
                    self.br(fcx.block, head, &back_args, location)?;
                }
                fcx.block = done;
                self.live_rebind(fcx, done, 0, &spec)?;
                fcx.terminated = false;
                Ok(())
            }
            Stat::NumFor(variable, from, to, step, body, span) => {
                let location = self.loc_at(*span);
                // Bounds evaluate once, as numbers (unwrap traps
                // otherwise — Lua errors there too).
                let from = self.emit_expr(fcx, from)?;
                let from = self.unwrap(fcx.block, TAG_NUM, self.f64_ty(), from, location)?;
                let to = self.emit_expr(fcx, to)?;
                let to = self.unwrap(fcx.block, TAG_NUM, self.f64_ty(), to, location)?;
                let step = match step {
                    Some(expression) => {
                        let value = self.emit_expr(fcx, expression)?;
                        self.unwrap(fcx.block, TAG_NUM, self.f64_ty(), value, location)?
                    }
                    None => self.const_f64(fcx.block, 1.0, location)?,
                };

                let head = fcx
                    .region
                    .append_block(Block::new(&[(self.f64_ty(), location)]));
                let bbody = fcx.region.append_block(Block::new(&[]));
                let done = fcx.region.append_block(Block::new(&[]));
                self.br(fcx.block, head, &[from], location)?;

                let counter = block_arg(head, 0)?;
                let zero = self.const_f64(head, 0.0, location)?;
                let ascending = self.cmpf(head, 2, step, zero, location)?; // ogt
                let le = self.cmpf(head, 5, counter, to, location)?; // ole
                let ge = self.cmpf(head, 3, counter, to, location)?; // oge
                let keep = self.build(
                    head,
                    "arith.select",
                    &[ascending, le, ge],
                    &[self.i1_ty()],
                    &[],
                    location,
                )?;
                self.cond_br(head, keep, bbody, done, location)?;

                fcx.block = bbody;
                fcx.terminated = false;
                // Fresh box per iteration: 5.1 closes upvalues per loop.
                let wrapped = self.wrap(fcx.block, TAG_NUM, counter, location)?;
                let boxed = self.build(
                    fcx.block,
                    "frk_mem.box_new",
                    &[wrapped],
                    &[self.box_ty()],
                    &[],
                    location,
                )?;
                let saved = fcx.env.clone();
                fcx.env.insert(variable.clone(), boxed);
                fcx.break_targets.push((done, None));
                self.emit_block(fcx, body)?;
                fcx.break_targets.pop();
                fcx.env = saved;
                if !fcx.terminated {
                    let next = self.build(
                        fcx.block,
                        "arith.addf",
                        &[counter, step],
                        &[self.f64_ty()],
                        &[],
                        location,
                    )?;
                    self.br(fcx.block, head, &[next], location)?;
                }
                fcx.block = done;
                fcx.terminated = false;
                Ok(())
            }
        }
    }

    /// Emits a call and returns its RAW values pack (D-058). In
    /// licensed lifted fns the result is GUARDED (D-084): the callee
    /// dyn rides live_temps across argument evaluation (arguments may
    /// themselves suspend), and the call site becomes a resume state.
    fn emit_call_pack<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        callee: &Expr,
        arguments: &[Expr],
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        let callee_value = self.emit_expr(fcx, callee)?;
        let base = fcx.live_temps.len();
        fcx.live_temps.push(callee_value);
        let pack = self.emit_explist_pack(fcx, arguments, location)?;
        let callee_value = fcx.live_temps[base];
        fcx.live_temps.truncate(base);
        let raw = self.call_lua_pack(fcx.block, callee_value, pack, location)?;
        self.guarded_result(fcx, raw, location)
    }

    // ---- expressions (every result is a dyn) ----

    fn emit_expr<'r>(&mut self, fcx: &mut Fcx<'c, 'r>, expression: &Expr) -> Result<Value<'c, 'r>> {
        let location = self.loc_at(expression.span());
        match expression {
            Expr::Nil(_) => self.nil_dyn(fcx.block, location),
            Expr::Vararg(_) => {
                // Expression context: `...` adjusts to ONE value
                // (nil-filled when the tail is empty).
                let varargs = fcx.varargs.ok_or_else(|| {
                    "internal: `...` survived parsing outside a vararg function".to_string()
                })?;
                self.pack_get(fcx.block, varargs, 0, location)
            }
            Expr::True(_) => {
                let value = self.const_bool(fcx.block, true, location)?;
                self.wrap(fcx.block, TAG_BOOL, value, location)
            }
            Expr::False(_) => {
                let value = self.const_bool(fcx.block, false, location)?;
                self.wrap(fcx.block, TAG_BOOL, value, location)
            }
            Expr::Num(value, _) => {
                let value = self.const_f64(fcx.block, *value, location)?;
                self.wrap(fcx.block, TAG_NUM, value, location)
            }
            Expr::Str(text, _) => {
                let value = self.str_lit(fcx.block, text, location)?;
                self.wrap(fcx.block, TAG_STR, value, location)
            }
            Expr::Name(name, _) => match fcx.env.get(name) {
                Some(boxed) => {
                    let boxed = *boxed;
                    self.build(fcx.block, "frk_mem.box_get", &[boxed], &[self.dyn_ty()], &[], location)
                }
                None => {
                    let key_lit = self.str_lit(fcx.block, name, location)?;
                    let key = self.wrap(fcx.block, TAG_STR, key_lit, location)?;
                    let globals = fcx.globals;
                    self.build(
                        fcx.block,
                        "frk_dyn.raw_get",
                        &[globals, key],
                        &[self.dyn_ty()],
                        &[],
                        location,
                    )
                }
            },
            Expr::Index(table, key, _) => {
                let table = self.emit_expr(fcx, table)?;
                let key = self.emit_expr(fcx, key)?;
                self.call(fcx.block, "__lua_index", &[table, key], &[self.dyn_ty()], location)
                    .map(|value| value.expect("result"))
            }
            Expr::Call(callee, arguments, _) => {
                // Expression context: the pack adjusts to ONE value.
                let pack = self.emit_call_pack(fcx, callee, arguments, location)?;
                self.pack_get(fcx.block, pack, 0, location)
            }
            Expr::Function(params, is_vararg, body, _) => {
                self.emit_closure(fcx, params, *is_vararg, body, location)
            }
            Expr::Table(fields, _) => {
                let table =
                    self.build(fcx.block, "frk_dyn.table_new", &[], &[self.dyn_ty()], &[], location)?;
                let mut position = 0i64;
                let count = fields.len();
                for (field_index, field) in fields.iter().enumerate() {
                    match field {
                        // The FINAL positional field expands when it is
                        // a call or `...` (Lua constructor adjustment,
                        // D-068): elements append from position+1 with
                        // runtime number keys.
                        Field::Positional(value)
                            if field_index + 1 == count
                                && matches!(value, Expr::Call(..) | Expr::Vararg(_)) =>
                        {
                            let tail = match value {
                                Expr::Call(callee, arguments, _) => {
                                    self.emit_call_pack(fcx, callee, arguments, location)?
                                }
                                Expr::Vararg(_) => fcx.varargs.ok_or_else(|| {
                                    "internal: `...` survived parsing outside a vararg \
                                     function"
                                        .to_string()
                                })?,
                                _ => unreachable!(),
                            };
                            let first =
                                self.const_f64(fcx.block, (position + 1) as f64, location)?;
                            self.call(
                                fcx.block,
                                "__lua_ctor_append",
                                &[table, first, tail],
                                &[],
                                location,
                            )?;
                        }
                        Field::Positional(value) => {
                            position += 1;
                            let value = self.emit_expr(fcx, value)?;
                            let index = self.const_f64(fcx.block, position as f64, location)?;
                            let key = self.wrap(fcx.block, TAG_NUM, index, location)?;
                            self.build0(
                                fcx.block,
                                "frk_dyn.raw_set",
                                &[table, key, value],
                                &[],
                                location,
                            )?;
                        }
                        Field::Named(name, value) => {
                            let value = self.emit_expr(fcx, value)?;
                            let key_lit = self.str_lit(fcx.block, name, location)?;
                            let key = self.wrap(fcx.block, TAG_STR, key_lit, location)?;
                            self.build0(
                                fcx.block,
                                "frk_dyn.raw_set",
                                &[table, key, value],
                                &[],
                                location,
                            )?;
                        }
                        Field::Keyed(key, value) => {
                            let key = self.emit_expr(fcx, key)?;
                            let value = self.emit_expr(fcx, value)?;
                            self.build0(
                                fcx.block,
                                "frk_dyn.raw_set",
                                &[table, key, value],
                                &[],
                                location,
                            )?;
                        }
                    }
                }
                Ok(table)
            }
            Expr::Unary(op, operand, _) => {
                let value = self.emit_expr(fcx, operand)?;
                match op {
                    UnOp::Neg => {
                        let number =
                            self.unwrap(fcx.block, TAG_NUM, self.f64_ty(), value, location)?;
                        let negated = self.build(
                            fcx.block,
                            "arith.negf",
                            &[number],
                            &[self.f64_ty()],
                            &[],
                            location,
                        )?;
                        self.wrap(fcx.block, TAG_NUM, negated, location)
                    }
                    UnOp::Not => {
                        let truthy = self
                            .call(fcx.block, "__lua_truthy", &[value], &[self.i1_ty()], location)?
                            .expect("result");
                        let one = self.const_bool(fcx.block, true, location)?;
                        let negated = self.build(
                            fcx.block,
                            "arith.xori",
                            &[truthy, one],
                            &[self.i1_ty()],
                            &[],
                            location,
                        )?;
                        self.wrap(fcx.block, TAG_BOOL, negated, location)
                    }
                    UnOp::Len => self
                        .call(fcx.block, "__lua_len", &[value], &[self.dyn_ty()], location)
                        .map(|value| value.expect("result")),
                }
            }
            Expr::Binary(op, lhs, rhs, _) => self.emit_binary(fcx, *op, lhs, rhs, location),
        }
    }

    fn emit_binary<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        op: BinOp,
        lhs: &Expr,
        rhs: &Expr,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        // and/or: VALUE-returning short circuits through truthiness.
        if matches!(op, BinOp::And | BinOp::Or) {
            let left = self.emit_expr(fcx, lhs)?;
            let truthy = self
                .call(fcx.block, "__lua_truthy", &[left], &[self.i1_ty()], location)?
                .expect("result");
            let brhs = fcx.region.append_block(Block::new(&[]));
            let join = fcx
                .region
                .append_block(Block::new(&[(self.dyn_ty(), location)]));
            match op {
                BinOp::And => {
                    // truthy(a) ? b : a
                    let bshort = fcx.region.append_block(Block::new(&[]));
                    self.cond_br(fcx.block, truthy, brhs, bshort, location)?;
                    self.br(bshort, join, &[left], location)?;
                }
                _ => {
                    // or: truthy(a) ? a : b
                    let bshort = fcx.region.append_block(Block::new(&[]));
                    self.cond_br(fcx.block, truthy, bshort, brhs, location)?;
                    self.br(bshort, join, &[left], location)?;
                }
            }
            fcx.block = brhs;
            let right = self.emit_expr(fcx, rhs)?;
            self.br(fcx.block, join, &[right], location)?;
            fcx.block = join;
            return Ok(block_arg(join, 0)?);
        }

        let left = self.emit_expr(fcx, lhs)?;
        let right = self.emit_expr(fcx, rhs)?;
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                let a = self.unwrap(fcx.block, TAG_NUM, self.f64_ty(), left, location)?;
                let b = self.unwrap(fcx.block, TAG_NUM, self.f64_ty(), right, location)?;
                let name = match op {
                    BinOp::Add => "arith.addf",
                    BinOp::Sub => "arith.subf",
                    BinOp::Mul => "arith.mulf",
                    _ => "arith.divf",
                };
                let value = self.build(fcx.block, name, &[a, b], &[self.f64_ty()], &[], location)?;
                self.wrap(fcx.block, TAG_NUM, value, location)
            }
            BinOp::Mod => {
                // Lua FLOOR-mod: a − ⌊a/b⌋·b. floor from trunc + fixup
                // (fptosi truncates toward zero; subtract one when the
                // truncation overshot a negative quotient).
                let a = self.unwrap(fcx.block, TAG_NUM, self.f64_ty(), left, location)?;
                let b = self.unwrap(fcx.block, TAG_NUM, self.f64_ty(), right, location)?;
                let q = self.build(fcx.block, "arith.divf", &[a, b], &[self.f64_ty()], &[], location)?;
                let ti = self.build(fcx.block, "arith.fptosi", &[q], &[self.i64_ty()], &[], location)?;
                let tf = self.build(fcx.block, "arith.sitofp", &[ti], &[self.f64_ty()], &[], location)?;
                let lt = self.cmpf(fcx.block, 4, q, tf, location)?; // olt
                let one = self.const_f64(fcx.block, 1.0, location)?;
                let tf_minus = self.build(
                    fcx.block,
                    "arith.subf",
                    &[tf, one],
                    &[self.f64_ty()],
                    &[],
                    location,
                )?;
                let floor = self.build(
                    fcx.block,
                    "arith.select",
                    &[lt, tf_minus, tf],
                    &[self.f64_ty()],
                    &[],
                    location,
                )?;
                let prod =
                    self.build(fcx.block, "arith.mulf", &[floor, b], &[self.f64_ty()], &[], location)?;
                let value =
                    self.build(fcx.block, "arith.subf", &[a, prod], &[self.f64_ty()], &[], location)?;
                self.wrap(fcx.block, TAG_NUM, value, location)
            }
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let a = self.unwrap(fcx.block, TAG_NUM, self.f64_ty(), left, location)?;
                let b = self.unwrap(fcx.block, TAG_NUM, self.f64_ty(), right, location)?;
                let predicate = match op {
                    BinOp::Lt => 4,
                    BinOp::Le => 5,
                    BinOp::Gt => 2,
                    _ => 3,
                };
                let value = self.cmpf(fcx.block, predicate, a, b, location)?;
                self.wrap(fcx.block, TAG_BOOL, value, location)
            }
            BinOp::Eq | BinOp::Ne => {
                let equal = self
                    .call(fcx.block, "__lua_eq", &[left, right], &[self.i1_ty()], location)?
                    .expect("result");
                let value = if op == BinOp::Ne {
                    let one = self.const_bool(fcx.block, true, location)?;
                    self.build(fcx.block, "arith.xori", &[equal, one], &[self.i1_ty()], &[], location)?
                } else {
                    equal
                };
                self.wrap(fcx.block, TAG_BOOL, value, location)
            }
            BinOp::Concat => {
                let a = self
                    .call(fcx.block, "__lua_costr", &[left], &[self.bstr_ty()], location)?
                    .expect("result");
                let b = self
                    .call(fcx.block, "__lua_costr", &[right], &[self.bstr_ty()], location)?
                    .expect("result");
                let joined =
                    self.build(fcx.block, "frk_bstr.concat", &[a, b], &[self.bstr_ty()], &[], location)?;
                self.wrap(fcx.block, TAG_STR, joined, location)
            }
            BinOp::And | BinOp::Or => unreachable!("handled above"),
        }
    }
}

fn block_arg<'c, 'r>(block: BlockRef<'c, 'r>, index: usize) -> Result<Value<'c, 'r>> {
    let raw = block
        .argument(index)
        .map_err(|e| e.to_string())?
        .to_raw();
    Ok(unsafe { Value::from_raw(raw) })
}

fn ret<'c, 'r>(
    block: BlockRef<'c, 'r>,
    values: &[Value<'c, 'r>],
    location: Location<'c>,
) -> Result<()> {
    block.append_operation(
        OperationBuilder::new("func.return", location)
            .add_operands(values)
            .build()
            .map_err(|e| e.to_string())?,
    );
    Ok(())
}

// ---- free-name analysis (captures vs globals, D-054) ----

fn free_names_block(block: &LBlock, bound: &mut HashSet<String>, out: &mut BTreeSet<String>) {
    let snapshot = bound.clone();
    for statement in block {
        free_names_stat(statement, bound, out);
    }
    *bound = snapshot;
}

fn free_names_stat(statement: &Stat, bound: &mut HashSet<String>, out: &mut BTreeSet<String>) {
    match statement {
        Stat::Local(name, value, _) => {
            free_names_expr(value, bound, out);
            bound.insert(name.clone());
        }
        Stat::LocalFunction(name, params, _, body, _) => {
            bound.insert(name.clone());
            let mut inner = bound.clone();
            inner.extend(params.iter().cloned());
            let mut inner_set = inner;
            free_names_block(body, &mut inner_set, out);
        }
        Stat::GlobalFunction(_, params, _, body, _) => {
            let mut inner = bound.clone();
            inner.extend(params.iter().cloned());
            free_names_block(body, &mut inner, out);
        }
        Stat::AssignName(name, value, _) => {
            free_names_expr(value, bound, out);
            // A write to a non-locally-bound name is FREE: an outer
            // local (needs its box captured) or a global (the capture
            // filter routes it to _G).
            if !bound.contains(name) {
                out.insert(name.clone());
            }
        }
        Stat::AssignIndex(table, key, value, _) => {
            free_names_expr(table, bound, out);
            free_names_expr(key, bound, out);
            free_names_expr(value, bound, out);
        }
        Stat::Call(expression, _) => free_names_expr(expression, bound, out),
        Stat::Return(values, _) => {
            for expression in values {
                free_names_expr(expression, bound, out);
            }
        }
        Stat::Break(_) => {}
        Stat::Repeat(body, condition, _) => {
            let mut inner = bound.clone();
            for statement in body {
                free_names_stat(statement, &mut inner, out);
            }
            free_names_expr(condition, &inner, out);
        }
        Stat::LocalMulti(names, values, _) => {
            for value in values {
                free_names_expr(value, bound, out);
            }
            for name in names {
                bound.insert(name.clone());
            }
        }
        Stat::AssignMulti(names, values, _) => {
            for value in values {
                free_names_expr(value, bound, out);
            }
            for name in names {
                if !bound.contains(name) {
                    out.insert(name.clone());
                }
            }
        }
        Stat::GenFor(names, iterator, body, _) => {
            for expression in iterator {
                free_names_expr(expression, bound, out);
            }
            let mut inner = bound.clone();
            inner.extend(names.iter().cloned());
            free_names_block(body, &mut inner, out);
        }
        Stat::Do(body, _) => free_names_block(body, &mut bound.clone(), out),
        Stat::If(arms, otherwise, _) => {
            for (condition, body) in arms {
                free_names_expr(condition, bound, out);
                free_names_block(body, &mut bound.clone(), out);
            }
            if let Some(body) = otherwise {
                free_names_block(body, &mut bound.clone(), out);
            }
        }
        Stat::While(condition, body, _) => {
            free_names_expr(condition, bound, out);
            free_names_block(body, &mut bound.clone(), out);
        }
        Stat::NumFor(variable, from, to, step, body, _) => {
            free_names_expr(from, bound, out);
            free_names_expr(to, bound, out);
            if let Some(step) = step {
                free_names_expr(step, bound, out);
            }
            let mut inner = bound.clone();
            inner.insert(variable.clone());
            free_names_block(body, &mut inner, out);
        }
    }
}

fn free_names_expr(expression: &Expr, bound: &HashSet<String>, out: &mut BTreeSet<String>) {
    match expression {
        Expr::Vararg(_) => {}
        Expr::Nil(_) | Expr::True(_) | Expr::False(_) | Expr::Num(..) | Expr::Str(..) => {}
        Expr::Name(name, _) => {
            if !bound.contains(name) {
                out.insert(name.clone());
            }
        }
        Expr::Index(table, key, _) => {
            free_names_expr(table, bound, out);
            free_names_expr(key, bound, out);
        }
        Expr::Call(callee, arguments, _) => {
            free_names_expr(callee, bound, out);
            for argument in arguments {
                free_names_expr(argument, bound, out);
            }
        }
        Expr::Function(params, _, body, _) => {
            let mut inner = bound.clone();
            inner.extend(params.iter().cloned());
            free_names_block(body, &mut inner, out);
        }
        Expr::Table(fields, _) => {
            for field in fields {
                match field {
                    Field::Positional(value) | Field::Named(_, value) => {
                        free_names_expr(value, bound, out);
                    }
                    Field::Keyed(key, value) => {
                        free_names_expr(key, bound, out);
                        free_names_expr(value, bound, out);
                    }
                }
            }
        }
        Expr::Binary(_, lhs, rhs, _) => {
            free_names_expr(lhs, bound, out);
            free_names_expr(rhs, bound, out);
        }
        Expr::Unary(_, operand, _) => free_names_expr(operand, bound, out),
    }
}
