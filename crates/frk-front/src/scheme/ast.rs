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
            | Expr::App(_, _, s) => *s,
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
            "define" => {
                return Err("nested define is fenced in r7rs_core v0".to_string());
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
