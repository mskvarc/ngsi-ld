//! The entity map operations (NGSI-LD clause 5.14), as builders that finish with `send()`.

use crate::{
    client::{
        Answering,
        Client,
        ClientError,
        OutcomeOnly,
        Transport,
        arguments::{SelectionBuilders, build, selectors_built},
    },
    http::{
        Retrieved,
        operation::{CreateEntityMap, CreateTemporalEntityMap, DeleteEntityMap, EntityMapLifetime, QueryMethod, RetrieveEntityMap, UpdateEntityMap},
    },
    model::{AttributeName, DatasetId, DateTime, EntityId, Tenant, evolution::TemporalAttributeName},
    query::{
        EntitySelectorDraft,
        EntitySplitting,
        IdPattern,
        Join,
        Locality,
        builders::SelectorTypeBuilder,
        geoquery::builders::GeoQueryBuilder,
        language_filter::builders::LanguageFilterBuilder,
        projection::builders::ProjectionBuilder,
        q::builders::QueryBuilder,
        scope_query::builders::ScopeQueryBuilder,
        temporal_query::{TemporalQuery, builders::TemporalQueryBuilder},
    },
    registration::{EntityMap, EntityMapId},
};
use bon::bon;
use std::num::NonZeroU32;

/// The entity map operations of a [`Client`], from [`Client::entity_maps`].
///
/// Each operation is a builder that finishes with `send()`.
///
/// ```
/// # use bytes::Bytes;
/// # use headers::Header;
/// # use http::{HeaderValue, Method, Request, Response, StatusCode};
/// # use ngsi_ld::http::headers::NgsiLdEntityMap;
/// # use std::convert::Infallible;
/// # struct Offline;
/// # impl ngsi_ld::client::Transport for Offline {
/// #     type Error = Infallible;
/// #     async fn send(
/// #         &self,
/// #         request: Request<Bytes>,
/// #     ) -> Result<Response<Bytes>, Infallible> {
/// #         let map = concat!(
/// #             r#"{"id":"urn:ngsi-ld:EntityMap:1","type":"EntityMap","#,
/// #             r#""expiresAt":"2030-01-01T00:00:00Z","#,
/// #             r#""entityMap":{"urn:ngsi-ld:Vehicle:A4567":["@none"]},"#,
/// #             r#""linkedMaps":{}}"#
/// #         )
/// #         .as_bytes();
/// #         let mut response = Response::new(Bytes::from_static(map));
/// #         if request.method() == Method::GET
/// #             && request.uri().path().ends_with("/entityMaps")
/// #         {
/// #             *response.status_mut() = StatusCode::CREATED;
/// #             response.headers_mut().insert(
/// #                 NgsiLdEntityMap::name(),
/// #                 HeaderValue::from_static(
/// #                     "/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1",
/// #                 ),
/// #             );
/// #         } else if request.method() != Method::GET {
/// #             *response.status_mut() = StatusCode::NO_CONTENT;
/// #             *response.body_mut() = Bytes::new();
/// #         }
/// #         Ok(response)
/// #     }
/// # }
/// use ngsi_ld::{client::Client, model::DateTime};
///
/// async fn pin_the_vehicles(client: &Client<Offline>) -> anyhow::Result<usize> {
///     let created = client
///         .entity_maps()
///         .create()
///         .entity_type("Vehicle")
///         .send()
///         .await?;
///     let id = &created.body().id;
///     let later: DateTime = "2030-06-01T00:00:00Z".parse()?;
///     client.entity_maps().update(id, &later).send().await?;
///     let map = client.entity_maps().retrieve(id).send().await?;
///     client.entity_maps().delete(id).send().await?;
///     Ok(map.entity_map.len())
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct EntityMaps<'c, T, M = OutcomeOnly> {
    /// The client the calls go through.
    client: &'c Client<T, M>,
}

impl<'c, T, M> EntityMaps<'c, T, M> {
    /// The handle of `client`.
    pub(crate) const fn new(client: &'c Client<T, M>) -> Self {
        Self { client }
    }
}

