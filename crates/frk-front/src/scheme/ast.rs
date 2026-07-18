//! Core forms for r7rs_core v0 (D-060): the `Datum` stream parsed into
//! a small expression tree. Design-independent — the emitter decides
//! how these map to kernel dialects. Scope per the ratified MANIFEST:
//! define / lambda / if / let / let* / letrec / begin, applications,
//! fixnum + boolean literals. `call/ec` and `error` are ordinary
//! applications here; the emitter recognizes the primitive names.

use super::reader::{Datum, Span, read};

#[derive(Clone, Debug)]
pub enum Expr {
    Int(i64, Span),
    Bool(bool, Span),
    Var(String, Span),
    If(Box<Expr>, Box<Expr>, Box<Expr>, Span),
    /// `let` (parallel), `let*` (sequential), `letrec` (recursive) —
    /// distinguished by [`LetKind`]; body is an implicit `begin`.
    Let(LetKind, Vec<(String, Expr)>, Vec<Expr>, Span),
    Lambda(Vec<String>, Vec<Expr>, Span),
    Begin(Vec<Expr>, Span),
    App(Box<Expr>, Vec<Expr>, Span),
    /// `(quote d)` / `'d` — the datum as data (v0.1, D-070).
    Quote(Datum, Span),
    /// A string literal (M31, D-077).
    Str(String, Span),
    /// `(guard (var clause… [else …]) body…)` — D-081.5. Clauses are
    /// `(test expr…)` pairs; `(test)` and `(test => proc)` are
    /// PARSE-TIME rejections (a lax misparse would silently diverge).
    Guard {
        var: String,
        clauses: Vec<(Expr, Vec<Expr>)>,
        else_body: Option<Vec<Expr>>,
        body: Vec<Expr>,
        span: Span,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LetKind {
    Let,
    LetStar,
    LetRec,
}

/// A top-level form: a definition, or a bare expression to evaluate
/// (e.g. `(display …)`).
#[derive(Clone, Debug)]
pub enum Top {
    /// `(define name expr)` or `(define (name params…) body…)`.
    Define(String, Expr, Span),
    Expr(Expr),
}

pub type Program = Vec<Top>;

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Int(_, s)
            | Expr::Bool(_, s)
            | Expr::Var(_, s)
            | Expr::If(_, _, _, s)
            | Expr::Let(_, _, _, s)
            | Expr::Lambda(_, _, s)
            | Expr::Begin(_, s)
            | Expr::App(_, _, s)
            | Expr::Quote(_, s)
            | Expr::Str(_, s) => *s,
            Expr::Guard { span, .. } => *span,
        }
    }
}

pub fn parse(source: &str) -> Result<Program, String> {
    let data = read(source)?;
    let mut program = Vec::new();
    for datum in &data {
        // `(import …)` is required by the chibi oracle (`(scheme base)`
        // supplies call/cc et al.) but carries no v0 semantics — the
        // whole subset is always in scope. Skip it.
        if is_import(datum) {
            continue;
        }
        program.push(parse_top(datum)?);
    }
    if program.is_empty() {
        return Err("empty program".to_string());
    }
    Ok(program)
}

fn is_import(datum: &Datum) -> bool {
    matches!(datum, Datum::List(items, _)
        if matches!(items.first(), Some(Datum::Symbol(head, _)) if head == "import"))
}

fn parse_top(datum: &Datum) -> Result<Top, String> {
    if let Datum::List(items, span) = datum {
        if let Some(Datum::Symbol(head, _)) = items.first() {
            if head == "define" {
                return parse_define(items, *span);
            }
        }
    }
    Ok(Top::Expr(parse_expr(datum)?))
}

fn parse_define(items: &[Datum], span: Span) -> Result<Top, String> {
    // (define name expr)  |  (define (name param…) body…)
    match items.get(1) {
        Some(Datum::Symbol(name, _)) => {
            if items.len() != 3 {
                return Err(format!("(define {name} expr) takes exactly one value"));
            }
            Ok(Top::Define(name.clone(), parse_expr(&items[2])?, span))
        }
        Some(Datum::List(signature, _)) => {
            let (name, params) = parse_signature(signature)?;
            let body = parse_body(&items[2..])?;
            let lambda = Expr::Lambda(params, body, span);
            Ok(Top::Define(name, lambda, span))
        }
        _ => Err("define expects a name or a (name params…) signature".to_string()),
    }
}

fn parse_signature(signature: &[Datum]) -> Result<(String, Vec<String>), String> {
    let mut names = Vec::new();
    for item in signature {
        match item {
            Datum::Symbol(name, _) => names.push(name.clone()),
            _ => return Err("procedure signature holds only symbols".to_string()),
        }
    }
    let name = names
        .first()
        .cloned()
        .ok_or_else(|| "empty procedure signature".to_string())?;
    Ok((name, names[1..].to_vec()))
}

fn parse_body(items: &[Datum]) -> Result<Vec<Expr>, String> {
    if items.is_empty() {
        return Err("a body needs at least one expression".to_string());
    }
    items.iter().map(parse_expr).collect()
}

