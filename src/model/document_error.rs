//! Why a document could not be read, and where in the document the problem lies.

use crate::model::{DocumentErrorKind, MemberPath, PathSegment};
use std::fmt;

pub(super) mod kind;
pub(super) mod member_error;
pub(super) mod path;

/// A document could not be read: what is wrong (`kind`), and where (`path`).
///
/// `K` is the kind of problem the reader reports: [`DocumentErrorKind`] for the documents of the data model, the
/// default, and [`ReadErrorKind`](crate::subscription::ReadErrorKind) for subscriptions, context source
/// registrations, snapshots and notifications.
///
/// Both kinds hold the problems every reader shares as a [`MemberError`](crate::model::MemberError).
///
/// ```
/// use ngsi_ld::model::{DocumentError, DocumentErrorKind, Entity, Lossless, Output};
///
/// let text = r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":{"value":80}}"#;
/// let error = Entity::<Output>::parse(text, Lossless::Normalized).err();
/// assert!(matches!(
///     error,
///     Some(DocumentError {
///         kind: DocumentErrorKind::MissingType,
///         ..
///     })
/// ));
/// assert_eq!(
///     error.map(|error| error.to_string()).as_deref(),
///     Some("an attribute in normalized representation needs a `type` member at speed")
/// );
/// ```
// Every kind a reader reports chains `iri_rs::InvalidIri`, which carries no `Clone` or equality, so neither can the
// error that holds the kind.
#[derive(Debug, thiserror::Error)]
#[error("{kind}{}", Location(&self.path))]
pub struct DocumentError<K = DocumentErrorKind> {
    /// The members and array elements leading to the problem; empty for the top of the document.
    pub path: MemberPath,
    /// What is wrong.
    pub kind: K,
}

impl<K> DocumentError<K> {
    /// An error at the top of the document.
    pub(crate) const fn top(kind: K) -> Self {
        Self { path: MemberPath::new(), kind }
    }

    /// Places the error under the member `member`.
    pub(crate) fn within(mut self, member: &str) -> Self {
        self.path.push(PathSegment::Member(member.to_owned()));
        self
    }

    /// Places the error under the array element at `index`.
    pub(crate) fn at_index(mut self, index: usize) -> Self {
        self.path.push(PathSegment::Index(index));
        self
    }

    /// Converts the kind with `convert`, keeping the path: a reader that reads another document places that
    /// document's error in its own, under the member that holds the document.
    pub(crate) fn map_kind<L>(self, convert: impl FnOnce(K) -> L) -> DocumentError<L> {
        DocumentError {
            path: self.path,
            kind: convert(self.kind),
        }
    }
}

/// Renders ` at a.b.0`, or nothing for the top of the document.
struct Location<'a>(&'a MemberPath);

// Whether anything is written depends on the path being empty, which no derive expresses.
impl fmt::Display for Location<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() { Ok(()) } else { write!(f, " at {}", self.0) }
    }
}

#[cfg(test)]
mod tests;