#[bon]
impl<T: Transport, M: Answering> EntityMaps<'_, T, M> {
    /// Retrieve `EntityMap` (clause 6.32.3.1).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the map does not exist (`ResourceNotFound`), the answer is not an entity map,
    /// or the call fails.
    #[builder(finish_fn = send)]
    pub async fn retrieve(&self, #[builder(start_fn)] id: &EntityMapId, tenant: Option<&Tenant>) -> Result<M::Answer<EntityMap>, ClientError<T::Error>> {
        self.client.send_on_tenant(&RetrieveEntityMap { id }, tenant).await
    }

    /// Update `EntityMap` (clause 6.32.3.2): moves the map's expiry to `expires_at`.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the map does not exist, the broker refuses the expiry, or the call fails.
    #[builder(finish_fn = send)]
    pub async fn update(
        &self,
        #[builder(start_fn)] id: &EntityMapId,
        #[builder(start_fn)] expires_at: &DateTime,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<()>, ClientError<T::Error>> {
        self.client.send_on_tenant(&UpdateEntityMap { id, expires_at }, tenant).await
    }

    /// Delete `EntityMap` (clause 6.32.3.3).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the map does not exist or the call fails.
    #[builder(finish_fn = send)]
    pub async fn delete(&self, #[builder(start_fn)] id: &EntityMapId, tenant: Option<&Tenant>) -> Result<M::Answer<()>, ClientError<T::Error>> {
        self.client.send_on_tenant(&DeleteEntityMap { id }, tenant).await
    }

    /// Create `EntityMap` for Query Entities (clauses 6.34.3.1 and 6.34.3.2): the map, with its location in
    /// [`Retrieved::entity_map`].
    ///
    /// At least an entity type, attributes, a `q` or a geoquery is needed unless the creation is local (clause
    /// 5.14.4.4); `send()` refuses the request otherwise. The linked entity retrieval is sent as given, as for Query
    /// Entities: whether a projection or a query reaches deeper than it is the broker's to check (clause 5.14.4.4).
    /// Further `selectors` (clause 5.2.33) are matched each on its own beside the type, ids and id pattern; only the
    /// POST binding carries more than one selection, so `send()` refuses several over GET.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when an argument is invalid, the request is refused before sending, the answer is
    /// not an entity map, or the call fails.
    #[builder(finish_fn = send)]
    pub async fn create(
        &self,
        #[builder(into)] entity_type: Option<SelectorTypeBuilder>,
        #[builder(default)] ids: Vec<EntityId>,
        #[builder(into)] id_pattern: Option<IdPattern>,
        #[builder(default, with = |selectors: impl IntoIterator<Item = impl Into<EntitySelectorDraft>>| selectors.into_iter().map(Into::into).collect())]
        selectors: Vec<EntitySelectorDraft>,
        #[builder(default)] attrs: Vec<AttributeName>,
        #[builder(into)] q: Option<QueryBuilder>,
        #[builder(into)] geo_q: Option<GeoQueryBuilder>,
        #[builder(into)] scope_q: Option<ScopeQueryBuilder>,
        #[builder(into)] csf: Option<QueryBuilder>,
        #[builder(into)] pick: Option<ProjectionBuilder>,
        #[builder(into)] omit: Option<ProjectionBuilder>,
        #[builder(default)] expand_values: Vec<AttributeName>,
        #[builder(default)] json_keys: Vec<AttributeName>,
        #[builder(default)] dataset_ids: Vec<DatasetId>,
        #[builder(into)] lang: Option<LanguageFilterBuilder>,
        join: Option<Join>,
        join_level: Option<NonZeroU32>,
        #[builder(default)] contained_by: Vec<EntityId>,
        lifetime: Option<EntityMapLifetime>,
        #[builder(default)] split_entities: EntitySplitting,
        #[builder(default)] locality: Locality,
        #[builder(default)] method: QueryMethod,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<Retrieved<EntityMap>>, ClientError<T::Error>> {
        let selection = SelectionBuilders {
            entity_type,
            q,
            geo_q,
            scope_q,
            csf,
            pick,
            omit,
            lang,
        }
        .build()?;
        let selectors = selectors_built(selectors)?;
        let create = CreateEntityMap::builder()
            .maybe_entity_type(selection.entity_type.as_ref())
            .ids(&ids)
            .maybe_id_pattern(id_pattern.as_ref())
            .selectors(&selectors)
            .attrs(&attrs)
            .maybe_q(selection.q.as_ref())
            .maybe_geo_q(selection.geo_q.as_ref())
            .maybe_scope_q(selection.scope_q.as_ref())
            .maybe_csf(selection.csf.as_ref())
            .maybe_pick(selection.pick.as_ref())
            .maybe_omit(selection.omit.as_ref())
            .expand_values(&expand_values)
            .json_keys(&json_keys)
            .dataset_ids(&dataset_ids)
            .maybe_lang(selection.lang.as_ref())
            .maybe_join(join)
            .maybe_join_level(join_level)
            .contained_by(&contained_by)
            .maybe_lifetime(lifetime)
            .split_entities(split_entities)
            .locality(locality)
            .method(method)
            .build();
        self.client.send_on_tenant(&create, tenant).await
    }

    /// Create `EntityMap` for Query Temporal Evolution of Entities (clauses 6.35.3.1 and 6.35.3.2): the map of the
    /// entities whose temporal evolution `temporal_q` matches, with its location in [`Retrieved::entity_map`].
    ///
    /// At least an entity type, attributes, a `q` or a geoquery is needed unless the creation is local (clause
    /// 5.14.5.4); `send()` refuses the request otherwise.
    /// Further `selectors` (clause 5.2.33) are matched each on its own beside the type, ids and id pattern; only the
    /// POST binding carries more than one selection, so `send()` refuses several over GET.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when an argument is invalid, the request is refused before sending, the answer is
    /// not an entity map, or the call fails.
    #[builder(finish_fn = send)]
    pub async fn create_temporal(
        &self,
        #[builder(start_fn, into)] temporal_q: TemporalQueryBuilder,
        #[builder(into)] entity_type: Option<SelectorTypeBuilder>,
        #[builder(default)] ids: Vec<EntityId>,
        #[builder(into)] id_pattern: Option<IdPattern>,
        #[builder(default, with = |selectors: impl IntoIterator<Item = impl Into<EntitySelectorDraft>>| selectors.into_iter().map(Into::into).collect())]
        selectors: Vec<EntitySelectorDraft>,
        #[builder(default)] attrs: Vec<TemporalAttributeName>,
        #[builder(into)] q: Option<QueryBuilder>,
        #[builder(into)] geo_q: Option<GeoQueryBuilder>,
        #[builder(into)] scope_q: Option<ScopeQueryBuilder>,
        #[builder(into)] csf: Option<QueryBuilder>,
        #[builder(into)] pick: Option<ProjectionBuilder>,
        #[builder(into)] omit: Option<ProjectionBuilder>,
        #[builder(default)] expand_values: Vec<AttributeName>,
        #[builder(default)] json_keys: Vec<AttributeName>,
        #[builder(default)] dataset_ids: Vec<DatasetId>,
        #[builder(into)] lang: Option<LanguageFilterBuilder>,
        lifetime: Option<EntityMapLifetime>,
        #[builder(default)] split_entities: EntitySplitting,
        #[builder(default)] locality: Locality,
        #[builder(default)] method: QueryMethod,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<Retrieved<EntityMap>>, ClientError<T::Error>> {
        let temporal_q: TemporalQuery = build(temporal_q)?;
        let selection = SelectionBuilders {
            entity_type,
            q,
            geo_q,
            scope_q,
            csf,
            pick,
            omit,
            lang,
        }
        .build()?;
        let selectors = selectors_built(selectors)?;
        let create = CreateTemporalEntityMap::builder()
            .maybe_entity_type(selection.entity_type.as_ref())
            .ids(&ids)
            .maybe_id_pattern(id_pattern.as_ref())
            .selectors(&selectors)
            .attrs(&attrs)
            .maybe_q(selection.q.as_ref())
            .maybe_geo_q(selection.geo_q.as_ref())
            .maybe_scope_q(selection.scope_q.as_ref())
            .maybe_csf(selection.csf.as_ref())
            .maybe_pick(selection.pick.as_ref())
            .maybe_omit(selection.omit.as_ref())
            .expand_values(&expand_values)
            .json_keys(&json_keys)
            .dataset_ids(&dataset_ids)
            .maybe_lang(selection.lang.as_ref())
            .temporal_q(&temporal_q)
            .maybe_lifetime(lifetime)
            .split_entities(split_entities)
            .locality(locality)
            .method(method)
            .build();
        self.client.send_on_tenant(&create, tenant).await
    }
}
