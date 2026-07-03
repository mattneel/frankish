//! The loanword consumer (SPEC §6.3, frozen v1 at M9 — D-046): parses
//! the canonical-JSON typed-AST artifact, verifies its SHA-256 content
//! id, and emits kernel/upstream dialects. First producer:
//! tools/loanword-ts (TS-0).
//!
//! §6.5 lands here: every loanword node carries a byte span into the
//! embedded source; emission threads them into `FileLineColLoc`s, so
//! traps and verifier findings finally point at source.
//!
//! TS-0 slice conventions (D-047):
//! - number = f64 (D-013), boolean = i1; functions are monomorphic,
//!   first-order, fully annotated — they lower to plain func.func +
//!   func.call (closure-lite arrives only when a corpus case demands
//!   it: the admission rule cuts both ways).
//! - `let` locals are frk_mem boxes (assignment is the idiom TS
//!   carries that ml_core lacked — the mem surface's first frontend
//!   consumer); parameters are immutable (assignment to them is
//!   fenced, rare and loud).
//! - `console.log` lowers to `func.call @frk_rt_print_f64 / _bool`
//!   against bodyless declarations; the interpreter answers them with
//!   builtins, the JIT with registered capture symbols, AOT with the
//!   real runtime.
//! - JS comparison semantics: === → cmpf oeq, !== → cmpf une (NaN
//!   !== NaN is true), <,<=,>,>= → olt/ole/ogt/oge (false on NaN);
//!   % → arith.remf (fmod, dividend sign — JS semantics).
//! - Statements after a `return` in the same block are dropped
//!   (tsc-legal dead code).

use std::collections::HashMap;

use melior::Context;
use melior::ir::attribute::{
    Attribute, FlatSymbolRefAttribute, IntegerAttribute, StringAttribute, TypeAttribute,
};
use melior::ir::operation::{OperationBuilder, OperationLike};
use melior::ir::r#type::{FunctionType, IntegerType};
use melior::ir::{
    Block, BlockLike, BlockRef, Identifier, Location, Module, Region, RegionLike, Type, Value,
    ValueLike,
};
use serde_json::Value as Json;
use sha2::{Digest, Sha256};

pub const PRINT_F64: &str = "frk_rt_print_f64";
pub const PRINT_BOOL: &str = "frk_rt_print_bool";

#[derive(Debug)]
pub struct LoanwordError(pub String);

impl std::fmt::Display for LoanwordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "loanword: {}", self.0)
    }
}
impl std::error::Error for LoanwordError {}

type Result<T> = std::result::Result<T, LoanwordError>;

fn err<T>(message: impl Into<String>) -> Result<T> {
    Err(LoanwordError(message.into()))
}

// ---- canonical form + content id ----

/// Canonical JSON: recursively sorted keys, no whitespace — must match
/// the producer's `canonical()` byte for byte.
fn canonical(value: &Json, out: &mut String) {
    match value {
        Json::Object(map) => {
            out.push('{');
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                canonical(&Json::String((*key).clone()), out);
                out.push(':');
                canonical(&map[*key], out);
            }
            out.push('}');
        }
        Json::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                canonical(item, out);
            }
            out.push(']');
        }
        other => out.push_str(&other.to_string()),
    }
}

/// Verifies the artifact's SHA-256 content id (D-046: the id is the
/// hash of the canonical bytes WITHOUT the sha256 field).
fn verify_sha(document: &Json) -> Result<()> {
    let Json::Object(map) = document else {
        return err("artifact root must be an object");
    };
    let Some(Json::String(claimed)) = map.get("sha256") else {
        return err("artifact has no sha256 content id");
    };
    let mut stripped = map.clone();
    stripped.remove("sha256");
    let mut bytes = String::new();
    canonical(&Json::Object(stripped), &mut bytes);
    let actual = format!("{:x}", Sha256::digest(bytes.as_bytes()));
    if &actual != claimed {
        return err(format!(
            "content id mismatch: artifact claims {claimed}, canonical bytes hash to {actual}"
        ));
    }
    Ok(())
}

// ---- typed AST ----

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TsTy {
    Num,
    Bool,
    Void,
}

struct TsFn {
    name: String,
    params: Vec<(String, TsTy)>,
    ret: TsTy,
    body: Vec<Json>,
}

