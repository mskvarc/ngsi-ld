//! What is wrong with a subscription, a context source registration, a snapshot or a notification document.

use crate::{
    model::{BuildError, ConformanceVersionError, DocumentError, MemberError, NotPositive, TenantError},
    query::{
        EntitySelectorError,
        ParseError,
        RegularExpressionError,
        geoquery::GeoQueryError,
        ordering::OrderingParamsError,
        temporal_query::{AggregationMethodsError, TemporalQueryError},
    },
    subscription::DocumentType,
};
use std::error::Error;

/// What is wrong with a document of the subscription family.
///
/// The document is a subscription, a context source registration, a snapshot or a notification.
///
/// These documents share the vocabulary of subscriptions (entity selectors, query languages, notification
/// parameters and the rules of their data types), so they share this kind; a reader of one reports a
/// [`DocumentError<ReadErrorKind<P>>`](DocumentError), whose `path` names the member at fault. `P` is the problem
/// of the document's family, which [`ReadErrorKind::Invalid`] lists: the `Problem` of subscriptions, registrations
/// or snapshots, and [`Infallible`](std::convert::Infallible) for a notification, whose data types relate no
/// members.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ReadErrorKind<P: Error> {
    /// A problem every reader can meet: a member of the wrong JSON type, a missing, repeated, unknown or empty
    /// member, or an invalid NGSI-LD Value, name, identifier, URL, scope, timestamp, geometry, duration, context
    /// source alias or `@context`.
    #[error(transparent)]
    Member(#[from] MemberError),
    /// The `type` names another document.
    #[error("expected type `{expected}`, found `{found}`")]
    WrongType {
        /// The type the document must have.
        expected: DocumentType,
        /// The type it has.
        found: String,
    },
    /// An `idPattern` is not a regular expression (clause 5.2.33).
    #[error("invalid id pattern")]
    IdPattern {
        /// Why.
        #[source]
        source: RegularExpressionError,
    },
    /// A tenant name is invalid.
    #[error("invalid tenant")]
    Tenant {
        /// Why.
        #[source]
        source: TenantError,
    },
    /// A number is not greater than 0.
    #[error("invalid number")]
    NotPositive {
        /// Why.
        #[source]
        source: NotPositive,
    },
    /// A query, scope query, language filter or projection does not follow its language.
    #[error("invalid query language text")]
    Query {
        /// Why.
        #[source]
        source: Box<ParseError>,
    },
    /// A geoquery is invalid.
    #[error("invalid geoquery")]
    GeoQuery {
        /// Why.
        #[source]
        source: Box<GeoQueryError>,
    },
    /// A temporal query is invalid.
    #[error("invalid temporal query")]
    TemporalQuery {
        /// Why.
        #[source]
        source: Box<TemporalQueryError>,
    },
    /// The `aggrMethods` of an `AggregationParams` object (clause 5.2.44) are not aggregation methods.
    #[error("invalid aggregation methods")]
    Aggregation {
        /// Why.
        #[source]
        source: AggregationMethodsError,
    },
    /// An `ordering` is not an `OrderingParams` object (clause 5.2.43).
    #[error("invalid ordering")]
    Ordering {
        /// Why.
        #[source]
        source: Box<OrderingParamsError>,
    },
    /// An entity selector is invalid.
    #[error("invalid entity selector")]
    EntitySelector {
        /// Why.
        #[source]
        source: Box<EntitySelectorError>,
    },
    /// An `ngsildConformance` is not a `major.minor` version.
    #[error("invalid conformance version")]
    Conformance {
        /// Why.
        #[source]
        source: ConformanceVersionError,
    },
    /// An entity, a Feature or a `FeatureCollection` in the `data` of a notification is invalid.
    #[error("invalid notified entity")]
    Entity {
        /// Why.
        #[source]
        source: Box<DocumentError>,
    },
    /// The members break a rule of Tables 5.2.9-1, 5.2.10-1, 5.2.12-1, 5.2.14.1-1, 5.2.15-1, 5.2.23-1, 5.2.33-1,
    /// 5.2.34-1 or 5.2.41-1.
    #[error("the document breaks the rules of its data type")]
    Invalid {
        /// The rules broken.
        #[source]
        source: BuildError<P>,
    },
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::{model::DocumentError, subscription::ReadErrorKind};
    use std::error::Error;

    /// The problems of a read refused for breaking the rules of the document's data type.
    pub(crate) fn broken_rules<P: Error + Clone>(error: DocumentError<ReadErrorKind<P>>) -> Vec<P> {
        let ReadErrorKind::Invalid { source } = error.kind else {
            panic!("expected broken rules, found {error}");
        };
        source.problems().to_vec()
    }
}
