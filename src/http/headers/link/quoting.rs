// SPDX-FileCopyrightText: 2014-2025 Sean McArthur
// SPDX-License-Identifier: MIT

//! The lexical rule of a `Link` header value: where it splits.
//!
//! This file supports the adapted `Link` implementation derived from <https://github.com/hyperium/headers> and is
//! distributed under the MIT License.
//!
//! Link-values are separated by `,` and parameters by `;`, but either character may appear inside a quoted
//! parameter value or inside a `<...>` target, so neither `str::split` nor `str::trim_matches` can be used directly.

use crate::http::headers::quoted_string::QuotedScan;

/// Where the scan of a header value stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scan {
    /// Outside any target or quoted string: a delimiter splits here.
    Plain,
    /// Inside a quoted string.
    Quoted(QuotedScan),
    /// Inside a `<...>` target, where quotes, backslashes, `,` and `;` are all data.
    Target,
}

/// Whether a `>` followed by `rest` closes a target.
///
/// A target ends at a `>` followed, after optional whitespace, by a delimiter or the end of the value. NGSI-LD
/// pagination targets carry `q` values verbatim, such as `q=speed>50`, so a `>` followed by anything else is part
/// of the target.
fn closes_target(rest: &str) -> bool {
    rest.trim_start_matches([' ', '\t'])
        .chars()
        .next()
        .is_none_or(|next| next == ';' || next == ',')
}

/// Splits a header value on one delimiter, never inside a quoted string or a `<...>` target.
#[derive(Debug, Clone)]
pub(crate) struct Elements<'a> {
    /// The text not yet split, or `None` once the last element was returned.
    rest: Option<&'a str>,
    /// The delimiter, an ASCII byte.
    delimiter: u8,
}

impl<'a> Elements<'a> {
    /// Splits `source` on every unprotected `delimiter`.
    pub(crate) const fn new(source: &'a str, delimiter: u8) -> Elements<'a> {
        Elements { rest: Some(source), delimiter }
    }
}

impl<'a> Iterator for Elements<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        let source = self.rest?;
        let mut scan = Scan::Plain;
        for (index, byte) in source.bytes().enumerate() {
            scan = match (scan, byte) {
                (Scan::Plain, delimiter) if delimiter == self.delimiter => {
                    // The delimiter is ASCII, so both slices start and end on character boundaries.
                    self.rest = Some(&source[index + 1..]);
                    return Some(&source[..index]);
                }
                (Scan::Plain, b'"') => Scan::Quoted(QuotedScan::Text),
                (Scan::Plain, b'<') => Scan::Target,
                (Scan::Quoted(quoted), found) => quoted.next(found).map_or(Scan::Plain, Scan::Quoted),
                (Scan::Target, b'>') if closes_target(&source[index + 1..]) => Scan::Plain,
                (unchanged @ (Scan::Plain | Scan::Target), _other) => unchanged,
            };
        }
        self.rest = None;
        Some(source)
    }
}

#[cfg(test)]
mod tests {
    use crate::http::headers::link::quoting::Elements;
    use pretty_assertions::assert_eq;

    fn split(source: &str, delimiter: u8) -> Vec<&str> {
        Elements::new(source, delimiter).collect()
    }

    #[test]
    fn a_delimiter_inside_quotes_does_not_split() {
        assert_eq!(split("a; \"b, c; d\"; e", b';'), vec!["a", " \"b, c; d\"", " e"]);
    }

    #[test]
    fn an_escaped_quote_does_not_end_the_quoted_string() {
        assert_eq!(split(r#"rel="next"; title="a\"b; c""#, b';'), vec![r#"rel="next""#, r#" title="a\"b; c""#]);
    }

    #[test]
    fn a_comma_inside_a_target_does_not_split() {
        assert_eq!(
            split("</entities?attrs=a,b>; rel=\"next\", </x>", b','),
            vec!["</entities?attrs=a,b>; rel=\"next\"", " </x>"]
        );
    }

    #[test]
    fn a_semicolon_quote_or_angle_bracket_inside_a_target_does_not_split() {
        let element = r#"</entities?type=Vehicle&q=speed>50;brandName!="Mercedes">; rel="next""#;
        assert_eq!(
            split(element, b';'),
            vec![r#"</entities?type=Vehicle&q=speed>50;brandName!="Mercedes">"#, r#" rel="next""#]
        );
        assert_eq!(split(element, b','), vec![element]);
    }

    #[test]
    fn a_value_without_delimiters_is_one_element() {
        assert_eq!(split("</a>", b','), vec!["</a>"]);
        assert_eq!(split("a,", b','), vec!["a", ""]);
    }
}
