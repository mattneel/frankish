//! HM inference with let-polymorphism for ml_core (SPEC §6.4).
//! Produces a fully-zonked typed AST for emission.
//!
//! Polymorphism stance (D-038): generalization and instantiation are
//! real (value restriction: only `fun` right-hand sides generalize), but
//! v0 EMISSION is monomorphic — a generalized binding used at zero
//! instantiations is dropped, at exactly one (distinct) instantiation it
//! is concretized to it, and at several it is a compile error until the
//! monomorphization pass lands (v0.2). Recursive ADTs are rejected here
//! too: the structural type encoding cannot express them until the
//! memory axis brings indirection (M7).

use std::collections::{HashMap, HashSet};

use ena::unify::InPlaceUnificationTable;

use crate::ast::{BinOp, Binding, Expr, NodeId, Param, Pattern, Program, TypeDef, TypeExpr};
use crate::types::{Ty, TyVid};

#[derive(Debug)]
pub struct TypeError(pub String);

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "type error: {}", self.0)
    }
}

impl std::error::Error for TypeError {}

#[derive(Clone, Debug)]
pub struct CtorInfo {
    pub adt: String,
    pub tag: usize,
    pub payload: Vec<Ty>,
}

#[derive(Clone, Debug)]
pub struct AdtInfo {
    /// Constructors in declaration (= tag) order: (name, payload types).
    pub ctors: Vec<(String, Vec<Ty>)>,
}

#[derive(Clone, Debug)]
pub struct TExpr {
    pub ty: Ty,
    pub kind: TKind,
}

#[derive(Clone, Debug)]
pub enum TKind {
    Unit,
    Int(i64),
    Bool(bool),
    Var(String),
    /// Constructor application, resolved: payload is one typed expr per
    /// FIELD (multi-payload constructors take syntactic tuples, mirrored
    /// from OCaml's arity rule).
    MakeCtor { adt: String, tag: usize, payload: Vec<TExpr> },
    Tuple(Vec<TExpr>),
    Neg(Box<TExpr>),
    Bin { op: BinOp, lhs: Box<TExpr>, rhs: Box<TExpr> },
    If { cond: Box<TExpr>, then: Box<TExpr>, els: Box<TExpr> },
    Fun { id: NodeId, param: Option<String>, param_ty: Ty, body: Box<TExpr> },
    App { func: Box<TExpr>, arg: Box<TExpr> },
    Let { rec: bool, bindings: Vec<TBinding>, body: Box<TExpr> },
    Match { scrutinee: Box<TExpr>, arms: Vec<(Pattern, TExpr)> },
}

#[derive(Clone, Debug)]
pub struct TBinding {
    pub id: NodeId,
    pub name: String,
    pub expr: TExpr,
    /// True when this binding was generalized and ended up unused —
    /// emission skips it.
    pub dead: bool,
}

#[derive(Debug)]
pub struct TypedProgram {
    pub adts: HashMap<String, AdtInfo>,
    pub ctors: HashMap<String, CtorInfo>,
    /// Top-level declarations, typed. `main : unit -> int` is enforced.
    pub decls: Vec<(bool, Vec<TBinding>)>,
}

#[derive(Clone)]
struct Scheme {
    vars: Vec<TyVid>,
    ty: Ty,
}

