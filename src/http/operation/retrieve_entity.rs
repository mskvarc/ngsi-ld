//! Retrieve Entity (NGSI-LD clauses 5.7.1 and 6.5.3.1).

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
            Flattened,
            GeoJson,
            InlineJoin,
            Operation,
            RepresentationSpelling,
            Simplified,
            SystemAttributes,
            entity_format::{EntityFormat, read_many, read_one},
            linked_entities::LinkedEntities,
            projection_rule::ProjectionRequest,
            sealed::{BuildsRequest, Sealed},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
        response::Success,
    },
    model::{AttributeName, Complete, Completeness, DatasetId, Entity, EntityId, Feature, Lossless, Output, Projected, SimplifiedEntity},
    query::{Join, language_filter::LanguageFilter, projection::Projection, type_selection::TypeSelection},
};
use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use std::num::NonZeroU32;

pub(super) mod parameters;

/// `GET /entities/{entityId}`: reads one entity (NGSI-LD clause 6.5.3.1).
///
/// `F` is the representation asked for: a [`Lossless`] one answers with an [`Entity`], [`Simplified`] with a
/// [`SimplifiedEntity`], [`GeoJson`] with a [`Feature`] (clause 6.3.15). `J` is the join: an optional
/// [`InlineJoin`] answers with one entity, [`Flattened`] with the entity followed by the entities its relationships
/// target (clause 4.5.23.3). `C` is what the entities hold of their `id` and `type`: [`Complete`] entities, so
/// `send()` refuses a `pick` or `omit` that removes either, or [`Projected`] ones, which may lack them (clause
/// 4.21). A Feature always holds both (Table 5.2.29-1).
///
/// A flattened join answers with an array of entities (clause 4.5.23.3) and the `GeoJSON` representation with a
/// single Feature (Table 6.5.3.1-3), so the two are not combined.
///
/// [`RetrieveEntity::builder`] starts the common request, for one complete entity; the other start functions name
/// the join and the completeness: [`projected_builder`](RetrieveEntity::projected_builder),
/// [`flattened_builder`](RetrieveEntity::flattened_builder) and
/// [`flattened_projected_builder`](RetrieveEntity::flattened_projected_builder).
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{http::{Endpoint, operation::{Operation, SystemAttributes, RetrieveEntity}}, model::{EntityId, Lossless}};
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
/// let retrieve = RetrieveEntity::builder()
///     .id(&id)
///     .system_attributes(SystemAttributes::Include)
///     .format(Lossless::Concise)
///     .build();
/// let request = retrieve.to_request(&endpoint)?;
/// assert_eq!(request.method(), "GET");
/// assert_eq!(request.uri(), "http://broker.test/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567?format=concise&options=sysAttrs");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, bon::Builder)]
#[builder(start_fn(name = generic_builder, vis = ""))]
pub struct RetrieveEntity<'a, F, J: Default = Option<InlineJoin>, C = Complete> {
    /// What the entities hold of their `id` and `type`, which the start function names.
    #[builder(start_fn)]
    pub completeness: C,
    /// The entity.
    pub id: &'a EntityId,
    /// Retrieves the entity only if its type matches (clause 4.17).
    pub entity_type: Option<&'a TypeSelection>,
    /// The attributes to keep (`attrs`): deprecated by the specification, a synonym of a `pick` of these attributes
    /// (Table 6.5.3.1-1), which also answers 404 when the entity has none of them; prefer `pick`. Empty sends no
    /// `attrs`.
    #[builder(default)]
    pub attrs: &'a [AttributeName],
    /// The members to keep (`pick`, clause 4.21).
    pub pick: Option<&'a Projection>,
    /// The members to remove (`omit`, clause 4.21).
    pub omit: Option<&'a Projection>,
    /// The attribute instances to keep, by dataset id, `@none` naming the default instance (`datasetId`, clause
    /// 4.5.5); empty keeps every instance.
    #[builder(default)]
    pub dataset_ids: &'a [DatasetId],
    /// The languages `LanguageProperty` attributes are reduced to (`lang`, clause 4.15).
    pub lang: Option<&'a LanguageFilter>,
    /// The linked entity retrieval (`join`, clause 4.5.23); none by default.
    #[builder(default)]
    pub join: J,
    /// The depth of linked entity retrieval (`joinLevel`); only with a `flat` or `inline` join.
    pub join_level: Option<NonZeroU32>,
    /// The linked entities already encountered while traversing the entity graph (`containedBy`); a broker only
    /// applies it with a `joinLevel`.
    #[builder(default)]
    pub contained_by: &'a [EntityId],
    /// The entity map to use or to return (`entityMap`, `entityMapLifetime`, `NGSILD-EntityMap`, clause 4.5.25).
    pub entity_map: Option<&'a EntityMapRequest>,
    /// Whether system attributes are included (clause 6.3.11).
    #[builder(default)]
    pub system_attributes: SystemAttributes,
    /// How the representation is written (clause 6.3.7).
    #[builder(default)]
    pub spelling: RepresentationSpelling,
    /// The representation asked for.
    pub format: F,
}

impl<'a, F> RetrieveEntity<'a, F> {
    /// Starts the builder of a request for one complete entity, with an optional inline join.
    pub fn builder() -> RetrieveEntityBuilder<'a, F> {
        Self::generic_builder(Complete)
    }
}

impl<'a, F> RetrieveEntity<'a, F, Option<InlineJoin>, Projected> {
    /// Starts the builder of a request for one entity a `pick` or `omit` may leave without its `id` or `type`.
    pub fn projected_builder() -> RetrieveEntityBuilder<'a, F, Option<InlineJoin>, Projected> {
        Self::generic_builder(Projected)
    }
}

