//! Lua 5.1 recursive-descent parser for the femto_lua v0.1 subset
//! (D-052 scope; D-054 fences). Spans are byte offsets of each
//! construct's first token — threaded into locations at emission.

use super::lex::{LexError, Spanned, Token, lex};

#[derive(Clone, Debug)]
pub enum Expr {
    Nil(usize),
    True(usize),
    False(usize),
    Num(f64, usize),
    Str(String, usize),
    Name(String, usize),
    Index(Box<Expr>, Box<Expr>, usize),
    Call(Box<Expr>, Vec<Expr>, usize),
    Function(Vec<String>, Block, usize),
    Table(Vec<Field>, usize),
    Binary(BinOp, Box<Expr>, Box<Expr>, usize),
    Unary(UnOp, Box<Expr>, usize),
}

impl Expr {
    pub fn span(&self) -> usize {
        match self {
            Expr::Nil(s) | Expr::True(s) | Expr::False(s) | Expr::Num(_, s)
            | Expr::Str(_, s) | Expr::Name(_, s) | Expr::Index(_, _, s)
            | Expr::Call(_, _, s) | Expr::Function(_, _, s) | Expr::Table(_, s)
            | Expr::Binary(_, _, _, s) | Expr::Unary(_, _, s) => *s,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Field {
    Positional(Expr),
    Named(String, Expr),
    Keyed(Expr, Expr),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod, Concat,
    Eq, Ne, Lt, Le, Gt, Ge, And, Or,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    Neg, Not, Len,
}

#[derive(Clone, Debug)]
pub enum Stat {
    Local(String, Expr, usize),
    /// `local a, b, c = f()` — names beyond the pack nil-fill (D-058).
    LocalMulti(Vec<String>, Expr, usize),
    /// `a, b = f()` — existing locals/globals only, names only.
    AssignMulti(Vec<String>, Expr, usize),
    Repeat(Block, Expr, usize),
    Break(usize),
    /// `for n1, n2 in e do` — e is the iterator-producing expression.
    GenFor(Vec<String>, Expr, Block, usize),
    LocalFunction(String, Vec<String>, Block, usize),
    AssignName(String, Expr, usize),
    AssignIndex(Expr, Expr, Expr, usize),
    Call(Expr, usize),
    If(Vec<(Expr, Block)>, Option<Block>, usize),
    While(Expr, Block, usize),
    NumFor(String, Expr, Expr, Option<Expr>, Block, usize),
    Return(Vec<Expr>, usize),
    Do(Block, usize),
    /// `function name(...)` — a global function declaration.
    GlobalFunction(String, Vec<String>, Block, usize),
}

pub type Block = Vec<Stat>;

#[derive(Debug)]
pub struct ParseError {
    pub offset: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "parse error at byte {}: {}", self.offset, self.message)
    }
}

pub fn parse(source: &str) -> Result<Block, ParseError> {
    let tokens = lex(source).map_err(|LexError { offset, message }| ParseError {
        offset,
        message,
    })?;
    let mut parser = Parser { tokens, position: 0 };
    let block = parser.block()?;
    if parser.position < parser.tokens.len() {
        return Err(parser.error("trailing input after chunk"));
    }
    Ok(block)
}

struct Parser {
    tokens: Vec<Spanned>,
    position: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position).map(|spanned| &spanned.token)
    }

    fn span(&self) -> usize {
        self.tokens
            .get(self.position)
            .map(|spanned| spanned.start)
            .unwrap_or_else(|| {
                self.tokens.last().map(|spanned| spanned.start).unwrap_or(0)
            })
    }

    fn bump(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.position).map(|s| s.token.clone());
        self.position += 1;
        token
    }

    fn error(&self, message: &str) -> ParseError {
        ParseError { offset: self.span(), message: message.to_string() }
    }

    fn expect(&mut self, token: Token, what: &str) -> Result<(), ParseError> {
        if self.peek() == Some(&token) {
            self.position += 1;
            Ok(())
        } else {
            Err(self.error(&format!("expected {what}")))
        }
    }

    fn name(&mut self) -> Result<String, ParseError> {
        match self.bump() {
            Some(Token::Name(name)) => Ok(name),
            _ => {
                self.position -= 1;
                Err(self.error("expected a name"))
            }
        }
    }

    /// Statements until a block-closing keyword (not consumed).
    fn block(&mut self) -> Result<Block, ParseError> {
        let mut statements = Vec::new();
        loop {
            match self.peek() {
                None
                | Some(Token::End)
                | Some(Token::Else)
                | Some(Token::Elseif)
                | Some(Token::Until) => break,
                Some(Token::Semi) => {
                    self.position += 1;
                }
                Some(Token::Return) => {
                    let span = self.span();
                    self.position += 1;
                    let mut values = Vec::new();
                    match self.peek() {
                        None
                        | Some(Token::End)
                        | Some(Token::Else)
                        | Some(Token::Elseif)
                        | Some(Token::Until)
                        | Some(Token::Semi) => {}
                        _ => loop {
                            values.push(self.expression()?);
                            if self.peek() == Some(&Token::Comma) {
                                self.position += 1;
                            } else {
                                break;
                            }
                        },
                    }
                    if self.peek() == Some(&Token::Semi) {
                        self.position += 1;
                    }
                    statements.push(Stat::Return(values, span));
                    break; // return ends the block
                }
                _ => statements.push(self.statement()?),
            }
        }
        Ok(statements)
    }

    fn statement(&mut self) -> Result<Stat, ParseError> {
        let span = self.span();
        match self.peek() {
            Some(Token::Local) => {
                self.position += 1;
                if self.peek() == Some(&Token::Function) {
                    self.position += 1;
                    let name = self.name()?;
                    let (params, body) = self.function_body()?;
                    return Ok(Stat::LocalFunction(name, params, body, span));
                }
                let name = self.name()?;
                if self.peek() == Some(&Token::Comma) {
                    // Multi-name local (D-058): RHS is one expression;
                    // a call's pack destructures with nil-fill.
                    let mut names = vec![name];
                    while self.peek() == Some(&Token::Comma) {
                        self.position += 1;
                        names.push(self.name()?);
                    }
                    self.expect(Token::Assign, "=")?;
                    let value = self.expression()?;
                    if self.peek() == Some(&Token::Comma) {
                        return Err(self.error(
                            "multi-expression RHS is fenced in v0.2 (D-058)",
                        ));
                    }
                    return Ok(Stat::LocalMulti(names, value, span));
                }
                self.expect(Token::Assign, "= (locals require an initializer)")?;
                let value = self.expression()?;
                Ok(Stat::Local(name, value, span))
            }
            Some(Token::Function) => {
                self.position += 1;
                let name = self.name()?;
                if matches!(self.peek(), Some(Token::Dot) | Some(Token::Colon)) {
                    return Err(self.error("method declarations are fenced in v0.1 (D-052)"));
                }
                let (params, body) = self.function_body()?;
                Ok(Stat::GlobalFunction(name, params, body, span))
            }
            Some(Token::If) => {
                self.position += 1;
                let mut arms = Vec::new();
                let condition = self.expression()?;
                self.expect(Token::Then, "then")?;
                arms.push((condition, self.block()?));
                let mut otherwise = None;
                loop {
                    match self.peek() {
                        Some(Token::Elseif) => {
                            self.position += 1;
                            let condition = self.expression()?;
                            self.expect(Token::Then, "then")?;
                            arms.push((condition, self.block()?));
                        }
                        Some(Token::Else) => {
                            self.position += 1;
                            otherwise = Some(self.block()?);
                            self.expect(Token::End, "end")?;
                            break;
                        }
                        Some(Token::End) => {
                            self.position += 1;
                            break;
                        }
                        _ => return Err(self.error("expected elseif/else/end")),
                    }
                }
                Ok(Stat::If(arms, otherwise, span))
            }
            Some(Token::While) => {
                self.position += 1;
                let condition = self.expression()?;
                self.expect(Token::Do, "do")?;
                let body = self.block()?;
                self.expect(Token::End, "end")?;
                Ok(Stat::While(condition, body, span))
            }
            Some(Token::For) => {
                self.position += 1;
                let variable = self.name()?;
                match self.peek() {
                    Some(Token::Assign) => {
                        self.position += 1;
                        let from = self.expression()?;
                        self.expect(Token::Comma, ",")?;
                        let to = self.expression()?;
                        let step = if self.peek() == Some(&Token::Comma) {
                            self.position += 1;
                            Some(self.expression()?)
                        } else {
                            None
                        };
                        self.expect(Token::Do, "do")?;
                        let body = self.block()?;
                        self.expect(Token::End, "end")?;
                        Ok(Stat::NumFor(variable, from, to, step, body, span))
                    }
                    Some(Token::Comma) | Some(Token::In) => {
                        // Generic for (D-058): for n1[, n2] in expr do
                        let mut names = vec![variable];
                        while self.peek() == Some(&Token::Comma) {
                            self.position += 1;
                            names.push(self.name()?);
                        }
                        self.expect(Token::In, "in")?;
                        let iterator = self.expression()?;
                        if self.peek() == Some(&Token::Comma) {
                            return Err(self.error(
                                "explicit iterator triples are fenced in v0.2 (D-058)",
                            ));
                        }
                        self.expect(Token::Do, "do")?;
                        let body = self.block()?;
                        self.expect(Token::End, "end")?;
                        Ok(Stat::GenFor(names, iterator, body, span))
                    }
                    _ => Err(self.error("malformed for")),
                }
            }
            Some(Token::Do) => {
                self.position += 1;
                let body = self.block()?;
                self.expect(Token::End, "end")?;
                Ok(Stat::Do(body, span))
            }
            Some(Token::Repeat) => {
                self.position += 1;
                let body = self.block()?;
                self.expect(Token::Until, "until")?;
                let condition = self.expression()?;
                Ok(Stat::Repeat(body, condition, span))
            }
            Some(Token::Break) => {
                self.position += 1;
                Ok(Stat::Break(span))
            }
            _ => {
                // Expression statement: a call, or the start of an
                // assignment through a name/index prefix.
                let prefix = self.prefix_expression()?;
                if self.peek() == Some(&Token::Comma) {
                    // a, b = expr (names only, D-058).
                    let Expr::Name(first, _) = prefix else {
                        return Err(self.error("multi-assignment targets are names only"));
                    };
                    let mut names = vec![first];
                    while self.peek() == Some(&Token::Comma) {
                        self.position += 1;
                        names.push(self.name()?);
                    }
                    self.expect(Token::Assign, "=")?;
                    let value = self.expression()?;
                    return Ok(Stat::AssignMulti(names, value, span));
                }
                if self.peek() == Some(&Token::Assign) {
                    self.position += 1;
                    let value = self.expression()?;
                    return match prefix {
                        Expr::Name(name, _) => Ok(Stat::AssignName(name, value, span)),
                        Expr::Index(table, key, _) => {
                            Ok(Stat::AssignIndex(*table, *key, value, span))
                        }
                        _ => Err(self.error("cannot assign to this expression")),
                    };
                }
                match prefix {
                    call @ Expr::Call(..) => Ok(Stat::Call(call, span)),
                    _ => Err(self.error("expression statements must be calls")),
                }
            }
        }
    }

    fn function_body(&mut self) -> Result<(Vec<String>, Block), ParseError> {
        self.expect(Token::LParen, "(")?;
        let mut params = Vec::new();
        if self.peek() != Some(&Token::RParen) {
            loop {
                params.push(self.name()?);
                if self.peek() == Some(&Token::Comma) {
                    self.position += 1;
                } else {
                    break;
                }
            }
        }
        self.expect(Token::RParen, ")")?;
        let body = self.block()?;
        self.expect(Token::End, "end")?;
        Ok((params, body))
    }

    // Precedence climbing, Lua 5.1 table: or < and < comparisons <
    // .. (right) < add < mul < unary. (^ is fenced.)
    fn expression(&mut self) -> Result<Expr, ParseError> {
        self.binary_expression(0)
    }

    fn binary_expression(&mut self, min_level: u8) -> Result<Expr, ParseError> {
        let mut left = self.unary_expression()?;
        loop {
            let (op, level, right_assoc) = match self.peek() {
                Some(Token::Or) => (BinOp::Or, 1, false),
                Some(Token::And) => (BinOp::And, 2, false),
                Some(Token::Less) => (BinOp::Lt, 3, false),
                Some(Token::Greater) => (BinOp::Gt, 3, false),
                Some(Token::LessEq) => (BinOp::Le, 3, false),
                Some(Token::GreaterEq) => (BinOp::Ge, 3, false),
                Some(Token::EqEq) => (BinOp::Eq, 3, false),
                Some(Token::NotEq) => (BinOp::Ne, 3, false),
                Some(Token::Concat) => (BinOp::Concat, 4, true),
                Some(Token::Plus) => (BinOp::Add, 5, false),
                Some(Token::Minus) => (BinOp::Sub, 5, false),
                Some(Token::Star) => (BinOp::Mul, 6, false),
                Some(Token::Slash) => (BinOp::Div, 6, false),
                Some(Token::Percent) => (BinOp::Mod, 6, false),
                Some(Token::Caret) => {
                    return Err(self.error("^ is fenced in v0.1 (D-052)"));
                }
                _ => break,
            };
            if level < min_level {
                break;
            }
            let span = self.span();
            self.position += 1;
            let next_min = if right_assoc { level } else { level + 1 };
            let right = self.binary_expression(next_min)?;
            left = Expr::Binary(op, Box::new(left), Box::new(right), span);
        }
        Ok(left)
    }

    fn unary_expression(&mut self) -> Result<Expr, ParseError> {
        let span = self.span();
        match self.peek() {
            Some(Token::Not) => {
                self.position += 1;
                Ok(Expr::Unary(UnOp::Not, Box::new(self.unary_expression()?), span))
            }
            Some(Token::Minus) => {
                self.position += 1;
                Ok(Expr::Unary(UnOp::Neg, Box::new(self.unary_expression()?), span))
            }
            Some(Token::Hash) => {
                self.position += 1;
                Ok(Expr::Unary(UnOp::Len, Box::new(self.unary_expression()?), span))
            }
            _ => self.simple_expression(),
        }
    }

    fn simple_expression(&mut self) -> Result<Expr, ParseError> {
        let span = self.span();
        match self.peek() {
            Some(Token::Nil) => { self.position += 1; Ok(Expr::Nil(span)) }
            Some(Token::True) => { self.position += 1; Ok(Expr::True(span)) }
            Some(Token::False) => { self.position += 1; Ok(Expr::False(span)) }
            Some(Token::Number(value)) => {
                let value = *value;
                self.position += 1;
                Ok(Expr::Num(value, span))
            }
            Some(Token::Str(text)) => {
                let text = text.clone();
                self.position += 1;
                Ok(Expr::Str(text, span))
            }
            Some(Token::Function) => {
                self.position += 1;
                let (params, body) = self.function_body()?;
                Ok(Expr::Function(params, body, span))
            }
            Some(Token::LBrace) => self.table_constructor(),
            _ => self.prefix_expression(),
        }
    }

    /// name | (expr) followed by any chain of .name / [expr] / (args).
    fn prefix_expression(&mut self) -> Result<Expr, ParseError> {
        let span = self.span();
        let mut expr = match self.bump() {
            Some(Token::Name(name)) => Expr::Name(name, span),
            Some(Token::LParen) => {
                let inner = self.expression()?;
                self.expect(Token::RParen, ")")?;
                inner
            }
            _ => {
                self.position -= 1;
                return Err(self.error("expected an expression"));
            }
        };
        loop {
            let span = self.span();
            match self.peek() {
                Some(Token::Dot) => {
                    self.position += 1;
                    let field = self.name()?;
                    expr = Expr::Index(
                        Box::new(expr),
                        Box::new(Expr::Str(field, span)),
                        span,
                    );
                }
                Some(Token::LBracket) => {
                    self.position += 1;
                    let key = self.expression()?;
                    self.expect(Token::RBracket, "]")?;
                    expr = Expr::Index(Box::new(expr), Box::new(key), span);
                }
                Some(Token::LParen) => {
                    self.position += 1;
                    let mut arguments = Vec::new();
                    if self.peek() != Some(&Token::RParen) {
                        loop {
                            arguments.push(self.expression()?);
                            if self.peek() == Some(&Token::Comma) {
                                self.position += 1;
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(Token::RParen, ")")?;
                    expr = Expr::Call(Box::new(expr), arguments, span);
                }
                Some(Token::Str(_)) | Some(Token::LBrace) => {
                    return Err(self.error(
                        "paren-free call sugar is fenced in v0.1 (D-052)",
                    ));
                }
                Some(Token::Colon) => {
                    return Err(self.error("method calls are fenced in v0.1 (D-052)"));
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn table_constructor(&mut self) -> Result<Expr, ParseError> {
        let span = self.span();
        self.expect(Token::LBrace, "{")?;
        let mut fields = Vec::new();
        while self.peek() != Some(&Token::RBrace) {
            match self.peek() {
                Some(Token::LBracket) => {
                    self.position += 1;
                    let key = self.expression()?;
                    self.expect(Token::RBracket, "]")?;
                    self.expect(Token::Assign, "=")?;
                    fields.push(Field::Keyed(key, self.expression()?));
                }
                Some(Token::Name(_))
                    if self.tokens.get(self.position + 1).map(|s| &s.token)
                        == Some(&Token::Assign) =>
                {
                    let name = self.name()?;
                    self.position += 1; // =
                    fields.push(Field::Named(name, self.expression()?));
                }
                _ => fields.push(Field::Positional(self.expression()?)),
            }
            match self.peek() {
                Some(Token::Comma) | Some(Token::Semi) => self.position += 1,
                _ => break,
            }
        }
        self.expect(Token::RBrace, "}")?;
        Ok(Expr::Table(fields, span))
    }
}
