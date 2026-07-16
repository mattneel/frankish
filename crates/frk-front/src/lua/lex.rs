//! Lua 5.1 lexer for the femto_lua v0.1 subset (D-052; hand-rolled per
//! D-054's D-019 scaffolding stance). Long strings/comments are
//! fenced; `--` line comments pass; `...` lexes as Dots (v0.3, D-068).

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Name(String),
    Number(f64),
    Str(String),
    // keywords
    And, Break, Do, Else, Elseif, End, False, For, Function, If, In,
    Local, Nil, Not, Or, Repeat, Return, Then, True, Until, While,
    // symbols
    Plus, Minus, Star, Slash, Percent, Caret, Hash,
    EqEq, NotEq, LessEq, GreaterEq, Less, Greater, Assign,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Semi, Colon, Comma, Dot, Concat, Dots,
}

#[derive(Debug)]
pub struct LexError {
    pub offset: usize,
    pub message: String,
}

pub struct Spanned {
    pub token: Token,
    pub start: usize,
}

pub fn lex(source: &str) -> Result<Vec<Spanned>, LexError> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0usize;
    let err = |offset, message: &str| LexError { offset, message: message.into() };

    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b' ' | b'\t' | b'\r' | b'\n' => i += 1,
            b'-' if bytes.get(i + 1) == Some(&b'-') => {
                if bytes.get(i + 2) == Some(&b'[') && bytes.get(i + 3) == Some(&b'[') {
                    return Err(err(i, "long comments are fenced in v0.1 (D-052)"));
                }
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'-' => { tokens.push(Spanned { token: Token::Minus, start: i }); i += 1; }
            b'+' => { tokens.push(Spanned { token: Token::Plus, start: i }); i += 1; }
            b'*' => { tokens.push(Spanned { token: Token::Star, start: i }); i += 1; }
            b'/' => { tokens.push(Spanned { token: Token::Slash, start: i }); i += 1; }
            b'%' => { tokens.push(Spanned { token: Token::Percent, start: i }); i += 1; }
            b'^' => { tokens.push(Spanned { token: Token::Caret, start: i }); i += 1; }
            b'#' => { tokens.push(Spanned { token: Token::Hash, start: i }); i += 1; }
            b'(' => { tokens.push(Spanned { token: Token::LParen, start: i }); i += 1; }
            b')' => { tokens.push(Spanned { token: Token::RParen, start: i }); i += 1; }
            b'{' => { tokens.push(Spanned { token: Token::LBrace, start: i }); i += 1; }
            b'}' => { tokens.push(Spanned { token: Token::RBrace, start: i }); i += 1; }
            b'[' => {
                if bytes.get(i + 1) == Some(&b'[') {
                    return Err(err(i, "long strings are fenced in v0.1 (D-052)"));
                }
                tokens.push(Spanned { token: Token::LBracket, start: i });
                i += 1;
            }
            b']' => { tokens.push(Spanned { token: Token::RBracket, start: i }); i += 1; }
            b';' => { tokens.push(Spanned { token: Token::Semi, start: i }); i += 1; }
            b':' => { tokens.push(Spanned { token: Token::Colon, start: i }); i += 1; }
            b',' => { tokens.push(Spanned { token: Token::Comma, start: i }); i += 1; }
            b'=' => {
                if bytes.get(i + 1) == Some(&b'=') {
                    tokens.push(Spanned { token: Token::EqEq, start: i });
                    i += 2;
                } else {
                    tokens.push(Spanned { token: Token::Assign, start: i });
                    i += 1;
                }
            }
            b'~' => {
                if bytes.get(i + 1) == Some(&b'=') {
                    tokens.push(Spanned { token: Token::NotEq, start: i });
                    i += 2;
                } else {
                    return Err(err(i, "stray ~"));
                }
            }
            b'<' => {
                if bytes.get(i + 1) == Some(&b'=') {
                    tokens.push(Spanned { token: Token::LessEq, start: i });
                    i += 2;
                } else {
                    tokens.push(Spanned { token: Token::Less, start: i });
                    i += 1;
                }
            }
            b'>' => {
                if bytes.get(i + 1) == Some(&b'=') {
                    tokens.push(Spanned { token: Token::GreaterEq, start: i });
                    i += 2;
                } else {
                    tokens.push(Spanned { token: Token::Greater, start: i });
                    i += 1;
                }
            }
            b'.' => {
                if bytes.get(i + 1) == Some(&b'.') {
                    if bytes.get(i + 2) == Some(&b'.') {
                        // `...` — varargs (v0.3, D-068).
                        tokens.push(Spanned { token: Token::Dots, start: i });
                        i += 3;
                    } else {
                        tokens.push(Spanned { token: Token::Concat, start: i });
                        i += 2;
                    }
                } else if bytes.get(i + 1).is_some_and(u8::is_ascii_digit) {
                    let (value, next) = lex_number(source, i)?;
                    tokens.push(Spanned { token: Token::Number(value), start: i });
                    i = next;
                } else {
                    tokens.push(Spanned { token: Token::Dot, start: i });
                    i += 1;
                }
            }
            b'\'' | b'"' => {
                let quote = c;
                let start = i;
                i += 1;
                let mut text = Vec::new();
                loop {
                    let Some(&b) = bytes.get(i) else {
                        return Err(err(start, "unterminated string"));
                    };
                    match b {
                        b'\n' => return Err(err(start, "unterminated string")),
                        b'\\' => {
                            let Some(&escape) = bytes.get(i + 1) else {
                                return Err(err(i, "dangling escape"));
                            };
                            let decoded = match escape {
                                b'n' => b'\n',
                                b't' => b'\t',
                                b'r' => b'\r',
                                b'a' => 0x07,
                                b'b' => 0x08,
                                b'f' => 0x0C,
                                b'v' => 0x0B,
                                b'\\' => b'\\',
                                b'"' => b'"',
                                b'\'' => b'\'',
                                b'0'..=b'9' => {
                                    let mut value = 0u32;
                                    let mut digits = 0;
                                    while digits < 3
                                        && bytes.get(i + 1 + digits).is_some_and(u8::is_ascii_digit)
                                    {
                                        value = value * 10
                                            + (bytes[i + 1 + digits] - b'0') as u32;
                                        digits += 1;
                                    }
                                    if value > 255 {
                                        return Err(err(i, "\\ddd escape above 255"));
                                    }
                                    i += digits - 1;
                                    value as u8
                                }
                                other => {
                                    return Err(err(
                                        i,
                                        &format!("unsupported escape \\{}", other as char),
                                    ));
                                }
                            };
                            if decoded >= 0x80 {
                                return Err(err(i, "non-ASCII literal byte (fenced, D-056)"));
                            }
                            text.push(decoded);
                            i += 2;
                        }
                        _ if b == quote => {
                            i += 1;
                            break;
                        }
                        _ => {
                            if b >= 0x80 {
                                return Err(err(i, "non-ASCII literal byte (fenced, D-056)"));
                            }
                            text.push(b);
                            i += 1;
                        }
                    }
                }
                let text = String::from_utf8(text).expect("ASCII by construction");
                tokens.push(Spanned { token: Token::Str(text), start });
            }
            b'0'..=b'9' => {
                let (value, next) = lex_number(source, i)?;
                tokens.push(Spanned { token: Token::Number(value), start: i });
                i = next;
            }
            b'A'..=b'Z' | b'a'..=b'z' | b'_' => {
                let start = i;
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_')
                {
                    i += 1;
                }
                let word = &source[start..i];
                let token = match word {
                    "and" => Token::And, "break" => Token::Break, "do" => Token::Do,
                    "else" => Token::Else, "elseif" => Token::Elseif, "end" => Token::End,
                    "false" => Token::False, "for" => Token::For,
                    "function" => Token::Function, "if" => Token::If, "in" => Token::In,
                    "local" => Token::Local, "nil" => Token::Nil, "not" => Token::Not,
                    "or" => Token::Or, "repeat" => Token::Repeat, "return" => Token::Return,
                    "then" => Token::Then, "true" => Token::True, "until" => Token::Until,
                    "while" => Token::While,
                    _ => Token::Name(word.to_string()),
                };
                tokens.push(Spanned { token, start });
            }
            other => {
                return Err(err(i, &format!("unexpected byte 0x{other:02x}")));
            }
        }
    }
    Ok(tokens)
}

fn lex_number(source: &str, start: usize) -> Result<(f64, usize), LexError> {
    let bytes = source.as_bytes();
    let mut i = start;
    if bytes[i] == b'0' && matches!(bytes.get(i + 1), Some(b'x') | Some(b'X')) {
        i += 2;
        let hex_start = i;
        while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
            i += 1;
        }
        let value = u64::from_str_radix(&source[hex_start..i], 16)
            .map_err(|_| LexError { offset: start, message: "bad hex literal".into() })?;
        return Ok((value as f64, i));
    }
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    if matches!(bytes.get(i), Some(b'e') | Some(b'E')) {
        i += 1;
        if matches!(bytes.get(i), Some(b'+') | Some(b'-')) {
            i += 1;
        }
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    source[start..i]
        .parse()
        .map(|value| (value, i))
        .map_err(|_| LexError { offset: start, message: "bad number literal".into() })
}
