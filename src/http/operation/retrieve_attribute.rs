//! Retrieve Attribute (ETSI TS 104 175 clause 10.4.4 and ETSI TS 104 176 clause 7.5.3.1).

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
            InlineJoin,
            Operation,
            SystemAttributes,
            linked_entities::LinkedEntities,
            sealed::{BuildsRequest, Sealed},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
    },
    model::{Attribute, AttributeDocument, AttributeName, DatasetId, EntityId, Lossless, NgsiLdMediaType, Output},
    query::{Join, language_filter::LanguageFilter, type_selection::TypeSelection},
};
use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use std::num::NonZeroU32;

/// Reads one attribute of an entity.
///
/// Its request is `GET /entities/{entityId}/attrs/{attrId}` (ETSI TS 104 176 clause 7.5.3.1), answered with the
/// attribute as an Attribute Fragment (ETSI TS 104 175 clause 10.4.4.5).
///
/// The answer is one instance: the one `dataset_id` names, or else the default instance; the broker answers 404 when
/// the entity has no such instance of the attribute (ETSI TS 104 175 clause 10.4.4.4). Table 7.5.3.1-1 gives the
/// operation no `format`, which ETSI TS 104 176 clause 6.4.2 defines for Retrieve Entity and Query Entities only, so the
/// attribute is asked for and read in normalized representation. An answer in `application/ld+json`, which a client
/// whose endpoint asks for it receives, carries its `@context`, which the [`AttributeDocument`] keeps.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, RetrieveAttribute, SystemAttributes},
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
/// let retrieve = RetrieveAttribute::builder()
///     .id(&id)
///     .name(&speed)
///     .dataset_id(&gps)
///     .system_attributes(SystemAttributes::Include)
///     .build();
/// let request = retrieve.to_request(&endpoint)?;
/// assert_eq!(request.method(), "GET");
/// assert_eq!(
///     request.uri(),
///     concat!(
///         "http://broker.test/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567/attrs/speed",
///         "?datasetId=urn%3Angsi-ld%3Adataset%3Agps&options=sysAttrs"
///     )
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, bon::Builder)]
pub struct RetrieveAttribute<'a> {
    /// The entity.
    pub id: &'a EntityId,
    /// The attribute.
    pub name: &'a AttributeName,
    /// Retrieves the attribute only if the entity's type matches (`type`, ETSI TS 104 175 clause 7.2.2).
    pub entity_type: Option<&'a TypeSelection>,
    /// The instance to retrieve, `@none` naming the default instance (`datasetId`); the default instance when
    /// `None`.
    pub dataset_id: Option<&'a DatasetId>,
    /// The languages a `LanguageProperty` is reduced to (`lang`); the answer is then a `Property` (ETSI TS 104 175
    /// clause 10.4.4.5).
    pub lang: Option<&'a LanguageFilter>,
    /// The linked entity retrieval (`join`): `inline` adds the linked entities to a `Relationship` or a
    /// `ListRelationship` (ETSI TS 104 175 clause 10.4.4.5).
    pub join: Option<InlineJoin>,
    /// The depth of linked entity retrieval (`joinLevel`); only with an `inline` join.
    pub join_level: Option<NonZeroU32>,
    /// The linked entities already encountered while traversing the entity graph (`containedBy`, ETSI TS 104 175
    /// clause 10.4.4.3); a broker only applies it with a `joinLevel`.
    #[builder(default)]
    pub contained_by: &'a [EntityId],
    /// The entity map to use or to return (`entityMap`, `entityMapLifetime`, `NGSILD-EntityMap`).
    pub entity_map: Option<&'a EntityMapRequest>,
    /// Whether system attributes are included (`options=sysAttrs`, ETSI TS 104 176 clause 6.4.5).
    #[builder(default)]
    pub system_attributes: SystemAttributes,
}

impl Sealed for RetrieveAttribute<'_> {}

impl Operation for RetrieveAttribute<'_> {
    type Outcome = Retrieved<AttributeDocument<Attribute<Output>>>;

    /// Reads 200, or 203 when the broker amended the attribute to another version (Table 7.5.3.1-3).
    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<Retrieved<AttributeDocument<Attribute<Output>>>, ResponseError> {
        Retrieved::decode(response, &[StatusCode::OK, StatusCode::NON_AUTHORITATIVE_INFORMATION], |success| {
            AttributeDocument::read(success.json()?, Lossless::Normalized).map_err(|source| success.body_error(source))
        })
    }
}

impl BuildsRequest for RetrieveAttribute<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        let linked = LinkedEntities {
            join: self.join.map(Join::from),
            level: self.join_level,
            contained_by: self.contained_by,
        };
        let mut query = QueryString::default();
        query.push_type_selection(QueryParameter::Type, self.entity_type);
        query.push_some(QueryParameter::DatasetId, self.dataset_id);
        query.push_language_filter(QueryParameter::Lang, self.lang);
        linked.push_query(&mut query);
        if let Some(entity_map) = self.entity_map {
            entity_map.push_query(&mut query);
        }
        self.system_attributes.push_query(&mut query);
        RequestParts::resource(Method::GET, ResourcePath::attribute(self.id, self.name), query)
            .with_entity_map(self.entity_map.and_then(EntityMapRequest::header))
            .accepting(NgsiLdMediaType::Json)
            .into_request(endpoint, scope)
    }
}
