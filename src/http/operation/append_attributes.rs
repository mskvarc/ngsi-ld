//! Append Attributes (NGSI-LD clauses 5.6.3 and 6.6.3.1).

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
            Overwrite,
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

pub(super) mod parameters;

/// `POST /entities/{entityId}/attrs`: adds attributes to an entity, replacing those that exist unless told
/// otherwise (NGSI-LD clause 6.6.3.1). The fragment may name its entity or not ([`Completeness`]).
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{AppendAttributes, Operation, Overwrite},
///     },
///     model::{EntityFragment, EntityId},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
/// let color = EntityFragment::builder().property("color", "red").build()?;
/// let append = AppendAttributes::builder()
///     .id(&id)
///     .fragment(&color)
///     .overwrite(Overwrite::Keep)
///     .build();
/// let request = append.to_request(&endpoint)?;
/// assert_eq!(request.method(), "POST");
/// assert_eq!(
///     request.uri(),
///     concat!(
///         "http://broker.test/ngsi-ld/v1/entities",
///         "/urn:ngsi-ld:Vehicle:A4567/attrs?options=noOverwrite"
///     )
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, bon::Builder)]
pub struct AppendAttributes<'a, C: Completeness = Projected> {
    /// The entity.
    pub id: &'a EntityId,
    /// The attributes.
    pub fragment: &'a EntityFragment<C>,
    /// Whether existing attributes are replaced.
    #[builder(default)]
    pub overwrite: Overwrite,
    /// Appends only if the entity's type matches (clause 4.17).
    pub entity_type: Option<&'a TypeSelection>,
    /// The representation the body is written in.
    #[builder(default)]
    pub form: Lossless,
}

impl<C: Completeness> Sealed for AppendAttributes<'_, C> {}

impl<C: Completeness> Operation for AppendAttributes<'_, C> {
    type Outcome = UpdateOutcome;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<UpdateOutcome, ResponseError> {
        UpdateOutcome::decode(&classify(response)?)
    }
}

impl<C: Completeness> BuildsRequest for AppendAttributes<'_, C> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        let mut query = QueryString::default();
        self.overwrite.push_query(&mut query);
        query.push_type_selection(QueryParameter::Type, self.entity_type);
        RequestParts::resource(Method::POST, ResourcePath::attributes(self.id), query)
            .with_body(write_fragment(self.fragment, self.form))
            .into_request(endpoint, scope)
    }
}
