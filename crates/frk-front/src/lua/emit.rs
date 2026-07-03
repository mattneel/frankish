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
    let module = Module::new(Location::unknown(context));
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
    };

    emitter.declare_runtime(&module)?;
    emitter.emit_helpers(&module)?;
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
    body: LBlock,
}

struct Emitter<'c> {
    context: &'c Context,
    file: String,
    line_starts: Vec<usize>,
    lift_queue: Vec<LiftJob>,
    next_fn: usize,
}

struct Fcx<'c, 'r> {
    region: &'r Region<'c>,
    block: BlockRef<'c, 'r>,
    /// name → its box<dyn> value.
    env: HashMap<String, Value<'c, 'r>>,
    /// The _G table (a dyn), threaded everywhere.
    globals: Value<'c, 'r>,
    terminated: bool,
}

impl<'c> Emitter<'c> {
    // ---- types & locations ----

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

    fn switch<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        flag: Value<'c, 'r>,
        cases: &[(i64, BlockRef<'c, 'r>)],
        default: BlockRef<'c, 'r>,
        location: Location<'c>,
    ) -> Result<()> {
        let case_text = cases
            .iter()
            .map(|(value, _)| value.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let dense = format!("dense<[{case_text}]> : vector<{}xi64>", cases.len());
        let mut successors: Vec<&Block<'c>> = vec![&default];
        for (_, target) in cases {
            successors.push(target);
        }
        block.append_operation(
            OperationBuilder::new("cf.switch", location)
                .add_attributes(&[
                    (
                        Identifier::new(self.context, "case_values"),
                        Attribute::parse(self.context, &dense)
                            .ok_or_else(|| format!("unparsable {dense}"))?,
                    ),
                    (
                        Identifier::new(self.context, "case_operand_segments"),
                        DenseI32ArrayAttribute::new(self.context, &vec![0; cases.len()]).into(),
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

    fn declare_runtime(&self, module: &Module<'c>) -> Result<()> {
        // Bodyless declarations answered by builtins (interp), capture
        // symbols (JIT), and the real runtime (AOT).
        for (name, inputs, outputs) in [
            ("frk_rt_bstr_from_num", vec![self.f64_ty()], vec![self.bstr_ty()]),
            ("frk_rt_print_lua_str", vec![self.bstr_ty()], vec![]),
            ("frk_rt_lua_error", vec![self.i64_ty()], vec![]),
        ] {
            let declaration = OperationBuilder::new("func.func", Location::unknown(self.context))
                .add_attributes(&[
                    (
                        Identifier::new(self.context, "sym_name"),
                        StringAttribute::new(self.context, name).into(),
                    ),
                    (
                        Identifier::new(self.context, "function_type"),
                        TypeAttribute::new(
                            FunctionType::new(self.context, &inputs, &outputs).into(),
                        )
                        .into(),
                    ),
                    (
                        Identifier::new(self.context, "sym_visibility"),
                        StringAttribute::new(self.context, "private").into(),
                    ),
                ])
                .add_regions([Region::new()])
                .build()
                .map_err(|e| e.to_string())?;
            module.body().append_operation(declaration);
        }
        Ok(())
    }

    // ---- the synthesized protocol helpers (D-056.2) ----

    fn emit_helpers(&mut self, module: &Module<'c>) -> Result<()> {
        let location = Location::unknown(self.context);
        let dynt = self.dyn_ty();

        // __lua_truthy(v) -> i1: nil/false falsy, everything else true.
        {
            let region = Region::new();
            let entry = region.append_block(Block::new(&[(dynt, location)]));
            let value = block_arg(entry, 0)?;
            let bfalse = region.append_block(Block::new(&[]));
            let btrue = region.append_block(Block::new(&[]));
            let bbool = region.append_block(Block::new(&[]));
            let tag = self.tag_of(entry, value, location)?;
            self.switch(entry, tag, &[(TAG_NIL, bfalse), (TAG_BOOL, bbool)], btrue, location)?;
            let f = self.const_bool(bfalse, false, location)?;
            ret(bfalse, &[f], location)?;
            let t = self.const_bool(btrue, true, location)?;
            ret(btrue, &[t], location)?;
            let b = self.unwrap(bbool, TAG_BOOL, self.i1_ty(), value, location)?;
            ret(bbool, &[b], location)?;
            self.func(module, "__lua_truthy", &[dynt], &[self.i1_ty()], region, false);
        }

        // __lua_tostring(v) -> dyn (a string dyn).
        {
            let region = Region::new();
            let entry = region.append_block(Block::new(&[(dynt, location)]));
            let value = block_arg(entry, 0)?;
            let bnil = region.append_block(Block::new(&[]));
            let bbool = region.append_block(Block::new(&[]));
            let bnum = region.append_block(Block::new(&[]));
            let bself = region.append_block(Block::new(&[]));
            let berr = region.append_block(Block::new(&[]));
            let tag = self.tag_of(entry, value, location)?;
            self.switch(
                entry,
                tag,
                &[(TAG_NIL, bnil), (TAG_BOOL, bbool), (TAG_NUM, bnum), (TAG_STR, bself)],
                berr,
                location,
            )?;
            let s = self.str_lit(bnil, "nil", location)?;
            let d = self.wrap(bnil, TAG_STR, s, location)?;
            ret(bnil, &[d], location)?;

            let b = self.unwrap(bbool, TAG_BOOL, self.i1_ty(), value, location)?;
            let btrue = region.append_block(Block::new(&[]));
            let bfalse = region.append_block(Block::new(&[]));
            self.cond_br(bbool, b, btrue, bfalse, location)?;
            for (block, text) in [(btrue, "true"), (bfalse, "false")] {
                let s = self.str_lit(block, text, location)?;
                let d = self.wrap(block, TAG_STR, s, location)?;
                ret(block, &[d], location)?;
            }

            let n = self.unwrap(bnum, TAG_NUM, self.f64_ty(), value, location)?;
            let s = self
                .call(bnum, "frk_rt_bstr_from_num", &[n], &[self.bstr_ty()], location)?
                .expect("result");
            let d = self.wrap(bnum, TAG_STR, s, location)?;
            ret(bnum, &[d], location)?;

            ret(bself, &[value], location)?;

            let one = self.const_i64(berr, 1, location)?;
            self.call(berr, "frk_rt_lua_error", &[one], &[], location)?;
            let n = self.nil_dyn(berr, location)?;
            ret(berr, &[n], location)?;
            self.func(module, "__lua_tostring", &[dynt], &[dynt], region, false);
        }

        // __lua_print(v) -> dyn (nil).
        {
            let region = Region::new();
            let entry = region.append_block(Block::new(&[(dynt, location)]));
            let value = block_arg(entry, 0)?;
            let s = self
                .call(entry, "__lua_tostring", &[value], &[dynt], location)?
                .expect("result");
            let raw = self.unwrap(entry, TAG_STR, self.bstr_ty(), s, location)?;
            self.call(entry, "frk_rt_print_lua_str", &[raw], &[], location)?;
            let nil = self.nil_dyn(entry, location)?;
            ret(entry, &[nil], location)?;
            self.func(module, "__lua_print", &[dynt], &[dynt], region, false);
        }

        // __lua_eq(a, b) -> i1.
        {
            let region = Region::new();
            let entry = region.append_block(Block::new(&[(dynt, location), (dynt, location)]));
            let a = block_arg(entry, 0)?;
            let b = block_arg(entry, 1)?;
            let bsame = region.append_block(Block::new(&[]));
            let bdiff = region.append_block(Block::new(&[]));
            let ta = self.tag_of(entry, a, location)?;
            let tb = self.tag_of(entry, b, location)?;
            let same = self.cmpi(entry, 0, ta, tb, location)?;
            self.cond_br(entry, same, bsame, bdiff, location)?;
            let f = self.const_bool(bdiff, false, location)?;
            ret(bdiff, &[f], location)?;

            let btrue = region.append_block(Block::new(&[]));
            let bb = region.append_block(Block::new(&[]));
            let bn = region.append_block(Block::new(&[]));
            let bs = region.append_block(Block::new(&[]));
            let bref = region.append_block(Block::new(&[]));
            self.switch(
                bsame,
                ta,
                &[(TAG_NIL, btrue), (TAG_BOOL, bb), (TAG_NUM, bn), (TAG_STR, bs)],
                bref,
                location,
            )?;
            let t = self.const_bool(btrue, true, location)?;
            ret(btrue, &[t], location)?;
            let xa = self.unwrap(bb, TAG_BOOL, self.i1_ty(), a, location)?;
            let xb = self.unwrap(bb, TAG_BOOL, self.i1_ty(), b, location)?;
            let e = self.cmpi(bb, 0, xa, xb, location)?;
            ret(bb, &[e], location)?;
            let na = self.unwrap(bn, TAG_NUM, self.f64_ty(), a, location)?;
            let nb = self.unwrap(bn, TAG_NUM, self.f64_ty(), b, location)?;
            let e = self.cmpf(bn, 1, na, nb, location)?; // oeq
            ret(bn, &[e], location)?;
            let sa = self.unwrap(bs, TAG_STR, self.bstr_ty(), a, location)?;
            let sb = self.unwrap(bs, TAG_STR, self.bstr_ty(), b, location)?;
            let e = self.build(bs, "frk_bstr.eq", &[sa, sb], &[self.i1_ty()], &[], location)?;
            ret(bs, &[e], location)?;
            let pa = self.build(bref, "frk_dyn.payload_word", &[a], &[self.i64_ty()], &[], location)?;
            let pb = self.build(bref, "frk_dyn.payload_word", &[b], &[self.i64_ty()], &[], location)?;
            let e = self.cmpi(bref, 0, pa, pb, location)?;
            ret(bref, &[e], location)?;
            self.func(module, "__lua_eq", &[dynt, dynt], &[self.i1_ty()], region, false);
        }

        // __lua_costr(v) -> !frk_bstr.str (concat coercion: str | num).
        {
            let region = Region::new();
            let entry = region.append_block(Block::new(&[(dynt, location)]));
            let value = block_arg(entry, 0)?;
            let bstr = region.append_block(Block::new(&[]));
            let bnum = region.append_block(Block::new(&[]));
            let berr = region.append_block(Block::new(&[]));
            let tag = self.tag_of(entry, value, location)?;
            self.switch(entry, tag, &[(TAG_STR, bstr), (TAG_NUM, bnum)], berr, location)?;
            let s = self.unwrap(bstr, TAG_STR, self.bstr_ty(), value, location)?;
            ret(bstr, &[s], location)?;
            let n = self.unwrap(bnum, TAG_NUM, self.f64_ty(), value, location)?;
            let s = self
                .call(bnum, "frk_rt_bstr_from_num", &[n], &[self.bstr_ty()], location)?
                .expect("result");
            ret(bnum, &[s], location)?;
            let two = self.const_i64(berr, 2, location)?;
            self.call(berr, "frk_rt_lua_error", &[two], &[], location)?;
            let dummy = self.str_lit(berr, "", location)?;
            ret(berr, &[dummy], location)?;
            self.func(module, "__lua_costr", &[dynt], &[self.bstr_ty()], region, false);
        }

        // __lua_len(v) -> dyn (a number dyn).
        {
            let region = Region::new();
            let entry = region.append_block(Block::new(&[(dynt, location)]));
            let value = block_arg(entry, 0)?;
            let bstr = region.append_block(Block::new(&[]));
            let btab = region.append_block(Block::new(&[]));
            let berr = region.append_block(Block::new(&[]));
            let tag = self.tag_of(entry, value, location)?;
            self.switch(entry, tag, &[(TAG_STR, bstr), (4, btab)], berr, location)?;
            let s = self.unwrap(bstr, TAG_STR, self.bstr_ty(), value, location)?;
            let n = self.build(bstr, "frk_bstr.len", &[s], &[self.i64_ty()], &[], location)?;
            let d = self.num_from_i64(bstr, n, location)?;
            ret(bstr, &[d], location)?;
            let n = self.build(btab, "frk_dyn.table_len", &[value], &[self.i64_ty()], &[], location)?;
            let d = self.num_from_i64(btab, n, location)?;
            ret(btab, &[d], location)?;
            let three = self.const_i64(berr, 3, location)?;
            self.call(berr, "frk_rt_lua_error", &[three], &[], location)?;
            let n = self.nil_dyn(berr, location)?;
            ret(berr, &[n], location)?;
            self.func(module, "__lua_len", &[dynt], &[dynt], region, false);
        }

        // __lua_arg(pack, i) -> dyn: bounds-checked nil-fill read —
        // the pack convention's adjuster (D-058).
        {
            let region = Region::new();
            let entry = region
                .append_block(Block::new(&[(self.pack_ty(), location), (self.i64_ty(), location)]));
            let pack = block_arg(entry, 0)?;
            let index = block_arg(entry, 1)?;
            let len = self.build(entry, "frk_mem.array_len", &[pack], &[self.i64_ty()], &[], location)?;
            let in_range = self.cmpi(entry, 2, index, len, location)?; // slt
            let bget = region.append_block(Block::new(&[]));
            let bnil = region.append_block(Block::new(&[]));
            self.cond_br(entry, in_range, bget, bnil, location)?;
            let value =
                self.build(bget, "frk_mem.array_get", &[pack, index], &[self.dyn_ty()], &[], location)?;
            ret(bget, &[value], location)?;
            let nil = self.nil_dyn(bnil, location)?;
            ret(bnil, &[nil], location)?;
            self.func(
                module,
                "__lua_arg",
                &[self.pack_ty(), self.i64_ty()],
                &[self.dyn_ty()],
                region,
                false,
            );
        }

        // __lua_setmetatable(t, mt) -> dyn: sets and returns t (Lua).
        {
            let region = Region::new();
            let entry = region.append_block(Block::new(&[(dynt, location), (dynt, location)]));
            let table = block_arg(entry, 0)?;
            let meta = block_arg(entry, 1)?;
            self.build0(entry, "frk_dyn.set_meta", &[table, meta], &[], location)?;
            ret(entry, &[table], location)?;
            self.func(module, "__lua_setmetatable", &[dynt, dynt], &[dynt], region, false);
        }

        // __lua_getmetatable(t) -> dyn.
        {
            let region = Region::new();
            let entry = region.append_block(Block::new(&[(dynt, location)]));
            let table = block_arg(entry, 0)?;
            let meta = self.build(entry, "frk_dyn.get_meta", &[table], &[dynt], &[], location)?;
            ret(entry, &[meta], location)?;
            self.func(module, "__lua_getmetatable", &[dynt], &[dynt], region, false);
        }

        // Pack-convention wrappers for the seeded stdlib (D-058): one
        // pack in, one pack out, delegating to the typed helpers.
        for (name, inner, arity) in [
            ("__lua_print_v", "__lua_print", 1),
            ("__lua_tostring_v", "__lua_tostring", 1),
            ("__lua_setmetatable_v", "__lua_setmetatable", 2),
            ("__lua_getmetatable_v", "__lua_getmetatable", 1),
        ] {
            let region = Region::new();
            let entry = region.append_block(Block::new(&[(self.pack_ty(), location)]));
            let pack = block_arg(entry, 0)?;
            let mut arguments = Vec::new();
            for index in 0..arity {
                arguments.push(self.pack_get(entry, pack, index, location)?);
            }
            let result = self
                .call(entry, inner, &arguments, &[dynt], location)?
                .expect("result");
            let out = self.make_pack(entry, &[result], location)?;
            ret(entry, &[out], location)?;
            self.func(module, name, &[self.pack_ty()], &[self.pack_ty()], region, false);
        }

        // __lua_index(t, k) -> dyn: rawget + the __index chain
        // (table recursion; function form via frk_closure.apply).
        {
            let region = Region::new();
            let entry = region.append_block(Block::new(&[(dynt, location), (dynt, location)]));
            let table = block_arg(entry, 0)?;
            let key = block_arg(entry, 1)?;
            let btab = region.append_block(Block::new(&[]));
            let berr = region.append_block(Block::new(&[]));
            let tt = self.tag_of(entry, table, location)?;
            let four = self.const_i64(entry, 4, location)?;
            let is_table = self.cmpi(entry, 0, tt, four, location)?;
            self.cond_br(entry, is_table, btab, berr, location)?;
            let five = self.const_i64(berr, 5, location)?;
            self.call(berr, "frk_rt_lua_error", &[five], &[], location)?;
            let n = self.nil_dyn(berr, location)?;
            ret(berr, &[n], location)?;

            let raw =
                self.build(btab, "frk_dyn.raw_get", &[table, key], &[dynt], &[], location)?;
            let rt = self.tag_of(btab, raw, location)?;
            let zero = self.const_i64(btab, 0, location)?;
            let is_nil = self.cmpi(btab, 0, rt, zero, location)?;
            let bmeta = region.append_block(Block::new(&[]));
            let bhit = region.append_block(Block::new(&[]));
            self.cond_br(btab, is_nil, bmeta, bhit, location)?;
            ret(bhit, &[raw], location)?;

            let meta = self.build(bmeta, "frk_dyn.get_meta", &[table], &[dynt], &[], location)?;
            let mt = self.tag_of(bmeta, meta, location)?;
            let zero = self.const_i64(bmeta, 0, location)?;
            let no_meta = self.cmpi(bmeta, 0, mt, zero, location)?;
            let bnil = region.append_block(Block::new(&[]));
            let bidx = region.append_block(Block::new(&[]));
            self.cond_br(bmeta, no_meta, bnil, bidx, location)?;
            ret(bnil, &[raw], location)?; // raw IS nil here

            let index_lit = self.str_lit(bidx, "__index", location)?;
            let index_key = self.wrap(bidx, TAG_STR, index_lit, location)?;
            let handler =
                self.build(bidx, "frk_dyn.raw_get", &[meta, index_key], &[dynt], &[], location)?;
            let ht = self.tag_of(bidx, handler, location)?;
            let bnil2 = region.append_block(Block::new(&[]));
            let brec = region.append_block(Block::new(&[]));
            let bcall = region.append_block(Block::new(&[]));
            let berr2 = region.append_block(Block::new(&[]));
            self.switch(bidx, ht, &[(TAG_NIL, bnil2), (4, brec), (TAG_FUN, bcall)], berr2, location)?;
            let n = self.nil_dyn(bnil2, location)?;
            ret(bnil2, &[n], location)?;
            let recursed = self
                .call(brec, "__lua_index", &[handler, key], &[dynt], location)?
                .expect("result");
            ret(brec, &[recursed], location)?;

            let result_pack = self.call_lua(bcall, handler, &[table, key], location)?;
            let result = self.pack_get(bcall, result_pack, 0, location)?;
            ret(bcall, &[result], location)?;

            let five = self.const_i64(berr2, 5, location)?;
            self.call(berr2, "frk_rt_lua_error", &[five], &[], location)?;
            let n = self.nil_dyn(berr2, location)?;
            ret(berr2, &[n], location)?;
            self.func(module, "__lua_index", &[dynt, dynt], &[dynt], region, false);
        }

        Ok(())
    }

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

    fn num_from_i64<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        value: Value<'c, 'r>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        let f = self.build(block, "arith.sitofp", &[value], &[self.f64_ty()], &[], location)?;
        self.wrap(block, TAG_NUM, f, location)
    }

    /// Wraps a pack-convention helper as a Lua function value.
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
        let function = self.unwrap(block, TAG_FUN, self.lua_fn_ty(), callee_dyn, location)?;
        let pack = self.make_pack(block, arguments, location)?;
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

    /// Packs dyn values into a product (for apply's arg pack).
    fn pack_dyns<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        values: &[Value<'c, 'r>],
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        let empty = Type::parse(self.context, "!frk_adt.product<[]>").ok_or("empty product")?;
        let mut acc = self.build(block, "frk_adt.product_new", &[], &[empty], &[], location)?;
        for (index, value) in values.iter().enumerate() {
            let spelling = format!(
                "!frk_adt.product<[{}]>",
                vec!["!frk_dyn.dyn"; index + 1].join(", ")
            );
            let next = Type::parse(self.context, &spelling).ok_or("product type")?;
            acc = self.build(block, "frk_adt.product_snoc", &[acc, *value], &[next], &[], location)?;
        }
        Ok(acc)
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

        let mut fcx = Fcx {
            region: &region,
            block: entry,
            env: HashMap::new(),
            globals,
            terminated: false,
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
        let location = Location::unknown(self.context);
        // D-058 pack convention: (_G, capture boxes..., args-pack).
        let mut inputs = vec![self.dyn_ty()];
        inputs.extend(std::iter::repeat_n(self.box_ty(), job.captures.len()));
        inputs.push(self.pack_ty());

        let region = Region::new();
        let entry = region.append_block(Block::new(
            &inputs.iter().map(|ty| (*ty, location)).collect::<Vec<_>>(),
        ));
        let globals = block_arg(entry, 0)?;
        let mut env = HashMap::new();
        for (index, name) in job.captures.iter().enumerate() {
            env.insert(name.clone(), block_arg(entry, 1 + index)?);
        }
        // Params: nil-filled reads from the pack (extras drop by
        // never being read) — Lua's arity adjustment, for free.
        let pack = block_arg(entry, 1 + job.captures.len())?;
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

        let mut fcx = Fcx { region: &region, block: entry, env, globals, terminated: false };
        self.emit_block(&mut fcx, &job.body)?;
        if !fcx.terminated {
            // Fall-off returns NO values (an empty pack).
            let empty = self.make_pack(fcx.block, &[], location)?;
            ret(fcx.block, &[empty], location)?;
        }
        self.func(module, &job.symbol, &inputs, &[self.pack_ty()], region, false);
        Ok(())
    }

    /// Emits a closure value (a fun dyn) for a function body.
    fn emit_closure<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        params: &[String],
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
            body: body.clone(),
        });

        // Env pack: [_G, capture boxes...].
        let mut spelling_parts = vec!["!frk_dyn.dyn".to_string()];
        spelling_parts
            .extend(std::iter::repeat_n("!frk_mem.box<!frk_dyn.dyn>".to_string(), captures.len()));
        let empty = Type::parse(self.context, "!frk_adt.product<[]>").ok_or("empty")?;
        let mut acc = self.build(fcx.block, "frk_adt.product_new", &[], &[empty], &[], location)?;
        let mut values: Vec<Value> = vec![fcx.globals];
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
        // Lua blocks scope locals; restore the env afterwards.
        let saved = fcx.env.clone();
        for statement in block {
            if fcx.terminated {
                break;
            }
            self.emit_stat(fcx, statement)?;
        }
        fcx.env = saved;
        Ok(())
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
                fcx.env.insert(name.clone(), boxed);
                Ok(())
            }
            Stat::LocalFunction(name, params, body, span) => {
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
                fcx.env.insert(name.clone(), boxed);
                let closure = self.emit_closure(fcx, params, body, location)?;
                self.build0(fcx.block, "frk_mem.box_set", &[boxed, closure], &[], location)?;
                Ok(())
            }
            Stat::GlobalFunction(name, params, body, span) => {
                let location = self.loc_at(*span);
                let closure = self.emit_closure(fcx, params, body, location)?;
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
                let location = self.loc_at(*span);
                let table = self.emit_expr(fcx, table)?;
                let key = self.emit_expr(fcx, key)?;
                let value = self.emit_expr(fcx, value)?;
                // v0.1: raw set (__newindex fenced, D-052).
                self.build0(fcx.block, "frk_dyn.raw_set", &[table, key, value], &[], location)?;
                Ok(())
            }
            Stat::Call(expression, _) => {
                let _ = self.emit_expr(fcx, expression)?;
                Ok(())
            }
            Stat::Do(body, _) => self.emit_block(fcx, body),
            Stat::Return(value, span) => {
                let location = self.loc_at(*span);
                let pack = match value {
                    Some(expression) => {
                        // A bare call in return position forwards its
                        // whole pack (Lua tail-position multi).
                        if let Expr::Call(callee, arguments, _) = expression {
                            self.emit_call_pack(fcx, callee, arguments, location)?
                        } else {
                            let value = self.emit_expr(fcx, expression)?;
                            self.make_pack(fcx.block, &[value], location)?
                        }
                    }
                    None => self.make_pack(fcx.block, &[], location)?,
                };
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
                let head = fcx.region.append_block(Block::new(&[]));
                let bbody = fcx.region.append_block(Block::new(&[]));
                let done = fcx.region.append_block(Block::new(&[]));
                self.br(fcx.block, head, &[], location)?;
                fcx.block = head;
                let condition_value = self.emit_expr(fcx, condition)?;
                let truthy = self
                    .call(fcx.block, "__lua_truthy", &[condition_value], &[self.i1_ty()], location)?
                    .expect("result");
                self.cond_br(fcx.block, truthy, bbody, done, location)?;
                fcx.block = bbody;
                fcx.terminated = false;
                self.emit_block(fcx, body)?;
                if !fcx.terminated {
                    self.br(fcx.block, head, &[], location)?;
                }
                fcx.block = done;
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
                self.emit_block(fcx, body)?;
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

    /// Emits a call and returns its RAW values pack (D-058).
    fn emit_call_pack<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        callee: &Expr,
        arguments: &[Expr],
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        let callee = self.emit_expr(fcx, callee)?;
        let mut values = Vec::new();
        for argument in arguments {
            values.push(self.emit_expr(fcx, argument)?);
        }
        self.call_lua(fcx.block, callee, &values, location)
    }

    // ---- expressions (every result is a dyn) ----

    fn emit_expr<'r>(&mut self, fcx: &mut Fcx<'c, 'r>, expression: &Expr) -> Result<Value<'c, 'r>> {
        let location = self.loc_at(expression.span());
        match expression {
            Expr::Nil(_) => self.nil_dyn(fcx.block, location),
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
            Expr::Function(params, body, _) => self.emit_closure(fcx, params, body, location),
            Expr::Table(fields, _) => {
                let table =
                    self.build(fcx.block, "frk_dyn.table_new", &[], &[self.dyn_ty()], &[], location)?;
                let mut position = 0i64;
                for field in fields {
                    match field {
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
        Stat::LocalFunction(name, params, body, _) => {
            bound.insert(name.clone());
            let mut inner = bound.clone();
            inner.extend(params.iter().cloned());
            let mut inner_set = inner;
            free_names_block(body, &mut inner_set, out);
        }
        Stat::GlobalFunction(_, params, body, _) => {
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
        Stat::Call(expression, _) | Stat::Return(Some(expression), _) => {
            free_names_expr(expression, bound, out);
        }
        Stat::Return(None, _) => {}
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
        Expr::Function(params, body, _) => {
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