pub fn infer(program: &Program) -> Result<TypedProgram, TypeError> {
    let mut cx = Cx {
        table: InPlaceUnificationTable::new(),
        adts: HashMap::new(),
        ctors: HashMap::new(),
        instantiations: HashMap::new(),
        generalized: HashMap::new(),
    };
    cx.declare_adts(&program.typedefs)?;

    let mut env: HashMap<String, Scheme> = HashMap::new();
    let mut decls = Vec::new();
    for (recursive, bindings) in &program.decls {
        let typed = cx.infer_bindings(&mut env, *recursive, bindings)?;
        decls.push((*recursive, typed));
    }

    // Entry protocol: main : unit -> int.
    let main = env
        .get("main")
        .ok_or_else(|| TypeError("no `main` declaration (expected `let main () = ...`)".into()))?
        .clone();
    let main_ty = cx.instantiate(&main);
    cx.unify(
        &main_ty,
        &Ty::Fun(Box::new(Ty::Unit), Box::new(Ty::Int)),
    )
    .map_err(|e| TypeError(format!("main must have type unit -> int: {}", e.0)))?;

    // Concretize single-instantiation generalized bindings; error on
    // several; mark unused ones dead (D-038).
    let generalized = cx.generalized.clone();
    let mut dead = HashSet::new();
    for (binding_id, (name, scheme_vars)) in &generalized {
        let instantiations = cx.instantiations.get(binding_id).cloned().unwrap_or_default();
        let mut distinct: Vec<Vec<Ty>> = Vec::new();
        for inst in &instantiations {
            let resolved: Vec<Ty> = inst.iter().map(|vid| cx.resolve(&Ty::Var(*vid))).collect();
            if !distinct.contains(&resolved) {
                distinct.push(resolved);
            }
        }
        match distinct.len() {
            0 => {
                dead.insert(*binding_id);
            }
            1 => {
                for (scheme_var, concrete) in scheme_vars.iter().zip(&distinct[0]) {
                    let var = Ty::Var(*scheme_var);
                    cx.unify(&var, concrete).map_err(|e| {
                        TypeError(format!("concretizing `{name}`: {}", e.0))
                    })?;
                }
            }
            n => {
                return Err(TypeError(format!(
                    "`{name}` is used at {n} distinct types; polymorphic emission is \
                     fenced to v0.2 (D-038)"
                )));
            }
        }
    }

    // Zonk everything to concrete types.
    let mut zonked = Vec::new();
    for (recursive, bindings) in decls {
        let bindings = bindings
            .into_iter()
            .map(|mut binding| {
                binding.dead = dead.contains(&binding.id);
                if !binding.dead {
                    cx.zonk_expr(&mut binding.expr, &dead)?;
                }
                Ok(binding)
            })
            .collect::<Result<Vec<_>, TypeError>>()?;
        zonked.push((recursive, bindings));
    }

    Ok(TypedProgram {
        adts: cx.adts,
        ctors: cx.ctors,
        decls: zonked,
    })
}

struct Cx {
    table: InPlaceUnificationTable<TyVid>,
    adts: HashMap<String, AdtInfo>,
    ctors: HashMap<String, CtorInfo>,
    /// binding id → one entry per use site: the fresh vars that
    /// instantiated the scheme there.
    instantiations: HashMap<NodeId, Vec<Vec<TyVid>>>,
    /// binding id → (name, the scheme's quantified vars).
    generalized: HashMap<NodeId, (String, Vec<TyVid>)>,
}

impl Cx {
    fn declare_adts(&mut self, typedefs: &[TypeDef]) -> Result<(), TypeError> {
        // First pass: names (so payloads may reference earlier adts).
        for def in typedefs {
            if self.adts.contains_key(&def.name) {
                return Err(TypeError(format!("duplicate type {}", def.name)));
            }
            self.adts.insert(def.name.clone(), AdtInfo { ctors: Vec::new() });
        }
        for def in typedefs {
            let mut ctors = Vec::new();
            for (tag, (ctor_name, payload)) in def.ctors.iter().enumerate() {
                let payload: Vec<Ty> = payload
                    .iter()
                    .map(|texpr| self.resolve_type_expr(texpr, &def.name))
                    .collect::<Result<_, _>>()?;
                if self.ctors.contains_key(ctor_name) {
                    return Err(TypeError(format!("duplicate constructor {ctor_name}")));
                }
                self.ctors.insert(
                    ctor_name.clone(),
                    CtorInfo { adt: def.name.clone(), tag, payload: payload.clone() },
                );
                ctors.push((ctor_name.clone(), payload));
            }
            self.adts.get_mut(&def.name).unwrap().ctors = ctors;
        }
        Ok(())
    }

