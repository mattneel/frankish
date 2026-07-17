//! r7rs_core → kernel dialects (M15, D-060/D-061).
//!
//! Every scheme value is a `!frk_dyn.dyn` (fixnum → num-tagged f64,
//! boolean → bool tag; the corpus stays fixnum-exact). Procedures are
//! LAMBDA-LIFTED to `func.func`s and called DIRECTLY by symbol, so tail
//! calls are M14 tail calls (the manifest's headline), not pack-closure
//! applies. Free variables (locals + escape tokens) thread through as
//! leading parameters. call/ec is the one place a closure appears: the
//! receiver of `call/cc` becomes an `fn<[i64],[dyn]>` closure over its
//! free vars and `frk_ctl.prompt` runs it — exactly the shape the ctl
//! interp verifiers use.
//!
//! Escape continuations are APPLY-ONLY in v0 (κ appears only in
//! operator position); `(k v)` lowers to `frk_ctl.abort`. Guard
//! discipline (D-061): after every NON-tail procedure call the emitter
//! threads `frk_ctl.pending` + a conditional early-return so an
//! in-flight abort propagates natively; the interpreter (real unwind)
//! sees pending==0 and the guard is inert. Tail calls are never guarded.

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

use super::ast::{Expr, LetKind, Program, Top};
use super::reader::Datum;
use super::reader::Span;

const TAG_BOOL: i64 = 1;
const TAG_NUM: i64 = 2;

type R<T> = Result<T, String>;

/// A procedure's lifted form.
#[derive(Clone)]
struct ProcInfo {
    symbol: String,
    captures: Vec<Capture>,
    params: Vec<String>,
}

#[derive(Clone)]
struct Capture {
    name: String,
    kind: CapKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CapKind {
    Val,
    Tok,
}

/// A deferred lambda-lift (emitted after the driver, so `emit_value`
/// need not hold the module). `escape = Some(k)` marks a call/ec
/// receiver closure (captures…, token) with `k` bound to the token;
/// `None` marks a plain procedure (captures…, params…).
struct Job {
    symbol: String,
    captures: Vec<Capture>,
    params: Vec<String>,
    escape: Option<String>,
    /// A dynamic-wind thunk (D-070): lifted as (captures…, pack) →
    /// pack — the uniform shape frk_ctl.wind applies.
    wind_thunk: bool,
    body: Vec<Expr>,
    procs: HashMap<String, ProcInfo>,
}

/// The enclosing function's return shape — guards and escapes must
/// early-return something well-typed (D-061/D-070).
#[derive(Clone, Copy, PartialEq, Eq)]
enum RetShape {
    Dyn,
    Void,
    Pack,
}

pub fn emit<'c>(
    context: &'c Context,
    file: &str,
    source: &str,
    program: &Program,
) -> R<Module<'c>> {
    // The seed module (M17, D-062): scheme's intrinsics are kernel IR
    // in intrinsics.mlir; the emitter appends the program around them.
    let module = crate::intrinsics::seed_module(
        context,
        "scheme",
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
        job_queue: Vec::new(),
        next_fn: 0,
    };

    // Top-level procedures see each other (mutual recursion). Bind all
    // before emitting any body.
    let mut top: HashMap<String, ProcInfo> = HashMap::new();
    for form in program {
        if let Top::Define(name, Expr::Lambda(params, _, _), _) = form {
            top.insert(
                name.clone(),
                ProcInfo {
                    symbol: format!("scm_{name}"),
                    captures: Vec::new(),
                    params: params.clone(),
                },
            );
        } else if let Top::Define(name, _, span) = form {
            return Err(format!(
                "top-level (define {name} <value>) is fenced in r7rs_core v0 at {}",
                emitter.loc_str(*span)
            ));
        }
    }
    for form in program {
        if let Top::Define(name, Expr::Lambda(params, body, _), _) = form {
            let info = top[name].clone();
            emitter.job_queue.push(Job {
                symbol: info.symbol.clone(),
                captures: Vec::new(),
                params: params.clone(),
                escape: None,
                wind_thunk: false,
                body: body.clone(),
                procs: top.clone(),
            });
        }
    }
    emitter.emit_main(&module, program, &top)?;

    while let Some(job) = emitter.job_queue.pop() {
        emitter.emit_job(&module, job)?;
    }

    if !module.as_operation().verify() {
        return Err(format!(
            "emitted scheme module failed MLIR verification:\n{}",
            module.as_operation()
        ));
    }
    Ok(module)
}

struct Emitter<'c> {
    context: &'c Context,
    file: String,
    line_starts: Vec<usize>,
    job_queue: Vec<Job>,
    next_fn: usize,
}

/// Per-function cursor. `locals` maps names to dyn/token values;
/// `procs` maps names to callable info. `returns_dyn` = the function's
/// result shape for guard early-returns.
struct Fcx<'c, 'r> {
    region: &'r Region<'c>,
    block: BlockRef<'c, 'r>,
    locals: HashMap<String, Local<'c, 'r>>,
    procs: HashMap<String, ProcInfo>,
    ret_shape: RetShape,
}

#[derive(Clone, Copy)]
enum Local<'c, 'r> {
    Val(Value<'c, 'r>),
    Tok(Value<'c, 'r>),
}

