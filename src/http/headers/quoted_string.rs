// SPDX-FileCopyrightText: 2014-2025 Sean McArthur
// SPDX-License-Identifier: MIT

//! The quoted string of RFC 9110 section 5.6.4: how a header value that is a token or a quoted string is read, alone
//! or as the value of a `name=value` parameter, and how text is written as one.
//!
//! Portions of this file are derived from the disabled `Link` implementation in <https://github.com/hyperium/headers>.
//! This file, including local modifications, is distributed under the MIT License.

use crate::model::token::is_token;
use std::fmt;

/// Where a scan through a quoted string stands after its opening quote, for a scanner that splits a header value
/// and must not split inside one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuotedScan {
    /// Inside the string.
    Text,
    /// Right after a backslash: the next character is data (`quoted-pair`).
    Pair,
}

impl QuotedScan {
    /// The state after `byte`, or `None` where `byte` is the closing quote.
    pub(crate) const fn next(self, byte: u8) -> Option<QuotedScan> {
        match (self, byte) {
            (QuotedScan::Text, b'"') => None,
            (QuotedScan::Text, b'\\') => Some(QuotedScan::Pair),
            (QuotedScan::Text | QuotedScan::Pair, _) => Some(QuotedScan::Text),
        }
    }
}

/// A quoted string that is not closed, or is followed by more text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MalformedQuotedString;

/// Reads a value that is either a token, kept as written, or a quoted string, unescaped.
pub(crate) fn parameter_value(raw: &str) -> Result<String, MalformedQuotedString> {
    let Some(quoted) = raw.strip_prefix('"') else {
        return Ok(raw.to_owned());
    };
    let mut value = String::with_capacity(quoted.len());
    let mut characters = quoted.chars();
    while let Some(character) = characters.next() {
        match character {
            '"' if characters.as_str().is_empty() => return Ok(value),
            '"' => return Err(MalformedQuotedString),
            '\\' => value.push(characters.next().ok_or(MalformedQuotedString)?),
            other => value.push(other),
        }
    }
    Err(MalformedQuotedString)
}

/// A `name=value` parameter whose name is not a token, or whose value is neither a token nor a quoted string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MalformedParameter;

/// Reads a parameter, `token "=" ( token / quoted-string )` (RFC 9110 section 5.6.6), with no whitespace around the
/// `=`: the name as written and the value unescaped.
pub(crate) fn parse_parameter(parameter: &str) -> Result<(&str, String), MalformedParameter> {
    let (name, raw_value) = parameter.split_once('=').ok_or(MalformedParameter)?;
    if !is_token(name) || !(raw_value.starts_with('"') || is_token(raw_value)) {
        return Err(MalformedParameter);
    }
    let value = parameter_value(raw_value).map_err(|MalformedQuotedString| MalformedParameter)?;
    Ok((name, value))
}

/// Writes `text` as a quoted string, escaping `"` and `\` (RFC 9110 section 5.6.4).
pub(crate) fn write_quoted(f: &mut fmt::Formatter<'_>, text: &str) -> fmt::Result {
    f.write_str("\"")?;
    for piece in text.split_inclusive(['"', '\\']) {
        match piece.char_indices().last() {
            Some((last, special @ ('"' | '\\'))) => write!(f, "{}\\{special}", &piece[..last])?,
            Some(_) | None => f.write_str(piece)?,
        }
    }
    f.write_str("\"")
}

/// Writes `text` as a token when it is one, and as a quoted string otherwise.
pub(crate) fn write_token_or_quoted(f: &mut fmt::Formatter<'_>, text: &str) -> fmt::Result {
    if is_token(text) { f.write_str(text) } else { write_quoted(f, text) }
}

#[cfg(test)]
mod tests {
    use crate::http::headers::quoted_string::{
        MalformedParameter,
        MalformedQuotedString,
        QuotedScan,
        parameter_value,
        parse_parameter,
        write_quoted,
        write_token_or_quoted,
    };
    use pretty_assertions::assert_eq;
    use std::fmt;

    #[test]
    fn a_quoted_string_scan_ends_at_the_first_unescaped_quote() {
        let mut scan = Some(QuotedScan::Text);
        let mut length = 0;
        for byte in br#"a\"b"c"# {
            let Some(state) = scan else { break };
            scan = state.next(*byte);
            length += 1;
        }
        assert_eq!((scan, length), (None, 5));
    }

    #[test]
    fn a_quoted_value_is_unescaped_and_a_token_is_kept() {
        assert_eq!(parameter_value(r#""a\"b\\c""#), Ok(r#"a"b\c"#.to_owned()));
        assert_eq!(parameter_value("next"), Ok("next".to_owned()));
    }

    #[test]
    fn an_unterminated_or_overrun_quoted_value_is_rejected() {
        for raw in [r#""abc"#, r#""abc\"#, r#""a"b"#, "\""] {
            assert_eq!(parameter_value(raw), Err(MalformedQuotedString), "{raw}");
        }
    }

    #[test]
    fn a_parameter_is_a_token_name_and_a_token_or_quoted_value() {
        assert_eq!(parse_parameter("ma=3600"), Ok(("ma", "3600".to_owned())));
        assert_eq!(parse_parameter(r#"for="[::1]:4711""#), Ok(("for", "[::1]:4711".to_owned())));
        assert_eq!(parse_parameter(r#"a="b\"c""#), Ok(("a", r#"b"c"#.to_owned())));
    }

    #[test]
    fn a_parameter_without_a_token_name_or_value_is_rejected() {
        for parameter in ["ma", "=1", "m a=1", "ma =1", "ma= 1", "ma=", "for=[::1]", r#"a="b"#, r#"a="b"c"#] {
            assert_eq!(parse_parameter(parameter), Err(MalformedParameter), "{parameter}");
        }
    }

    struct Quoted(&'static str);

    impl fmt::Display for Quoted {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write_quoted(f, self.0)
        }
    }

    struct TokenOrQuoted(&'static str);

    impl fmt::Display for TokenOrQuoted {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write_token_or_quoted(f, self.0)
        }
    }

    #[test]
    fn writing_a_quoted_string_escapes_quotes_and_backslashes_and_reads_back() {
        let written = Quoted(r#"text/plain; a="b\c""#).to_string();
        assert_eq!(written, r#""text/plain; a=\"b\\c\"""#);
        assert_eq!(parameter_value(&written), Ok(r#"text/plain; a="b\c""#.to_owned()));
    }

    #[test]
    fn a_token_is_written_bare_and_anything_else_quoted() {
        assert_eq!(TokenOrQuoted("https").to_string(), "https");
        assert_eq!(TokenOrQuoted("[2001:db8::1]:4711").to_string(), "\"[2001:db8::1]:4711\"");
        assert_eq!(TokenOrQuoted("").to_string(), "\"\"");
    }
}