    fn resolve_type_expr(&self, texpr: &TypeExpr, defining: &str) -> Result<Ty, TypeError> {
        Ok(match texpr {
            TypeExpr::Int => Ty::Int,
            TypeExpr::Bool => Ty::Bool,
            TypeExpr::Unit => Ty::Unit,
            TypeExpr::Tuple(items) => Ty::Tuple(
                items
                    .iter()
                    .map(|item| self.resolve_type_expr(item, defining))
                    .collect::<Result<_, _>>()?,
            ),
            TypeExpr::Named(name) => {
                if name == defining {
                    return Err(TypeError(format!(
                        "recursive ADT `{name}` is fenced to v0.2 (D-038): the \
                         structural type encoding needs the memory axis (M7)"
                    )));
                }
                if !self.adts.contains_key(name) {
                    return Err(TypeError(format!("unknown type {name}")));
                }
                Ty::Adt(name.clone())
            }
        })
    }

    fn fresh(&mut self) -> Ty {
        Ty::Var(self.table.new_key(None))
    }

    fn resolve(&mut self, ty: &Ty) -> Ty {
        match ty {
            Ty::Var(vid) => {
                let root = self.table.find(*vid);
                match self.table.probe_value(root) {
                    Some(bound) => self.resolve(&bound),
                    None => Ty::Var(root),
                }
            }
            Ty::Tuple(items) => Ty::Tuple(items.iter().map(|t| self.resolve(t)).collect()),
            Ty::Fun(a, b) => Ty::Fun(Box::new(self.resolve(a)), Box::new(self.resolve(b))),
            other => other.clone(),
        }
    }

    fn occurs(&mut self, vid: TyVid, ty: &Ty) -> bool {
        match self.resolve(ty) {
            Ty::Var(other) => other == vid,
            Ty::Tuple(items) => items.iter().any(|t| self.occurs(vid, t)),
            Ty::Fun(a, b) => self.occurs(vid, &a) || self.occurs(vid, &b),
            _ => false,
        }
    }

    fn unify(&mut self, a: &Ty, b: &Ty) -> Result<(), TypeError> {
        let a = self.resolve(a);
        let b = self.resolve(b);
        match (&a, &b) {
            (Ty::Var(x), Ty::Var(y)) if x == y => Ok(()),
            (Ty::Var(x), Ty::Var(y)) => self
                .table
                .unify_var_var(*x, *y)
                .map_err(|_| TypeError("variable merge conflict".into())),
            (Ty::Var(x), other) | (other, Ty::Var(x)) => {
                if self.occurs(*x, other) {
                    return Err(TypeError(format!("infinite type: 't{} = {other}", x.0)));
                }
                self.table
                    .unify_var_value(*x, Some(other.clone()))
                    .map_err(|_| TypeError("variable assignment conflict".into()))
            }
            (Ty::Unit, Ty::Unit) | (Ty::Bool, Ty::Bool) | (Ty::Int, Ty::Int) => Ok(()),
            (Ty::Adt(x), Ty::Adt(y)) if x == y => Ok(()),
            (Ty::Tuple(xs), Ty::Tuple(ys)) if xs.len() == ys.len() => {
                for (x, y) in xs.iter().zip(ys) {
                    self.unify(x, y)?;
                }
                Ok(())
            }
            (Ty::Fun(a1, r1), Ty::Fun(a2, r2)) => {
                self.unify(a1, a2)?;
                self.unify(r1, r2)
            }
            _ => Err(TypeError(format!("cannot unify {a} with {b}"))),
        }
    }

    fn free_vars(&mut self, ty: &Ty, out: &mut Vec<TyVid>) {
        match self.resolve(ty) {
            Ty::Var(vid) => {
                if !out.contains(&vid) {
                    out.push(vid);
                }
            }
            Ty::Tuple(items) => {
                for item in items {
                    self.free_vars(&item, out);
                }
            }
            Ty::Fun(a, b) => {
                self.free_vars(&a, out);
                self.free_vars(&b, out);
            }
            _ => {}
        }
    }

    fn instantiate(&mut self, scheme: &Scheme) -> Ty {
        if scheme.vars.is_empty() {
            return scheme.ty.clone();
        }
        let fresh: Vec<TyVid> = scheme
            .vars
            .iter()
            .map(|_| match self.fresh() {
                Ty::Var(vid) => vid,
                _ => unreachable!(),
            })
            .collect();
        let map: HashMap<TyVid, TyVid> =
            scheme.vars.iter().copied().zip(fresh.iter().copied()).collect();
        self.substitute(&scheme.ty, &map)
    }