impl<'a, F> RetrieveEntity<'a, F, Flattened> {
    /// Starts the builder of a request with a flattened join (`join=flat`), answered with the complete entity
    /// followed by the entities its relationships target.
    pub fn flattened_builder() -> RetrieveEntityBuilder<'a, F, Flattened> {
        Self::generic_builder(Complete)
    }
}

impl<'a, F> RetrieveEntity<'a, F, Flattened, Projected> {
    /// Starts the builder of a request with a flattened join (`join=flat`), answered with entities a `pick` or
    /// `omit` may leave without their `id` or `type`.
    pub fn flattened_projected_builder() -> RetrieveEntityBuilder<'a, F, Flattened, Projected> {
        Self::generic_builder(Projected)
    }
}

/// The join a Retrieve Entity request asks for, which decides whether it answers with one entity or several.
pub(crate) trait RetrieveJoin: Copy + Default {
    /// The `join` parameter.
    fn parameter(self) -> Option<Join>;
}

impl RetrieveJoin for Option<InlineJoin> {
    fn parameter(self) -> Option<Join> {
        self.map(Join::from)
    }
}

impl RetrieveJoin for Flattened {
    fn parameter(self) -> Option<Join> {
        Some(Join::Flat)
    }
}

impl<F, J: Default, C> Sealed for RetrieveEntity<'_, F, J, C> {}

/// Implements [`Operation`] for one representation, one join and one completeness, reading the body with `read`.
macro_rules! retrieve_operation {
    ($format:ty, $join:ty, $completeness:ty, $body:ty, $read:ident) => {
        impl Operation for RetrieveEntity<'_, $format, $join, $completeness> {
            type Outcome = Retrieved<$body>;

            fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<Retrieved<$body>, ResponseError> {
                decode(response, |success| $read::<$completeness, _>(success, self.format))
            }
        }

        impl BuildsRequest for RetrieveEntity<'_, $format, $join, $completeness> {
            fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
                parts(self)?.into_request(endpoint, scope)
            }
        }
    };
}

retrieve_operation!(Lossless, Option<InlineJoin>, Complete, Entity<Output>, read_one);
retrieve_operation!(Simplified, Option<InlineJoin>, Complete, SimplifiedEntity, read_one);
retrieve_operation!(Lossless, Flattened, Complete, Vec<Entity<Output>>, read_many);
retrieve_operation!(Simplified, Flattened, Complete, Vec<SimplifiedEntity>, read_many);
retrieve_operation!(Lossless, Option<InlineJoin>, Projected, Entity<Output, Projected>, read_one);
retrieve_operation!(Simplified, Option<InlineJoin>, Projected, SimplifiedEntity<Projected>, read_one);
retrieve_operation!(Lossless, Flattened, Projected, Vec<Entity<Output, Projected>>, read_many);
retrieve_operation!(Simplified, Flattened, Projected, Vec<SimplifiedEntity<Projected>>, read_many);
retrieve_operation!(GeoJson<'_, Lossless>, Option<InlineJoin>, Complete, Feature<Entity<Output>>, read_one);
retrieve_operation!(GeoJson<'_, Simplified>, Option<InlineJoin>, Complete, Feature<SimplifiedEntity>, read_one);

/// The request, whichever the format, the join and the completeness, after the checks of Table 6.5.3.1-1 and the
/// check that the answer can be read in the format asked for.
fn parts<C: Completeness, F: EntityFormat<C>, J: RetrieveJoin>(retrieve: &RetrieveEntity<'_, F, J, C>) -> Result<RequestParts, RequestError> {
    let linked = LinkedEntities {
        join: retrieve.join.parameter(),
        level: retrieve.join_level,
        contained_by: retrieve.contained_by,
    };
    retrieve.format.check_projection(ProjectionRequest {
        pick: retrieve.pick,
        omit: retrieve.omit,
        join: retrieve.join.parameter(),
    })?;
    let mut query = QueryString::default();
    query.push_type_selection(QueryParameter::Type, retrieve.entity_type);
    query.push_list(QueryParameter::Attrs, retrieve.attrs);
    query.push_projection(QueryParameter::Pick, retrieve.pick);
    query.push_projection(QueryParameter::Omit, retrieve.omit);
    query.push_list(QueryParameter::DatasetId, retrieve.dataset_ids);
    query.push_language_filter(QueryParameter::Lang, retrieve.lang);
    linked.push_query(&mut query);
    if let Some(entity_map) = retrieve.entity_map {
        entity_map.push_query(&mut query);
    }
    retrieve.format.push_query(&mut query, retrieve.spelling);
    retrieve
        .spelling
        .push_options(&mut query, retrieve.format.representation(), retrieve.system_attributes);
    Ok(RequestParts::resource(Method::GET, ResourcePath::entity(retrieve.id), query)
        .with_entity_map(retrieve.entity_map.and_then(EntityMapRequest::header))
        .accepting(retrieve.format.media_type()))
}

/// Reads 200, or 203 when the broker amended the entity to another version (clause 4.3.6.8), with `read`.
fn decode<T>(response: Response<Bytes>, read: impl FnOnce(&Success) -> Result<T, ResponseError>) -> Result<Retrieved<T>, ResponseError> {
    Retrieved::decode(response, &[StatusCode::OK, StatusCode::NON_AUTHORITATIVE_INFORMATION], read)
}