impl<'c> Emitter<'c> {
    fn dyn_ty(&self) -> Type<'c> {
        Type::parse(self.context, "!frk_dyn.dyn").expect("dyn")
    }
    fn i64_ty(&self) -> Type<'c> {
        IntegerType::new(self.context, 64).into()
    }
    fn i1_ty(&self) -> Type<'c> {
        IntegerType::new(self.context, 1).into()
    }
    fn f64_ty(&self) -> Type<'c> {
        Type::parse(self.context, "f64").expect("f64")
    }
    fn fn_ty(&self) -> Type<'c> {
        Type::parse(self.context, "!frk_closure.fn<[i64], [!frk_dyn.dyn]>").expect("fn")
    }

    fn line_col(&self, offset: usize) -> (usize, usize) {
        match self.line_starts.binary_search(&offset) {
            Ok(index) => (index + 1, 1),
            Err(index) => (index, offset - self.line_starts[index - 1] + 1),
        }
    }
    fn loc_at(&self, span: Span) -> Location<'c> {
        let (line, col) = self.line_col(span.start);
        Location::new(self.context, &self.file, line, col)
    }
    fn loc_str(&self, span: Span) -> String {
        let (line, col) = self.line_col(span.start);
        format!("{}:{}:{}", self.file, line, col)
    }

    fn op1<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        op: melior::ir::Operation<'c>,
    ) -> R<Value<'c, 'r>> {
        let inserted = block.append_operation(op);
        let raw = inserted.result(0).map_err(|_| "op has no result".to_string())?.to_raw();
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
    ) -> R<Value<'c, 'r>> {
        let mut builder =
            OperationBuilder::new(name, location).add_operands(operands).add_results(results);
        for (key, attribute) in attributes {
            builder = builder.add_attributes(&[(Identifier::new(self.context, key), *attribute)]);
        }
        self.op1(block, builder.build().map_err(|e| e.to_string())?)
    }

    fn const_i64<'r>(&self, b: BlockRef<'c, 'r>, v: i64, l: Location<'c>) -> R<Value<'c, 'r>> {
        self.op1(
            b,
            melior::dialect::arith::constant(
                self.context,
                IntegerAttribute::new(self.i64_ty(), v).into(),
                l,
            ),
        )
    }
    fn const_f64<'r>(&self, b: BlockRef<'c, 'r>, v: f64, l: Location<'c>) -> R<Value<'c, 'r>> {
        let attr = Attribute::parse(self.context, &format!("{v:?} : f64")).ok_or("f64 attr")?;
        self.op1(b, melior::dialect::arith::constant(self.context, attr, l))
    }
    fn const_i1<'r>(&self, b: BlockRef<'c, 'r>, v: bool, l: Location<'c>) -> R<Value<'c, 'r>> {
        self.op1(
            b,
            melior::dialect::arith::constant(
                self.context,
                IntegerAttribute::new(self.i1_ty(), v as i64).into(),
                l,
            ),
        )
    }

    fn wrap<'r>(
        &self,
        b: BlockRef<'c, 'r>,
        tag: i64,
        v: Value<'c, 'r>,
        l: Location<'c>,
    ) -> R<Value<'c, 'r>> {
        self.build(
            b,
            "frk_dyn.wrap",
            &[v],
            &[self.dyn_ty()],
            &[("tag", IntegerAttribute::new(self.i64_ty(), tag).into())],
            l,
        )
    }
    fn unwrap<'r>(
        &self,
        b: BlockRef<'c, 'r>,
        tag: i64,
        result: Type<'c>,
        v: Value<'c, 'r>,
        l: Location<'c>,
    ) -> R<Value<'c, 'r>> {
        self.build(
            b,
            "frk_dyn.unwrap",
            &[v],
            &[result],
            &[("tag", IntegerAttribute::new(self.i64_ty(), tag).into())],
            l,
        )
    }
    fn tag_of<'r>(&self, b: BlockRef<'c, 'r>, v: Value<'c, 'r>, l: Location<'c>) -> R<Value<'c, 'r>> {
        self.build(b, "frk_dyn.tag_of", &[v], &[self.i64_ty()], &[], l)
    }
    fn num_dyn<'r>(&self, b: BlockRef<'c, 'r>, v: f64, l: Location<'c>) -> R<Value<'c, 'r>> {
        let n = self.const_f64(b, v, l)?;
        self.wrap(b, TAG_NUM, n, l)
    }
    fn bool_dyn<'r>(&self, b: BlockRef<'c, 'r>, v: bool, l: Location<'c>) -> R<Value<'c, 'r>> {
        let x = self.const_i1(b, v, l)?;
        self.wrap(b, TAG_BOOL, x, l)
    }
    /// A throwaway dyn for dead early-return slots (guards/aborts).
    fn dummy_dyn<'r>(&self, b: BlockRef<'c, 'r>, l: Location<'c>) -> R<Value<'c, 'r>> {
        self.num_dyn(b, 0.0, l)
    }

    fn cond_br<'r>(
        &self,
        b: BlockRef<'c, 'r>,
        c: Value<'c, 'r>,
        t: BlockRef<'c, 'r>,
        f: BlockRef<'c, 'r>,
        l: Location<'c>,
    ) -> R<()> {
        b.append_operation(
            OperationBuilder::new("cf.cond_br", l)
                .add_attributes(&[(
                    Identifier::new(self.context, "operandSegmentSizes"),
                    DenseI32ArrayAttribute::new(self.context, &[1, 0, 0]).into(),
                )])
                .add_operands(&[c])
                .add_successors(&[&t, &f])
                .build()
                .map_err(|e| e.to_string())?,
        );
        Ok(())
    }
    fn br<'r>(
        &self,
        b: BlockRef<'c, 'r>,
        target: BlockRef<'c, 'r>,
        values: &[Value<'c, 'r>],
        l: Location<'c>,
    ) -> R<()> {
        b.append_operation(
            OperationBuilder::new("cf.br", l)
                .add_operands(values)
                .add_successors(&[&target])
                .build()
                .map_err(|e| e.to_string())?,
        );
        Ok(())
    }
    fn call<'r>(
        &self,
        b: BlockRef<'c, 'r>,
        callee: &str,
        operands: &[Value<'c, 'r>],
        results: &[Type<'c>],
        l: Location<'c>,
    ) -> R<Option<Value<'c, 'r>>> {
        let attribute: Attribute = FlatSymbolRefAttribute::new(self.context, callee).into();
        let op = OperationBuilder::new("func.call", l)
            .add_operands(operands)
            .add_results(results)
            .add_attributes(&[(Identifier::new(self.context, "callee"), attribute)])
            .build()
            .map_err(|e| e.to_string())?;
        if results.is_empty() {
            b.append_operation(op);
            Ok(None)
        } else {
            Ok(Some(self.op1(b, op)?))
        }
    }

    fn func(
        &self,
        module: &Module<'c>,
        name: &str,
        inputs: &[Type<'c>],
        outputs: &[Type<'c>],
        region: Region<'c>,
        entry_iface: bool,
    ) {
        let mut attrs = Vec::new();
        if entry_iface {
            attrs.push((
                Identifier::new(self.context, "llvm.emit_c_interface"),
                Attribute::unit(self.context),
            ));
        }
        module.body().append_operation(melior::dialect::func::func(
            self.context,
            StringAttribute::new(self.context, name),
            TypeAttribute::new(FunctionType::new(self.context, inputs, outputs).into()),
            region,
            &attrs,
            Location::unknown(self.context),
        ));
    }

    fn ret_raw<'r>(&self, b: BlockRef<'c, 'r>, values: &[Value<'c, 'r>], l: Location<'c>) -> R<()> {
        self.ret(b, values, l)
    }

    fn ret<'r>(&self, b: BlockRef<'c, 'r>, values: &[Value<'c, 'r>], l: Location<'c>) -> R<()> {
        b.append_operation(
            OperationBuilder::new("func.return", l)
                .add_operands(values)
                .build()
                .map_err(|e| e.to_string())?,
        );
        Ok(())
    }

    /// scheme truthiness: only `#f` is false; everything else (incl. 0)
    /// is true. Emits an i1; may split `fcx.block`.
    fn truthy<'r>(
        &self,
        fcx: &mut Fcx<'c, 'r>,
        value: Value<'c, 'r>,
        l: Location<'c>,
    ) -> R<Value<'c, 'r>> {
        let tag = self.tag_of(fcx.block, value, l)?;
        let bool_tag = self.const_i64(fcx.block, TAG_BOOL, l)?;
        let is_bool = self.build(
            fcx.block,
            "arith.cmpi",
            &[tag, bool_tag],
            &[self.i1_ty()],
            &[("predicate", IntegerAttribute::new(self.i64_ty(), 0).into())], // eq
            l,
        )?;
        let bbool = fcx.region.append_block(Block::new(&[]));
        let btrue = fcx.region.append_block(Block::new(&[]));
        let join = fcx.region.append_block(Block::new(&[(self.i1_ty(), l)]));
        self.cond_br(fcx.block, is_bool, bbool, btrue, l)?;
        let payload = self.unwrap(bbool, TAG_BOOL, self.i1_ty(), value, l)?;
        self.br(bbool, join, &[payload], l)?;
        let t = self.const_i1(btrue, true, l)?;
        self.br(btrue, join, &[t], l)?;
        fcx.block = join;
        block_arg(join, 0)
    }

    // ---- deferred lambda-lifting ----

    /// Emits one queued job: a procedure `@sym(captures…, params…) ->
    /// dyn`, or (escape=Some) a call/ec receiver `@sym(captures…,
    /// token:i64) -> dyn` with the escape name bound to the token.
    fn emit_job(&mut self, module: &Module<'c>, job: Job) -> R<()> {
        let l = Location::unknown(self.context);
        let mut inputs = Vec::new();
        for capture in &job.captures {
            inputs.push(match capture.kind {
                CapKind::Val => self.dyn_ty(),
                CapKind::Tok => self.i64_ty(),
            });
        }
        if job.escape.is_some() {
            inputs.push(self.i64_ty());
        } else if job.wind_thunk {
            // Uniform pack fn (D-070/D-071): parameters — if any —
            // read out of the pack via __scm_arg (nil-fill).
            inputs.push(self.pack_ty());
        } else {
            inputs.extend(std::iter::repeat_n(self.dyn_ty(), job.params.len()));
        }

        let region = Region::new();
        let entry = region
            .append_block(Block::new(&inputs.iter().map(|t| (*t, l)).collect::<Vec<_>>()));
        let mut locals: HashMap<String, Local<'c, '_>> = HashMap::new();
        for (index, capture) in job.captures.iter().enumerate() {
            let v = block_arg(entry, index)?;
            locals.insert(
                capture.name.clone(),
                match capture.kind {
                    CapKind::Val => Local::Val(v),
                    CapKind::Tok => Local::Tok(v),
                },
            );
        }
        if let Some(escape) = &job.escape {
            let token = block_arg(entry, job.captures.len())?;
            locals.insert(escape.clone(), Local::Tok(token));
        } else {
            for (index, name) in job.params.iter().enumerate() {
                let v = block_arg(entry, job.captures.len() + index)?;
                locals.insert(name.clone(), Local::Val(v));
            }
        }
        let mut fcx =
            Fcx {
                region: &region,
                block: entry,
                locals,
                procs: job.procs.clone(),
                ret_shape: if job.wind_thunk { RetShape::Pack } else { RetShape::Dyn },
            };
        if job.wind_thunk {
            // Body evaluates as a sequence; the final value returns as
            // a ONE-element pack (the uniform shape). Divergence (an
            // escape inside the thunk) has already early-returned.
            let l = Location::unknown(self.context);
            let pack_arg = block_arg(entry, job.captures.len())?;
            for (index, name) in job.params.clone().iter().enumerate() {
                let idx = self.const_i64(fcx.block, index as i64, l)?;
                let value = self
                    .call(fcx.block, "__scm_arg", &[pack_arg, idx], &[self.dyn_ty()], l)?
                    .ok_or("__scm_arg produced no value")?;
                fcx.locals.insert(name.clone(), Local::Val(value));
            }
            if let Some(value) = self.emit_seq_value(&mut fcx, &job.body.clone())? {
                let one = self.const_i64(fcx.block, 1, l)?;
                let pack = self.build(
                    fcx.block,
                    "frk_mem.array_new",
                    &[one],
                    &[self.pack_ty()],
                    &[],
                    l,
                )?;
                let zero = self.const_i64(fcx.block, 0, l)?;
                self.build(
                    fcx.block,
                    "frk_mem.array_set",
                    &[pack, zero, value],
                    &[],
                    &[],
                    l,
                )
                .ok();
                self.ret_raw(fcx.block, &[pack], l)?;
            }
            self.func(module, &job.symbol, &inputs, &[self.pack_ty()], region, false);
            return Ok(());
        }
        self.emit_body_tail(&mut fcx, &job.body)?;
        self.func(module, &job.symbol, &inputs, &[self.dyn_ty()], region, false);
        Ok(())
    }

    // ---- the driver ----

    fn emit_main(
        &mut self,
        module: &Module<'c>,
        program: &Program,
        top: &HashMap<String, ProcInfo>,
    ) -> R<()> {
        let l = Location::unknown(self.context);
        let region = Region::new();
        let entry = region.append_block(Block::new(&[]));
        let mut fcx = Fcx {
            region: &region,
            block: entry,
            locals: HashMap::new(),
            procs: top.clone(),
            ret_shape: RetShape::Void,
        };
        for form in program {
            if let Top::Expr(expr) = form {
                if self.emit_value(&mut fcx, expr)?.is_none() {
                    break;
                }
            }
        }
        self.ret(fcx.block, &[], l)?;
        self.func(module, "main", &[], &[], region, true);
        Ok(())
    }

    // ---- expression emission ----

    /// Non-tail: returns the expression's dyn value, or None if control
    /// diverged (an escape fired; the block is already terminated).
    fn emit_value<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        expr: &Expr,
    ) -> R<Option<Value<'c, 'r>>> {
        let l = self.loc_at(expr.span());
        match expr {
            Expr::Int(v, _) => Ok(Some(self.num_dyn(fcx.block, *v as f64, l)?)),
            Expr::Bool(v, _) => Ok(Some(self.bool_dyn(fcx.block, *v, l)?)),
            Expr::Var(name, _) => match fcx.locals.get(name) {
                Some(Local::Val(v)) => Ok(Some(*v)),
                Some(Local::Tok(_)) => {
                    Err(format!("escape continuation `{name}` used as a value (fenced, v0)"))
                }
                None => Err(format!("unbound variable `{name}` (procedures-as-values are fenced in v0)")),
            },
            Expr::If(c, t, e, _) => self.emit_if_value(fcx, c, t, e, l),
            Expr::Begin(exprs, _) => self.emit_seq_value(fcx, exprs),
            Expr::Let(kind, binds, body, _) => self.emit_let_value(fcx, *kind, binds, body),
            Expr::App(callee, args, _) => self.emit_app_value(fcx, callee, args, l),
            Expr::Quote(datum, _) => self.emit_quoted(fcx, datum, l).map(Some),
            Expr::Lambda(..) => {
                // First-class lambdas (M26, D-071): a uniform pack-fn
                // closure wrapped as a fun dyn.
                let closure = self.emit_lambda_packfn(fcx, expr, l)?;
                Ok(Some(self.wrap(fcx.block, 5, closure, l)?))
            }
        }
    }

    /// Tail position: terminates `fcx.block` with a return (or a
    /// recursive tail construct). Never guards.
    fn emit_tail<'r>(&mut self, fcx: &mut Fcx<'c, 'r>, expr: &Expr) -> R<()> {
        let l = self.loc_at(expr.span());
        match expr {
            Expr::If(c, t, e, _) => {
                let cond = match self.emit_value(fcx, c)? {
                    Some(v) => v,
                    None => return Ok(()),
                };
                let truthy = self.truthy(fcx, cond, l)?;
                let bt = fcx.region.append_block(Block::new(&[]));
                let be = fcx.region.append_block(Block::new(&[]));
                self.cond_br(fcx.block, truthy, bt, be, l)?;
                fcx.block = bt;
                self.emit_tail(fcx, t)?;
                fcx.block = be;
                self.emit_tail(fcx, e)?;
                Ok(())
            }
            Expr::Begin(exprs, _) => {
                let (last, init) = exprs.split_last().ok_or("empty begin")?;
                for e in init {
                    if self.emit_value(fcx, e)?.is_none() {
                        return Ok(());
                    }
                }
                self.emit_tail(fcx, last)
            }
            Expr::Let(kind, binds, body, _) => {
                if self.bind_let(fcx, *kind, binds)?.is_none() {
                    return Ok(());
                }
                let (last, init) = body.split_last().ok_or("empty let body")?;
                for e in init {
                    if self.emit_value(fcx, e)?.is_none() {
                        return Ok(());
                    }
                }
                self.emit_tail(fcx, last)
            }
            Expr::App(callee, args, _) => self.emit_app_tail(fcx, callee, args, l),
            // literals / var / (values that just return)
            _ => {
                if let Some(v) = self.emit_value(fcx, expr)? {
                    self.ret(fcx.block, &[v], l)?;
                }
                Ok(())
            }
        }
    }

    fn emit_body_tail<'r>(&mut self, fcx: &mut Fcx<'c, 'r>, body: &[Expr]) -> R<()> {
        let (last, init) = body.split_last().ok_or("empty body")?;
        for e in init {
            if self.emit_value(fcx, e)?.is_none() {
                return Ok(());
            }
        }
        self.emit_tail(fcx, last)
    }

    fn emit_seq_value<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        exprs: &[Expr],
    ) -> R<Option<Value<'c, 'r>>> {
        let (last, init) = exprs.split_last().ok_or("empty begin")?;
        for e in init {
            if self.emit_value(fcx, e)?.is_none() {
                return Ok(None);
            }
        }
        self.emit_value(fcx, last)
    }

    fn emit_if_value<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        c: &Expr,
        t: &Expr,
        e: &Expr,
        l: Location<'c>,
    ) -> R<Option<Value<'c, 'r>>> {
        let cond = match self.emit_value(fcx, c)? {
            Some(v) => v,
            None => return Ok(None),
        };
        let truthy = self.truthy(fcx, cond, l)?;
        let bt = fcx.region.append_block(Block::new(&[]));
        let be = fcx.region.append_block(Block::new(&[]));
        let join = fcx.region.append_block(Block::new(&[(self.dyn_ty(), l)]));
        self.cond_br(fcx.block, truthy, bt, be, l)?;
        fcx.block = bt;
        if let Some(v) = self.emit_value(fcx, t)? {
            self.br(fcx.block, join, &[v], l)?;
        }
        fcx.block = be;
        if let Some(v) = self.emit_value(fcx, e)? {
            self.br(fcx.block, join, &[v], l)?;
        }
        fcx.block = join;
        Ok(Some(block_arg(join, 0)?))
    }

    fn emit_let_value<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        kind: LetKind,
        binds: &[(String, Expr)],
        body: &[Expr],
    ) -> R<Option<Value<'c, 'r>>> {
        if self.bind_let(fcx, kind, binds)?.is_none() {
            return Ok(None);
        }
        self.emit_seq_value(fcx, body)
    }

    /// Binds a let/let*/letrec's bindings into `fcx.locals`/`fcx.procs`.
    /// Returns None if a binding initializer diverged.
    fn bind_let<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        kind: LetKind,
        binds: &[(String, Expr)],
    ) -> R<Option<()>> {
        // letrec of lambdas → mutually-recursive lifted procedures.
        if kind == LetKind::LetRec && binds.iter().all(|(_, e)| matches!(e, Expr::Lambda(..))) {
            self.bind_letrec_procs(fcx, binds)?;
            return Ok(Some(()));
        }
        // let / let* / letrec of values: sequential or parallel value
        // bindings (v0 treats let and let* alike for the corpus — no
        // shadowing hazard in the cases; letrec-of-values is rare).
        for (name, init) in binds {
            let v = match self.emit_value(fcx, init)? {
                Some(v) => v,
                None => return Ok(None),
            };
            fcx.locals.insert(name.clone(), Local::Val(v));
        }
        Ok(Some(()))
    }

    /// Lifts a letrec group of lambdas: shared capture set = the union
    /// of every binding's free locals/tokens, so any sibling can call
    /// any other by passing the shared captures.
    fn bind_letrec_procs<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        binds: &[(String, Expr)],
    ) -> R<()> {
        let sibling_names: HashSet<String> = binds.iter().map(|(n, _)| n.clone()).collect();
        // Union of free vars across all bodies, minus siblings + own
        // params, resolving to enclosing locals.
        let mut union: BTreeSet<String> = BTreeSet::new();
        for (_, lambda) in binds {
            if let Expr::Lambda(params, body, _) = lambda {
                let mut bound: HashSet<String> = params.iter().cloned().collect();
                bound.extend(sibling_names.iter().cloned());
                let mut free = BTreeSet::new();
                free_names_body(body, &mut bound, &mut free);
                union.extend(free);
            }
        }
        let captures: Vec<Capture> = union
            .into_iter()
            .filter_map(|name| {
                fcx.locals.get(&name).map(|local| Capture {
                    name: name.clone(),
                    kind: match local {
                        Local::Val(_) => CapKind::Val,
                        Local::Tok(_) => CapKind::Tok,
                    },
                })
            })
            .collect();

        // Register all siblings (so bodies can reference each other),
        // then queue each with the shared capture set.
        let mut members = Vec::new();
        for (name, lambda) in binds {
            if let Expr::Lambda(params, _, _) = lambda {
                let symbol = format!("scm_letrec{}_{}", self.next_fn, name);
                self.next_fn += 1;
                members.push((
                    name.clone(),
                    ProcInfo { symbol, captures: captures.clone(), params: params.clone() },
                    lambda.clone(),
                ));
            }
        }
        for (name, info, _) in &members {
            fcx.procs.insert(name.clone(), info.clone());
        }
        // The lifted bodies see the enclosing top procs plus all
        // siblings.
        let mut inner_procs = fcx.procs.clone();
        for (name, info, _) in &members {
            inner_procs.insert(name.clone(), info.clone());
        }
        for (_, info, lambda) in members {
            if let Expr::Lambda(params, body, _) = lambda {
                self.job_queue.push(Job {
                    symbol: info.symbol.clone(),
                    captures: captures.clone(),
                    params,
                    escape: None,
                    wind_thunk: false,
                    body,
                    procs: inner_procs.clone(),
                });
            }
        }
        Ok(())
    }

    // ---- applications ----

    fn emit_app_value<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        callee: &Expr,
        args: &[Expr],
        l: Location<'c>,
    ) -> R<Option<Value<'c, 'r>>> {
        let Expr::Var(op, _) = callee else {
            return Err("only symbol operators are supported in r7rs_core v0".into());
        };
        // call/cc is a special form.
        if op == "call/cc" || op == "call-with-current-continuation" || op == "call/ec" {
            return self.emit_callcc(fcx, args, l);
        }
        if let Some(v) = self.emit_primitive(fcx, op, args, l)? {
            return Ok(v);
        }
        // Escape application: (k v) → abort; never returns.
        if let Some(Local::Tok(token)) = fcx.locals.get(op).copied() {
            self.emit_escape(fcx, token, args, l)?;
            return Ok(None);
        }
        // Procedure application.
        if let Some(info) = fcx.procs.get(op).cloned() {
            let operands = match self.eval_call_operands(fcx, &info, args)? {
                Some(v) => v,
                None => return Ok(None),
            };
            let result = self
                .call(fcx.block, &info.symbol, &operands, &[self.dyn_ty()], l)?
                .ok_or("proc call produced no value")?;
            // Non-tail call → guard for a propagating abort.
            self.emit_guard(fcx, l)?;
            return Ok(Some(result));
        }
        // First-class application (M26, D-071): the operator evaluates
        // to a fun dyn; apply through the uniform convention with an
        // args pack, result = head, guard after (the callee may abort).
        if fcx.locals.contains_key(op) {
            let callee = match self.emit_value(fcx, &Expr::Var(op.clone(), Span { start: 0, end: 0 }))? {
                Some(v) => v,
                None => return Ok(None),
            };
            return self.emit_apply_dyn(fcx, callee, args, l);
        }
        Err(format!("unbound operator `{op}`"))
    }

    /// (with-exception-handler h t) as handle{label="exn"} over the
    /// STATIC wrapper intrinsics, per-site closures carrying h / t in
    /// their envs (M26, D-071).
    fn emit_handle_exn<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        handler: Value<'c, 'r>,
        thunk: Value<'c, 'r>,
        l: Location<'c>,
    ) -> R<Value<'c, 'r>> {
        let empty = Type::parse(self.context, "!frk_adt.product<[]>").ok_or("empty product")?;
        let dyn_product =
            Type::parse(self.context, "!frk_adt.product<[!frk_dyn.dyn]>").ok_or("dyn product")?;
        let mut make = |value: Value<'c, 'r>, callee: &str, ty: Type<'c>| -> R<Value<'c, 'r>> {
            let base = self.build(fcx.block, "frk_adt.product_new", &[], &[empty], &[], l)?;
            let env = self.build(
                fcx.block,
                "frk_adt.product_snoc",
                &[base, value],
                &[dyn_product],
                &[],
                l,
            )?;
            self.build(
                fcx.block,
                "frk_closure.make",
                &[env],
                &[ty],
                &[("callee", FlatSymbolRefAttribute::new(self.context, callee).into())],
                l,
            )
        };
        let clause = make(handler, "__scm_exn_clause", self.pack_fn_ty())?;
        let body = make(thunk, "__scm_exn_body", self.fn_ty())?;
        self.build(
            fcx.block,
            "frk_ctl.handle",
            &[clause, body],
            &[self.dyn_ty()],
            &[("label", StringAttribute::new(self.context, "exn").into())],
            l,
        )
    }

    /// Applies a fun-dyn value to evaluated args (M26): unwrap at the
    /// uniform type, build the pack, closure.apply, head via
    /// __scm_arg, guard.
    fn emit_apply_dyn<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        callee: Value<'c, 'r>,
        args: &[Expr],
        l: Location<'c>,
    ) -> R<Option<Value<'c, 'r>>> {
        let function = self.build(
            fcx.block,
            "frk_dyn.unwrap",
            &[callee],
            &[self.pack_fn_ty()],
            &[("tag", IntegerAttribute::new(self.i64_ty(), 5).into())],
            l,
        )?;
        let count = self.const_i64(fcx.block, args.len() as i64, l)?;
        let pack = self.build(
            fcx.block,
            "frk_mem.array_new",
            &[count],
            &[self.pack_ty()],
            &[],
            l,
        )?;
        for (index, arg) in args.iter().enumerate() {
            let value = match self.emit_value(fcx, arg)? {
                Some(v) => v,
                None => return Ok(None),
            };
            let idx = self.const_i64(fcx.block, index as i64, l)?;
            self.build(fcx.block, "frk_mem.array_set", &[pack, idx, value], &[], &[], l)
                .ok();
        }
        let empty = Type::parse(self.context, "!frk_adt.product<[]>").ok_or("empty product")?;
        let product = self.build(fcx.block, "frk_adt.product_new", &[], &[empty], &[], l)?;
        let wrapped = Type::parse(
            self.context,
            "!frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>",
        )
        .ok_or("arg product")?;
        let arg_product = self.build(
            fcx.block,
            "frk_adt.product_snoc",
            &[product, pack],
            &[wrapped],
            &[],
            l,
        )?;
        let result_pack = self.build(
            fcx.block,
            "frk_closure.apply",
            &[function, arg_product],
            &[self.pack_ty()],
            &[],
            l,
        )?;
        let zero = self.const_i64(fcx.block, 0, l)?;
        let head = self
            .call(fcx.block, "__scm_arg", &[result_pack, zero], &[self.dyn_ty()], l)?
            .ok_or("__scm_arg produced no value")?;
        self.emit_guard(fcx, l)?;
        Ok(Some(head))
    }

    fn emit_app_tail<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        callee: &Expr,
        args: &[Expr],
        l: Location<'c>,
    ) -> R<()> {
        let Expr::Var(op, _) = callee else {
            return Err("only symbol operators are supported in r7rs_core v0".into());
        };
        // Direct procedure tail call → func.call feeding func.return
        // (M14 tail shape; no guard).
        if op != "call/cc"
            && op != "call-with-current-continuation"
            && op != "call/ec"
            && !is_primitive(op)
            && fcx.locals.get(op).map(|b| matches!(b, Local::Tok(_))) != Some(true)
        {
            if let Some(info) = fcx.procs.get(op).cloned() {
                let operands = match self.eval_call_operands(fcx, &info, args)? {
                    Some(v) => v,
                    None => return Ok(()),
                };
                let result = self
                    .call(fcx.block, &info.symbol, &operands, &[self.dyn_ty()], l)?
                    .ok_or("proc call produced no value")?;
                self.ret(fcx.block, &[result], l)?;
                return Ok(());
            }
        }
        // Everything else: evaluate as a value, then return it (escape
        // diverts on its own).
        if let Some(v) = self.emit_app_value(fcx, callee, args, l)? {
            self.ret(fcx.block, &[v], l)?;
        }
        Ok(())
    }

    /// Gathers a proc's capture values (from the current scope) followed
    /// by the evaluated argument dyns. None if an argument diverged.
    fn eval_call_operands<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        info: &ProcInfo,
        args: &[Expr],
    ) -> R<Option<Vec<Value<'c, 'r>>>> {
        let mut operands = Vec::new();
        for capture in &info.captures {
            match fcx.locals.get(&capture.name).copied() {
                Some(Local::Val(v)) | Some(Local::Tok(v)) => operands.push(v),
                None => return Err(format!("capture `{}` not in scope at call", capture.name)),
            }
        }
        if args.len() != info.params.len() {
            return Err(format!(
                "procedure `{}` expects {} args, got {}",
                info.symbol,
                info.params.len(),
                args.len()
            ));
        }
        for arg in args {
            match self.emit_value(fcx, arg)? {
                Some(v) => operands.push(v),
                None => return Ok(None),
            }
        }
        Ok(Some(operands))
    }

    fn emit_escape<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        token: Value<'c, 'r>,
        args: &[Expr],
        l: Location<'c>,
    ) -> R<()> {
        let value = match args {
            [one] => match self.emit_value(fcx, one)? {
                Some(v) => v,
                None => return Ok(()),
            },
            _ => return Err("an escape continuation takes exactly one argument in v0".into()),
        };
        fcx.block.append_operation(
            OperationBuilder::new("frk_ctl.abort", l)
                .add_operands(&[token, value])
                .build()
                .map_err(|e| e.to_string())?,
        );
        // divert: return a dummy (dead — pending is set).
        self.emit_early_return(fcx, l)
    }

    /// Emits the post-non-tail-call guard: if an abort is pending,
    /// early-return; otherwise fall through. Splits `fcx.block`.
    fn emit_guard<'r>(&mut self, fcx: &mut Fcx<'c, 'r>, l: Location<'c>) -> R<()> {
        let pending = self
            .build(fcx.block, "frk_ctl.pending", &[], &[self.i64_ty()], &[], l)?;
        let zero = self.const_i64(fcx.block, 0, l)?;
        let is_pending = self.build(
            fcx.block,
            "arith.cmpi",
            &[pending, zero],
            &[self.i1_ty()],
            &[("predicate", IntegerAttribute::new(self.i64_ty(), 1).into())], // ne
            l,
        )?;
        let prop = fcx.region.append_block(Block::new(&[]));
        let cont = fcx.region.append_block(Block::new(&[]));
        self.cond_br(fcx.block, is_pending, prop, cont, l)?;
        fcx.block = prop;
        self.emit_early_return(fcx, l)?;
        fcx.block = cont;
        Ok(())
    }

    /// Returns the enclosing function's dead dummy (dyn) or nothing
    /// (void @main) — reached only when an abort is propagating.
    fn emit_early_return<'r>(&mut self, fcx: &mut Fcx<'c, 'r>, l: Location<'c>) -> R<()> {
        match fcx.ret_shape {
            RetShape::Dyn => {
                let dummy = self.dummy_dyn(fcx.block, l)?;
                self.ret(fcx.block, &[dummy], l)
            }
            RetShape::Void => self.ret(fcx.block, &[], l),
            RetShape::Pack => {
                let zero = self.const_i64(fcx.block, 0, l)?;
                let empty = self.build(
                    fcx.block,
                    "frk_mem.array_new",
                    &[zero],
                    &[self.pack_ty()],
                    &[],
                    l,
                )?;
                self.ret(fcx.block, &[empty], l)
            }
        }
    }

    fn pack_ty(&self) -> Type<'c> {
        Type::parse(self.context, "!frk_mem.arr<!frk_dyn.dyn>").expect("pack")
    }
    fn pack_fn_ty(&self) -> Type<'c> {
        Type::parse(
            self.context,
            "!frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>",
        )
        .expect("pack fn")
    }
    fn nil_dyn<'r>(&self, b: BlockRef<'c, 'r>, l: Location<'c>) -> R<Value<'c, 'r>> {
        let w = self.const_i64(b, 0, l)?;
        self.wrap(b, 0, w, l)
    }
    fn symbol_dyn<'r>(&self, b: BlockRef<'c, 'r>, name: &str, l: Location<'c>) -> R<Value<'c, 'r>> {
        let lit = self.build(
            b,
            "frk_bstr.lit",
            &[],
            &[Type::parse(self.context, "!frk_bstr.str").ok_or("bstr type")?],
            &[("text", StringAttribute::new(self.context, name).into())],
            l,
        )?;
        self.wrap(b, 3, lit, l)
    }

    /// Quoted data (D-070): fixnums, booleans, symbols (interned
    /// bstrs), and proper/improper lists via right-folded cons.
    fn emit_quoted<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        datum: &Datum,
        l: Location<'c>,
    ) -> R<Value<'c, 'r>> {
        match datum {
            Datum::Int(v, _) => self.num_dyn(fcx.block, *v as f64, l),
            Datum::Bool(v, _) => self.bool_dyn(fcx.block, *v, l),
            Datum::Symbol(name, _) => self.symbol_dyn(fcx.block, name, l),
            Datum::List(items, _) => {
                let mut acc = self.nil_dyn(fcx.block, l)?;
                for item in items.iter().rev() {
                    let head = self.emit_quoted(fcx, item, l)?;
                    acc = self
                        .call(fcx.block, "__scm_cons", &[head, acc], &[self.dyn_ty()], l)?
                        .ok_or("cons produced no value")?;
                }
                Ok(acc)
            }
        }
    }

    /// Lifts a zero-parameter lambda as a WIND THUNK closure —
    /// (captures…, pack) → pack, the uniform shape frk_ctl.wind
    /// applies (D-070).
    fn emit_wind_thunk<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        lambda: &Expr,
        l: Location<'c>,
    ) -> R<Value<'c, 'r>> {
        let Expr::Lambda(params, _, _) = lambda else {
            return Err("dynamic-wind takes three (lambda () …) thunks in v0.1".into());
        };
        if !params.is_empty() {
            return Err("dynamic-wind thunks take no parameters".into());
        }
        self.emit_lambda_packfn(fcx, lambda, l)
    }

    /// Lifts ANY lambda as a uniform pack-fn closure (M26, D-071):
    /// (captures…, pack) → pack, parameters read via __scm_arg —
    /// the shape wind applies directly and first-class values wrap.
    fn emit_lambda_packfn<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        lambda: &Expr,
        l: Location<'c>,
    ) -> R<Value<'c, 'r>> {
        let Expr::Lambda(params, body, _) = lambda else {
            return Err("expected a lambda".into());
        };
        let mut bound: HashSet<String> = params.iter().cloned().collect();
        let mut free = BTreeSet::new();
        free_names_body(body, &mut bound, &mut free);
        let captures: Vec<Capture> = free
            .into_iter()
            .filter_map(|name| {
                fcx.locals.get(&name).map(|local| Capture {
                    name,
                    kind: match local {
                        Local::Val(_) => CapKind::Val,
                        Local::Tok(_) => CapKind::Tok,
                    },
                })
            })
            .collect();
        let symbol = format!("scm_fn{}", self.next_fn);
        self.next_fn += 1;
        let empty = Type::parse(self.context, "!frk_adt.product<[]>").ok_or("empty product")?;
        let mut env = self.build(fcx.block, "frk_adt.product_new", &[], &[empty], &[], l)?;
        let mut field_types: Vec<String> = Vec::new();
        for capture in &captures {
            let value = match fcx.locals.get(&capture.name).copied() {
                Some(Local::Val(v)) | Some(Local::Tok(v)) => v,
                None => return Err(format!("capture `{}` not in scope", capture.name)),
            };
            field_types.push(match capture.kind {
                CapKind::Val => "!frk_dyn.dyn".to_string(),
                CapKind::Tok => "i64".to_string(),
            });
            let product_ty = Type::parse(
                self.context,
                &format!("!frk_adt.product<[{}]>", field_types.join(", ")),
            )
            .ok_or("product type")?;
            env = self.build(
                fcx.block,
                "frk_adt.product_snoc",
                &[env, value],
                &[product_ty],
                &[],
                l,
            )?;
        }
        let closure = self.build(
            fcx.block,
            "frk_closure.make",
            &[env],
            &[self.pack_fn_ty()],
            &[("callee", FlatSymbolRefAttribute::new(self.context, &symbol).into())],
            l,
        )?;
        self.job_queue.push(Job {
            symbol,
            captures,
            params: params.clone(),
            escape: None,
            wind_thunk: true,
            body: body.clone(),
            procs: fcx.procs.clone(),
        });
        Ok(closure)
    }

    fn emit_callcc<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        args: &[Expr],
        l: Location<'c>,
    ) -> R<Option<Value<'c, 'r>>> {
        let [Expr::Lambda(params, body, _)] = args else {
            return Err("call/cc takes one (lambda (k) …) receiver in v0".into());
        };
        let [escape] = params.as_slice() else {
            return Err("the call/cc receiver takes exactly one parameter (the escape)".into());
        };
        // Receiver captures = free vars minus the escape, resolving to
        // enclosing locals.
        let mut bound: HashSet<String> = HashSet::new();
        bound.insert(escape.clone());
        let mut free = BTreeSet::new();
        free_names_body(body, &mut bound, &mut free);
        let captures: Vec<Capture> = free
            .into_iter()
            .filter_map(|name| {
                fcx.locals.get(&name).map(|local| Capture {
                    name,
                    kind: match local {
                        Local::Val(_) => CapKind::Val,
                        Local::Tok(_) => CapKind::Tok,
                    },
                })
            })
            .collect();
        let symbol = format!("scm_recv{}", self.next_fn);
        self.next_fn += 1;
        // Build the closure env product from the capture VALUES.
        let empty = Type::parse(self.context, "!frk_adt.product<[]>").ok_or("empty product")?;
        let mut env = self.build(fcx.block, "frk_adt.product_new", &[], &[empty], &[], l)?;
        let mut field_types: Vec<String> = Vec::new();
        for capture in &captures {
            let value = match fcx.locals.get(&capture.name).copied() {
                Some(Local::Val(v)) | Some(Local::Tok(v)) => v,
                None => return Err(format!("capture `{}` not in scope", capture.name)),
            };
            field_types.push(match capture.kind {
                CapKind::Val => "!frk_dyn.dyn".to_string(),
                CapKind::Tok => "i64".to_string(),
            });
            let product_ty = Type::parse(
                self.context,
                &format!("!frk_adt.product<[{}]>", field_types.join(", ")),
            )
            .ok_or("product type")?;
            env = self.build(
                fcx.block,
                "frk_adt.product_snoc",
                &[env, value],
                &[product_ty],
                &[],
                l,
            )?;
        }
        let closure = self.build(
            fcx.block,
            "frk_closure.make",
            &[env],
            &[self.fn_ty()],
            &[("callee", FlatSymbolRefAttribute::new(self.context, &symbol).into())],
            l,
        )?;
        let result = self.build(
            fcx.block,
            "frk_ctl.prompt",
            &[closure],
            &[self.dyn_ty()],
            &[],
            l,
        )?;
        // The receiver is a deferred closure job.
        self.job_queue.push(Job {
            symbol,
            captures,
            params: Vec::new(),
            escape: Some(escape.clone()),
            wind_thunk: false,
            body: body.clone(),
            procs: fcx.procs.clone(),
        });
        // Non-tail prompt: a passing-through abort leaves pending set.
        self.emit_guard(fcx, l)?;
        Ok(Some(result))
    }

    fn emit_primitive<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        op: &str,
        args: &[Expr],
        l: Location<'c>,
    ) -> R<Option<Option<Value<'c, 'r>>>> {
        // outer Option: is this a primitive? inner Option: diverged?
        let arith2 = |emitter: &mut Self, fcx: &mut Fcx<'c, 'r>, mlir: &str| -> R<Option<Option<Value<'c, 'r>>>> {
            let nums = match emitter.eval_nums(fcx, args, l)? {
                Some(n) => n,
                None => return Ok(Some(None)),
            };
            let [a, b] = nums.as_slice() else {
                return Err(format!("`{op}` takes two arguments in v0"));
            };
            let r = emitter.build(fcx.block, mlir, &[*a, *b], &[emitter.f64_ty()], &[], l)?;
            Ok(Some(Some(emitter.wrap(fcx.block, TAG_NUM, r, l)?)))
        };
        match op {
            "+" | "*" => {
                let nums = match self.eval_nums(fcx, args, l)? {
                    Some(n) => n,
                    None => return Ok(Some(None)),
                };
                let (mlir, ident) = if op == "+" { ("arith.addf", 0.0) } else { ("arith.mulf", 1.0) };
                let mut acc = self.const_f64(fcx.block, ident, l)?;
                for n in &nums {
                    acc = self.build(fcx.block, mlir, &[acc, *n], &[self.f64_ty()], &[], l)?;
                }
                Ok(Some(Some(self.wrap(fcx.block, TAG_NUM, acc, l)?)))
            }
            "-" => arith2(self, fcx, "arith.subf"),
            "quotient" | "remainder" => {
                let nums = match self.eval_nums(fcx, args, l)? {
                    Some(n) => n,
                    None => return Ok(Some(None)),
                };
                let [a, b] = nums.as_slice() else {
                    return Err(format!("`{op}` takes two arguments"));
                };
                let q = self.trunc_div(fcx, *a, *b, l)?;
                if op == "quotient" {
                    Ok(Some(Some(self.wrap(fcx.block, TAG_NUM, q, l)?)))
                } else {
                    let bq = self.build(fcx.block, "arith.mulf", &[*b, q], &[self.f64_ty()], &[], l)?;
                    let r = self.build(fcx.block, "arith.subf", &[*a, bq], &[self.f64_ty()], &[], l)?;
                    Ok(Some(Some(self.wrap(fcx.block, TAG_NUM, r, l)?)))
                }
            }
            "=" | "<" | ">" | "<=" | ">=" => {
                let nums = match self.eval_nums(fcx, args, l)? {
                    Some(n) => n,
                    None => return Ok(Some(None)),
                };
                let [a, b] = nums.as_slice() else {
                    return Err(format!("`{op}` takes two arguments in v0"));
                };
                // cmpf predicates: oeq=1, ogt=2, oge=3, olt=4, ole=5.
                let pred = match op {
                    "=" => 1,
                    ">" => 2,
                    ">=" => 3,
                    "<" => 4,
                    _ => 5,
                };
                let c = self.build(
                    fcx.block,
                    "arith.cmpf",
                    &[*a, *b],
                    &[self.i1_ty()],
                    &[("predicate", IntegerAttribute::new(self.i64_ty(), pred).into())],
                    l,
                )?;
                Ok(Some(Some(self.wrap(fcx.block, TAG_BOOL, c, l)?)))
            }
            "display" => {
                let [arg] = args else { return Err("display takes one argument".into()) };
                let v = match self.emit_value(fcx, arg)? {
                    Some(v) => v,
                    None => return Ok(Some(None)),
                };
                self.call(fcx.block, "__scm_display", &[v], &[], l)?;
                // display returns an unspecified value; use nil-ish 0.
                Ok(Some(Some(self.num_dyn(fcx.block, 0.0, l)?)))
            }
            "newline" => {
                self.call(fcx.block, "frk_rt_scm_newline", &[], &[], l)?;
                Ok(Some(Some(self.num_dyn(fcx.block, 0.0, l)?)))
            }
            "cons" | "eq?" => {
                let [a, b] = args else {
                    return Err(format!("`{op}` takes two arguments"));
                };
                let Some(av) = self.emit_value(fcx, a)? else { return Ok(Some(None)) };
                let Some(bv) = self.emit_value(fcx, b)? else { return Ok(Some(None)) };
                let callee = if op == "cons" { "__scm_cons" } else { "__scm_eq" };
                let r = self
                    .call(fcx.block, callee, &[av, bv], &[self.dyn_ty()], l)?
                    .ok_or("intrinsic produced no value")?;
                Ok(Some(Some(r)))
            }
            "car" | "cdr" | "null?" | "pair?" => {
                let [a] = args else {
                    return Err(format!("`{op}` takes one argument"));
                };
                let Some(av) = self.emit_value(fcx, a)? else { return Ok(Some(None)) };
                let callee = match op {
                    "car" => "__scm_car",
                    "cdr" => "__scm_cdr",
                    "null?" => "__scm_nullp",
                    _ => "__scm_pairp",
                };
                let r = self
                    .call(fcx.block, callee, &[av], &[self.dyn_ty()], l)?
                    .ok_or("intrinsic produced no value")?;
                Ok(Some(Some(r)))
            }
            "list" => {
                let mut values = Vec::new();
                for arg in args {
                    match self.emit_value(fcx, arg)? {
                        Some(v) => values.push(v),
                        None => return Ok(Some(None)),
                    }
                }
                let mut acc = self.nil_dyn(fcx.block, l)?;
                for value in values.into_iter().rev() {
                    acc = self
                        .call(fcx.block, "__scm_cons", &[value, acc], &[self.dyn_ty()], l)?
                        .ok_or("cons produced no value")?;
                }
                Ok(Some(Some(acc)))
            }
            "raise-continuable" => {
                let [e] = args else {
                    return Err("raise-continuable takes one argument".into());
                };
                let Some(ev) = self.emit_value(fcx, e)? else { return Ok(Some(None)) };
                let r = self.build(
                    fcx.block,
                    "frk_ctl.perform",
                    &[ev],
                    &[self.dyn_ty()],
                    &[("label", StringAttribute::new(self.context, "exn").into())],
                    l,
                )?;
                self.emit_guard(fcx, l)?;
                Ok(Some(Some(r)))
            }
            "with-exception-handler" => {
                // (with-exception-handler h thunk) ⇒ handle{label=exn}
                // with a synthesized tail-resume CLAUSE wrapping h and
                // a prompt-shaped BODY wrapping thunk (D-071). Both h
                // and thunk are general expressions (fun dyns).
                let [handler, thunk] = args else {
                    return Err("with-exception-handler takes a handler and a thunk".into());
                };
                let Some(hv) = self.emit_value(fcx, handler)? else { return Ok(Some(None)) };
                let Some(tv) = self.emit_value(fcx, thunk)? else { return Ok(Some(None)) };
                let r = self.emit_handle_exn(fcx, hv, tv, l)?;
                self.emit_guard(fcx, l)?;
                Ok(Some(Some(r)))
            }
            "dynamic-wind" => {
                let [before, thunk, after] = args else {
                    return Err("dynamic-wind takes three thunks".into());
                };
                let bf = self.emit_wind_thunk(fcx, before, l)?;
                let th = self.emit_wind_thunk(fcx, thunk, l)?;
                let af = self.emit_wind_thunk(fcx, after, l)?;
                let r = self.build(
                    fcx.block,
                    "frk_ctl.wind",
                    &[bf, th, af],
                    &[self.dyn_ty()],
                    &[],
                    l,
                )?;
                // A crossing escape leaves pending set — same guard
                // discipline as any non-tail call (D-061/D-070).
                self.emit_guard(fcx, l)?;
                Ok(Some(Some(r)))
            }
            _ => Ok(None),
        }
    }

    fn eval_nums<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        args: &[Expr],
        l: Location<'c>,
    ) -> R<Option<Vec<Value<'c, 'r>>>> {
        let mut nums = Vec::new();
        for arg in args {
            let v = match self.emit_value(fcx, arg)? {
                Some(v) => v,
                None => return Ok(None),
            };
            nums.push(self.unwrap(fcx.block, TAG_NUM, self.f64_ty(), v, l)?);
        }
        Ok(Some(nums))
    }

    /// trunc(a / b) toward zero, as an f64 (scheme quotient semantics).
    fn trunc_div<'r>(
        &mut self,
        fcx: &mut Fcx<'c, 'r>,
        a: Value<'c, 'r>,
        b: Value<'c, 'r>,
        l: Location<'c>,
    ) -> R<Value<'c, 'r>> {
        let d = self.build(fcx.block, "arith.divf", &[a, b], &[self.f64_ty()], &[], l)?;
        let i = self.op1(fcx.block, melior::dialect::arith::fptosi(d, self.i64_ty(), l))?;
        Ok(self.op1(fcx.block, melior::dialect::arith::sitofp(i, self.f64_ty(), l))?)
    }
}

