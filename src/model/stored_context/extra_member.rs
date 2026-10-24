//! The members of a local context document beside its `@context` (NGSI-LD clause 5.13.2.3).

use crate::model::keyword::CONTEXT;

/// Why text cannot name a member of a local context document beside its `@context`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum ExtraMemberNameError {
    /// The name is empty.
    #[error("a member name cannot be empty")]
    Empty,
    /// The name is `@context`, the root node of the document itself.
    #[error("`@context` is the root node of the document, not a member beside it")]
    Context,
}

/// Validates a member name beside `@context`: any JSON member name but the empty one and `@context`.
fn validate_extra_member_name(name: &str) -> Result<(), ExtraMemberNameError> {
    match name {
        "" => Err(ExtraMemberNameError::Empty),
        CONTEXT => Err(ExtraMemberNameError::Context),
        _ => Ok(()),
    }
}

/// The name of a member a local context document carries beside its `@context` (NGSI-LD clause 5.13.2.3): any
/// JSON member name but `@context` and the empty name.
///
/// Clause 5.13.2.3 has the broker discard "all extra information located outside of the @context subtree", so
/// such members change nothing the broker stores; a request may still carry them, and a served document may still
/// hold them.
///
/// ```
/// use ngsi_ld::model::ExtraMemberName;
///
/// let note: ExtraMemberName = "note".parse()?;
/// assert_eq!(note.to_string(), "note");
/// assert!("@context".parse::<ExtraMemberName>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[nutype::nutype(
    validate(with = validate_extra_member_name, error = ExtraMemberNameError),
    derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display, AsRef, FromStr, TryFrom)
)]
pub struct ExtraMemberName(String);
