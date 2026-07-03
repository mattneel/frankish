//! Lexer for the ml_core v0.1 subset (specimens/ml_core/MANIFEST.md).
//! Hand-rolled scaffolding-grade per D-019/D-038: zero research budget,
//! replaceable wholesale.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    Int(i64),
    LIdent(String),
    UIdent(String),
    // Keywords.
    Let,
    Rec,
    And,
    In,
    Fun,
    If,
    Then,
    Else,
    Match,
    With,
    Type,
    Of,
    True,
    False,
    // Symbols.
    LParen,
    RParen,
    Comma,
    Arrow,
    Bar,
    Underscore,
    Plus,
    Minus,
    Star,
    Slash,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEq,
    GreaterEq,
    AndAnd,
    OrOr,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(value) => write!(f, "{value}"),
            Self::LIdent(name) | Self::UIdent(name) => write!(f, "{name}"),
            other => write!(f, "{other:?}"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Spanned {
    pub token: Token,
    pub offset: usize,
}

#[derive(Debug)]
pub struct LexError {
    pub offset: usize,
    pub message: String,
}

pub fn lex(source: &str) -> Result<Vec<Spanned>, LexError> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0usize;

    while i < bytes.len() {
        let c = bytes[i] as char;
        match c {
            ' ' | '\t' | '\r' | '\n' => i += 1,
            '(' if bytes.get(i + 1) == Some(&b'*') => {
                // Nested (* ... *) comments.
                let start = i;
                let mut depth = 1;
                i += 2;
                while depth > 0 {
                    if i + 1 >= bytes.len() {
                        return Err(LexError {
                            offset: start,
                            message: "unterminated comment".into(),
                        });
                    }
                    if bytes[i] == b'(' && bytes[i + 1] == b'*' {
                        depth += 1;
                        i += 2;
                    } else if bytes[i] == b'*' && bytes[i + 1] == b')' {
                        depth -= 1;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
            }
            '(' => {
                tokens.push(Spanned { token: Token::LParen, offset: i });
                i += 1;
            }
            ')' => {
                tokens.push(Spanned { token: Token::RParen, offset: i });
                i += 1;
            }
            ',' => {
                tokens.push(Spanned { token: Token::Comma, offset: i });
                i += 1;
            }
            '|' if bytes.get(i + 1) == Some(&b'|') => {
                tokens.push(Spanned { token: Token::OrOr, offset: i });
                i += 2;
            }
            '|' => {
                tokens.push(Spanned { token: Token::Bar, offset: i });
                i += 1;
            }
            '&' if bytes.get(i + 1) == Some(&b'&') => {
                tokens.push(Spanned { token: Token::AndAnd, offset: i });
                i += 2;
            }
            '+' => {
                tokens.push(Spanned { token: Token::Plus, offset: i });
                i += 1;
            }
            '-' if bytes.get(i + 1) == Some(&b'>') => {
                tokens.push(Spanned { token: Token::Arrow, offset: i });
                i += 2;
            }
            '-' => {
                tokens.push(Spanned { token: Token::Minus, offset: i });
                i += 1;
            }
            '*' => {
                tokens.push(Spanned { token: Token::Star, offset: i });
                i += 1;
            }
            '/' => {
                tokens.push(Spanned { token: Token::Slash, offset: i });
                i += 1;
            }
            '=' => {
                tokens.push(Spanned { token: Token::Equal, offset: i });
                i += 1;
            }
            '<' if bytes.get(i + 1) == Some(&b'>') => {
                tokens.push(Spanned { token: Token::NotEqual, offset: i });
                i += 2;
            }
            '<' if bytes.get(i + 1) == Some(&b'=') => {
                tokens.push(Spanned { token: Token::LessEq, offset: i });
                i += 2;
            }
            '<' => {
                tokens.push(Spanned { token: Token::Less, offset: i });
                i += 1;
            }
            '>' if bytes.get(i + 1) == Some(&b'=') => {
                tokens.push(Spanned { token: Token::GreaterEq, offset: i });
                i += 2;
            }
            '>' => {
                tokens.push(Spanned { token: Token::Greater, offset: i });
                i += 1;
            }
            ';' if bytes.get(i + 1) == Some(&b';') => i += 2, // ;; is noise
            '0'..='9' => {
                let start = i;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                let text = &source[start..i];
                let value: i64 = text.parse().map_err(|_| LexError {
                    offset: start,
                    message: format!("integer literal out of range: {text}"),
                })?;
                tokens.push(Spanned { token: Token::Int(value), offset: start });
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let start = i;
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'\'')
                {
                    i += 1;
                }
                let word = &source[start..i];
                let token = match word {
                    "let" => Token::Let,
                    "rec" => Token::Rec,
                    "and" => Token::And,
                    "in" => Token::In,
                    "fun" => Token::Fun,
                    "if" => Token::If,
                    "then" => Token::Then,
                    "else" => Token::Else,
                    "match" => Token::Match,
                    "with" => Token::With,
                    "type" => Token::Type,
                    "of" => Token::Of,
                    "true" => Token::True,
                    "false" => Token::False,
                    "_" => Token::Underscore,
                    _ if word.starts_with(|c: char| c.is_ascii_uppercase()) => {
                        Token::UIdent(word.to_string())
                    }
                    _ => Token::LIdent(word.to_string()),
                };
                tokens.push(Spanned { token, offset: start });
            }
            other => {
                return Err(LexError {
                    offset: i,
                    message: format!("unexpected character {other:?}"),
                });
            }
        }
    }
    Ok(tokens)
}
