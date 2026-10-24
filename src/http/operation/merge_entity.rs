//! Merge Entity (NGSI-LD clauses 5.6.17 and 6.5.3.4), with a fragment in normalized or concise representation or in
//! simplified representation (`format=simplified`).

use crate::{
    http::{
        BatchOutcome,
        Endpoint,
        QueryParameter,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            Operation,
            sealed::{BuildsRequest, Sealed},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
        response::classify,
    },
    model::{Completeness, DateTime, EntityFragment, EntityId, Lossless, Projected, Representation, SimplifiedFragment, representation::write::write_fragment},
    query::type_selection::TypeSelection,
};
use bytes::Bytes;
use http::{Method, Request, Response};
use langtag::LangTagBuf;

/// `PATCH /entities/{entityId}`: merges a fragment into an entity (NGSI-LD clause 6.5.3.4). Attributes the
/// fragment sets are merged, attributes it deletes are removed (clause 5.6.17). The fragment may name its entity
/// or not ([`Completeness`]).
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{MergeEntity, Operation},
///     },
///     model::{EntityFragment, EntityId},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
/// let stopped = EntityFragment::builder()
///     .property("speed", 0)
///     .delete("isParked")
///     .build()?;
/// let merge = MergeEntity::builder().id(&id).fragment(&stopped).build();
/// let request = merge.to_request(&endpoint)?;
/// assert_eq!(request.method(), "PATCH");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, bon::Builder)]
pub struct MergeEntity<'a, C: Completeness = Projected> {
    /// The entity.
    pub id: &'a EntityId,
    /// The changes.
    pub fragment: &'a EntityFragment<C>,
    /// Merges only if the entity's type matches (clause 4.17).
    pub entity_type: Option<&'a TypeSelection>,
    /// The `observedAt` the broker applies to merged attributes that had one, when the fragment gives none.
    pub observed_at: Option<&'a DateTime>,
    /// The representation the body is written in.
    #[builder(default)]
    pub form: Lossless,
}

impl<C: Completeness> Sealed for MergeEntity<'_, C> {}

impl<C: Completeness> Operation for MergeEntity<'_, C> {
    type Outcome = BatchOutcome;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<BatchOutcome, ResponseError> {
        BatchOutcome::decode(&classify(response)?)
    }
}

impl<C: Completeness> BuildsRequest for MergeEntity<'_, C> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        let query = merge_query(self.entity_type, self.observed_at);
        RequestParts::resource(Method::PATCH, ResourcePath::entity(self.id), query)
            .with_body(write_fragment(self.fragment, self.form))
            .into_request(endpoint, scope)
    }
}

/// `PATCH /entities/{entityId}?format=simplified`: merges a fragment in simplified representation into an entity
/// (NGSI-LD clause 6.5.3.4). Each attribute the fragment names is merged into the attribute the entity holds, whose
/// kind is kept (clause 5.6.17.4).
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{MergeSimplifiedEntity, Operation},
///     },
///     model::{EntityId, SimplifiedFragment},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
/// let faster = SimplifiedFragment::builder().value("speed", 55).build()?;
/// let merge = MergeSimplifiedEntity::builder()
///     .id(&id)
///     .fragment(&faster)
///     .build();
/// let request = merge.to_request(&endpoint)?;
/// assert_eq!(request.method(), "PATCH");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567?format=simplified"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, bon::Builder)]
pub struct MergeSimplifiedEntity<'a> {
    /// The entity.
    pub id: &'a EntityId,
    /// The changes.
    pub fragment: &'a SimplifiedFragment,
    /// Merges only if the entity's type matches (clause 4.17).
    pub entity_type: Option<&'a TypeSelection>,
    /// The `observedAt` the broker applies to merged attributes that had one, when the fragment gives none.
    pub observed_at: Option<&'a DateTime>,
    /// `lang`: the language of the strings the fragment holds. A string merged into a `LanguageProperty` replaces
    /// the text of this language only, keeping the rest of the language map (clause 5.6.17.4).
    pub lang: Option<&'a LangTagBuf>,
}

impl Sealed for MergeSimplifiedEntity<'_> {}

impl Operation for MergeSimplifiedEntity<'_> {
    type Outcome = BatchOutcome;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<BatchOutcome, ResponseError> {
        BatchOutcome::decode(&classify(response)?)
    }
}

impl BuildsRequest for MergeSimplifiedEntity<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        let mut query = merge_query(self.entity_type, self.observed_at);
        // `format` is the parameter Table 6.5.3.4-1 defines for this; `options=keyValues` is its deprecated synonym.
        query.push(QueryParameter::Format, Representation::Simplified);
        query.push_some(QueryParameter::Lang, self.lang);
        RequestParts::resource(Method::PATCH, ResourcePath::entity(self.id), query)
            .with_body(self.fragment.to_object())
            .into_request(endpoint, scope)
    }
}

/// The parameters every Merge Entity request takes, whatever the representation of its fragment: `type` and
/// `observedAt` (Table 6.5.3.4-1).
fn merge_query(entity_type: Option<&TypeSelection>, observed_at: Option<&DateTime>) -> QueryString {
    let mut query = QueryString::default();
    query.push_type_selection(QueryParameter::Type, entity_type);
    query.push_some(QueryParameter::ObservedAt, observed_at);
    query
}
