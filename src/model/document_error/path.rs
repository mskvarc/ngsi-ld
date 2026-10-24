//! Where in a document a reader found a problem.

use std::fmt;

/// One step of a [`MemberPath`]: a member of an object or an element of an array.
///
/// JSON has only these two containers, so the enum is exhaustive.
///
/// ```
/// use ngsi_ld::model::PathSegment;
///
/// assert_eq!(
///     PathSegment::Member("entities".to_owned()).to_string(),
///     "entities"
/// );
/// assert_eq!(PathSegment::Index(1).to_string(), "1");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum PathSegment {
    /// A member, named as the document writes it: a member the reader does not know is named too.
    #[display("{_0}")]
    Member(String),
    /// The element at this index of an array.
    #[display("{_0}")]
    Index(usize),
}

/// The members and array elements leading from the top of a document to a problem, written `a.b.0`.
///
/// ```
/// use ngsi_ld::{model::{Input, PathSegment}, subscription::Subscription};
///
/// let text = r#"{"type":"Subscription","entities":[{"type":"Vehicle"},{"id":"urn:ngsi-ld:Vehicle:A4567"}],"notification":{"endpoint":{"uri":"http://my.endpoint.org/notify"}}}"#;
/// let path = Subscription::<Input>::parse(text).err().map(|error| error.path);
/// assert_eq!(path.as_ref().map(ToString::to_string).as_deref(), Some("entities.1"));
/// assert_eq!(path.as_ref().and_then(|path| path.segments().last()), Some(&PathSegment::Index(1)));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct MemberPath(Vec<PathSegment>);

impl MemberPath {
    /// The empty path: the top of the document.
    pub(crate) const fn new() -> Self {
        Self(Vec::new())
    }

    /// Places the path under `segment`. The segments are stored innermost first, as a reader adds them while the
    /// error travels outwards.
    pub(crate) fn push(&mut self, segment: PathSegment) {
        self.0.push(segment);
    }

    /// Returns the segments, outermost first.
    pub fn segments(&self) -> impl DoubleEndedIterator<Item = &PathSegment> + ExactSizeIterator {
        self.0.iter().rev()
    }

    /// Whether the path is empty: the problem is at the top of the document.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

// Joining the segments with `.` has no derive.
impl fmt::Display for MemberPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, segment) in self.segments().enumerate() {
            if index > 0 {
                f.write_str(".")?;
            }
            write!(f, "{segment}")?;
        }
        Ok(())
    }
}