    /// Instantiate AND remember which fresh vars stand for the scheme
    /// vars at this use site (for D-038 concretization).
    fn instantiate_recorded(&mut self, binding_id: NodeId, scheme: &Scheme) -> Ty {
        if scheme.vars.is_empty() {
            return scheme.ty.clone();
        }
        let fresh: Vec<TyVid> = scheme
            .vars
            .iter()
            .map(|_| match self.fresh() {
                Ty::Var(vid) => vid,
                _ => unreachable!(),
            })
            .collect();
        self.instantiations
            .entry(binding_id)
            .or_default()
            .push(fresh.clone());
        let map: HashMap<TyVid, TyVid> =
            scheme.vars.iter().copied().zip(fresh).collect();
        self.substitute(&scheme.ty, &map)
    }

    fn substitute(&mut self, ty: &Ty, map: &HashMap<TyVid, TyVid>) -> Ty {
        match self.resolve(ty) {
            Ty::Var(vid) => match map.get(&vid) {
                Some(fresh) => Ty::Var(*fresh),
                None => Ty::Var(vid),
            },
            Ty::Tuple(items) => {
                Ty::Tuple(items.iter().map(|t| self.substitute(t, map)).collect())
            }
            Ty::Fun(a, b) => Ty::Fun(
                Box::new(self.substitute(&a, map)),
                Box::new(self.substitute(&b, map)),
            ),
            other => other,
        }
    }

    fn infer_bindings(
        &mut self,
        env: &mut HashMap<String, Scheme>,
        recursive: bool,
        bindings: &[Binding],
    ) -> Result<Vec<TBinding>, TypeError> {
        let mut typed = Vec::new();
        if recursive {
            for binding in bindings {
                if !matches!(binding.expr, Expr::Fun { .. }) {
                    return Err(TypeError(format!(
                        "`let rec {}` must bind a function",
                        binding.name
                    )));
                }
            }
            // Monomorphic assumptions for the whole group.
            let assumed: Vec<Ty> = bindings.iter().map(|_| self.fresh()).collect();
            let mut group_env = env.clone();
            for (binding, ty) in bindings.iter().zip(&assumed) {
                group_env.insert(
                    binding.name.clone(),
                    Scheme { vars: Vec::new(), ty: ty.clone() },
                );
            }
            for (binding, assumed_ty) in bindings.iter().zip(&assumed) {
                let texpr = self.infer_expr(&group_env, &binding.expr)?;
                self.unify(&texpr.ty, assumed_ty)?;
                typed.push(TBinding {
                    id: binding.id,
                    name: binding.name.clone(),
                    expr: texpr,
                    dead: false,
                });
            }
            // Generalize after the group (value restriction holds: all funs).
            for (binding, assumed_ty) in bindings.iter().zip(&assumed) {
                let scheme = self.generalize(env, assumed_ty);
                if !scheme.vars.is_empty() {
                    self.generalized
                        .insert(binding.id, (binding.name.clone(), scheme.vars.clone()));
                }
                env.insert(binding.name.clone(), scheme);
            }
        } else {
            for binding in bindings {
                let texpr = self.infer_expr(env, &binding.expr)?;
                let scheme = if matches!(binding.expr, Expr::Fun { .. }) {
                    let scheme = self.generalize(env, &texpr.ty);
                    if !scheme.vars.is_empty() {
                        self.generalized
                            .insert(binding.id, (binding.name.clone(), scheme.vars.clone()));
                    }
                    scheme
                } else {
                    Scheme { vars: Vec::new(), ty: texpr.ty.clone() }
                };
                env.insert(binding.name.clone(), scheme);
                typed.push(TBinding {
                    id: binding.id,
                    name: binding.name.clone(),
                    expr: texpr,
                    dead: false,
                });
            }
        }
        Ok(typed)
    }

    fn generalize(&mut self, env: &HashMap<String, Scheme>, ty: &Ty) -> Scheme {
        let mut env_vars = Vec::new();
        for scheme in env.values() {
            let inner = scheme.ty.clone();
            self.free_vars(&inner, &mut env_vars);
        }
        let mut ty_vars = Vec::new();
        self.free_vars(ty, &mut ty_vars);
        let vars: Vec<TyVid> = ty_vars
            .into_iter()
            .filter(|vid| !env_vars.contains(vid))
            .collect();
        Scheme { vars, ty: self.resolve(ty) }
    }

