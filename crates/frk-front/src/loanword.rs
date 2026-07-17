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
use std::rc::Rc;

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
pub const PRINT_STR: &str = "frk_rt_print_str";

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

#[derive(Clone, PartialEq, Eq, Debug)]
enum TsTy {
    Num,
    Bool,
    Void,
    Str,
    Arr(Box<TsTy>),
    /// A discriminated union (D-072) — the value IS an frk_adt sum.
    Union(Rc<UnionDef>),
    /// A union value NARROWED to one variant: same sum representation,
    /// the index licenses checkless `extract`s downstream.
    Variant(Rc<UnionDef>, usize),
    /// A class instance (D-073): a managed box of a product, NOMINAL
    /// by type-row index (which also breaks Rc cycles for recursive
    /// classes — the def lives in the artifact's side table).
    Class(usize),
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct ClassDef {
    name: String,
    /// Declaration order; class-typed fields store as !frk_mem.recref
    /// in the product (D-074).
    fields: Vec<(String, TsTy)>,
}

struct MethodDecl {
    name: String,
    params: Vec<(String, TsTy)>,
    ret: TsTy,
    body: Vec<Json>,
}

struct CtorDecl {
    params: Vec<(String, TsTy)>,
    /// (field name, rhs expr) in SOURCE order — evaluation order is
    /// the program's; the record builds in declaration order after.
    /// None = `this.f = this`, the D-074 construction knot: the slot
    /// seeds null and back-patches right after box_new.
    sets: Vec<(String, Option<Json>)>,
}

struct ClassDecl {
    ty: usize,
    name: String,
    ctor: CtorDecl,
    methods: Vec<MethodDecl>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct UnionDef {
    variants: Vec<VariantDef>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct VariantDef {
    /// The `kind` string literal — NOT a stored field (D-072): tests
    /// lower to tag compares, reads to tag-selected literals.
    kind: String,
    fields: Vec<(String, TsTy)>,
}

/// One interned type row: a value type, or a variant row (referenced
/// by union rows; not a value type by itself — D-072 fence).
enum Row {
    Ty(TsTy),
    Obj(VariantDef),
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
    /// The resolved type-row table — `obj`/`narrow` nodes reference
    /// union rows by index (D-072).
    types: Vec<Row>,
    /// Class definitions by type-row index (D-073).
    classes: HashMap<usize, ClassDef>,
    /// Class declarations (constructor + method bodies) for emission.
    class_decls: Vec<ClassDecl>,
}

impl Artifact {
    fn union_at(&self, index: usize) -> Result<Rc<UnionDef>> {
        match self.types.get(index) {
            Some(Row::Ty(TsTy::Union(def))) => Ok(def.clone()),
            _ => err(format!("type ref {index} is not a union row (D-072)")),
        }
    }

    fn class_at(&self, index: usize) -> Result<&ClassDef> {
        self.classes
            .get(&index)
            .ok_or_else(|| LoanwordError(format!("type ref {index} is not a class row (D-073)")))
    }

    fn class_decl(&self, index: usize) -> Result<&ClassDecl> {
        self.class_decls
            .iter()
            .find(|decl| decl.ty == index)
            .ok_or_else(|| LoanwordError(format!("class row {index} has no declaration")))
    }
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

    // One ordered pass: producers intern depth-first, so every ref
    // points backward (the D-049 arr precedent, now general — obj rows
    // reference scalar rows, union rows reference obj rows; D-072).
    // The ONE forward reference is D-074's knot: a `cref` row names a
    // class whose row comes later, so class indices prescan by name.
    let raw_rows = field(&document, "types")?
        .as_array()
        .ok_or_else(|| LoanwordError("types must be an array".into()))?;
    let mut class_indices: HashMap<String, usize> = HashMap::new();
    for (index, row) in raw_rows.iter().enumerate() {
        if kind(row)? == "class" {
            let name = field(row, "name")?
                .as_str()
                .ok_or_else(|| LoanwordError("class row name".into()))?;
            if class_indices.insert(name.to_string(), index).is_some() {
                return err(format!("duplicate class {name:?}"));
            }
        }
    }
    let mut classes: HashMap<usize, ClassDef> = HashMap::new();
    let mut rows: Vec<Row> = Vec::new();
    for row in raw_rows {
        let resolved_ty = |key: &Json| -> Result<TsTy> {
            let index = key
                .as_u64()
                .ok_or_else(|| LoanwordError("type ref must be an index".into()))?;
            match rows.get(index as usize) {
                Some(Row::Ty(ty)) => Ok(ty.clone()),
                Some(Row::Obj(_)) => err("a variant row is not a value type (D-072)"),
                None => err(format!("type ref {index} out of range")),
            }
        };
        let parsed = match kind(row)? {
            "num" => Row::Ty(TsTy::Num),
            "bool" => Row::Ty(TsTy::Bool),
            "void" => Row::Ty(TsTy::Void),
            "str" => Row::Ty(TsTy::Str),
            "arr" => {
                let elem = resolved_ty(field(row, "elem")?)?;
                if matches!(elem, TsTy::Arr(_)) {
                    return err("nested arrays are fenced in TS-0 (D-049)");
                }
                Row::Ty(TsTy::Arr(Box::new(elem)))
            }
            "obj" => {
                let kind_lit = field(row, "kind")?
                    .as_str()
                    .ok_or_else(|| LoanwordError("obj row kind".into()))?
                    .to_string();
                let mut fields = Vec::new();
                for entry in field(row, "fields")?
                    .as_array()
                    .ok_or_else(|| LoanwordError("obj row fields".into()))?
                {
                    let name = field(entry, "name")?
                        .as_str()
                        .ok_or_else(|| LoanwordError("obj field name".into()))?
                        .to_string();
                    let ty = resolved_ty(field(entry, "ty")?)?;
                    if !matches!(ty, TsTy::Num | TsTy::Bool | TsTy::Str) {
                        return err("variant fields are num/bool/str in TS-1 (D-072)");
                    }
                    fields.push((name, ty));
                }
                Row::Obj(VariantDef { kind: kind_lit, fields })
            }
            "union" => {
                let mut variants = Vec::new();
                for reference in field(row, "variants")?
                    .as_array()
                    .ok_or_else(|| LoanwordError("union row variants".into()))?
                {
                    let index = reference
                        .as_u64()
                        .ok_or_else(|| LoanwordError("union variant ref".into()))?;
                    match rows.get(index as usize) {
                        Some(Row::Obj(def)) => variants.push(def.clone()),
                        _ => return err("union row must reference variant rows (D-072)"),
                    }
                }
                if variants.is_empty() {
                    return err("a union needs at least one variant");
                }
                Row::Ty(TsTy::Union(Rc::new(UnionDef { variants })))
            }
            "cref" => {
                // D-074: a class reference by name — forward-legal.
                let name = field(row, "name")?
                    .as_str()
                    .ok_or_else(|| LoanwordError("cref name".into()))?;
                let index = class_indices
                    .get(name)
                    .ok_or_else(|| LoanwordError(format!("cref to unknown class {name:?}")))?;
                Row::Ty(TsTy::Class(*index))
            }
            "class" => {
                let name = field(row, "name")?
                    .as_str()
                    .ok_or_else(|| LoanwordError("class row name".into()))?
                    .to_string();
                let mut fields = Vec::new();
                for entry in field(row, "fields")?
                    .as_array()
                    .ok_or_else(|| LoanwordError("class row fields".into()))?
                {
                    let field_name = field(entry, "name")?
                        .as_str()
                        .ok_or_else(|| LoanwordError("class field name".into()))?
                        .to_string();
                    let ty = resolved_ty(field(entry, "ty")?)?;
                    if !matches!(ty, TsTy::Num | TsTy::Bool | TsTy::Str | TsTy::Class(_)) {
                        return err("class fields are num/bool/str/class refs (D-073)");
                    }
                    fields.push((field_name, ty));
                }
                let own_index = rows.len();
                classes.insert(own_index, ClassDef { name, fields });
                Row::Ty(TsTy::Class(own_index))
            }
            other => return err(format!("unsupported interned type {other:?}")),
        };
        rows.push(parsed);
    }
    let type_at = |node: &Json, key: &str| -> Result<TsTy> {
        let index = field(node, key)?
            .as_u64()
            .ok_or_else(|| LoanwordError("type ref must be an index".into()))?;
        match rows.get(index as usize) {
            Some(Row::Ty(ty)) => Ok(ty.clone()),
            Some(Row::Obj(_)) => {
                err("a variant row is not a value type — annotate with its union (D-072)")
            }
            None => err(format!("type ref {index} out of range")),
        }
    };

    let parse_params = |node: &Json, key: &str| -> Result<Vec<(String, TsTy)>> {
        field(node, key)?
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
            .collect::<Result<Vec<_>>>()
    };

    let mut functions = Vec::new();
    let mut class_decls = Vec::new();
    for decl in field(&document, "decls")?
        .as_array()
        .ok_or_else(|| LoanwordError("decls must be an array".into()))?
    {
        match kind(decl)? {
            "fn" => {
                functions.push(TsFn {
                    name: field(decl, "name")?
                        .as_str()
                        .ok_or_else(|| LoanwordError("fn name".into()))?
                        .to_string(),
                    params: parse_params(decl, "params")?,
                    ret: type_at(decl, "ret")?,
                    body: field(decl, "body")?
                        .as_array()
                        .ok_or_else(|| LoanwordError("fn body".into()))?
                        .clone(),
                });
            }
            "class" => {
                let ty = field(decl, "ty")?
                    .as_u64()
                    .ok_or_else(|| LoanwordError("class ty ref".into()))?
                    as usize;
                let ctor_node = field(decl, "ctor")?;
                let sets = field(ctor_node, "sets")?
                    .as_array()
                    .ok_or_else(|| LoanwordError("ctor sets".into()))?
                    .iter()
                    .map(|set| {
                        let name = field(set, "name")?
                            .as_str()
                            .ok_or_else(|| LoanwordError("set name".into()))?
                            .to_string();
                        let is_self =
                            set.get("self").and_then(Json::as_bool).unwrap_or(false);
                        let rhs = if is_self {
                            None
                        } else {
                            Some(field(set, "e")?.clone())
                        };
                        Ok((name, rhs))
                    })
                    .collect::<Result<Vec<_>>>()?;
                let methods = field(decl, "methods")?
                    .as_array()
                    .ok_or_else(|| LoanwordError("class methods".into()))?
                    .iter()
                    .map(|method| {
                        Ok(MethodDecl {
                            name: field(method, "name")?
                                .as_str()
                                .ok_or_else(|| LoanwordError("method name".into()))?
                                .to_string(),
                            params: parse_params(method, "params")?,
                            ret: type_at(method, "ret")?,
                            body: field(method, "body")?
                                .as_array()
                                .ok_or_else(|| LoanwordError("method body".into()))?
                                .clone(),
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                class_decls.push(ClassDecl {
                    ty,
                    name: field(decl, "name")?
                        .as_str()
                        .ok_or_else(|| LoanwordError("class name".into()))?
                        .to_string(),
                    ctor: CtorDecl { params: parse_params(ctor_node, "params")?, sets },
                    methods,
                });
            }
            other => return err(format!("unsupported decl kind {other:?}")),
        }
    }

    Ok(Artifact {
        functions,
        top: field(&document, "stmts")?
            .as_array()
            .ok_or_else(|| LoanwordError("stmts".into()))?
            .clone(),
        line_starts,
        file,
        types: rows,
        classes,
        class_decls,
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
    let _ = IntegerType::new(context, 1);
    let str_type = Type::parse(context, "!frk_str.str").ok_or(LoanwordError("str".into()))?;
    let i64_type: Type = IntegerType::new(context, 64).into();
    for (symbol, param) in [
        (PRINT_F64, f64_type),
        // i64 flag per the registered ABI (D-062 finish): booleans
        // widen at the call site; no sub-word integer crosses the ABI.
        (PRINT_BOOL, i64_type),
        (PRINT_STR, str_type),
    ] {
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
    for decl in &artifact.class_decls {
        emitter.emit_ctor(&module, decl)?;
        // Methods are plain functions taking `this` first (D-073) —
        // direct calls; dispatch waits for the itab milestone.
        for method in &decl.methods {
            let mut params = vec![("this".to_string(), TsTy::Class(decl.ty))];
            params.extend(method.params.iter().cloned());
            let synthetic = TsFn {
                name: format!("{}__{}", decl.name, method.name),
                params,
                ret: method.ret.clone(),
                body: method.body.clone(),
            };
            emitter.emit_fn(&module, &synthetic)?;
        }
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
#[derive(Clone)]
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
    /// Span → 1-based (line, column) via the artifact line table.
    fn line_col(&self, node: &Json) -> Option<(usize, usize)> {
        let span = node.get("span").and_then(Json::as_array)?;
        let start = span.first().and_then(Json::as_u64)? as usize;
        let line = match self.artifact.line_starts.binary_search(&start) {
            Ok(exact) => exact,
            Err(insert) => insert - 1,
        };
        let column = start - self.artifact.line_starts[line];
        Some((line + 1, column + 1))
    }

    fn loc_of(&self, node: &Json) -> Location<'c> {
        // Span → FileLineColLoc via the artifact's line table (§6.5).
        let Some((line, column)) = self.line_col(node) else {
            return Location::unknown(self.context);
        };
        Location::new(self.context, &self.artifact.file, line, column)
    }

    fn mlir_ty(&self, ty: &TsTy) -> Result<Type<'c>> {
        match ty {
            TsTy::Num => Type::parse(self.context, "f64").ok_or(LoanwordError("f64".into())),
            TsTy::Bool => Ok(IntegerType::new(self.context, 1).into()),
            TsTy::Str => {
                Type::parse(self.context, "!frk_str.str").ok_or(LoanwordError("str".into()))
            }
            TsTy::Arr(elem) => {
                let inner = self.mlir_ty(elem)?;
                Type::parse(self.context, &format!("!frk_mem.arr<{inner}>"))
                    .ok_or(LoanwordError("arr".into()))
            }
            TsTy::Union(def) | TsTy::Variant(def, _) => {
                // Union and narrowed-variant values share one sum
                // representation (D-072) — narrowing is a fact, not a
                // representation change.
                Type::parse(self.context, &self.sum_text(def)?)
                    .ok_or(LoanwordError("sum".into()))
            }
            TsTy::Class(index) => {
                // A managed box of a product (D-073); class-typed
                // fields store type-erased (D-074), closing the knot.
                Type::parse(self.context, &self.class_box_text(*index)?)
                    .ok_or(LoanwordError("class box".into()))
            }
            TsTy::Void => err("void has no value type"),
        }
    }

    /// `!frk_mem.box<!frk_adt.product<[…]>>` for a class — field slot
    /// types in declaration order; class refs are `!frk_mem.recref`.
    fn class_box_text(&self, index: usize) -> Result<String> {
        let def = self.artifact.class_at(index)?;
        let mut slots = Vec::new();
        for (_, ty) in &def.fields {
            slots.push(self.field_slot_text(ty)?);
        }
        Ok(format!(
            "!frk_mem.box<!frk_adt.product<[{}]>>",
            slots.join(", ")
        ))
    }

    /// The PRODUCT-slot type of a class field: erased for class refs.
    fn field_slot_text(&self, ty: &TsTy) -> Result<String> {
        Ok(match ty {
            TsTy::Class(_) => "!frk_mem.recref".to_string(),
            other => format!("{}", self.mlir_ty(other)?),
        })
    }

    /// `!frk_adt.sum<[[…],[…]]>` for a union — variant payload types
    /// in declaration order, `kind` excluded (D-072).
    fn sum_text(&self, def: &UnionDef) -> Result<String> {
        let mut variants = Vec::new();
        for variant in &def.variants {
            let mut fields = Vec::new();
            for (_, ty) in &variant.fields {
                fields.push(format!("{}", self.mlir_ty(ty)?));
            }
            variants.push(format!("[{}]", fields.join(", ")));
        }
        Ok(format!("!frk_adt.sum<[{}]>", variants.join(", ")))
    }

    fn signature(&self, function: &TsFn) -> Result<FunctionType<'c>> {
        let params: Vec<Type> = function
            .params
            .iter()
            .map(|(_, ty)| self.mlir_ty(ty))
            .collect::<Result<_>>()?;
        let results: Vec<Type> = match &function.ret {
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
            .map(|(_, ty)| Ok((self.mlir_ty(ty)?, location)))
            .collect::<Result<_>>()?;
        let entry = region.append_block(Block::new(&param_types));

        // Exit block carries the return value (void: no args).
        let exit = match &function.ret {
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
                Binding::Value(unsafe { Value::from_raw(raw) }, ty.clone()),
            );
        }
        let mut fcx = Fcx {
            region: &region,
            block: entry,
            env,
            exit,
            ret: function.ret.clone(),
            terminated: false,
        };
        for statement in &function.body {
            self.emit_stmt(&mut fcx, statement)?;
        }
        // Fall-off-the-end: void returns. For value functions this is
        // DEFENSIVE DEAD CODE since D-050: the producer sets
        // noImplicitReturns, so tsc rejects fall-off before we ever
        // see it; the zero synthesis remains as belt-and-suspenders
        // against hand-written artifacts.
        if !fcx.terminated {
            match &function.ret {
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
                other => {
                    // Str/Arr functions must return on every path — no
                    // zero value exists to synthesize (D-049 fence).
                    return err(format!(
                        "function {:?} can fall off the end but returns {other:?} —                          add a return on every path",
                        function.name
                    ));
                }
            }
        }

        // exit: func.return its argument (if any).
        let operands: Vec<Value> = match &function.ret {
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

    /// `@{Class}__new(params) -> box`: evaluate the constructor's
    /// `this.f = e` right-hand sides in SOURCE order, then build the
    /// record in DECLARATION order (class-ref values erase, D-074).
    fn emit_ctor(&self, module: &Module<'c>, decl: &ClassDecl) -> Result<()> {
        let location = Location::unknown(self.context);
        let def = self.artifact.class_at(decl.ty)?.clone();
        let region = Region::new();
        let param_types: Vec<(Type, Location)> = decl
            .ctor
            .params
            .iter()
            .map(|(_, ty)| Ok((self.mlir_ty(ty)?, location)))
            .collect::<Result<_>>()?;
        let entry = region.append_block(Block::new(&param_types));
        let mut env = HashMap::new();
        for (index, (name, ty)) in decl.ctor.params.iter().enumerate() {
            let raw = entry
                .argument(index)
                .map_err(|e| LoanwordError(e.to_string()))?
                .to_raw();
            env.insert(
                name.clone(),
                Binding::Value(unsafe { Value::from_raw(raw) }, ty.clone()),
            );
        }
        let mut fcx = Fcx {
            region: &region,
            block: entry,
            env,
            // No `return` statement can occur inside set expressions,
            // so the exit protocol is never exercised here.
            exit: entry,
            ret: TsTy::Class(decl.ty),
            terminated: false,
        };
        let mut values: HashMap<String, (Value, TsTy)> = HashMap::new();
        let mut self_fields: Vec<String> = Vec::new();
        for (name, set_expr) in &decl.ctor.sets {
            match set_expr {
                Some(set_expr) => {
                    let (value, ty) = self.emit_expr(&mut fcx, set_expr)?;
                    values.insert(name.clone(), (value, ty));
                }
                None => self_fields.push(name.clone()),
            }
        }
        let mut product = self.op_result(
            fcx.block,
            OperationBuilder::new("frk_adt.product_new", location)
                .add_results(&[Type::parse(self.context, "!frk_adt.product<[]>")
                    .ok_or(LoanwordError("product".into()))?])
                .build()
                .map_err(|e| LoanwordError(e.to_string()))?,
        )?;
        let mut grown = Vec::new();
        for (field_name, field_ty) in &def.fields {
            let slot_value = if self_fields.contains(field_name) {
                // The knot (D-074): seed null, back-patch after box_new.
                if field_ty != &TsTy::Class(decl.ty) {
                    return err(format!(
                        "`this.{field_name} = this` needs field type {}, not {field_ty:?}",
                        def.name
                    ));
                }
                self.op_result(
                    fcx.block,
                    OperationBuilder::new("frk_mem.recref_null", location)
                        .add_results(&[Type::parse(self.context, "!frk_mem.recref")
                            .ok_or(LoanwordError("recref".into()))?])
                        .build()
                        .map_err(|e| LoanwordError(e.to_string()))?,
                )?
            } else {
                let (value, ty) = values
                    .get(field_name)
                    .cloned()
                    .ok_or_else(|| {
                        LoanwordError(format!("constructor did not assign {field_name:?}"))
                    })?;
                if &ty != field_ty {
                    return err(format!(
                        "field {field_name:?} of {} is {field_ty:?}, constructor assigns {ty:?}",
                        def.name
                    ));
                }
                if matches!(field_ty, TsTy::Class(_)) {
                    self.rec_ref(&fcx, value, location)?
                } else {
                    value
                }
            };
            grown.push(self.field_slot_text(field_ty)?);
            let product_ty = Type::parse(
                self.context,
                &format!("!frk_adt.product<[{}]>", grown.join(", ")),
            )
            .ok_or(LoanwordError("product".into()))?;
            product = self.op_result(
                fcx.block,
                OperationBuilder::new("frk_adt.product_snoc", location)
                    .add_operands(&[product, slot_value])
                    .add_results(&[product_ty])
                    .build()
                    .map_err(|e| LoanwordError(e.to_string()))?,
            )?;
        }
        let boxed = self.op_result(
            fcx.block,
            OperationBuilder::new("frk_mem.box_new", location)
                .add_operands(&[product])
                .add_results(&[self.mlir_ty(&TsTy::Class(decl.ty))?])
                .build()
                .map_err(|e| LoanwordError(e.to_string()))?,
        )?;
        // Back-patch the knot fields now that the box exists (D-074).
        if !self_fields.is_empty() {
            let i64_type: Type = IntegerType::new(self.context, 64).into();
            let self_ref = self.rec_ref(&fcx, boxed, location)?;
            for field_name in &self_fields {
                let index = def
                    .fields
                    .iter()
                    .position(|(name, _)| name == field_name)
                    .expect("self field validated against the class");
                fcx.block.append_operation(
                    OperationBuilder::new("frk_mem.field_set", location)
                        .add_attributes(&[(
                            Identifier::new(self.context, "field"),
                            IntegerAttribute::new(i64_type, index as i64).into(),
                        )])
                        .add_operands(&[boxed, self_ref])
                        .build()
                        .map_err(|e| LoanwordError(e.to_string()))?,
                );
            }
        }
        fcx.block.append_operation(
            OperationBuilder::new("func.return", location)
                .add_operands(&[boxed])
                .build()
                .map_err(|e| LoanwordError(e.to_string()))?,
        );
        let signature = FunctionType::new(
            self.context,
            &decl
                .ctor
                .params
                .iter()
                .map(|(_, ty)| self.mlir_ty(ty))
                .collect::<Result<Vec<_>>>()?,
            &[self.mlir_ty(&TsTy::Class(decl.ty))?],
        );
        let op = melior::dialect::func::func(
            self.context,
            StringAttribute::new(self.context, &format!("{}__new", decl.name)),
            TypeAttribute::new(signature.into()),
            region,
            &[],
            location,
        );
        module.body().append_operation(op);
        Ok(())
    }

    /// `obj.m(args)` → `func.call @Class__m(this, args…)` (D-073;
    /// direct — dispatch waits for itabs). Returns None for void.
    fn emit_mcall<'r>(
        &self,
        fcx: &mut Fcx<'c, 'r>,
        node: &Json,
        location: Location<'c>,
    ) -> Result<(Option<Value<'c, 'r>>, TsTy)> {
        let class = field(node, "c")?
            .as_u64()
            .ok_or_else(|| LoanwordError("mcall class ref".into()))? as usize;
        let method_name = field(node, "m")?
            .as_str()
            .ok_or_else(|| LoanwordError("mcall method".into()))?;
        let decl = self.artifact.class_decl(class)?;
        let method = decl
            .methods
            .iter()
            .find(|method| method.name == method_name)
            .ok_or_else(|| {
                LoanwordError(format!("class {} has no method {method_name:?}", decl.name))
            })?;
        let ret = method.ret.clone();
        let symbol = format!("{}__{}", decl.name, method_name);
        let (this, this_ty) = self.emit_expr(fcx, field(node, "e")?)?;
        if this_ty != TsTy::Class(class) {
            return err(format!("method receiver is {this_ty:?}"));
        }
        let mut operands = vec![this];
        for argument in field(node, "args")?
            .as_array()
            .ok_or_else(|| LoanwordError("mcall args".into()))?
        {
            operands.push(self.emit_expr(fcx, argument)?.0);
        }
        let builder = OperationBuilder::new("func.call", location)
            .add_attributes(&[(
                Identifier::new(self.context, "callee"),
                FlatSymbolRefAttribute::new(self.context, &symbol).into(),
            )])
            .add_operands(&operands);
        if ret == TsTy::Void {
            fcx.block.append_operation(
                builder.build().map_err(|e| LoanwordError(e.to_string()))?,
            );
            Ok((None, TsTy::Void))
        } else {
            let value = self.op_result(
                fcx.block,
                builder
                    .add_results(&[self.mlir_ty(&ret)?])
                    .build()
                    .map_err(|e| LoanwordError(e.to_string()))?,
            )?;
            Ok((Some(value), ret))
        }
    }

    fn rec_ref<'r>(
        &self,
        fcx: &Fcx<'c, 'r>,
        value: Value<'c, 'r>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        self.op_result(
            fcx.block,
            OperationBuilder::new("frk_mem.rec_ref", location)
                .add_operands(&[value])
                .add_results(&[Type::parse(self.context, "!frk_mem.recref")
                    .ok_or(LoanwordError("recref".into()))?])
                .build()
                .map_err(|e| LoanwordError(e.to_string()))?,
        )
    }

    fn rec_cast<'r>(
        &self,
        fcx: &Fcx<'c, 'r>,
        value: Value<'c, 'r>,
        class: usize,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        self.op_result(
            fcx.block,
            OperationBuilder::new("frk_mem.rec_cast", location)
                .add_operands(&[value])
                .add_results(&[self.mlir_ty(&TsTy::Class(class))?])
                .build()
                .map_err(|e| LoanwordError(e.to_string()))?,
        )
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
                let symbol = match &ty {
                    TsTy::Num => PRINT_F64,
                    TsTy::Bool => PRINT_BOOL,
                    TsTy::Str => PRINT_STR,
                    other => return err(format!("console.log of {other:?}")),
                };
                // Booleans widen to the registered i64 flag (D-062).
                let value = if matches!(&ty, TsTy::Bool) {
                    let i64_type: Type = IntegerType::new(self.context, 64).into();
                    let widened = fcx.block.append_operation(
                        melior::dialect::arith::extui(value, i64_type, location),
                    );
                    let raw = widened
                        .result(0)
                        .map_err(|e| LoanwordError(e.to_string()))?
                        .to_raw();
                    unsafe { Value::from_raw(raw) }
                } else {
                    value
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
                if matches!(ty, TsTy::Union(_) | TsTy::Variant(_, _)) {
                    // Producer-fenced (D-072); defense in depth — box
                    // reads have no SSA identity, so narrow facts on a
                    // boxed union would silently demote.
                    return err("union-typed locals are fenced in TS-1 (D-072)");
                }
                let name = field(node, "name")?
                    .as_str()
                    .ok_or_else(|| LoanwordError("let name".into()))?;
                let boxed_ty = Type::parse(
                    self.context,
                    &format!("!frk_mem.box<{}>", self.mlir_ty(&ty)?),
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
            "iset" => {
                let (array, array_ty) = self.emit_expr(fcx, field(node, "a")?)?;
                if !matches!(array_ty, TsTy::Arr(_)) {
                    return err("indexed assignment to a non-array");
                }
                let index = self.index_value(fcx, field(node, "i")?, location)?;
                let (value, _) = self.emit_expr(fcx, field(node, "e")?)?;
                fcx.block.append_operation(
                    OperationBuilder::new("frk_mem.array_set", location)
                        .add_operands(&[array, index, value])
                        .build()
                        .map_err(|e| LoanwordError(e.to_string()))?,
                );
                Ok(())
            }
            "pset" => {
                // Field mutation (D-073): obj.f = e / this.f = e —
                // field_set at the slot; class-ref values erase.
                let name = field(node, "name")?
                    .as_str()
                    .ok_or_else(|| LoanwordError("pset name".into()))?;
                let (target, target_ty) = self.emit_expr(fcx, field(node, "e")?)?;
                let TsTy::Class(class) = target_ty else {
                    return err(format!("property assignment on {target_ty:?}"));
                };
                let def = self.artifact.class_at(class)?;
                let index = def
                    .fields
                    .iter()
                    .position(|(field_name, _)| field_name == name)
                    .ok_or_else(|| {
                        LoanwordError(format!("class {} has no field {name:?}", def.name))
                    })?;
                let field_ty = def.fields[index].1.clone();
                let (value, value_ty) = self.emit_expr(fcx, field(node, "v")?)?;
                if value_ty != field_ty {
                    return err(format!(
                        "field {name:?} is {field_ty:?}, assignment supplies {value_ty:?}"
                    ));
                }
                let slot_value = if matches!(field_ty, TsTy::Class(_)) {
                    self.rec_ref(fcx, value, location)?
                } else {
                    value
                };
                let i64_type: Type = IntegerType::new(self.context, 64).into();
                fcx.block.append_operation(
                    OperationBuilder::new("frk_mem.field_set", location)
                        .add_attributes(&[(
                            Identifier::new(self.context, "field"),
                            IntegerAttribute::new(i64_type, index as i64).into(),
                        )])
                        .add_operands(&[target, slot_value])
                        .build()
                        .map_err(|e| LoanwordError(e.to_string()))?,
                );
                Ok(())
            }
            "if" => {
                let (condition, _) = self.emit_expr(fcx, field(node, "c")?)?;
                let then_block = fcx.region.append_block(Block::new(&[]));
                let else_block = fcx.region.append_block(Block::new(&[]));
                fcx.block.append_operation(self.cond_br(
                    condition, then_block, else_block, location,
                )?);

                // The join is LAZY: when both arms return, appending a
                // predecessor-less join block leaves dead unconverted
                // ops for the LLVM translation to choke on (found by
                // TS-1's trailing if/else-return shape). Statements
                // after a fully-returning if are tsc-visible dead code
                // and drop like statements after `return`.
                let mut fallthroughs = Vec::new();
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
                        fallthroughs.push(fcx.block);
                    }
                }
                if fallthroughs.is_empty() {
                    fcx.terminated = true;
                    return Ok(());
                }
                let join_block = fcx.region.append_block(Block::new(&[]));
                for exit in fallthroughs {
                    exit.append_operation(self.br(join_block, None, location)?);
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
                match (fcx.ret.clone(), value) {
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
                if kind(inner)? == "mcall" {
                    // Method calls in statement position: void is
                    // fine; a value result simply drops.
                    let _ = self.emit_mcall(fcx, inner, location)?;
                    return Ok(());
                }
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
                match fcx.env.get(name).cloned() {
                    Some(Binding::Value(value, ty)) => Ok((value, ty)),
                    Some(Binding::Boxed(cell, ty)) => {
                        let value = self.op_result(
                            fcx.block,
                            OperationBuilder::new("frk_mem.box_get", location)
                                .add_operands(&[cell])
                                .add_results(&[self.mlir_ty(&ty)?])
                                .build()
                                .map_err(|e| LoanwordError(e.to_string()))?,
                        )?;
                        Ok((value, ty))
                    }
                    None => err(format!("unbound variable {name:?}")),
                }
            }
            "str" => {
                let text = field(node, "v")?
                    .as_str()
                    .ok_or_else(|| LoanwordError("str literal".into()))?;
                let value = self.op_result(
                    fcx.block,
                    OperationBuilder::new("frk_str.lit", location)
                        .add_attributes(&[(
                            Identifier::new(self.context, "text"),
                            StringAttribute::new(self.context, text).into(),
                        )])
                        .add_results(&[self.mlir_ty(&TsTy::Str)?])
                        .build()
                        .map_err(|e| LoanwordError(e.to_string()))?,
                )?;
                Ok((value, TsTy::Str))
            }
            "arr" => {
                let items = field(node, "items")?
                    .as_array()
                    .ok_or_else(|| LoanwordError("arr items".into()))?;
                if items.is_empty() {
                    return err("empty array literals need an annotation (fenced in TS-0)");
                }
                let mut values = Vec::new();
                let mut elem_ty = None;
                for item in items {
                    let (value, ty) = self.emit_expr(fcx, item)?;
                    if elem_ty.get_or_insert_with(|| ty.clone()) != &ty {
                        return err("heterogeneous array literal");
                    }
                    values.push(value);
                }
                let elem = elem_ty.unwrap();
                let arr_ty = TsTy::Arr(Box::new(elem));
                let i64_type: Type = IntegerType::new(self.context, 64).into();
                let len = self.op_result(
                    fcx.block,
                    melior::dialect::arith::constant(
                        self.context,
                        IntegerAttribute::new(i64_type, values.len() as i64).into(),
                        location,
                    ),
                )?;
                let array = self.op_result(
                    fcx.block,
                    OperationBuilder::new("frk_mem.array_new", location)
                        .add_operands(&[len])
                        .add_results(&[self.mlir_ty(&arr_ty)?])
                        .build()
                        .map_err(|e| LoanwordError(e.to_string()))?,
                )?;
                for (position, value) in values.into_iter().enumerate() {
                    let index = self.op_result(
                        fcx.block,
                        melior::dialect::arith::constant(
                            self.context,
                            IntegerAttribute::new(i64_type, position as i64).into(),
                            location,
                        ),
                    )?;
                    fcx.block.append_operation(
                        OperationBuilder::new("frk_mem.array_set", location)
                            .add_operands(&[array, index, value])
                            .build()
                            .map_err(|e| LoanwordError(e.to_string()))?,
                    );
                }
                Ok((array, arr_ty))
            }
            "index" => {
                let (array, array_ty) = self.emit_expr(fcx, field(node, "a")?)?;
                let TsTy::Arr(elem) = array_ty else {
                    return err("indexing a non-array");
                };
                let index = self.index_value(fcx, field(node, "i")?, location)?;
                let value = self.op_result(
                    fcx.block,
                    OperationBuilder::new("frk_mem.array_get", location)
                        .add_operands(&[array, index])
                        .add_results(&[self.mlir_ty(&elem)?])
                        .build()
                        .map_err(|e| LoanwordError(e.to_string()))?,
                )?;
                Ok((value, *elem))
            }
            "len" => {
                let (value, ty) = self.emit_expr(fcx, field(node, "e")?)?;
                let op_name = match ty {
                    TsTy::Str => "frk_str.len",
                    TsTy::Arr(_) => "frk_mem.array_len",
                    other => return err(format!(".length of {other:?}")),
                };
                let i64_type: Type = IntegerType::new(self.context, 64).into();
                let raw = self.op_result(
                    fcx.block,
                    OperationBuilder::new(op_name, location)
                        .add_operands(&[value])
                        .add_results(&[i64_type])
                        .build()
                        .map_err(|e| LoanwordError(e.to_string()))?,
                )?;
                // JS lengths are numbers (D-049).
                let as_f64 = self.op_result(
                    fcx.block,
                    OperationBuilder::new("arith.sitofp", location)
                        .add_operands(&[raw])
                        .add_results(&[self.mlir_ty(&TsTy::Num)?])
                        .build()
                        .map_err(|e| LoanwordError(e.to_string()))?,
                )?;
                Ok((as_f64, TsTy::Num))
            }
            "bin" => {
                let op = field(node, "op")?
                    .as_str()
                    .ok_or_else(|| LoanwordError("bin op".into()))?;
                // A discriminant test on an unnarrowed union lowers to
                // a TAG compare, not a string compare (D-072) — this is
                // what the promotion pass re-derives facts from.
                if matches!(op, "===" | "!==") {
                    if let Some(result) = self.try_kind_test(
                        fcx,
                        op,
                        field(node, "l")?,
                        field(node, "r")?,
                        location,
                    )? {
                        return Ok(result);
                    }
                }
                let (lhs, lhs_ty) = self.emit_expr(fcx, field(node, "l")?)?;
                let (rhs, _) = self.emit_expr(fcx, field(node, "r")?)?;
                let f64_type = self.mlir_ty(&TsTy::Num)?;
                let i1_type = self.mlir_ty(&TsTy::Bool)?;
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
                    "+" if lhs_ty == TsTy::Str => {
                        let value = self.op_result(
                            block,
                            OperationBuilder::new("frk_str.concat", location)
                                .add_operands(&[lhs, rhs])
                                .add_results(&[self.mlir_ty(&TsTy::Str)?])
                                .build()
                                .map_err(|e| LoanwordError(e.to_string()))?,
                        )?;
                        Ok((value, TsTy::Str))
                    }
                    "===" | "!==" if lhs_ty == TsTy::Str => {
                        let equal = self.op_result(
                            block,
                            OperationBuilder::new("frk_str.eq", location)
                                .add_operands(&[lhs, rhs])
                                .add_results(&[i1_type])
                                .build()
                                .map_err(|e| LoanwordError(e.to_string()))?,
                        )?;
                        if op == "===" {
                            Ok((equal, TsTy::Bool))
                        } else {
                            Ok((self.bool_not_at(block, equal, location)?, TsTy::Bool))
                        }
                    }
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
                                .add_results(&[self.mlir_ty(&TsTy::Num)?])
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
                    .append_block(Block::new(&[(self.mlir_ty(&ty)?, location)]));
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
                        .add_results(&[self.mlir_ty(&target.ret)?])
                        .build()
                        .map_err(|e| LoanwordError(e.to_string()))?,
                )?;
                Ok((value, target.ret.clone()))
            }
            "new" => {
                // `new C(args)` → func.call @C__new (D-073).
                let class = field(node, "c")?
                    .as_u64()
                    .ok_or_else(|| LoanwordError("new class ref".into()))?
                    as usize;
                let decl = self.artifact.class_decl(class)?;
                let name = format!("{}__new", decl.name);
                let mut operands = Vec::new();
                for argument in field(node, "args")?
                    .as_array()
                    .ok_or_else(|| LoanwordError("new args".into()))?
                {
                    operands.push(self.emit_expr(fcx, argument)?.0);
                }
                let value = self.op_result(
                    fcx.block,
                    OperationBuilder::new("func.call", location)
                        .add_attributes(&[(
                            Identifier::new(self.context, "callee"),
                            FlatSymbolRefAttribute::new(self.context, &name).into(),
                        )])
                        .add_operands(&operands)
                        .add_results(&[self.mlir_ty(&TsTy::Class(class))?])
                        .build()
                        .map_err(|e| LoanwordError(e.to_string()))?,
                )?;
                Ok((value, TsTy::Class(class)))
            }
            "mcall" => {
                let (value, ret) = self.emit_mcall(fcx, node, location)?;
                match value {
                    Some(value) => Ok((value, ret)),
                    None => err("void method call in expression position"),
                }
            }
            "obj" => {
                // Union-variant construction (D-072): product chain +
                // make_sum, fields in variant declaration order, kind
                // not stored.
                let index = field(node, "u")?
                    .as_u64()
                    .ok_or_else(|| LoanwordError("obj union ref".into()))?;
                let def = self.artifact.union_at(index as usize)?;
                let v = field(node, "v")?
                    .as_u64()
                    .ok_or_else(|| LoanwordError("obj variant".into()))?
                    as usize;
                let variant = def
                    .variants
                    .get(v)
                    .ok_or_else(|| LoanwordError(format!("variant {v} out of range")))?
                    .clone();
                let items = field(node, "fields")?
                    .as_array()
                    .ok_or_else(|| LoanwordError("obj fields".into()))?;
                if items.len() != variant.fields.len() {
                    return err(format!(
                        "variant '{}' takes {} field(s), literal has {}",
                        variant.kind,
                        variant.fields.len(),
                        items.len()
                    ));
                }
                let mut product = self.op_result(
                    fcx.block,
                    OperationBuilder::new("frk_adt.product_new", location)
                        .add_results(&[Type::parse(self.context, "!frk_adt.product<[]>")
                            .ok_or(LoanwordError("product".into()))?])
                        .build()
                        .map_err(|e| LoanwordError(e.to_string()))?,
                )?;
                let mut grown = Vec::new();
                for (item, (name, declared)) in items.iter().zip(&variant.fields) {
                    let (value, ty) = self.emit_expr(fcx, item)?;
                    if &ty != declared {
                        return err(format!(
                            "field '{name}' of '{}' is {declared:?}, literal supplies {ty:?}",
                            variant.kind
                        ));
                    }
                    grown.push(format!("{}", self.mlir_ty(&ty)?));
                    let product_ty =
                        Type::parse(self.context, &format!("!frk_adt.product<[{}]>", grown.join(", ")))
                            .ok_or(LoanwordError("product".into()))?;
                    product = self.op_result(
                        fcx.block,
                        OperationBuilder::new("frk_adt.product_snoc", location)
                            .add_operands(&[product, value])
                            .add_results(&[product_ty])
                            .build()
                            .map_err(|e| LoanwordError(e.to_string()))?,
                    )?;
                }
                let i64_type: Type = IntegerType::new(self.context, 64).into();
                let sum = self.op_result(
                    fcx.block,
                    OperationBuilder::new("frk_adt.make_sum", location)
                        .add_attributes(&[(
                            Identifier::new(self.context, "variant"),
                            IntegerAttribute::new(i64_type, v as i64).into(),
                        )])
                        .add_operands(&[product])
                        .add_results(&[self.mlir_ty(&TsTy::Union(def.clone()))?])
                        .build()
                        .map_err(|e| LoanwordError(e.to_string()))?,
                )?;
                Ok((sum, TsTy::Union(def)))
            }
            "narrow" => {
                // An IMPORTED flow fact (D-072): emitted as a checked
                // cast; the promotion pass deletes it if provable,
                // else it runs with this blame at runtime.
                let (value, ty) = self.emit_expr(fcx, field(node, "e")?)?;
                let TsTy::Union(def) = ty else {
                    return err(format!("narrow of a non-union value ({ty:?})"));
                };
                let v = field(node, "v")?
                    .as_u64()
                    .ok_or_else(|| LoanwordError("narrow variant".into()))?
                    as usize;
                let variant = def
                    .variants
                    .get(v)
                    .ok_or_else(|| LoanwordError(format!("variant {v} out of range")))?;
                let (line, column) = self.line_col(node).unwrap_or((0, 0));
                let blame = format!(
                    "cast to '{}' at {}:{line}:{column}",
                    variant.kind, self.artifact.file
                );
                let i64_type: Type = IntegerType::new(self.context, 64).into();
                let narrowed = self.op_result(
                    fcx.block,
                    OperationBuilder::new("frk_contract.narrow", location)
                        .add_attributes(&[
                            (
                                Identifier::new(self.context, "variant"),
                                IntegerAttribute::new(i64_type, v as i64).into(),
                            ),
                            (
                                Identifier::new(self.context, "blame"),
                                StringAttribute::new(self.context, &blame).into(),
                            ),
                        ])
                        .add_operands(&[value])
                        .add_results(&[self.mlir_ty(&TsTy::Union(def.clone()))?])
                        .build()
                        .map_err(|e| LoanwordError(e.to_string()))?,
                )?;
                Ok((narrowed, TsTy::Variant(def, v)))
            }
            "prop" => {
                let name = field(node, "name")?
                    .as_str()
                    .ok_or_else(|| LoanwordError("prop name".into()))?;
                let (value, ty) = self.emit_expr(fcx, field(node, "e")?)?;
                match ty {
                    TsTy::Class(class) => {
                        // Field read (D-073): field_get at the slot;
                        // class-ref fields un-erase on the way out.
                        let def = self.artifact.class_at(class)?;
                        let index = def
                            .fields
                            .iter()
                            .position(|(field_name, _)| field_name == name)
                            .ok_or_else(|| {
                                LoanwordError(format!(
                                    "class {} has no field {name:?}",
                                    def.name
                                ))
                            })?;
                        let field_ty = def.fields[index].1.clone();
                        let i64_type: Type = IntegerType::new(self.context, 64).into();
                        let slot_ty =
                            Type::parse(self.context, &self.field_slot_text(&field_ty)?)
                                .ok_or(LoanwordError("slot type".into()))?;
                        let raw = self.op_result(
                            fcx.block,
                            OperationBuilder::new("frk_mem.field_get", location)
                                .add_attributes(&[(
                                    Identifier::new(self.context, "field"),
                                    IntegerAttribute::new(i64_type, index as i64).into(),
                                )])
                                .add_operands(&[value])
                                .add_results(&[slot_ty])
                                .build()
                                .map_err(|e| LoanwordError(e.to_string()))?,
                        )?;
                        let projected = match &field_ty {
                            TsTy::Class(target) => {
                                self.rec_cast(fcx, raw, *target, location)?
                            }
                            _ => raw,
                        };
                        Ok((projected, field_ty))
                    }
                    TsTy::Variant(def, v) => {
                        let variant = &def.variants[v];
                        if name == "kind" {
                            // The discriminant of a KNOWN variant is a
                            // literal (kind is not stored — D-072).
                            let text = variant.kind.clone();
                            return Ok((
                                self.str_lit(fcx, &text, location)?,
                                TsTy::Str,
                            ));
                        }
                        let index = variant
                            .fields
                            .iter()
                            .position(|(field_name, _)| field_name == name)
                            .ok_or_else(|| {
                                LoanwordError(format!(
                                    "variant '{}' has no field '{name}'",
                                    variant.kind
                                ))
                            })?;
                        let field_ty = variant.fields[index].1.clone();
                        let i64_type: Type = IntegerType::new(self.context, 64).into();
                        let extracted = self.op_result(
                            fcx.block,
                            OperationBuilder::new("frk_adt.extract", location)
                                .add_attributes(&[
                                    (
                                        Identifier::new(self.context, "variant"),
                                        IntegerAttribute::new(i64_type, v as i64).into(),
                                    ),
                                    (
                                        Identifier::new(self.context, "field"),
                                        IntegerAttribute::new(i64_type, index as i64).into(),
                                    ),
                                ])
                                .add_operands(&[value])
                                .add_results(&[self.mlir_ty(&field_ty)?])
                                .build()
                                .map_err(|e| LoanwordError(e.to_string()))?,
                        )?;
                        Ok((extracted, field_ty))
                    }
                    TsTy::Union(def) => {
                        if name != "kind" {
                            return err(format!(
                                "field '{name}' on an unnarrowed union (only the discriminant reads)"
                            ));
                        }
                        // tag-selected literal chain: tag_of + selects
                        // over the kind literals, last variant as base.
                        let i64_type: Type = IntegerType::new(self.context, 64).into();
                        let tag = self.op_result(
                            fcx.block,
                            OperationBuilder::new("frk_adt.tag_of", location)
                                .add_operands(&[value])
                                .add_results(&[i64_type])
                                .build()
                                .map_err(|e| LoanwordError(e.to_string()))?,
                        )?;
                        let str_ty = self.mlir_ty(&TsTy::Str)?;
                        let last = def.variants.len() - 1;
                        let mut selected =
                            self.str_lit(fcx, &def.variants[last].kind.clone(), location)?;
                        for v in (0..last).rev() {
                            let expected = self.op_result(
                                fcx.block,
                                melior::dialect::arith::constant(
                                    self.context,
                                    IntegerAttribute::new(i64_type, v as i64).into(),
                                    location,
                                ),
                            )?;
                            let hit = self.op_result(
                                fcx.block,
                                OperationBuilder::new("arith.cmpi", location)
                                    .add_attributes(&[(
                                        Identifier::new(self.context, "predicate"),
                                        IntegerAttribute::new(i64_type, 0).into(),
                                    )])
                                    .add_operands(&[tag, expected])
                                    .add_results(&[IntegerType::new(self.context, 1).into()])
                                    .build()
                                    .map_err(|e| LoanwordError(e.to_string()))?,
                            )?;
                            let literal =
                                self.str_lit(fcx, &def.variants[v].kind.clone(), location)?;
                            selected = self.op_result(
                                fcx.block,
                                OperationBuilder::new("arith.select", location)
                                    .add_operands(&[hit, literal, selected])
                                    .add_results(&[str_ty])
                                    .build()
                                    .map_err(|e| LoanwordError(e.to_string()))?,
                            )?;
                        }
                        Ok((selected, TsTy::Str))
                    }
                    other => err(format!("property '{name}' of {other:?}")),
                }
            }
            other => err(format!("unsupported expression kind {other:?}")),
        }
    }

    /// The static type of an expression node WITHOUT emitting it —
    /// only the shapes a discriminant test can sit on (D-072).
    fn peek_ty(&self, fcx: &Fcx<'c, '_>, node: &Json) -> Result<Option<TsTy>> {
        Ok(match kind(node)? {
            "var" => {
                let name = field(node, "name")?
                    .as_str()
                    .ok_or_else(|| LoanwordError("var name".into()))?;
                fcx.env.get(name).map(|binding| match binding {
                    Binding::Value(_, ty) | Binding::Boxed(_, ty) => ty.clone(),
                })
            }
            "obj" => {
                let index = field(node, "u")?
                    .as_u64()
                    .ok_or_else(|| LoanwordError("obj union ref".into()))?;
                Some(TsTy::Union(self.artifact.union_at(index as usize)?))
            }
            _ => None,
        })
    }

    /// `<union>.kind === "<lit>"` (either side) → tag_of + cmpi
    /// (D-072). Returns None when the shape does not match — the
    /// caller falls through to ordinary emission (e.g. a NARROWED
    /// kind read, which is a constant string compare).
    fn try_kind_test<'r>(
        &self,
        fcx: &mut Fcx<'c, 'r>,
        op: &str,
        left: &Json,
        right: &Json,
        location: Location<'c>,
    ) -> Result<Option<(Value<'c, 'r>, TsTy)>> {
        for (subject, literal) in [(left, right), (right, left)] {
            if kind(subject)? != "prop" {
                continue;
            }
            if field(subject, "name")?.as_str() != Some("kind") {
                continue;
            }
            let inner = field(subject, "e")?;
            let Some(TsTy::Union(def)) = self.peek_ty(fcx, inner)? else {
                continue;
            };
            if kind(literal)? != "str" {
                continue;
            }
            let text = field(literal, "v")?
                .as_str()
                .ok_or_else(|| LoanwordError("str literal".into()))?;
            let Some(v) = def.variants.iter().position(|m| m.kind == text) else {
                return err(format!(
                    "kind test against {text:?} — not a variant (tsc refuses this comparison)"
                ));
            };
            let (value, _) = self.emit_expr(fcx, inner)?;
            let i64_type: Type = IntegerType::new(self.context, 64).into();
            let tag = self.op_result(
                fcx.block,
                OperationBuilder::new("frk_adt.tag_of", location)
                    .add_operands(&[value])
                    .add_results(&[i64_type])
                    .build()
                    .map_err(|e| LoanwordError(e.to_string()))?,
            )?;
            let expected = self.op_result(
                fcx.block,
                melior::dialect::arith::constant(
                    self.context,
                    IntegerAttribute::new(i64_type, v as i64).into(),
                    location,
                ),
            )?;
            // arith CmpIPredicate: eq = 0, ne = 1.
            let predicate = if op == "===" { 0 } else { 1 };
            let compared = self.op_result(
                fcx.block,
                OperationBuilder::new("arith.cmpi", location)
                    .add_attributes(&[(
                        Identifier::new(self.context, "predicate"),
                        IntegerAttribute::new(i64_type, predicate).into(),
                    )])
                    .add_operands(&[tag, expected])
                    .add_results(&[IntegerType::new(self.context, 1).into()])
                    .build()
                    .map_err(|e| LoanwordError(e.to_string()))?,
            )?;
            return Ok(Some((compared, TsTy::Bool)));
        }
        Ok(None)
    }

    /// An `frk_str.lit` value (UTF-16 string literal, D-049).
    fn str_lit<'r>(
        &self,
        fcx: &Fcx<'c, 'r>,
        text: &str,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        self.op_result(
            fcx.block,
            OperationBuilder::new("frk_str.lit", location)
                .add_attributes(&[(
                    Identifier::new(self.context, "text"),
                    StringAttribute::new(self.context, text).into(),
                )])
                .add_results(&[self.mlir_ty(&TsTy::Str)?])
                .build()
                .map_err(|e| LoanwordError(e.to_string()))?,
        )
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
                .add_results(&[self.mlir_ty(&TsTy::Num)?])
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
                .add_results(&[self.mlir_ty(&TsTy::Bool)?])
                .build()
                .map_err(|e| LoanwordError(e.to_string()))?,
        )
    }

    /// JS index (a number) → i64 via fptosi; corpus fence: integral,
    /// in-bounds (D-049).
    fn index_value<'r>(
        &self,
        fcx: &mut Fcx<'c, 'r>,
        node: &Json,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        let (raw, _) = self.emit_expr(fcx, node)?;
        let i64_type: Type = IntegerType::new(self.context, 64).into();
        self.op_result(
            fcx.block,
            OperationBuilder::new("arith.fptosi", location)
                .add_operands(&[raw])
                .add_results(&[i64_type])
                .build()
                .map_err(|e| LoanwordError(e.to_string()))?,
        )
    }

    fn bool_not_at<'r>(
        &self,
        block: BlockRef<'c, 'r>,
        value: Value<'c, 'r>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'r>> {
        let i1_type: Type = IntegerType::new(self.context, 1).into();
        let one = self.op_result(
            block,
            melior::dialect::arith::constant(
                self.context,
                IntegerAttribute::new(i1_type, 1).into(),
                location,
            ),
        )?;
        self.op_result(
            block,
            OperationBuilder::new("arith.xori", location)
                .add_operands(&[value, one])
                .add_results(&[i1_type])
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