fn is_primitive(op: &str) -> bool {
    matches!(
        op,
        "+" | "-" | "*" | "quotient" | "remainder" | "=" | "<" | ">" | "<=" | ">=" | "display"
            | "newline" | "cons" | "car" | "cdr" | "null?" | "pair?" | "eq?" | "list"
            | "dynamic-wind" | "with-exception-handler" | "raise-continuable"
    )
}

// ---- free-name analysis ----

fn free_names_body(body: &[Expr], bound: &mut HashSet<String>, out: &mut BTreeSet<String>) {
    for e in body {
        free_names(e, bound, out);
    }
}

fn free_names(expr: &Expr, bound: &mut HashSet<String>, out: &mut BTreeSet<String>) {
    match expr {
        Expr::Int(_, _) | Expr::Bool(_, _) => {}
        Expr::Var(name, _) => {
            if !bound.contains(name) {
                out.insert(name.clone());
            }
        }
        Expr::If(a, b, c, _) => {
            free_names(a, bound, out);
            free_names(b, bound, out);
            free_names(c, bound, out);
        }
        Expr::Begin(exprs, _) => free_names_body(exprs, bound, out),
        Expr::Quote(_, _) => {}
        Expr::App(callee, args, _) => {
            free_names(callee, bound, out);
            for a in args {
                free_names(a, bound, out);
            }
        }
        Expr::Lambda(params, body, _) => {
            let snapshot = bound.clone();
            bound.extend(params.iter().cloned());
            free_names_body(body, bound, out);
            *bound = snapshot;
        }
        Expr::Let(_, binds, body, _) => {
            let snapshot = bound.clone();
            for (name, init) in binds {
                free_names(init, bound, out);
                bound.insert(name.clone());
            }
            free_names_body(body, bound, out);
            *bound = snapshot;
        }
    }
}

fn block_arg<'c, 'r>(block: BlockRef<'c, 'r>, index: usize) -> R<Value<'c, 'r>> {
    let raw = block
        .argument(index)
        .map_err(|_| format!("missing block argument {index}"))?
        .to_raw();
    Ok(unsafe { Value::from_raw(raw) })
}