    fn infer_expr(
        &mut self,
        env: &HashMap<String, Scheme>,
        expr: &Expr,
    ) -> Result<TExpr, TypeError> {
        Ok(match expr {
            Expr::Unit => TExpr { ty: Ty::Unit, kind: TKind::Unit },
            Expr::Int(value) => TExpr { ty: Ty::Int, kind: TKind::Int(*value) },
            Expr::Bool(value) => TExpr { ty: Ty::Bool, kind: TKind::Bool(*value) },
            Expr::Var(name) => {
                let scheme = env
                    .get(name)
                    .ok_or_else(|| TypeError(format!("unbound variable {name}")))?
                    .clone();
                // Recorded instantiation, keyed by the binding that
                // introduced this name — the env doesn't track ids, so
                // record under the generalized table's entry if any.
                let ty = if scheme.vars.is_empty() {
                    scheme.ty.clone()
                } else {
                    let binding_id = self
                        .generalized
                        .iter()
                        .find(|(_, (n, _))| n == name)
                        .map(|(id, _)| *id);
                    match binding_id {
                        Some(id) => self.instantiate_recorded(id, &scheme),
                        None => self.instantiate(&scheme),
                    }
                };
                TExpr { ty, kind: TKind::Var(name.clone()) }
            }
            Expr::Ctor { name, arg } => {
                let info = self
                    .ctors
                    .get(name)
                    .ok_or_else(|| TypeError(format!("unknown constructor {name}")))?
                    .clone();
                let payload = self.check_ctor_payload(env, &info, arg.as_deref(), name)?;
                TExpr {
                    ty: Ty::Adt(info.adt.clone()),
                    kind: TKind::MakeCtor { adt: info.adt, tag: info.tag, payload },
                }
            }
            Expr::Tuple(items) => {
                let typed: Vec<TExpr> = items
                    .iter()
                    .map(|item| self.infer_expr(env, item))
                    .collect::<Result<_, _>>()?;
                TExpr {
                    ty: Ty::Tuple(typed.iter().map(|t| t.ty.clone()).collect()),
                    kind: TKind::Tuple(typed),
                }
            }
            Expr::Neg(inner) => {
                let typed = self.infer_expr(env, inner)?;
                self.unify(&typed.ty, &Ty::Int)?;
                TExpr { ty: Ty::Int, kind: TKind::Neg(Box::new(typed)) }
            }
            Expr::Bin { op, lhs, rhs } => {
                let lhs = self.infer_expr(env, lhs)?;
                let rhs = self.infer_expr(env, rhs)?;
                let ty = match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                        self.unify(&lhs.ty, &Ty::Int)?;
                        self.unify(&rhs.ty, &Ty::Int)?;
                        Ty::Int
                    }
                    BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                        self.unify(&lhs.ty, &Ty::Int)?;
                        self.unify(&rhs.ty, &Ty::Int)?;
                        Ty::Bool
                    }
                    BinOp::Eq | BinOp::Ne => {
                        self.unify(&lhs.ty, &rhs.ty)?;
                        let resolved = self.resolve(&lhs.ty);
                        if !matches!(resolved, Ty::Int | Ty::Bool) {
                            return Err(TypeError(format!(
                                "= and <> compare ints or bools in v0.1, got {resolved}"
                            )));
                        }
                        Ty::Bool
                    }
                    BinOp::AndAlso | BinOp::OrElse => {
                        self.unify(&lhs.ty, &Ty::Bool)?;
                        self.unify(&rhs.ty, &Ty::Bool)?;
                        Ty::Bool
                    }
                };
                TExpr {
                    ty,
                    kind: TKind::Bin { op: *op, lhs: Box::new(lhs), rhs: Box::new(rhs) },
                }
            }
            Expr::If { cond, then, els } => {
                let cond = self.infer_expr(env, cond)?;
                self.unify(&cond.ty, &Ty::Bool)?;
                let then = self.infer_expr(env, then)?;
                let els = self.infer_expr(env, els)?;
                self.unify(&then.ty, &els.ty)?;
                TExpr {
                    ty: then.ty.clone(),
                    kind: TKind::If {
                        cond: Box::new(cond),
                        then: Box::new(then),
                        els: Box::new(els),
                    },
                }
            }
            Expr::Fun { id, param, body } => {
                let param_ty = match param {
                    Param::Named(_) => self.fresh(),
                    Param::Unit => Ty::Unit,
                };
                let mut inner = env.clone();
                let param_name = match param {
                    Param::Named(name) => {
                        inner.insert(
                            name.clone(),
                            Scheme { vars: Vec::new(), ty: param_ty.clone() },
                        );
                        Some(name.clone())
                    }
                    Param::Unit => None,
                };
                let body = self.infer_expr(&inner, body)?;
                TExpr {
                    ty: Ty::Fun(Box::new(param_ty.clone()), Box::new(body.ty.clone())),
                    kind: TKind::Fun {
                        id: *id,
                        param: param_name,
                        param_ty,
                        body: Box::new(body),
                    },
                }
            }
            Expr::App { func, arg } => {
                let func = self.infer_expr(env, func)?;
                let arg = self.infer_expr(env, arg)?;
                let result = self.fresh();
                self.unify(
                    &func.ty,
                    &Ty::Fun(Box::new(arg.ty.clone()), Box::new(result.clone())),
                )?;
                TExpr {
                    ty: result,
                    kind: TKind::App { func: Box::new(func), arg: Box::new(arg) },
                }
            }
            Expr::Let { rec, bindings, body } => {
                let mut inner = env.clone();
                let typed = self.infer_bindings(&mut inner, *rec, bindings)?;
                let body = self.infer_expr(&inner, body)?;
                TExpr {
                    ty: body.ty.clone(),
                    kind: TKind::Let { rec: *rec, bindings: typed, body: Box::new(body) },
                }
            }
            Expr::Match { scrutinee, arms } => {
                let scrutinee = self.infer_expr(env, scrutinee)?;
                let result = self.fresh();
                let mut typed_arms = Vec::new();
                for (pattern, body) in arms {
                    let mut inner = env.clone();
                    self.check_pattern(&mut inner, pattern, &scrutinee.ty)?;
                    let body = self.infer_expr(&inner, body)?;
                    self.unify(&body.ty, &result)?;
                    typed_arms.push((pattern.clone(), body));
                }
                TExpr {
                    ty: result,
                    kind: TKind::Match { scrutinee: Box::new(scrutinee), arms: typed_arms },
                }
            }
        })
    }

    fn check_ctor_payload(
        &mut self,
        env: &HashMap<String, Scheme>,
        info: &CtorInfo,
        arg: Option<&Expr>,
        name: &str,
    ) -> Result<Vec<TExpr>, TypeError> {
        match (info.payload.len(), arg) {
            (0, None) => Ok(Vec::new()),
            (0, Some(_)) => Err(TypeError(format!("{name} takes no payload"))),
            (_, None) => Err(TypeError(format!("{name} expects a payload"))),
            (1, Some(expr)) => {
                let typed = self.infer_expr(env, expr)?;
                self.unify(&typed.ty, &info.payload[0])?;
                Ok(vec![typed])
            }
            (n, Some(Expr::Tuple(items))) if items.len() == n => {
                let mut fields = Vec::new();
                for (item, field_ty) in items.iter().zip(&info.payload) {
                    let typed = self.infer_expr(env, item)?;
                    self.unify(&typed.ty, field_ty)?;
                    fields.push(typed);
                }
                Ok(fields)
            }
            (n, Some(_)) => Err(TypeError(format!(
                "{name} expects a {n}-tuple payload (OCaml constructor arity rule)"
            ))),
        }
    }

    fn check_pattern(
        &mut self,
        env: &mut HashMap<String, Scheme>,
        pattern: &Pattern,
        ty: &Ty,
    ) -> Result<(), TypeError> {
        match pattern {
            Pattern::Wild => Ok(()),
            Pattern::Var(name) => {
                env.insert(name.clone(), Scheme { vars: Vec::new(), ty: ty.clone() });
                Ok(())
            }
            Pattern::Int(_) => self.unify(ty, &Ty::Int),
            Pattern::Bool(_) => self.unify(ty, &Ty::Bool),
            Pattern::Unit => self.unify(ty, &Ty::Unit),
            Pattern::Tuple(items) => {
                let vars: Vec<Ty> = items.iter().map(|_| self.fresh()).collect();
                self.unify(ty, &Ty::Tuple(vars.clone()))?;
                for (item, item_ty) in items.iter().zip(&vars) {
                    self.check_pattern(env, item, item_ty)?;
                }
                Ok(())
            }
            Pattern::Ctor { name, arg } => {
                let info = self
                    .ctors
                    .get(name)
                    .ok_or_else(|| TypeError(format!("unknown constructor {name}")))?
                    .clone();
                self.unify(ty, &Ty::Adt(info.adt.clone()))?;
                match (info.payload.len(), arg) {
                    (0, None) => Ok(()),
                    (0, Some(_)) => Err(TypeError(format!("{name} takes no payload"))),
                    (_, None) => Err(TypeError(format!("{name} expects a payload"))),
                    (1, Some(inner)) => self.check_pattern(env, inner, &info.payload[0]),
                    (n, Some(inner)) => match inner.as_ref() {
                        Pattern::Tuple(items) if items.len() == n => {
                            for (item, field_ty) in items.iter().zip(&info.payload) {
                                self.check_pattern(env, item, field_ty)?;
                            }
                            Ok(())
                        }
                        Pattern::Wild => Ok(()),
                        _ => Err(TypeError(format!(
                            "{name} patterns take a {n}-tuple payload"
                        ))),
                    },
                }
            }
        }
    }

    fn zonk_ty(&mut self, ty: &Ty) -> Result<Ty, TypeError> {
        let resolved = self.resolve(ty);
        match &resolved {
            Ty::Var(vid) => Err(TypeError(format!(
                "ambiguous type 't{} — add an annotation-free monomorphic use",
                vid.0
            ))),
            Ty::Tuple(items) => Ok(Ty::Tuple(
                items.iter().map(|t| self.zonk_ty(t)).collect::<Result<_, _>>()?,
            )),
            Ty::Fun(a, b) => Ok(Ty::Fun(
                Box::new(self.zonk_ty(a)?),
                Box::new(self.zonk_ty(b)?),
            )),
            _ => Ok(resolved),
        }
    }

    fn zonk_expr(
        &mut self,
        expr: &mut TExpr,
        dead: &HashSet<NodeId>,
    ) -> Result<(), TypeError> {
        expr.ty = self.zonk_ty(&expr.ty)?;
        match &mut expr.kind {
            TKind::Unit | TKind::Int(_) | TKind::Bool(_) | TKind::Var(_) => Ok(()),
            TKind::MakeCtor { payload, .. } => {
                for field in payload {
                    self.zonk_expr(field, dead)?;
                }
                Ok(())
            }
            TKind::Tuple(items) => {
                for item in items {
                    self.zonk_expr(item, dead)?;
                }
                Ok(())
            }
            TKind::Neg(inner) => self.zonk_expr(inner, dead),
            TKind::Bin { lhs, rhs, .. } => {
                self.zonk_expr(lhs, dead)?;
                self.zonk_expr(rhs, dead)
            }
            TKind::If { cond, then, els } => {
                self.zonk_expr(cond, dead)?;
                self.zonk_expr(then, dead)?;
                self.zonk_expr(els, dead)
            }
            TKind::Fun { param_ty, body, .. } => {
                *param_ty = self.zonk_ty(param_ty)?;
                self.zonk_expr(body, dead)
            }
            TKind::App { func, arg } => {
                self.zonk_expr(func, dead)?;
                self.zonk_expr(arg, dead)
            }
            TKind::Let { bindings, body, .. } => {
                for binding in bindings {
                    binding.dead = dead.contains(&binding.id);
                    if !binding.dead {
                        self.zonk_expr(&mut binding.expr, dead)?;
                    }
                }
                self.zonk_expr(body, dead)
            }
            TKind::Match { scrutinee, arms } => {
                self.zonk_expr(scrutinee, dead)?;
                for (_, body) in arms {
                    self.zonk_expr(body, dead)?;
                }
                Ok(())
            }
        }
    }
}
