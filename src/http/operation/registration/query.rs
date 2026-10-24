//! Query Context Source Registrations (NGSI-LD clauses 5.10.2 and 6.8.3.2), and the requests for its following
//! pages.

use crate::{
    http::{
        Endpoint,
        Page,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            ArgumentError,
            Operation,
            ResultsCount,
            SystemAttributes,
            entity_selection::{EntitySelection, targets::EntityTargets},
            parameters::{check_limit, push_paging},
            registration::READER,
            sealed::{BuildsRequest, Sealed},
            temporal::push_time_bound,
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
    },
    model::{AttributeName, EntityId, NgsiLdMediaType, Output},
    query::{
        IdPattern,
        geoquery::GeoQuery,
        language_filter::LanguageFilter,
        q::Query,
        scope_query::ScopeQuery,
        temporal_query::TemporalQuery,
        type_selection::TypeSelection,
    },
    registration::CSourceRegistration,
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// `GET /csourceRegistrations`: finds the registrations of the context sources that may provide the entities and
/// attributes described, one page at a time (NGSI-LD clauses 5.10.2 and 6.8.3.2).
///
/// The parameters are those of Table 6.8.3.2-1, written by the same code as the entity selection of Query
/// Entities, followed by the deprecated `attrs`, which names the attributes a registration is matched against
/// (clause 5.10.2.4), the time bound of the temporal query, matched against the registration's
/// `observationInterval` or `managementInterval`, and the paging of clause 5.5.9 (`limit`, `offset`, `count`) and
/// `options=sysAttrs` (clause 6.3.11). `send()` refuses, before sending, a query with no type, attribute, `q` or
/// geoquery ([`ArgumentError::UnboundedRegistrationQuery`]), a temporal query carrying `lastN` or an aggregation
/// ([`ArgumentError::InstanceOptionsOnRegistrations`]), and `limit=0` without `count`.
///
/// `pick`, `omit` and `geometryProperty`, which the table also lists, are not sent: they name the members of an
/// entity and the geometry of its `GeoJSON` representation (clauses 4.21 and 6.3.15), and a registration is neither.
///
/// ```
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, QueryRegistrations},
///     },
///     query::type_selection::TypeSelection,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let vehicles: TypeSelection = "Vehicle".parse()?;
/// let query = QueryRegistrations::builder()
///     .entity_type(&vehicles)
///     .limit(10)
///     .build();
/// let request = query.to_request(&endpoint)?;
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/csourceRegistrations?type=Vehicle&limit=10"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Default, bon::Builder)]
pub struct QueryRegistrations<'a> {
    /// The entity types (`type`, clause 4.17).
    pub entity_type: Option<&'a TypeSelection>,
    /// The entity ids (`id`).
    #[builder(default)]
    pub ids: &'a [EntityId],
    /// The pattern of the entity ids (`idPattern`).
    pub id_pattern: Option<&'a IdPattern>,
    /// The attributes (`attrs`).
    #[builder(default)]
    pub attrs: &'a [AttributeName],
    /// The query on the entities (`q`, clause 4.9), whose attributes a registration is matched against.
    pub q: Option<&'a Query>,
    /// The context source filter (`csf`, clause 4.9), matched against the registration's own members.
    pub csf: Option<&'a Query>,
    /// The geoquery (clause 4.10), matched against the registration's `GeoProperty`.
    pub geo_q: Option<&'a GeoQuery>,
    /// The temporal query (clause 4.11): without one, only the registrations without a time interval match.
    pub temporal_q: Option<&'a TemporalQuery>,
    /// The scope query (`scopeQ`, clause 4.19).
    pub scope_q: Option<&'a ScopeQuery>,
    /// The language filter (`lang`, clause 4.15).
    pub lang: Option<&'a LanguageFilter>,
    /// Whether `createdAt` and `modifiedAt` are included (clause 6.3.11).
    #[builder(default)]
    pub system_attributes: SystemAttributes,
    /// The page size (clause 6.3.10).
    pub limit: Option<u32>,
    /// The index of the first registration of the page (clause 6.3.10).
    pub offset: Option<u32>,
    /// Whether the total is reported (clause 6.3.13).
    #[builder(default)]
    pub count: ResultsCount,
}

impl Sealed for QueryRegistrations<'_> {}

impl QueryRegistrations<'_> {
    /// The parameters a query of entities shares, from `type` to `lang`; the others are not registration
    /// parameters.
    const fn selection(&self) -> EntitySelection<'_> {
        EntitySelection {
            attrs: self.attrs,
            q: self.q,
            geo_q: self.geo_q,
            scope_q: self.scope_q,
            csf: self.csf,
            lang: self.lang,
            ..EntitySelection::of(EntityTargets::parameters(self.entity_type, self.ids, self.id_pattern))
        }
    }

    /// Refuses what clauses 5.10.2.4 and 6.3.13 make a `BadRequest`, and a temporal query with options a
    /// registration cannot answer.
    fn check(&self) -> Result<(), ArgumentError> {
        check_limit(self.limit, self.count)?;
        if self.entity_type.is_none() && self.attrs.is_empty() && self.q.is_none() && self.geo_q.is_none() {
            return Err(ArgumentError::UnboundedRegistrationQuery);
        }
        match self.temporal_q {
            Some(temporal) if temporal.last_n().is_some() || temporal.aggregation_methods().is_some() || temporal.aggregation_period().is_some() => {
                Err(ArgumentError::InstanceOptionsOnRegistrations)
            }
            Some(_) | None => Ok(()),
        }
    }
}

impl Operation for QueryRegistrations<'_> {
    type Outcome = Page<CSourceRegistration<Output>>;

    fn decode(&self, context: &ResponseContext, response: Response<Bytes>) -> Result<Page<CSourceRegistration<Output>>, ResponseError> {
        READER.decode_page(context, response)
    }
}

impl BuildsRequest for QueryRegistrations<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        self.check()?;
        let mut query = QueryString::default();
        self.selection().push_query(&mut query);
        if let Some(temporal) = self.temporal_q {
            push_time_bound(&mut query, temporal);
        }
        self.system_attributes.push_query(&mut query);
        push_paging(&mut query, self.limit, self.offset, self.count);
        RequestParts::resource(Method::GET, ResourcePath::registrations(), query)
            .accepting(NgsiLdMediaType::Json)
            .into_request(endpoint, scope)
    }
}

page_operation!(
    impl for CSourceRegistration<Output> => |context, response| READER.decode_page(context, response),
    NgsiLdMediaType::Json
);
