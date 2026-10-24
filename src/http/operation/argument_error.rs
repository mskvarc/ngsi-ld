//! The arguments of an operation that the specification refuses, caught before a request is sent.

use crate::{
    http::operation::RepresentationToken,
    model::{BuildError, LanguageTagError},
    query::{Problem, projection::EntityMember},
};

/// Why the arguments of an operation make a request the specification refuses.
///
/// The enum is `#[non_exhaustive]`: later versions of the specification add operations and parameters, and with them
/// combinations they refuse.
#[derive(Debug, Clone, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum ArgumentError {
    /// `limit=0` without `count=true`, which clause 6.3.13 answers with a `BadRequest`.
    #[error("a limit of 0 is only allowed together with count")]
    ZeroLimitWithoutCount,
    /// A query names no entity type, attribute, `q` or geoquery and is not limited to local scope, which clause
    /// 6.4.3.2 does not allow.
    #[error("a query needs an entity type, attributes, a q or a geoquery unless it is local")]
    UnboundedQuery,
    /// A query sent through POST selects entities by id without a type, which the `EntitySelector` of clause
    /// 5.2.33 cannot express.
    #[error("a query sent through POST needs an entity type to select entities by id")]
    SelectorWithoutType,
    /// A query sent through GET selects entities with more than one entity selector, which the `type`, `id` and
    /// `idPattern` parameters cannot carry (Table 6.4.3.2-1); the `Query` body of the POST binding carries any number
    /// (clause 5.2.23).
    #[error("only a query sent through POST can carry several entity selectors")]
    SeveralSelectorsInQueryString,
    /// A query sent through POST selects entities by id and by id pattern in the one selection its `type`, `id` and
    /// `idPattern` make: the GET binding matches entities that have one of the ids and match the pattern (clause
    /// 5.7.2.4), while an `EntitySelector` lets `id` take precedence over `idPattern` (clause 5.2.33), so the body
    /// would ask for other entities. Several selectors carry the two apart.
    #[error("a query sent through POST cannot select by id and by id pattern in one selection")]
    IdsWithIdPatternInSelector,
    /// A temporal request asks for the normalized representation, which Table 6.3.12-1 gives no `format` token, with
    /// an `options` keyword beside it: with no `format` to take precedence over it (clause 6.3.12), the keyword would
    /// choose the representation of the answer instead of the one the request reads.
    #[error("the options keyword `{keyword}` needs a format to take precedence over it")]
    OptionsKeywordWithoutFormat {
        /// The keyword.
        keyword: RepresentationToken,
    },
    /// A query of every entity type (`*`) with `local=false`, which Tables 6.4.3.2-1 and 5.2.33-1 forbid: `*` implies
    /// local scope.
    #[error("a query of every entity type is local and cannot be distributed")]
    AllTypesNotLocal,
    /// A `pick` or `omit` removes the entity member `id` or `type` (Table 6.4.3.2-1), so the answer cannot be read
    /// as complete entities; a request for projected entities reads it.
    #[error("the projection removes `{member}`, so the entities must be read as projected entities")]
    ProjectionRemovesMember {
        /// The member removed.
        member: EntityMember,
    },
    /// A query-language argument (a type selection, `q`, `csf`, `pick`, `omit`, a language filter or an entity
    /// ordering) is invalid.
    #[error("a query-language argument is invalid")]
    InvalidArgument {
        /// Every problem found.
        #[source]
        source: BuildError<Problem>,
    },
    /// The `lang` of a simplified Merge Entity is not a language tag (IETF RFC 5646, Table 6.5.3.4-1).
    #[error("the language of the request data is not a language tag")]
    LanguageTag {
        /// Why.
        #[source]
        source: LanguageTagError,
    },
    /// A batch operation with no element, which clauses 5.6.7.4 to 5.6.10.4 and 5.6.20 answer with a
    /// `BadRequestData`.
    #[error("a batch operation needs at least one element")]
    EmptyBatch,
    /// The aggregated temporal representation without aggregation methods, which Tables 6.18.3.2-1 and 6.19.3.1-1
    /// require with `aggregatedValues`.
    #[error("an aggregated temporal representation needs aggregation methods")]
    AggregationWithoutMethods,
    /// A Query Temporal Evolution ordered by something other than `id`, the only Entity member Table 6.18.3.2-1
    /// and clause 5.7.4.3 allow to order a temporal query by, or ordered by distance.
    #[error("a temporal query orders entities by id only")]
    TemporalOrdering,
    /// A `lastN` of 0; Tables 6.18.3.2-1 and 6.19.3.1-1 make it a positive integer.
    #[error("lastN is a positive integer")]
    ZeroLastN,
    /// A Retrieve Temporal Evolution given a temporal query and, beside it, `lastN`, an aggregation or a
    /// `timeproperty`, which the temporal query carries itself (Table 5.2.21-1).
    #[error("lastN, the aggregation and the time property of a request with a temporal query belong to the temporal query")]
    InstanceOptionsBesideTemporalQuery,
    /// A Query Context Source Registrations names no entity type, attribute, `q` or geoquery, which clause
    /// 5.10.2.4 answers with `BadRequestData` (too wide a query); entity ids, a scope query or a temporal query alone
    /// do not count, and a registry operation is always local (clause 6.3.18), so locality does not count either.
    #[error("a query of context source registrations needs an entity type, attributes, a q or a geoquery")]
    UnboundedRegistrationQuery,
    /// A Query Context Source Registrations given a temporal query that carries `lastN` or an aggregation, which
    /// select instances of an attribute and which Table 6.8.3.2-1 does not list: a registration has no instances.
    #[error("a query of context source registrations takes no lastN or aggregation")]
    InstanceOptionsOnRegistrations,
}