fn parse_expr(datum: &Datum) -> Result<Expr, String> {
    match datum {
        Datum::Int(value, span) => Ok(Expr::Int(*value, *span)),
        Datum::Bool(value, span) => Ok(Expr::Bool(*value, *span)),
        Datum::Symbol(name, span) => Ok(Expr::Var(name.clone(), *span)),
        Datum::Str(text, span) => Ok(Expr::Str(text.clone(), *span)),
        Datum::List(items, span) => parse_list(items, *span),
    }
}

fn parse_list(items: &[Datum], span: Span) -> Result<Expr, String> {
    let Some(head) = items.first() else {
        return Err("() is not a valid expression".to_string());
    };
    if let Datum::Symbol(keyword, _) = head {
        match keyword.as_str() {
            "if" => return parse_if(items, span),
            "lambda" => return parse_lambda(items, span),
            "begin" => return Ok(Expr::Begin(parse_body(&items[1..])?, span)),
            "let" => return parse_let(LetKind::Let, items, span),
            "let*" => return parse_let(LetKind::LetStar, items, span),
            "letrec" => return parse_let(LetKind::LetRec, items, span),
            "parameterize" => return parse_parameterize(items, span),
            "guard" => return parse_guard(items, span),
            "define" => {
                return Err("nested define is fenced in r7rs_core v0".to_string());
            }
            "quote" => {
                let [_, datum] = items else {
                    return Err("quote takes exactly one datum".to_string());
                };
                return Ok(Expr::Quote(datum.clone(), span));
            }
            _ => {}
        }
    }
    // Application.
    let callee = parse_expr(head)?;
    let args = items[1..].iter().map(parse_expr).collect::<Result<_, _>>()?;
    Ok(Expr::App(Box::new(callee), args, span))
}

fn parse_if(items: &[Datum], span: Span) -> Result<Expr, String> {
    if items.len() != 4 {
        return Err("if takes exactly (if test then else) in v0".to_string());
    }
    Ok(Expr::If(
        Box::new(parse_expr(&items[1])?),
        Box::new(parse_expr(&items[2])?),
        Box::new(parse_expr(&items[3])?),
        span,
    ))
}

fn parse_lambda(items: &[Datum], span: Span) -> Result<Expr, String> {
    let params = match items.get(1) {
        Some(Datum::List(params, _)) => {
            let mut names = Vec::new();
            for param in params {
                match param {
                    Datum::Symbol(name, _) => names.push(name.clone()),
                    _ => return Err("lambda parameters are symbols".to_string()),
                }
            }
            names
        }
        _ => return Err("lambda expects a parameter list".to_string()),
    };
    let body = parse_body(&items[2..])?;
    Ok(Expr::Lambda(params, body, span))
}

/// A gensym for the parameterize desugar: space-prefixed names are
/// unspellable as source symbols; span.start disambiguates sites.
fn gname(kind: &str, span: Span, index: usize) -> String {
    format!(" {kind}{}_{}", span.start, index)
}

/// (parameterize ((p v) …) body…) — D-081.3, desugared HERE onto
/// existing nodes only (binding lists are not expression-shaped; the
/// define-of-lambda precedent). Order, chibi-pinned: ALL param exprs,
/// then ALL value exprs, then ALL old reads (aliased params must see
/// pre-set values); raw-set all; then dynamic-wind with a NO-OP
/// before (keeps the D-081.0 wind-before-abort path unreachable from
/// parameterize by construction) and a LIFO raw restore in the
/// after-thunk (an aliased param bound twice restores
/// innermost-first — param_alias pins it). Raw sets are the 2-arg
/// protocol spelling: they never convert, so restores cannot
/// double-convert when the fenced converter lands.
fn parse_parameterize(items: &[Datum], span: Span) -> Result<Expr, String> {
    let bindings = match items.get(1) {
        Some(Datum::List(pairs, _)) => {
            let mut out = Vec::new();
            for pair in pairs {
                let Datum::List(kv, _) = pair else {
                    return Err("a parameterize binding is (param expr)".to_string());
                };
                let [param, value] = kv.as_slice() else {
                    return Err("a parameterize binding is (param expr)".to_string());
                };
                out.push((parse_expr(param)?, parse_expr(value)?));
            }
            out
        }
        _ => return Err("parameterize expects a binding list".to_string()),
    };
    if bindings.is_empty() {
        return Err("parameterize needs at least one binding".to_string());
    }
    let body = parse_body(&items[2..])?;
    let mut binds: Vec<(String, Expr)> = Vec::new();
    for (index, (param, _)) in bindings.iter().enumerate() {
        binds.push((gname("prm", span, index), param.clone()));
    }
    for (index, (_, value)) in bindings.iter().enumerate() {
        binds.push((gname("new", span, index), value.clone()));
    }
    for index in 0..bindings.len() {
        binds.push((
            gname("old", span, index),
            Expr::App(
                Box::new(Expr::Var(gname("prm", span, index), span)),
                Vec::new(),
                span,
            ),
        ));
    }
    let raw_set = |param_index: usize, arg: String| {
        Expr::App(
            Box::new(Expr::Var(gname("prm", span, param_index), span)),
            vec![Expr::Var(arg, span), Expr::Int(0, span)],
            span,
        )
    };
    let mut seq: Vec<Expr> = Vec::new();
    for index in 0..bindings.len() {
        seq.push(raw_set(index, gname("new", span, index)));
    }
    let mut restore: Vec<Expr> = Vec::new();
    for index in (0..bindings.len()).rev() {
        restore.push(raw_set(index, gname("old", span, index)));
    }
    seq.push(Expr::App(
        Box::new(Expr::Var("dynamic-wind".to_string(), span)),
        vec![
            Expr::Lambda(Vec::new(), vec![Expr::Bool(false, span)], span),
            Expr::Lambda(Vec::new(), body, span),
            Expr::Lambda(Vec::new(), restore, span),
        ],
        span,
    ));
    Ok(Expr::Let(LetKind::LetStar, binds, seq, span))
}