struct Artifact {
    functions: Vec<TsFn>,
    top: Vec<Json>,
    /// Byte offset of each line start, for span → line/col.
    line_starts: Vec<usize>,
    file: String,
}

fn field<'j>(node: &'j Json, key: &str) -> Result<&'j Json> {
    node.get(key)
        .ok_or_else(|| LoanwordError(format!("node missing {key:?}: {node}")))
}

fn kind(node: &Json) -> Result<&str> {
    field(node, "k")?
        .as_str()
        .ok_or_else(|| LoanwordError("non-string node kind".into()))
}

fn parse_artifact(text: &str) -> Result<Artifact> {
    let document: Json = serde_json::from_str(text)
        .map_err(|e| LoanwordError(format!("artifact is not JSON: {e}")))?;
    verify_sha(&document)?;

    let version = field(&document, "loanword")?.as_i64();
    if version != Some(1) {
        return err(format!("unsupported loanword version {version:?} (v1 frozen at M9)"));
    }
    let source = field(&document, "source")?
        .as_str()
        .ok_or_else(|| LoanwordError("non-string source".into()))?;
    let file = field(&document, "file")?
        .as_str()
        .unwrap_or("<loanword>")
        .to_string();

    let mut line_starts = vec![0usize];
    for (offset, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            line_starts.push(offset + 1);
        }
    }

    let types: Vec<TsTy> = field(&document, "types")?
        .as_array()
        .ok_or_else(|| LoanwordError("types must be an array".into()))?
        .iter()
        .map(|row| match kind(row)? {
            "num" => Ok(TsTy::Num),
            "bool" => Ok(TsTy::Bool),
            "void" => Ok(TsTy::Void),
            other => err(format!("unsupported interned type {other:?}")),
        })
        .collect::<Result<_>>()?;
    let type_at = |node: &Json, key: &str| -> Result<TsTy> {
        let index = field(node, key)?
            .as_u64()
            .ok_or_else(|| LoanwordError("type ref must be an index".into()))?;
        types
            .get(index as usize)
            .copied()
            .ok_or_else(|| LoanwordError(format!("type ref {index} out of range")))
    };

    let mut functions = Vec::new();
    for decl in field(&document, "decls")?
        .as_array()
        .ok_or_else(|| LoanwordError("decls must be an array".into()))?
    {
        if kind(decl)? != "fn" {
            return err(format!("unsupported decl kind {:?}", kind(decl)?));
        }
        let params = field(decl, "params")?
            .as_array()
            .ok_or_else(|| LoanwordError("params must be an array".into()))?
            .iter()
            .map(|param| {
                Ok((
                    field(param, "name")?
                        .as_str()
                        .ok_or_else(|| LoanwordError("param name".into()))?
                        .to_string(),
                    type_at(param, "ty")?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        functions.push(TsFn {
            name: field(decl, "name")?
                .as_str()
                .ok_or_else(|| LoanwordError("fn name".into()))?
                .to_string(),
            params,
            ret: type_at(decl, "ret")?,
            body: field(decl, "body")?
                .as_array()
                .ok_or_else(|| LoanwordError("fn body".into()))?
                .clone(),
        });
    }

    Ok(Artifact {
        functions,
        top: field(&document, "stmts")?
            .as_array()
            .ok_or_else(|| LoanwordError("stmts".into()))?
            .clone(),
        line_starts,
        file,
    })
}

// ---- emission ----

/// Compiles a loanword artifact to a kernel/upstream module. The entry
/// protocol (D-047): `@main() -> ()` runs the top-level statements;
/// output happens through the print runtime.
pub fn compile_loanword<'c>(context: &'c Context, text: &str) -> Result<Module<'c>> {
    let artifact = parse_artifact(text)?;
    let module = Module::new(Location::unknown(context));

    let emitter = Emitter { context, artifact: &artifact };

    // Print runtime declarations (bodyless; every execution path
    // resolves them its own way — D-047).
    let f64_type = Type::parse(context, "f64").ok_or(LoanwordError("f64".into()))?;
    let i1_type: Type = IntegerType::new(context, 1).into();
    for (symbol, param) in [(PRINT_F64, f64_type), (PRINT_BOOL, i1_type)] {
        let declaration = OperationBuilder::new("func.func", Location::unknown(context))
            .add_attributes(&[
                (
                    Identifier::new(context, "sym_name"),
                    StringAttribute::new(context, symbol).into(),
                ),
                (
                    Identifier::new(context, "function_type"),
                    TypeAttribute::new(FunctionType::new(context, &[param], &[]).into()).into(),
                ),
                (
                    Identifier::new(context, "sym_visibility"),
                    StringAttribute::new(context, "private").into(),
                ),
            ])
            .add_regions([Region::new()])
            .build()
            .map_err(|e| LoanwordError(e.to_string()))?;
        module.body().append_operation(declaration);
    }

    for function in &artifact.functions {
        emitter.emit_fn(&module, function)?;
    }
    emitter.emit_main(&module)?;

    if !module.as_operation().verify() {
        return err(format!(
            "emitted module failed MLIR verification:\n{}",
            module.as_operation()
        ));
    }
    Ok(module)
}

struct Emitter<'c, 'p> {
    context: &'c Context,
    artifact: &'p Artifact,
}

/// One local: parameters bind values, `let` locals bind boxes.
#[derive(Clone, Copy)]
enum Binding<'c, 'r> {
    Value(Value<'c, 'r>, TsTy),
    Boxed(Value<'c, 'r>, TsTy),
}

struct Fcx<'c, 'r> {
    region: &'r Region<'c>,
    block: BlockRef<'c, 'r>,
    env: HashMap<String, Binding<'c, 'r>>,
    /// The function's return protocol: exit block + result type.
    exit: BlockRef<'c, 'r>,
    ret: TsTy,
    /// True once the current block is terminated (a `return` was
    /// emitted); remaining statements in the block are tsc-legal dead
    /// code and are dropped.
    terminated: bool,
}

impl<'c, 'p> Emitter<'c, 'p> {
    fn loc_of(&self, node: &Json) -> Location<'c> {
        // Span → FileLineColLoc via the artifact's line table (§6.5).
        let Some(span) = node.get("span").and_then(Json::as_array) else {
            return Location::unknown(self.context);
        };
        let Some(start) = span.first().and_then(Json::as_u64) else {
            return Location::unknown(self.context);
        };
        let start = start as usize;
        let line = match self.artifact.line_starts.binary_search(&start) {
            Ok(exact) => exact,
            Err(insert) => insert - 1,
        };
        let column = start - self.artifact.line_starts[line];
        Location::new(
            self.context,
            &self.artifact.file,
            line + 1,
            column + 1,
        )
    }

    fn mlir_ty(&self, ty: TsTy) -> Result<Type<'c>> {
        match ty {
            TsTy::Num => Type::parse(self.context, "f64").ok_or(LoanwordError("f64".into())),
            TsTy::Bool => Ok(IntegerType::new(self.context, 1).into()),
            TsTy::Void => err("void has no value type"),
        }
    }

    fn signature(&self, function: &TsFn) -> Result<FunctionType<'c>> {
        let params: Vec<Type> = function
            .params
            .iter()
            .map(|(_, ty)| self.mlir_ty(*ty))
            .collect::<Result<_>>()?;
        let results: Vec<Type> = match function.ret {
            TsTy::Void => vec![],
            other => vec![self.mlir_ty(other)?],
        };
        Ok(FunctionType::new(self.context, &params, &results))
    }

    fn emit_fn(&self, module: &Module<'c>, function: &TsFn) -> Result<()> {
        let location = Location::unknown(self.context);
        let signature = self.signature(function)?;
        let region = Region::new();
        let param_types: Vec<(Type, Location)> = function
            .params
            .iter()
            .map(|(_, ty)| Ok((self.mlir_ty(*ty)?, location)))
            .collect::<Result<_>>()?;
        let entry = region.append_block(Block::new(&param_types));

        // Exit block carries the return value (void: no args).
        let exit = match function.ret {
            TsTy::Void => region.append_block(Block::new(&[])),
            other => region.append_block(Block::new(&[(self.mlir_ty(other)?, location)])),
        };

        let mut env = HashMap::new();
        for (index, (name, ty)) in function.params.iter().enumerate() {
            let raw = entry
                .argument(index)
                .map_err(|e| LoanwordError(e.to_string()))?
                .to_raw();
            env.insert(
                name.clone(),
                Binding::Value(unsafe { Value::from_raw(raw) }, *ty),
            );
        }
        let mut fcx = Fcx {
            region: &region,
            block: entry,
            env,
            exit,
            ret: function.ret,
            terminated: false,
        };
        for statement in &function.body {
            self.emit_stmt(&mut fcx, statement)?;
        }
        // Fall-off-the-end: void returns; value functions return zero
        // (tsc without noImplicitReturns allows this; D-047 fence note).
        if !fcx.terminated {
            match function.ret {
                TsTy::Void => {
                    fcx.block
                        .append_operation(self.br(fcx.exit, None, location)?);
                }
                TsTy::Num => {
                    let zero = self.const_f64(&fcx, 0.0, location)?;
                    fcx.block
                        .append_operation(self.br(fcx.exit, Some(zero), location)?);
                }
                TsTy::Bool => {
                    let value = self.const_bool(&fcx, false, location)?;
                    fcx.block
                        .append_operation(self.br(fcx.exit, Some(value), location)?);
                }
            }
        }

        // exit: func.return its argument (if any).
        let operands: Vec<Value> = match function.ret {
            TsTy::Void => vec![],
            _ => {
                let raw = exit
                    .argument(0)
                    .map_err(|e| LoanwordError(e.to_string()))?
                    .to_raw();
                vec![unsafe { Value::from_raw(raw) }]
            }
        };
        exit.append_operation(
            OperationBuilder::new("func.return", location)
                .add_operands(&operands)
                .build()
                .map_err(|e| LoanwordError(e.to_string()))?,
        );

        let op = melior::dialect::func::func(
            self.context,
            StringAttribute::new(self.context, &function.name),
            TypeAttribute::new(signature.into()),
            region,
            &[],
            location,
        );
        module.body().append_operation(op);
        Ok(())
    }

    fn emit_main(&self, module: &Module<'c>) -> Result<()> {
        let location = Location::unknown(self.context);
        let region = Region::new();
        let entry = region.append_block(Block::new(&[]));
        let exit = region.append_block(Block::new(&[]));
        let mut fcx = Fcx {
            region: &region,
            block: entry,
            env: HashMap::new(),
            exit,
            ret: TsTy::Void,
            terminated: false,
        };
        for statement in &self.artifact.top {
            self.emit_stmt(&mut fcx, statement)?;
        }
        if !fcx.terminated {
            fcx.block
                .append_operation(self.br(fcx.exit, None, location)?);
        }
        exit.append_operation(
            OperationBuilder::new("func.return", location)
                .build()
                .map_err(|e| LoanwordError(e.to_string()))?,
        );
        let op = melior::dialect::func::func(
            self.context,
            StringAttribute::new(self.context, "main"),
            TypeAttribute::new(FunctionType::new(self.context, &[], &[]).into()),
            region,
            &[(
                Identifier::new(self.context, "llvm.emit_c_interface"),
                Attribute::unit(self.context),
            )],
            location,
        );
        module.body().append_operation(op);
        Ok(())
    }

    // ---- statements ----

    fn emit_stmt<'r>(&self, fcx: &mut Fcx<'c, 'r>, node: &Json) -> Result<()> {
        if fcx.terminated {
            return Ok(()); // dead code after return (tsc-legal)
        }
        let location = self.loc_of(node);
        match kind(node)? {
            "log" => {
                let (value, ty) = self.emit_expr(fcx, field(node, "e")?)?;
                let symbol = match ty {
                    TsTy::Num => PRINT_F64,
                    TsTy::Bool => PRINT_BOOL,
                    TsTy::Void => return err("console.log of void"),
                };
                fcx.block.append_operation(
                    OperationBuilder::new("func.call", location)
                        .add_attributes(&[(
                            Identifier::new(self.context, "callee"),
                            FlatSymbolRefAttribute::new(self.context, symbol).into(),
                        )])
                        .add_operands(&[value])
                        .build()
                        .map_err(|e| LoanwordError(e.to_string()))?,
                );
                Ok(())
            }
            "let" => {
                let (value, ty) = self.emit_expr(fcx, field(node, "e")?)?;
                let name = field(node, "name")?
                    .as_str()
                    .ok_or_else(|| LoanwordError("let name".into()))?;
                let boxed_ty = Type::parse(
                    self.context,
                    &format!("!frk_mem.box<{}>", self.mlir_ty(ty)?),
                )
                .ok_or_else(|| LoanwordError("box type".into()))?;
                let boxed = self.op_result(
                    fcx.block,
                    OperationBuilder::new("frk_mem.box_new", location)
                        .add_operands(&[value])
                        .add_results(&[boxed_ty])
                        .build()
                        .map_err(|e| LoanwordError(e.to_string()))?,
                )?;
                fcx.env.insert(name.to_string(), Binding::Boxed(boxed, ty));
                Ok(())
            }
            "assign" => {
                let name = field(node, "name")?
                    .as_str()
                    .ok_or_else(|| LoanwordError("assign name".into()))?;
                let (value, _) = self.emit_expr(fcx, field(node, "e")?)?;
                match fcx.env.get(name) {
                    Some(Binding::Boxed(cell, _)) => {
                        let cell = *cell;
                        fcx.block.append_operation(
                            OperationBuilder::new("frk_mem.box_set", location)
                                .add_operands(&[cell, value])
                                .build()
                                .map_err(|e| LoanwordError(e.to_string()))?,
                        );
                        Ok(())
                    }
                    Some(Binding::Value(..)) => err(format!(
                        "assignment to parameter {name:?} is fenced in TS-0 (D-047)"
                    )),
                    None => err(format!("assignment to unknown {name:?}")),
                }
            }
            "if" => {
                let (condition, _) = self.emit_expr(fcx, field(node, "c")?)?;
                let then_block = fcx.region.append_block(Block::new(&[]));
                let else_block = fcx.region.append_block(Block::new(&[]));
                let join_block = fcx.region.append_block(Block::new(&[]));
                fcx.block.append_operation(self.cond_br(
                    condition, then_block, else_block, location,
                )?);

                for (start, statements) in [
                    (then_block, Some(field(node, "then")?)),
                    (else_block, node.get("else").filter(|e| !e.is_null())),
                ] {
                    fcx.block = start;
                    fcx.terminated = false;
                    if let Some(list) = statements {
                        for statement in list
                            .as_array()
                            .ok_or_else(|| LoanwordError("if arm".into()))?
                        {
                            self.emit_stmt(fcx, statement)?;
                        }
                    }
                    if !fcx.terminated {
                        fcx.block
                            .append_operation(self.br(join_block, None, location)?);
                    }
                }
                fcx.block = join_block;
                fcx.terminated = false;
                Ok(())
            }
            "while" => {
                let head = fcx.region.append_block(Block::new(&[]));
                let body = fcx.region.append_block(Block::new(&[]));
                let done = fcx.region.append_block(Block::new(&[]));
                fcx.block
                    .append_operation(self.br(head, None, location)?);

                fcx.block = head;
                let (condition, _) = self.emit_expr(fcx, field(node, "c")?)?;
                fcx.block
                    .append_operation(self.cond_br(condition, body, done, location)?);

                fcx.block = body;
                fcx.terminated = false;
                for statement in field(node, "body")?
                    .as_array()
                    .ok_or_else(|| LoanwordError("while body".into()))?
                {
                    self.emit_stmt(fcx, statement)?;
                }
                if !fcx.terminated {
                    fcx.block
                        .append_operation(self.br(head, None, location)?);
                }
                fcx.block = done;
                fcx.terminated = false;
                Ok(())
            }
            "ret" => {
                let value = match node.get("e").filter(|e| !e.is_null()) {
                    Some(expr) => Some(self.emit_expr(fcx, expr)?.0),
                    None => None,
                };
                match (fcx.ret, value) {
                    (TsTy::Void, None) => {
                        fcx.block
                            .append_operation(self.br(fcx.exit, None, location)?);
                    }
                    (TsTy::Void, Some(_)) => return err("return with a value in void"),
                    (_, Some(value)) => {
                        fcx.block
                            .append_operation(self.br(fcx.exit, Some(value), location)?);
                    }
                    (_, None) => return err("bare return in a value function"),
                }
                fcx.terminated = true;
                Ok(())
            }
            "expr" => {
                // Void calls are legal here (and only here).
                let inner = field(node, "e")?;
                if kind(inner)? == "call" {
                    let name = field(inner, "name")?
                        .as_str()
                        .ok_or_else(|| LoanwordError("call name".into()))?;
                    let is_void = self
                        .artifact
                        .functions
                        .iter()
                        .find(|function| function.name == name)
                        .is_some_and(|function| function.ret == TsTy::Void);
                    if is_void {
                        let mut operands = Vec::new();
                        for argument in field(inner, "args")?
                            .as_array()
                            .ok_or_else(|| LoanwordError("call args".into()))?
                        {
                            operands.push(self.emit_expr(fcx, argument)?.0);
                        }
                        fcx.block.append_operation(
                            OperationBuilder::new("func.call", location)
                                .add_attributes(&[(
                                    Identifier::new(self.context, "callee"),
                                    FlatSymbolRefAttribute::new(self.context, name).into(),
                                )])
                                .add_operands(&operands)
                                .build()
                                .map_err(|e| LoanwordError(e.to_string()))?,
                        );
                        return Ok(());
                    }
                }
                let _ = self.emit_expr(fcx, inner)?;
                Ok(())
            }
            other => err(format!("unsupported statement kind {other:?}")),
        }
    }

    // ---- expressions ----

    fn emit_expr<'r>(&self, fcx: &mut Fcx<'c, 'r>, node: &Json) -> Result<(Value<'c, 'r>, TsTy)> {
        let location = self.loc_of(node);
        match kind(node)? {
            "num" => {
                let text = field(node, "v")?
                    .as_str()
                    .ok_or_else(|| LoanwordError("num literal".into()))?;
                let value: f64 = text
                    .parse()
                    .map_err(|_| LoanwordError(format!("bad f64 literal {text:?}")))?;
                Ok((self.const_f64(fcx, value, location)?, TsTy::Num))
            }
            "bool" => {
                let value = field(node, "v")?
                    .as_bool()
                    .ok_or_else(|| LoanwordError("bool literal".into()))?;
                Ok((self.const_bool(fcx, value, location)?, TsTy::Bool))
            }
            "var" => {
                let name = field(node, "name")?
                    .as_str()
                    .ok_or_else(|| LoanwordError("var name".into()))?;
                match fcx.env.get(name).copied() {
                    Some(Binding::Value(value, ty)) => Ok((value, ty)),
                    Some(Binding::Boxed(cell, ty)) => {
                        let value = self.op_result(
                            fcx.block,
                            OperationBuilder::new("frk_mem.box_get", location)
                                .add_operands(&[cell])
                                .add_results(&[self.mlir_ty(ty)?])
                                .build()
                                .map_err(|e| LoanwordError(e.to_string()))?,
                        )?;
                        Ok((value, ty))
                    }
                    None => err(format!("unbound variable {name:?}")),
                }
            }
            "bin" => {
                let op = field(node, "op")?
                    .as_str()
                    .ok_or_else(|| LoanwordError("bin op".into()))?;
                let (lhs, lhs_ty) = self.emit_expr(fcx, field(node, "l")?)?;
                let (rhs, _) = self.emit_expr(fcx, field(node, "r")?)?;
                let f64_type = self.mlir_ty(TsTy::Num)?;
                let i1_type = self.mlir_ty(TsTy::Bool)?;
                let block = fcx.block;
                let arith = |name: &str| -> Result<(Value<'c, 'r>, TsTy)> {
                    Ok((
                        self.op_result(
                            block,
                            OperationBuilder::new(name, location)
                                .add_operands(&[lhs, rhs])
                                .add_results(&[f64_type])
                                .build()
                                .map_err(|e| LoanwordError(e.to_string()))?,
                        )?,
                        TsTy::Num,
                    ))
                };
                let cmpf = |predicate: i64| -> Result<(Value<'c, 'r>, TsTy)> {
                    let i64_type: Type = IntegerType::new(self.context, 64).into();
                    Ok((
                        self.op_result(
                            block,
                            OperationBuilder::new("arith.cmpf", location)
                                .add_attributes(&[(
                                    Identifier::new(self.context, "predicate"),
                                    IntegerAttribute::new(i64_type, predicate).into(),
                                )])
                                .add_operands(&[lhs, rhs])
                                .add_results(&[i1_type])
                                .build()
                                .map_err(|e| LoanwordError(e.to_string()))?,
                        )?,
                        TsTy::Bool,
                    ))
                };
                match op {
                    "+" => arith("arith.addf"),
                    "-" => arith("arith.subf"),
                    "*" => arith("arith.mulf"),
                    "/" => arith("arith.divf"),
                    "%" => arith("arith.remf"),
                    // MLIR CmpFPredicate: oeq=1 ogt=2 oge=3 olt=4 ole=5 une=13.
                    "<" => cmpf(4),
                    "<=" => cmpf(5),
                    ">" => cmpf(2),
                    ">=" => cmpf(3),
                    "===" => {
                        if lhs_ty == TsTy::Bool {
                            // Bool ===: xor then not.
                            let x = self.bool_xor(fcx, lhs, rhs, location)?;
                            let value = self.bool_not(fcx, x, location)?;
                            Ok((value, TsTy::Bool))
                        } else {
                            cmpf(1)
                        }
                    }
                    "!==" => {
                        if lhs_ty == TsTy::Bool {
                            Ok((self.bool_xor(fcx, lhs, rhs, location)?, TsTy::Bool))
                        } else {
                            cmpf(13)
                        }
                    }
                    "&&" | "||" => {
                        // Pure subset: strict select (no observable
                        // effects to short-circuit around — D-047).
                        let (on_true, on_false) = if op == "&&" {
                            (rhs, self.const_bool(fcx, false, location)?)
                        } else {
                            (self.const_bool(fcx, true, location)?, rhs)
                        };
                        Ok((
                            self.op_result(
                                fcx.block,
                                OperationBuilder::new("arith.select", location)
                                    .add_operands(&[lhs, on_true, on_false])
                                    .add_results(&[i1_type])
                                    .build()
                                    .map_err(|e| LoanwordError(e.to_string()))?,
                            )?,
                            TsTy::Bool,
                        ))
                    }
                    other => err(format!("unsupported operator {other:?}")),
                }
            }
            "un" => {
                let op = field(node, "op")?
                    .as_str()
                    .ok_or_else(|| LoanwordError("un op".into()))?;
                let (value, ty) = self.emit_expr(fcx, field(node, "e")?)?;
                match op {
                    "-" => Ok((
                        self.op_result(
                            fcx.block,
                            OperationBuilder::new("arith.negf", location)
                                .add_operands(&[value])
                                .add_results(&[self.mlir_ty(TsTy::Num)?])
                                .build()
                                .map_err(|e| LoanwordError(e.to_string()))?,
                        )?,
                        TsTy::Num,
                    )),
                    "!" => Ok((self.bool_not(fcx, value, location)?, ty)),
                    other => err(format!("unsupported unary {other:?}")),
                }
            }
            "cond" => {
                let (condition, _) = self.emit_expr(fcx, field(node, "c")?)?;
                // Evaluate the true arm first, then the false arm, in
                // separate blocks (arms may themselves branch).
                let then_block = fcx.region.append_block(Block::new(&[]));
                let else_block = fcx.region.append_block(Block::new(&[]));
                fcx.block.append_operation(self.cond_br(
                    condition, then_block, else_block, location,
                )?);

                fcx.block = then_block;
                let (true_value, ty) = self.emit_expr(fcx, field(node, "t")?)?;
                let true_exit = fcx.block;

                fcx.block = else_block;
                let (false_value, _) = self.emit_expr(fcx, field(node, "e")?)?;
                let false_exit = fcx.block;

                let join = fcx
                    .region
                    .append_block(Block::new(&[(self.mlir_ty(ty)?, location)]));
                true_exit.append_operation(self.br(join, Some(true_value), location)?);
                false_exit.append_operation(self.br(join, Some(false_value), location)?);
                fcx.block = join;
                let raw = join
                    .argument(0)
                    .map_err(|e| LoanwordError(e.to_string()))?
                    .to_raw();
                Ok((unsafe { Value::from_raw(raw) }, ty))
            }
            "call" => {
                let name = field(node, "name")?
                    .as_str()
                    .ok_or_else(|| LoanwordError("call name".into()))?;
                let target = self
                    .artifact
                    .functions
                    .iter()
                    .find(|function| function.name == name)
                    .ok_or_else(|| LoanwordError(format!("call to unknown {name:?}")))?;
                let mut operands = Vec::new();
                for argument in field(node, "args")?
                    .as_array()
                    .ok_or_else(|| LoanwordError("call args".into()))?
                {
                    operands.push(self.emit_expr(fcx, argument)?.0);
                }
                if target.ret == TsTy::Void {
                    return err(format!(
                        "void call to {name:?} in expression position"
                    ));
                }
                let value = self.op_result(
                    fcx.block,
                    OperationBuilder::new("func.call", location)
                        .add_attributes(&[(
                            Identifier::new(self.context, "callee"),
                            FlatSymbolRefAttribute::new(self.context, name).into(),
                        )])
                        .add_operands(&operands)
                        .add_results(&[self.mlir_ty(target.ret)?])
                        .build()
                        .map_err(|e| LoanwordError(e.to_string()))?,
                )?;
                Ok((value, target.ret))
            }
            other => err(format!("unsupported expression kind {other:?}")),
        }
    }

    // ---- op helpers ----

    fn op_result<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        op: melior::ir::Operation<'c>,
    ) -> Result<Value<'c, 'r>> {
        let inserted = block.append_operation(op);
        let raw = inserted
            .result(0)
            .map_err(|_| LoanwordError("op has no result".into()))?
            .to_raw();
        Ok(unsafe { Value::from_raw(raw) })
    }

    fn const_f64<'r>(
        &self,
        fcx: &Fcx<'c, 'r>,
        value: f64,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        let attribute = Attribute::parse(self.context, &format!("{value:?} : f64"))
            .ok_or_else(|| LoanwordError(format!("unparsable f64 {value:?}")))?;
        self.op_result(
            fcx.block,
            OperationBuilder::new("arith.constant", location)
                .add_attributes(&[(Identifier::new(self.context, "value"), attribute)])
                .add_results(&[self.mlir_ty(TsTy::Num)?])
                .build()
                .map_err(|e| LoanwordError(e.to_string()))?,
        )
    }

    fn const_bool<'r>(
        &self,
        fcx: &Fcx<'c, 'r>,
        value: bool,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        let i1_type: Type = IntegerType::new(self.context, 1).into();
        self.op_result(
            fcx.block,
            melior::dialect::arith::constant(
                self.context,
                IntegerAttribute::new(i1_type, value as i64).into(),
                location,
            ),
        )
    }

    fn bool_xor<'r>(
        &self,
        fcx: &Fcx<'c, 'r>,
        lhs: Value<'c, 'r>,
        rhs: Value<'c, 'r>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        self.op_result(
            fcx.block,
            OperationBuilder::new("arith.xori", location)
                .add_operands(&[lhs, rhs])
                .add_results(&[self.mlir_ty(TsTy::Bool)?])
                .build()
                .map_err(|e| LoanwordError(e.to_string()))?,
        )
    }

    fn bool_not<'r>(
        &self,
        fcx: &Fcx<'c, 'r>,
        value: Value<'c, 'r>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        let one = self.const_bool(fcx, true, location)?;
        self.bool_xor(fcx, value, one, location)
    }

    fn br<'r>(
        &self,
        target: BlockRef<'c, 'r>,
        value: Option<Value<'c, 'r>>,
        location: Location<'c>,
    ) -> Result<melior::ir::Operation<'c>> {
        let operands: Vec<Value> = value.into_iter().collect();
        OperationBuilder::new("cf.br", location)
            .add_operands(&operands)
            .add_successors(&[&target])
            .build()
            .map_err(|e| LoanwordError(e.to_string()))
    }

    fn cond_br<'r>(
        &self,
        condition: Value<'c, 'r>,
        on_true: BlockRef<'c, 'r>,
        on_false: BlockRef<'c, 'r>,
        location: Location<'c>,
    ) -> Result<melior::ir::Operation<'c>> {
        OperationBuilder::new("cf.cond_br", location)
            .add_attributes(&[(
                Identifier::new(self.context, "operandSegmentSizes"),
                melior::ir::attribute::DenseI32ArrayAttribute::new(self.context, &[1, 0, 0])
                    .into(),
            )])
            .add_operands(&[condition])
            .add_successors(&[&on_true, &on_false])
            .build()
            .map_err(|e| LoanwordError(e.to_string()))
    }
}
