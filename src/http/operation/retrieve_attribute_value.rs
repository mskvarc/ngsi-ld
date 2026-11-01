//! Retrieve Attribute Value (ETSI TS 104 175 clause 10.4.4 and ETSI TS 104 176 clause 7.6.3.1).

use crate::{
    http::{
        Endpoint,
        QueryParameter,
        RequestError,
        ResponseContext,
        ResponseError,
        Retrieved,
        operation::{
            EntityMapRequest,
            Operation,
            sealed::{BuildsRequest, Sealed},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
    },
    model::{AttributeName, DatasetId, EntityId, NgsiLdMediaType, RawJson},
    query::{language_filter::LanguageFilter, type_selection::TypeSelection},
};
use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};

/// Reads the value of one attribute of an entity.
///
/// Its request is `GET /entities/{entityId}/attrs/{attrId}/value` (ETSI TS 104 176 clause 7.6.3.1), answered with a
/// value only Attribute Fragment: the attribute value itself, a JSON primitive, object or array (ETSI TS 104 175
/// clause 5.4.2).
///
/// The value is one instance's: the one `dataset_id` names, or else the default instance; the broker answers 404 when
/// the entity has no such instance of the attribute (ETSI TS 104 175 clause 10.4.4.4). A value only fragment carries
/// no `type`, so nothing in it says which kind of attribute it is the value of, and it is returned as [`RawJson`], as
/// the broker wrote it. It cannot embed an `@context` either: where one applies, the broker sends it in a `Link`
/// header (ETSI TS 104 175 clause 5.4.2).
///
/// Table 7.6.3.1-1 gives the operation no `join`, `joinLevel`, `containedBy` or `options`: neither the Linked Entities
/// nor the system attributes they ask for have a place in a value only fragment.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, RetrieveAttributeValue},
///     },
///     model::{AttributeName, DatasetId, EntityId},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
/// let speed: AttributeName = "speed".parse()?;
/// let gps: DatasetId = "urn:ngsi-ld:dataset:gps".parse()?;
/// let retrieve = RetrieveAttributeValue::builder()
///     .id(&id)
///     .name(&speed)
///     .dataset_id(&gps)
///     .build();
/// let request = retrieve.to_request(&endpoint)?;
/// assert_eq!(request.method(), "GET");
/// assert_eq!(
///     request.uri(),
///     concat!(
///         "http://broker.test/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567/attrs/speed/value",
///         "?datasetId=urn%3Angsi-ld%3Adataset%3Agps"
///     )
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, bon::Builder)]
pub struct RetrieveAttributeValue<'a> {
    /// The entity.
    pub id: &'a EntityId,
    /// The attribute.
    pub name: &'a AttributeName,
    /// Retrieves the value only if the entity's type matches (`type`, ETSI TS 104 175 clause 7.2.2).
    pub entity_type: Option<&'a TypeSelection>,
    /// The instance whose value is retrieved, `@none` naming the default instance (`datasetId`); the default instance
    /// when `None`.
    pub dataset_id: Option<&'a DatasetId>,
    /// The languages the value of a `LanguageProperty` is reduced to (`lang`): the value is then the string or the
    /// strings of the matching language.
    pub lang: Option<&'a LanguageFilter>,
    /// The entity map to use or to return (`entityMap`, `entityMapLifetime`, `NGSILD-EntityMap`).
    pub entity_map: Option<&'a EntityMapRequest>,
}

impl Sealed for RetrieveAttributeValue<'_> {}

impl Operation for RetrieveAttributeValue<'_> {
    type Outcome = Retrieved<RawJson>;

    /// Reads 200, or 203 when the broker amended the value to another version (Table 7.6.3.1-2).
    fn decode(&self, context: &ResponseContext, response: Response<Bytes>) -> Result<Retrieved<RawJson>, ResponseError> {
        Retrieved::decode(context, response, &[StatusCode::OK, StatusCode::NON_AUTHORITATIVE_INFORMATION], |success| {
            success.json().map(RawJson::from_json)
        })
    }
}

impl BuildsRequest for RetrieveAttributeValue<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        let mut query = QueryString::default();
        query.push_type_selection(QueryParameter::Type, self.entity_type);
        query.push_some(QueryParameter::DatasetId, self.dataset_id);
        query.push_language_filter(QueryParameter::Lang, self.lang);
        if let Some(entity_map) = self.entity_map {
            entity_map.push_query(&mut query);
        }
        RequestParts::resource(Method::GET, ResourcePath::attribute_value(self.id, self.name), query)
            .with_entity_map(self.entity_map.and_then(EntityMapRequest::header))
            .accepting(NgsiLdMediaType::Json)
            .into_request(endpoint, scope)
    }
}
