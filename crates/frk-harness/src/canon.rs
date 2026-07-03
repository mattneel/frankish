//! The canonicalization filter — the single implementation of
//! docs/canon.md. No diff is judged outside this filter (SPEC §7.4);
//! golden comparison, differential comparison, and oracle comparison all
//! pass through here.

/// Normalizes raw runner/oracle output to canonical bytes per
/// docs/canon.md §1: CRLF and lone CR become LF; non-empty output gains a
/// final LF if missing. Interior bytes are untouched, and extra trailing
/// LFs are preserved (the filter hides line-ending flavor, not bugs).
pub fn canonicalize(raw: &str) -> String {
    let mut text = raw.replace("\r\n", "\n").replace('\r', "\n");
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text
}

/// Renders an entry function's `i64` result per docs/canon.md §2 — the
/// v0 definition of a golden's output.
pub fn render_i64(value: i64) -> String {
    format!("{value}\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crlf_and_cr_normalize_to_lf() {
        assert_eq!(canonicalize("a\r\nb\rc"), "a\nb\nc\n");
    }

    #[test]
    fn trailing_newline_added_exactly_once() {
        assert_eq!(canonicalize("42"), "42\n");
        assert_eq!(canonicalize("42\n"), "42\n");
    }

    #[test]
    fn empty_stays_empty() {
        assert_eq!(canonicalize(""), "");
    }

    #[test]
    fn extra_trailing_newlines_are_preserved() {
        assert_eq!(canonicalize("x\n\n"), "x\n\n");
    }

    #[test]
    fn canonicalize_is_idempotent() {
        for sample in ["", "a", "a\r\nb\r", "x\n\n", "-9\n", "\r\n"] {
            let once = canonicalize(sample);
            assert_eq!(canonicalize(&once), once, "sample {sample:?}");
        }
    }

    #[test]
    fn render_i64_is_decimal_with_one_lf() {
        assert_eq!(render_i64(42), "42\n");
        assert_eq!(render_i64(0), "0\n");
        assert_eq!(render_i64(-7), "-7\n");
        assert_eq!(render_i64(i64::MIN), "-9223372036854775808\n");
    }
}
