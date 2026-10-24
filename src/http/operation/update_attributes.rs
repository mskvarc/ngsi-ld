//! Update Attributes (NGSI-LD clauses 5.6.2 and 6.6.3.2).

use crate::{
    http::{
        Endpoint,
        QueryParameter,
        RequestError,
        ResponseContext,
        ResponseError,
        UpdateOutcome,
        operation::{
            Operation,
            sealed::{BuildsRequest, Sealed},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
        response::classify,
    },
    model::{Completeness, EntityFragment, EntityId, Lossless, Projected, representation::write::write_fragment},
    query::type_selection::TypeSelection,
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// Updates the attributes an entity already has.
///
/// Its request is `PATCH /entities/{entityId}/attrs` (NGSI-LD clause 6.6.3.2). Attributes the entity does not have are
/// reported in the [`UpdateResult`](crate::http::UpdateResult). The fragment may name its entity or not
/// ([`Completeness`]).
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, UpdateAttributes},
///     },
///     model::{EntityFragment, EntityId},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
/// let slower = EntityFragment::builder().property("speed", 30).build()?;
/// let update = UpdateAttributes::builder()
///     .id(&id)
///     .fragment(&slower)
///     .build();
/// let request = update.to_request(&endpoint)?;
/// assert_eq!(request.method(), "PATCH");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567/attrs"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, bon::Builder)]
pub struct UpdateAttributes<'a, C: Completeness = Projected> {
    /// The entity.
    pub id: &'a EntityId,
    /// The attributes, with deletions as the NGSI-LD Null (clause 5.5.8).
    pub fragment: &'a EntityFragment<C>,
    /// Updates only if the entity's type matches (clause 4.17).
    pub entity_type: Option<&'a TypeSelection>,
    /// The representation the body is written in.
    #[builder(default)]
    pub form: Lossless,
}

impl<C: Completeness> Sealed for UpdateAttributes<'_, C> {}

impl<C: Completeness> Operation for UpdateAttributes<'_, C> {
    type Outcome = UpdateOutcome;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<UpdateOutcome, ResponseError> {
        UpdateOutcome::decode(&classify(response)?)
    }
}

impl<C: Completeness> BuildsRequest for UpdateAttributes<'_, C> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        let mut query = QueryString::default();
        query.push_type_selection(QueryParameter::Type, self.entity_type);
        RequestParts::resource(Method::PATCH, ResourcePath::attributes(self.id), query)
            .with_body(write_fragment(self.fragment, self.form))
            .into_request(endpoint, scope)
    }
}