/// (guard (var clause… [else expr…]) body…) — D-081.5. Admitted
/// clause forms: (test expr…) and a final (else expr…). REJECTED at
/// parse (chibi honors both, so a lax parse would silently diverge):
/// (test => proc), and (test) with no expressions (R7RS yields the
/// test's value — fenced until a case prices it).
fn parse_guard(items: &[Datum], span: Span) -> Result<Expr, String> {
    let Some(Datum::List(spec, _)) = items.get(1) else {
        return Err("guard expects a (var clause…) spec".to_string());
    };
    let Some(Datum::Symbol(var, _)) = spec.first() else {
        return Err("guard's spec starts with the condition variable".to_string());
    };
    let mut clauses = Vec::new();
    let mut else_body = None;
    for clause in &spec[1..] {
        let Datum::List(kv, _) = clause else {
            return Err("a guard clause is (test expr…) or (else expr…)".to_string());
        };
        if else_body.is_some() {
            return Err("guard's else clause must be last".to_string());
        }
        match kv.as_slice() {
            [Datum::Symbol(head, _), rest @ ..] if head == "else" => {
                if rest.is_empty() {
                    return Err("guard's else clause needs a body".to_string());
                }
                else_body = Some(parse_body(rest)?);
            }
            [_, Datum::Symbol(arrow, _), ..] if arrow == "=>" => {
                return Err(
                    "guard (test => proc) clauses are fenced in v0.4 (D-081)".to_string()
                );
            }
            [test, rest @ ..] => {
                if rest.is_empty() {
                    return Err(
                        "guard (test) clauses without expressions are fenced in v0.4 \
                         (D-081) — a bare test yields the test's value in R7RS and a \
                         lax parse would silently diverge"
                            .to_string(),
                    );
                }
                clauses.push((parse_expr(test)?, parse_body(rest)?));
            }
            [] => return Err("a guard clause is (test expr…)".to_string()),
        }
    }
    if clauses.is_empty() && else_body.is_none() {
        return Err("guard needs at least one clause".to_string());
    }
    let body = parse_body(&items[2..])?;
    Ok(Expr::Guard { var: var.clone(), clauses, else_body, body, span })
}

fn parse_let(kind: LetKind, items: &[Datum], span: Span) -> Result<Expr, String> {
    let bindings = match items.get(1) {
        Some(Datum::List(pairs, _)) => {
            let mut out = Vec::new();
            for pair in pairs {
                let Datum::List(kv, _) = pair else {
                    return Err("a let binding is (name expr)".to_string());
                };
                let [Datum::Symbol(name, _), value] = kv.as_slice() else {
                    return Err("a let binding is (name expr)".to_string());
                };
                out.push((name.clone(), parse_expr(value)?));
            }
            out
        }
        _ => return Err("let expects a binding list".to_string()),
    };
    let body = parse_body(&items[2..])?;
    Ok(Expr::Let(kind, bindings, body, span))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_define_forms() {
        let program = parse("(define x 5)\n(define (f n) (+ n x))\n(display (f 37))").unwrap();
        assert_eq!(program.len(), 3);
        assert!(matches!(&program[0], Top::Define(n, Expr::Int(5, _), _) if n == "x"));
        assert!(matches!(&program[1], Top::Define(n, Expr::Lambda(p, _, _), _)
            if n == "f" && p == &["n"]));
        assert!(matches!(&program[2], Top::Expr(Expr::App(..))));
    }

    #[test]
    fn parses_let_family_and_if() {
        let p = parse("(let* ((a 1) (b 2)) (if (< a b) a b))").unwrap();
        let Top::Expr(Expr::Let(kind, binds, body, _)) = &p[0] else { panic!() };
        assert_eq!(*kind, LetKind::LetStar);
        assert_eq!(binds.len(), 2);
        assert!(matches!(&body[0], Expr::If(..)));
    }

    #[test]
    fn nested_define_is_fenced() {
        assert!(parse("(define (f) (define y 1) y)").is_err());
    }

    #[test]
    fn import_is_a_noop() {
        let program =
            parse("(import (scheme base) (scheme write))\n(display 1)").unwrap();
        assert_eq!(program.len(), 1);
        assert!(matches!(&program[0], Top::Expr(Expr::App(..))));
    }
}
