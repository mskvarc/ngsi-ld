//! Running the recursive walks of a document on a stack that grows as deep as the document nests.
//!
//! Three things in a document nest without a bound the specification sets, so a broker may return any of them
//! thousands of levels deep:
//!
//! - an attribute holds sub-attributes (clauses 4.5.2.2 and 4.5.3.2), and a Relationship of a response holds the
//!   entities inline linked entity retrieval adds, which hold Relationships in turn, as many levels deep as
//!   `joinLevel` asks (clause 4.5.23.2): every walk of those passes through
//!   [`Attributes`](crate::model::Attributes);
//! - a [`Value`](crate::model::Value) is any JSON but `null` (clause 3.1), arrays and objects nested at any
//!   depth: its reading, writing, cloning, comparing, debug-printing, dropping and the search for a nested NGSI-LD
//!   Null each recurse through it;
//! - [`RawJson`](crate::model::RawJson), the `json` of a `JsonProperty` (clause 4.5.24) and the value of
//!   a foreign member of a geometry (RFC 7946 section 6.1), is any JSON: it is cloned, compared, hashed and copied
//!   into a written document by walks of its own.
//!
//! Each of those walks passes through [`grow`] once per level, which moves to a fresh stack segment when the current
//! one runs low, instead of imposing a depth limit the specification does not have. Every other type of the model
//! nests to a depth its clause fixes: a geometry (clause 4.7.1 excludes `GeometryCollection`, and its foreign members
//! are raw JSON), a language map, a
//! scope and the `dataset` of a simplified attribute, which holds instances and never another `dataset`.
//!
//! The query languages a broker returns in its subscriptions and registrations nest without a bound as well: the
//! groups of a query (clause 4.9) and the linked projections of `pick` and `omit` (clause 4.21). Their parsers and
//! the walks of their trees pass through [`grow`] once per level too, through
//! [`Operands`](crate::query::q::Operands) and [`Projection`](crate::query::projection::Projection); a linked entity
//! path of a query is a flat list of links, which needs no walk of its own.

/// How much stack must remain before a walk descends one more level without a new segment.
const RED_ZONE: usize = 128 * 1024;

/// The size of each new stack segment.
const SEGMENT: usize = 2 * 1024 * 1024;

/// Runs `walk` on the current stack, or on a new segment when fewer than [`RED_ZONE`] bytes remain.
pub(crate) fn grow<R>(walk: impl FnOnce() -> R) -> R {
    stacker::maybe_grow(RED_ZONE, SEGMENT, walk)
}
