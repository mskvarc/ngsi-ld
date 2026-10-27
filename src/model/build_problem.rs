//! One problem a data-model builder found: what is wrong, and the attribute it belongs to.

use crate::model::{
    AttributeKind,
    AttributeName,
    AttributesError,
    BuildError,
    InvalidJson,
    LanguageMapError,
    NameError,
    NullMember,
    PatchMember,
    ReservedMember,
    ScopeError,
    UriError,
    geometry::GeometryError,
};
use std::{error::Error, fmt};

/// A value a builder could not resolve; its problem is already recorded among the builder's problems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Recorded;

/// One problem, and the attributes it is nested in.
///
/// It displays as the attribute path, outermost first, each name followed by `: `, then the problem.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BuildProblem {
    /// The attribute path of the attribute the problem belongs to, innermost first, as the problem is moved out
    /// of each attribute; empty for the entity.
    within: Vec<AttributeName>,
    /// What is wrong.
    pub kind: ProblemKind,
}

impl BuildProblem {
    /// Returns the attribute path, outermost first, of the attribute the problem belongs to; empty for the entity.
    pub fn within(&self) -> impl DoubleEndedIterator<Item = &AttributeName> + ExactSizeIterator {
        self.within.iter().rev()
    }

    /// A problem at the top level.
    pub(crate) const fn top(kind: ProblemKind) -> Self {
        Self { within: Vec::new(), kind }
    }

    /// Moves the problem under the attribute `name`.
    pub(crate) fn nested_in(mut self, name: &AttributeName) -> Self {
        // Each problem keeps its own path, so the name is copied into every one.
        self.within.push(name.clone());
        self
    }

    /// A problem of the attribute `name`.
    pub(crate) fn of(name: &AttributeName, kind: ProblemKind) -> Self {
        Self {
            // Clone: the problem owns its path, while the caller keeps the name.
            within: vec![name.clone()],
            kind,
        }
    }
}

// No derive writes the attribute path before the problem.
impl fmt::Display for BuildProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for name in self.within() {
            write!(f, "{name}: ")?;
        }
        fmt::Display::fmt(&self.kind, f)
    }
}

// No derive: the source is the kind's, which the problem displays itself.
impl Error for BuildProblem {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.kind.source()
    }
}

/// Generates the conversions that let `?` turn the error of a value validated by hand into a one-problem
/// [`BuildError`]. No derive wraps an error into a list of problems.
macro_rules! build_error_from {
    ($($error:ty),+) => {$(
        impl From<$error> for BuildError {
            fn from(source: $error) -> Self {
                Self::single(BuildProblem::top(ProblemKind::from(source)))
            }
        }
    )+};
}

build_error_from!(NameError, ScopeError, UriError, LanguageMapError, GeometryError, InvalidJson);

/// What is wrong with a value given to a builder.
#[derive(Debug, Clone, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum ProblemKind {
    /// The name given to the attribute at `index`, counting from 0 in the order the builder was given them, is
    /// not an attribute name.
    #[error("attribute {index} is not a valid attribute name")]
    AttributeName {
        /// The position of the attribute among the ones given.
        index: usize,
        /// Why the name is invalid.
        #[source]
        source: NameError,
    },
    /// An entity type, `objectType`, `vocab` or `valueType` is not a valid name or term.
    #[error("invalid name or term")]
    Term {
        /// Why it is invalid.
        #[from]
        source: NameError,
    },
    /// A scope is not valid.
    #[error("invalid scope")]
    Scope {
        /// Why it is invalid.
        #[from]
        source: ScopeError,
    },
    /// An identifier is not a URI.
    #[error("invalid identifier")]
    Identifier {
        /// Why it is invalid.
        #[from]
        source: UriError,
    },
    /// A language map is not valid.
    #[error("invalid language map")]
    LanguageMap {
        /// Why it is invalid.
        #[from]
        source: LanguageMapError,
    },
    /// A geometry is not valid.
    #[error("invalid geometry")]
    Geometry {
        /// Why it is invalid.
        #[from]
        source: GeometryError,
    },
    /// The `json` of a `JsonProperty` is not valid JSON.
    #[error("invalid JSON")]
    Json {
        /// Why it is invalid.
        #[from]
        source: InvalidJson,
    },
    /// The `json` of a `JsonProperty` is neither an object nor an array of objects (clause 4.5.24.2).
    #[error("the json of a JsonProperty must be an object or an array of objects")]
    JsonShape,
    /// An attribute cannot be added under its name: its instances break clause 4.5.5, or the name is reserved for
    /// a `GeoProperty` (clause 5.2.4).
    #[error("`{name}` cannot be added")]
    Attributes {
        /// The attribute.
        name: AttributeName,
        /// Which rule is broken.
        #[source]
        source: AttributesError,
    },
    /// A list that needs at least one value is empty: an entity's `type` or `scope`, a Relationship's `object` or
    /// `objectType`, a `VocabProperty`'s `vocab`.
    #[error("`{member}` needs at least one value")]
    EmptyList {
        /// The member.
        member: ReservedMember,
    },
    /// A request attribute holds the NGSI-LD Null, which clause 5.5.4 allows only in fragments.
    #[error("a request cannot hold the NGSI-LD Null in {member}")]
    NgsiLdNull {
        /// Where the Null is.
        member: NullMember,
    },
    /// A fragment deletes a sub-attribute of an attribute it does not set (as its default instance).
    #[error("`{name}` must be set before one of its sub-attributes can be deleted")]
    SubAttributeOfUnsetAttribute {
        /// The attribute.
        name: AttributeName,
    },
    /// The value of an `ngsildproof` is not a JSON object (clause 4.5.2.2).
    #[error("the value of an ngsildproof must be a JSON object")]
    ProofNotAnObject,
    /// A fragment names a member the attribute kind it names has not (clauses 4.5.2.2 and 4.5.3.2).
    #[error("a {kind} has no `{member}` member")]
    MemberOfOtherKind {
        /// The member.
        member: PatchMember,
        /// The kind the fragment names.
        kind: AttributeKind,
    },
    /// A simplified fragment is given a `dataset` value; its instances are named one by one, each with its dataset
    /// id (clause 4.5.4).
    #[error("the instances of a simplified attribute are given one by one, not as a dataset value")]
    DatasetValue,
}
