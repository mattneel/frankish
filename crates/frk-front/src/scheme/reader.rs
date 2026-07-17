//! The scheme reader: source text → `Datum` trees (r7rs_core, D-060).
//! S-expressions are universal, so this layer is independent of how the
//! core forms are later interpreted or emitted. v0 lexical surface per
//! the ratified MANIFEST: parens, symbols, decimal integers (optional
//! leading `-`), booleans `#t`/`#f`, `;` line comments, and datum
//! spans (byte offsets) for §6.5 location threading.

#[derive(Clone, Debug, PartialEq)]
pub enum Datum {
    /// A symbol (identifier or operator name).
    Symbol(String, Span),
    /// A fixnum literal (i64; the corpus stays in range).
    Int(i64, Span),
    /// `#t` / `#f`.
    Bool(bool, Span),
    /// A proper list `( d… )`.
    List(Vec<Datum>, Span),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Datum {
    pub fn span(&self) -> Span {
        match self {
            Datum::Symbol(_, s) | Datum::Int(_, s) | Datum::Bool(_, s) | Datum::List(_, s) => *s,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Tok {
    Open(usize),
    Close(usize),
    Quote(usize),
    Atom(String, usize, usize),
}

fn is_delimiter(byte: u8) -> bool {
    byte.is_ascii_whitespace()
        || byte == b'('
        || byte == b')'
        || byte == b';'
        || byte == b'\''
}

fn tokenize(source: &str) -> Result<Vec<Tok>, String> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        if byte.is_ascii_whitespace() {
            i += 1;
        } else if byte == b';' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if byte == b'\'' {
            tokens.push(Tok::Quote(i));
            i += 1;
        } else if byte == b'(' {
            tokens.push(Tok::Open(i));
            i += 1;
        } else if byte == b')' {
            tokens.push(Tok::Close(i));
            i += 1;
        } else {
            let start = i;
            while i < bytes.len() && !is_delimiter(bytes[i]) {
                i += 1;
            }
            let text = source[start..i].to_string();
            tokens.push(Tok::Atom(text, start, i));
        }
    }
    Ok(tokens)
}

fn atom(text: &str, start: usize, end: usize) -> Result<Datum, String> {
    let span = Span { start, end };
    match text {
        "#t" => return Ok(Datum::Bool(true, span)),
        "#f" => return Ok(Datum::Bool(false, span)),
        _ => {}
    }
    // A fixnum: optional leading '-' then all digits (and not a bare '-').
    let is_int = {
        let body = text.strip_prefix('-').unwrap_or(text);
        !body.is_empty() && body.bytes().all(|b| b.is_ascii_digit()) && text != "-"
    };
    if is_int {
        text.parse::<i64>()
            .map(|value| Datum::Int(value, span))
            .map_err(|_| format!("integer literal out of range: {text}"))
    } else {
        Ok(Datum::Symbol(text.to_string(), span))
    }
}

/// Reads every top-level datum in `source`.
pub fn read(source: &str) -> Result<Vec<Datum>, String> {
    let tokens = tokenize(source)?;
    let mut position = 0;
    let mut data = Vec::new();
    while position < tokens.len() {
        data.push(read_datum(&tokens, &mut position, source.len())?);
    }
    Ok(data)
}

fn read_datum(tokens: &[Tok], position: &mut usize, source_len: usize) -> Result<Datum, String> {
    let token = tokens
        .get(*position)
        .ok_or_else(|| "unexpected end of input".to_string())?;
    match token {
        Tok::Quote(start) => {
            // 'd — reader sugar for (quote d) (v0.1, D-070).
            let start = *start;
            *position += 1;
            let quoted = read_datum(tokens, position, source_len)?;
            let end = quoted.span().end;
            let span = Span { start, end };
            Ok(Datum::List(
                vec![Datum::Symbol("quote".to_string(), Span { start, end: start + 1 }), quoted],
                span,
            ))
        }
        Tok::Atom(text, start, end) => {
            *position += 1;
            atom(text, *start, *end)
        }
        Tok::Open(start) => {
            let list_start = *start;
            *position += 1;
            let mut items = Vec::new();
            loop {
                match tokens.get(*position) {
                    None => return Err("unterminated list: missing `)`".to_string()),
                    Some(Tok::Close(end)) => {
                        let span = Span { start: list_start, end: end + 1 };
                        *position += 1;
                        return Ok(Datum::List(items, span));
                    }
                    Some(_) => items.push(read_datum(tokens, position, source_len)?),
                }
            }
        }
        Tok::Close(at) => Err(format!("unexpected `)` at byte {at}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_atoms_and_nesting() {
        let data = read("(define (f x) (+ x 1)) ; comment\n42 #t -3").unwrap();
        assert_eq!(data.len(), 4);
        assert!(matches!(&data[0], Datum::List(items, _) if items.len() == 3));
        assert_eq!(data[1], Datum::Int(42, data[1].span()));
        assert_eq!(data[2], Datum::Bool(true, data[2].span()));
        assert_eq!(data[3], Datum::Int(-3, data[3].span()));
    }

    #[test]
    fn bare_minus_is_a_symbol() {
        let data = read("(- 5 2)").unwrap();
        let Datum::List(items, _) = &data[0] else { panic!() };
        assert_eq!(items[0], Datum::Symbol("-".into(), items[0].span()));
    }

    #[test]
    fn unterminated_list_errs() {
        assert!(read("(+ 1 2").is_err());
        assert!(read(")").is_err());
    }
}
