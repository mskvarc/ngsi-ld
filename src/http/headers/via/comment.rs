//! The comment that may close a `Via` element (RFC 9110 sections 5.6.5 and 7.6.3).

use crate::http::headers::field_value::is_field_text;

/// Why a string is not the text of a comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum ViaCommentError {
    /// The text holds a character other than visible ASCII, a space or a tab.
    #[error("a comment may only hold visible ASCII, spaces and tabs")]
    InvalidCharacter,
    /// A parenthesis is not matched, or a backslash escapes nothing.
    #[error("a comment must balance its parentheses and escape a character after each backslash")]
    Unbalanced,
}

/// Checks the text of a comment: `*( ctext / quoted-pair / comment )` (RFC 9110 section 5.6.5).
fn validate_comment(text: &str) -> Result<(), ViaCommentError> {
    if !is_field_text(text) {
        return Err(ViaCommentError::InvalidCharacter);
    }
    let mut depth = 0_usize;
    let mut bytes = text.bytes();
    while let Some(byte) = bytes.next() {
        match byte {
            b'\\' => {
                bytes.next().ok_or(ViaCommentError::Unbalanced)?;
            }
            b'(' => depth += 1,
            b')' => depth = depth.checked_sub(1).ok_or(ViaCommentError::Unbalanced)?,
            _other => {}
        }
    }
    if depth == 0 { Ok(()) } else { Err(ViaCommentError::Unbalanced) }
}

/// The text of the comment of a `Via` element (RFC 9110 section 7.6.3), such as the software of the recipient,
/// without the parentheses that enclose it; nested comments and escapes are kept as written.
///
/// ```
/// use ngsi_ld::http::headers::{ViaComment, ViaCommentError};
///
/// let comment: ViaComment = "broker (build 7)".parse()?;
/// assert_eq!(comment.to_string(), "broker (build 7)");
/// assert_eq!(
///     "broker (".parse::<ViaComment>(),
///     Err(ViaCommentError::Unbalanced)
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[nutype::nutype(
    validate(with = validate_comment, error = ViaCommentError),
    derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display, AsRef, FromStr, TryFrom)
)]
pub struct ViaComment(String);

#[cfg(test)]
mod tests {
    use crate::http::headers::{ViaComment, ViaCommentError};
    use pretty_assertions::assert_eq;

    #[test]
    fn text_nested_comments_and_escapes_are_kept() {
        for text in ["", "Apache/2.4", "a, b; c", "a (b (c))", r"a \) b", "tab\there"] {
            assert_eq!(text.parse::<ViaComment>().map(|comment| comment.to_string()), Ok(text.to_owned()), "{text}");
        }
    }

    #[test]
    fn an_unbalanced_comment_or_a_dangling_escape_is_rejected() {
        for text in ["a (b", "a) (b", ")", r"a \"] {
            assert_eq!(text.parse::<ViaComment>(), Err(ViaCommentError::Unbalanced), "{text}");
        }
    }

    #[test]
    fn a_control_or_non_ascii_character_is_rejected() {
        for text in ["a\nb", "caf\u{e9}", "\u{7f}"] {
            assert_eq!(text.parse::<ViaComment>(), Err(ViaCommentError::InvalidCharacter), "{text}");
        }
    }
}
