//! AST + recursive-descent parser for ml_core v0.1. Scaffolding-grade
//! (D-019/D-038): the subset is small enough that hand-rolled descent
//! costs less than any borrowed grammar. Pattern lets desugar to match
//! at parse time (`let P = e in b` ≡ `match e with P -> b`), multi-param
//! bindings desugar to nested single-param funs, so everything
//! downstream sees a tiny core.
//!
//! Program shape: interleaved `type` definitions and top-level
//! `let`/`let rec` declarations; one of them must be `let main () = e`
//! with `e : int` — the entry protocol (and the ocaml oracle appends
//! `print_int (main ())` to the same file).

use crate::lex::{LexError, Spanned, Token, lex};
use std::fmt;

pub type NodeId = usize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Param {
    Named(String),
    Unit,
}

#[derive(Clone, Debug)]
pub enum Expr {
    Unit,
    Int(i64),
    Bool(bool),
    Var(String),
    /// Constructor application; multi-payload uses a tuple argument.
    Ctor { name: String, arg: Option<Box<Expr>> },
    Tuple(Vec<Expr>),
    Neg(Box<Expr>),
    Bin { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    If { cond: Box<Expr>, then: Box<Expr>, els: Box<Expr> },
    /// Single-param lambda (parser nests multi-param forms).
    Fun { id: NodeId, param: Param, body: Box<Expr> },
    App { func: Box<Expr>, arg: Box<Expr> },
    Let { rec: bool, bindings: Vec<Binding>, body: Box<Expr> },
    Match { scrutinee: Box<Expr>, arms: Vec<(Pattern, Expr)> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    AndAlso,
    OrElse,
}

#[derive(Clone, Debug)]
pub struct Binding {
    pub id: NodeId,
    pub name: String,
    pub expr: Expr,
}

#[derive(Clone, Debug)]
pub enum Pattern {
    Wild,
    Var(String),
    Int(i64),
    Bool(bool),
    Unit,
    Tuple(Vec<Pattern>),
    Ctor { name: String, arg: Option<Box<Pattern>> },
}

#[derive(Clone, Debug)]
pub enum TypeExpr {
    Int,
    Bool,
    Unit,
    Named(String),
    Tuple(Vec<TypeExpr>),
}

#[derive(Clone, Debug)]
pub struct TypeDef {
    pub name: String,
    pub ctors: Vec<(String, Vec<TypeExpr>)>,
}

#[derive(Clone, Debug)]
pub struct Program {
    pub typedefs: Vec<TypeDef>,
    pub decls: Vec<(bool, Vec<Binding>)>,
}

#[derive(Debug)]
pub struct ParseError {
    pub offset: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parse error at byte {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for ParseError {}

impl From<LexError> for ParseError {
    fn from(error: LexError) -> Self {
        Self { offset: error.offset, message: error.message }
    }
}

pub fn parse(source: &str) -> Result<Program, ParseError> {
    let tokens = lex(source)?;
    let mut parser = Parser { tokens, pos: 0, next_id: 0 };
    parser.program()
}

struct Parser {
    tokens: Vec<Spanned>,
    pos: usize,
    next_id: NodeId,
}

impl Parser {
    fn fresh_id(&mut self) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|s| &s.token)
    }

    fn peek2(&self) -> Option<&Token> {
        self.tokens.get(self.pos + 1).map(|s| &s.token)
    }

    fn offset(&self) -> usize {
        self.tokens
            .get(self.pos)
            .map(|s| s.offset)
            .unwrap_or(usize::MAX)
    }

    fn bump(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos).map(|s| s.token.clone());
        self.pos += 1;
        token
    }

    fn eat(&mut self, expected: &Token) -> Result<(), ParseError> {
        if self.peek() == Some(expected) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.err(format!(
                "expected {expected}, found {}",
                self.peek().map(|t| t.to_string()).unwrap_or("end of input".into())
            )))
        }
    }

    fn err(&self, message: String) -> ParseError {
        ParseError { offset: self.offset(), message }
    }

    fn program(&mut self) -> Result<Program, ParseError> {
        let mut typedefs = Vec::new();
        let mut decls = Vec::new();
        while let Some(token) = self.peek() {
            match token {
                Token::Type => typedefs.push(self.typedef()?),
                Token::Let => decls.push(self.top_let()?),
                other => {
                    return Err(self.err(format!("expected `type` or `let`, found {other}")));
                }
            }
        }
        Ok(Program { typedefs, decls })
    }

    fn typedef(&mut self) -> Result<TypeDef, ParseError> {
        self.eat(&Token::Type)?;
        let name = self.lident()?;
        self.eat(&Token::Equal)?;
        // Leading bar optional.
        if self.peek() == Some(&Token::Bar) {
            self.pos += 1;
        }
        let mut ctors = vec![self.ctor_decl()?];
        while self.peek() == Some(&Token::Bar) {
            self.pos += 1;
            ctors.push(self.ctor_decl()?);
        }
        Ok(TypeDef { name, ctors })
    }

    fn ctor_decl(&mut self) -> Result<(String, Vec<TypeExpr>), ParseError> {
        let name = self.uident()?;
        let mut payload = Vec::new();
        if self.peek() == Some(&Token::Of) {
            self.pos += 1;
            payload.push(self.type_atom()?);
            while self.peek() == Some(&Token::Star) {
                self.pos += 1;
                payload.push(self.type_atom()?);
            }
        }
        Ok((name, payload))
    }

    fn type_atom(&mut self) -> Result<TypeExpr, ParseError> {
        match self.bump() {
            Some(Token::LIdent(name)) => Ok(match name.as_str() {
                "int" => TypeExpr::Int,
                "bool" => TypeExpr::Bool,
                "unit" => TypeExpr::Unit,
                _ => TypeExpr::Named(name),
            }),
            Some(Token::LParen) => {
                let mut items = vec![self.type_atom()?];
                while self.peek() == Some(&Token::Star) {
                    self.pos += 1;
                    items.push(self.type_atom()?);
                }
                self.eat(&Token::RParen)?;
                Ok(if items.len() == 1 {
                    items.pop().unwrap()
                } else {
                    TypeExpr::Tuple(items)
                })
            }
            other => Err(self.err(format!(
                "expected a type, found {}",
                other.map(|t| t.to_string()).unwrap_or("end of input".into())
            ))),
        }
    }

    fn top_let(&mut self) -> Result<(bool, Vec<Binding>), ParseError> {
        self.eat(&Token::Let)?;
        let recursive = if self.peek() == Some(&Token::Rec) {
            self.pos += 1;
            true
        } else {
            false
        };
        let mut bindings = vec![self.binding()?];
        while self.peek() == Some(&Token::And) {
            self.pos += 1;
            bindings.push(self.binding()?);
        }
        Ok((recursive, bindings))
    }

    /// `name param* = expr` — params desugar to nested single-param funs.
    fn binding(&mut self) -> Result<Binding, ParseError> {
        let name = self.lident()?;
        let mut params = Vec::new();
        loop {
            match self.peek() {
                Some(Token::LIdent(p)) => {
                    params.push(Param::Named(p.clone()));
                    self.pos += 1;
                }
                Some(Token::LParen) if self.peek2() == Some(&Token::RParen) => {
                    params.push(Param::Unit);
                    self.pos += 2;
                }
                _ => break,
            }
        }
        self.eat(&Token::Equal)?;
        let mut expr = self.expr()?;
        for param in params.into_iter().rev() {
            let id = self.fresh_id();
            expr = Expr::Fun { id, param, body: Box::new(expr) };
        }
        Ok(Binding { id: self.fresh_id(), name, expr })
    }

    fn expr(&mut self) -> Result<Expr, ParseError> {
        match self.peek() {
            Some(Token::Let) => {
                self.pos += 1;
                let recursive = if self.peek() == Some(&Token::Rec) {
                    self.pos += 1;
                    true
                } else {
                    false
                };
                // Pattern let: `let ( ... ) = e in b` or `let _ = e in b`
                // desugars to match (never recursive, never `and`).
                let is_pattern = matches!(
                    self.peek(),
                    Some(Token::LParen) | Some(Token::Underscore)
                ) && !(self.peek() == Some(&Token::LParen)
                    && self.peek2() == Some(&Token::RParen));
                if is_pattern {
                    if recursive {
                        return Err(self.err("`let rec` with a pattern is unsupported".into()));
                    }
                    let pattern = self.pattern()?;
                    self.eat(&Token::Equal)?;
                    let value = self.expr()?;
                    self.eat(&Token::In)?;
                    let body = self.expr()?;
                    return Ok(Expr::Match {
                        scrutinee: Box::new(value),
                        arms: vec![(pattern, body)],
                    });
                }
                let mut bindings = vec![self.binding()?];
                while self.peek() == Some(&Token::And) {
                    self.pos += 1;
                    bindings.push(self.binding()?);
                }
                self.eat(&Token::In)?;
                let body = self.expr()?;
                Ok(Expr::Let { rec: recursive, bindings, body: Box::new(body) })
            }
            Some(Token::Fun) => {
                self.pos += 1;
                let mut params = Vec::new();
                loop {
                    match self.peek() {
                        Some(Token::LIdent(p)) => {
                            params.push(Param::Named(p.clone()));
                            self.pos += 1;
                        }
                        Some(Token::LParen) if self.peek2() == Some(&Token::RParen) => {
                            params.push(Param::Unit);
                            self.pos += 2;
                        }
                        Some(Token::Arrow) => break,
                        other => {
                            return Err(self.err(format!(
                                "expected a parameter or ->, found {}",
                                other.map(|t| t.to_string()).unwrap_or("end".into())
                            )));
                        }
                    }
                }
                if params.is_empty() {
                    return Err(self.err("fun needs at least one parameter".into()));
                }
                self.eat(&Token::Arrow)?;
                let mut body = self.expr()?;
                for param in params.into_iter().rev() {
                    let id = self.fresh_id();
                    body = Expr::Fun { id, param, body: Box::new(body) };
                }
                Ok(body)
            }
            Some(Token::If) => {
                self.pos += 1;
                let cond = self.expr()?;
                self.eat(&Token::Then)?;
                let then = self.expr()?;
                self.eat(&Token::Else)?;
                let els = self.expr()?;
                Ok(Expr::If {
                    cond: Box::new(cond),
                    then: Box::new(then),
                    els: Box::new(els),
                })
            }
            Some(Token::Match) => {
                self.pos += 1;
                let scrutinee = self.expr()?;
                self.eat(&Token::With)?;
                if self.peek() == Some(&Token::Bar) {
                    self.pos += 1;
                }
                let mut arms = vec![self.arm()?];
                while self.peek() == Some(&Token::Bar) {
                    self.pos += 1;
                    arms.push(self.arm()?);
                }
                Ok(Expr::Match { scrutinee: Box::new(scrutinee), arms })
            }
            _ => self.or_else(),
        }
    }

    fn arm(&mut self) -> Result<(Pattern, Expr), ParseError> {
        let pattern = self.pattern()?;
        self.eat(&Token::Arrow)?;
        let body = self.expr()?;
        Ok((pattern, body))
    }

    fn or_else(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.and_also()?;
        while self.peek() == Some(&Token::OrOr) {
            self.pos += 1;
            let rhs = self.and_also()?;
            lhs = Expr::Bin { op: BinOp::OrElse, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn and_also(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.compare()?;
        while self.peek() == Some(&Token::AndAnd) {
            self.pos += 1;
            let rhs = self.compare()?;
            lhs = Expr::Bin { op: BinOp::AndAlso, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn compare(&mut self) -> Result<Expr, ParseError> {
        let lhs = self.additive()?;
        let op = match self.peek() {
            Some(Token::Equal) => BinOp::Eq,
            Some(Token::NotEqual) => BinOp::Ne,
            Some(Token::Less) => BinOp::Lt,
            Some(Token::LessEq) => BinOp::Le,
            Some(Token::Greater) => BinOp::Gt,
            Some(Token::GreaterEq) => BinOp::Ge,
            _ => return Ok(lhs),
        };
        self.pos += 1;
        let rhs = self.additive()?;
        Ok(Expr::Bin { op, lhs: Box::new(lhs), rhs: Box::new(rhs) })
    }

    fn additive(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.multiplicative()?;
        loop {
            let op = match self.peek() {
                Some(Token::Plus) => BinOp::Add,
                Some(Token::Minus) => BinOp::Sub,
                _ => break,
            };
            self.pos += 1;
            let rhs = self.multiplicative()?;
            lhs = Expr::Bin { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.unary()?;
        loop {
            let op = match self.peek() {
                Some(Token::Star) => BinOp::Mul,
                Some(Token::Slash) => BinOp::Div,
                _ => break,
            };
            self.pos += 1;
            let rhs = self.unary()?;
            lhs = Expr::Bin { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        if self.peek() == Some(&Token::Minus) {
            self.pos += 1;
            let inner = self.unary()?;
            return Ok(match inner {
                Expr::Int(value) => Expr::Int(-value),
                other => Expr::Neg(Box::new(other)),
            });
        }
        self.application()
    }

    fn application(&mut self) -> Result<Expr, ParseError> {
        // Constructor application binds one atom; function application
        // chains left.
        if let Some(Token::UIdent(name)) = self.peek() {
            let name = name.clone();
            self.pos += 1;
            let arg = if self.starts_atom() {
                Some(Box::new(self.atom()?))
            } else {
                None
            };
            return Ok(Expr::Ctor { name, arg });
        }
        let mut expr = self.atom()?;
        while self.starts_atom() {
            let arg = self.atom()?;
            expr = Expr::App { func: Box::new(expr), arg: Box::new(arg) };
        }
        Ok(expr)
    }

    fn starts_atom(&self) -> bool {
        matches!(
            self.peek(),
            Some(Token::Int(_))
                | Some(Token::LIdent(_))
                | Some(Token::UIdent(_))
                | Some(Token::True)
                | Some(Token::False)
                | Some(Token::LParen)
        )
    }

    fn atom(&mut self) -> Result<Expr, ParseError> {
        match self.bump() {
            Some(Token::Int(value)) => Ok(Expr::Int(value)),
            Some(Token::True) => Ok(Expr::Bool(true)),
            Some(Token::False) => Ok(Expr::Bool(false)),
            Some(Token::LIdent(name)) => Ok(Expr::Var(name)),
            Some(Token::UIdent(name)) => Ok(Expr::Ctor { name, arg: None }),
            Some(Token::LParen) => {
                if self.peek() == Some(&Token::RParen) {
                    self.pos += 1;
                    return Ok(Expr::Unit);
                }
                let mut items = vec![self.expr()?];
                while self.peek() == Some(&Token::Comma) {
                    self.pos += 1;
                    items.push(self.expr()?);
                }
                self.eat(&Token::RParen)?;
                Ok(if items.len() == 1 {
                    items.pop().unwrap()
                } else {
                    Expr::Tuple(items)
                })
            }
            other => Err(self.err(format!(
                "expected an expression, found {}",
                other.map(|t| t.to_string()).unwrap_or("end of input".into())
            ))),
        }
    }

    fn pattern(&mut self) -> Result<Pattern, ParseError> {
        match self.bump() {
            Some(Token::Underscore) => Ok(Pattern::Wild),
            Some(Token::Int(value)) => Ok(Pattern::Int(value)),
            Some(Token::Minus) => match self.bump() {
                Some(Token::Int(value)) => Ok(Pattern::Int(-value)),
                _ => Err(self.err("expected an integer after - in a pattern".into())),
            },
            Some(Token::True) => Ok(Pattern::Bool(true)),
            Some(Token::False) => Ok(Pattern::Bool(false)),
            Some(Token::LIdent(name)) => Ok(Pattern::Var(name)),
            Some(Token::UIdent(name)) => {
                let arg = if matches!(
                    self.peek(),
                    Some(Token::Int(_))
                        | Some(Token::LIdent(_))
                        | Some(Token::UIdent(_))
                        | Some(Token::True)
                        | Some(Token::False)
                        | Some(Token::LParen)
                        | Some(Token::Underscore)
                ) {
                    Some(Box::new(self.pattern()?))
                } else {
                    None
                };
                Ok(Pattern::Ctor { name, arg })
            }
            Some(Token::LParen) => {
                if self.peek() == Some(&Token::RParen) {
                    self.pos += 1;
                    return Ok(Pattern::Unit);
                }
                let mut items = vec![self.pattern()?];
                while self.peek() == Some(&Token::Comma) {
                    self.pos += 1;
                    items.push(self.pattern()?);
                }
                self.eat(&Token::RParen)?;
                Ok(if items.len() == 1 {
                    items.pop().unwrap()
                } else {
                    Pattern::Tuple(items)
                })
            }
            other => Err(self.err(format!(
                "expected a pattern, found {}",
                other.map(|t| t.to_string()).unwrap_or("end of input".into())
            ))),
        }
    }

    fn lident(&mut self) -> Result<String, ParseError> {
        match self.bump() {
            Some(Token::LIdent(name)) => Ok(name),
            other => Err(self.err(format!(
                "expected an identifier, found {}",
                other.map(|t| t.to_string()).unwrap_or("end of input".into())
            ))),
        }
    }

    fn uident(&mut self) -> Result<String, ParseError> {
        match self.bump() {
            Some(Token::UIdent(name)) => Ok(name),
            other => Err(self.err(format!(
                "expected a constructor name, found {}",
                other.map(|t| t.to_string()).unwrap_or("end of input".into())
            ))),
        }
    }
}
