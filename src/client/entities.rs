//! The operations on entities and their attributes (NGSI-LD clauses 5.6 and 5.7), as builders that finish with
//! `send()`.

use crate::{
    client::{
        Answering,
        ClientError,
        OutcomeOnly,
        Transport,
        arguments::{SelectionBuilders, built},
        route::Route,
    },
    http::{
        BatchOutcome,
        CreateOutcome,
        UpdateOutcome,
        headers::NgsiLdEntityMap,
        operation::{
            AppendAttributes,
            CreateEntity,
            DeleteAttribute,
            DeleteEntity,
            InstanceSelection,
            Overwrite,
            PartialAttributeUpdate,
            PurgeEntities,
            ReplaceAttribute,
            ReplaceEntity,
            UpdateAttributes,
        },
    },
    model::{Attribute, AttributeName, Change, Completeness, Entity, EntityFragment, EntityId, Input, InstanceDeletion, Lossless, Patch, Tenant},
    query::{
        IdPattern,
        Locality,
        builders::SelectorTypeBuilder,
        geoquery::builders::GeoQueryBuilder,
        q::builders::QueryBuilder,
        scope_query::builders::ScopeQueryBuilder,
        type_selection::{TypeSelection, builders::TypeSelectionBuilder},
    },
};
use bon::bon;

pub(super) mod batching;
pub(super) mod merging;
pub(super) mod querying;
pub(super) mod retrieving;

/// The entity operations of a [`Client`](crate::client::Client), from
/// [`Client::entities`](crate::client::Client::entities).
///
/// Each operation is a builder that finishes with `send()`. Query-language arguments (`entity_type`, `q`, `geo_q`,
/// `scope_q`, `csf`, `pick`, `omit`, `lang`, `ordering`) take the query builders, so a call reads
/// `.q(q::attr("speed").gt(50))` without a `?` per argument; every problem in them is reported by `send()` as
/// [`ArgumentError::InvalidArgument`](crate::http::operation::ArgumentError::InvalidArgument).
///
/// Retrieve Entity and Query Entities exist once per representation family: a lossless one answered with
/// [`Entity`](crate::model::Entity) values, the simplified one answered with
/// [`SimplifiedEntity`](crate::model::SimplifiedEntity) values, and their `GeoJSON` counterparts answered with
/// [`Feature`](crate::model::Feature) values (clause 6.3.15); once per completeness: complete entities, or entities
/// a projection may have reduced (clause 4.21); and, for Retrieve Entity, once per join shape: one entity, or the
/// flattened entity graph (clause 4.5.23.3).
///
/// The batch operations (clauses 6.14 to 6.17 and 6.31) send several entities in one request, each entity reported
/// on its own. The elements are sent in the order given, which is the order the broker processes them in (clause
/// 4.6.6); an empty batch is refused before sending with
/// [`ArgumentError::EmptyBatch`](crate::http::operation::ArgumentError::EmptyBatch).
///
/// ```
/// # use bytes::Bytes;
/// # use http::{Request, Response, StatusCode};
/// # use std::convert::Infallible;
/// # struct Offline;
/// # impl ngsi_ld::client::Transport for Offline {
/// #     type Error = Infallible;
/// #     async fn send(&self, _: Request<Bytes>) -> Result<Response<Bytes>, Infallible> {
/// #         let mut response =
/// #             Response::new(Bytes::from_static(br#"["urn:ngsi-ld:Vehicle:A4567"]"#));
/// #         *response.status_mut() = StatusCode::CREATED;
/// #         Ok(response)
/// #     }
/// # }
/// use iri_rs::iri;
/// use ngsi_ld::{
///     client::Client,
///     http::BatchCreateOutcome,
///     model::{Entity, EntityFragment},
/// };
///
/// async fn park(client: &Client<Offline>) -> anyhow::Result<()> {
///     let vehicles = [Entity::builder()
///         .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
///         .entity_type("Vehicle")
///         .property("speed", 80)
///         .build()?];
///     if let BatchCreateOutcome::Partial(result) =
///         client.entities().batch_create(&vehicles).send().await?
///     {
///         for error in &result.errors {
///             println!("{} failed: {:?}", error.entity_id, error.error.error_type);
///         }
///     }
///     let stopped = [
///         EntityFragment::for_entity(iri!("urn:ngsi-ld:Vehicle:A4567"), "Vehicle")
///             .property("speed", 0)
///             .delete("isParked")
///             .build()?,
///     ];
///     client.entities().batch_merge(&stopped).send().await?;
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Entities<'c, T, M = OutcomeOnly> {
    /// Where the calls go: the client, and the snapshot they are aimed at.
    route: Route<'c, T, M>,
}

impl<'c, T, M> Entities<'c, T, M> {
    /// The handle sending its calls through `route`.
    pub(crate) const fn new(route: Route<'c, T, M>) -> Self {
        Self { route }
    }
}

#[bon]
impl<T: Transport, M: Answering> Entities<'_, T, M> {
    /// Create Entity (clause 6.4.3.1).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the broker refuses the entity (for instance `AlreadyExists`) or the call fails.
    #[builder(finish_fn = send)]
    pub async fn create(
        &self,
        #[builder(start_fn)] entity: &Entity<Input>,
        #[builder(default)] form: Lossless,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<CreateOutcome>, ClientError<T::Error>> {
        self.route
            .send_with_locality(locality, &CreateEntity::builder().entity(entity).form(form).build(), tenant)
            .await
    }

    /// Replace Entity (clause 6.5.3.3): the entity named by the new representation's id is replaced by it.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the entity does not exist or the call fails.
    #[builder(finish_fn = send)]
    pub async fn replace(
        &self,
        #[builder(start_fn)] entity: &Entity<Input>,
        #[builder(into)] entity_type: Option<TypeSelectionBuilder>,
        #[builder(default)] form: Lossless,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<BatchOutcome>, ClientError<T::Error>> {
        let entity_type: Option<TypeSelection> = built(entity_type)?;
        self.route
            .send_with_locality(
                locality,
                &ReplaceEntity::builder()
                    .entity(entity)
                    .maybe_entity_type(entity_type.as_ref())
                    .form(form)
                    .build(),
                tenant,
            )
            .await
    }

    /// Delete Entity (clause 6.5.3.2).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the entity does not exist or the call fails.
    #[builder(finish_fn = send)]
    pub async fn delete(
        &self,
        #[builder(start_fn)] id: &EntityId,
        #[builder(into)] entity_type: Option<TypeSelectionBuilder>,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<BatchOutcome>, ClientError<T::Error>> {
        let entity_type: Option<TypeSelection> = built(entity_type)?;
        self.route
            .send_with_locality(
                locality,
                &DeleteEntity::builder().id(id).maybe_entity_type(entity_type.as_ref()).build(),
                tenant,
            )
            .await
    }

    /// Update Attributes (clause 6.6.3.2): changes attributes the entity has.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the entity does not exist or the call fails.
    #[builder(finish_fn = send)]
    pub async fn update_attributes<C: Completeness>(
        &self,
        #[builder(start_fn)] id: &EntityId,
        #[builder(start_fn)] fragment: &EntityFragment<C>,
        #[builder(into)] entity_type: Option<TypeSelectionBuilder>,
        #[builder(default)] form: Lossless,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<UpdateOutcome>, ClientError<T::Error>> {
        let entity_type: Option<TypeSelection> = built(entity_type)?;
        self.route
            .send_with_locality(
                locality,
                &UpdateAttributes::builder()
                    .id(id)
                    .fragment(fragment)
                    .maybe_entity_type(entity_type.as_ref())
                    .form(form)
                    .build(),
                tenant,
            )
            .await
    }

    /// Append Attributes (clause 6.6.3.1): adds attributes, replacing existing ones unless `overwrite` is
    /// [`Overwrite::Keep`].
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the entity does not exist or the call fails.
    #[builder(finish_fn = send)]
    pub async fn append_attributes<C: Completeness>(
        &self,
        #[builder(start_fn)] id: &EntityId,
        #[builder(start_fn)] fragment: &EntityFragment<C>,
        #[builder(default)] overwrite: Overwrite,
        #[builder(into)] entity_type: Option<TypeSelectionBuilder>,
        #[builder(default)] form: Lossless,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<UpdateOutcome>, ClientError<T::Error>> {
        let entity_type: Option<TypeSelection> = built(entity_type)?;
        self.route
            .send_with_locality(
                locality,
                &AppendAttributes::builder()
                    .id(id)
                    .fragment(fragment)
                    .overwrite(overwrite)
                    .maybe_entity_type(entity_type.as_ref())
                    .form(form)
                    .build(),
                tenant,
            )
            .await
    }

    /// Partial Attribute Update (clause 6.7.3.1): changes some members of one attribute, deleting those the fragment
    /// sets to the NGSI-LD Null (clause 5.5.8), or deletes the instance when given an [`InstanceDeletion`].
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the entity or the attribute does not exist or the call fails.
    #[builder(finish_fn = send)]
    pub async fn update_attribute(
        &self,
        #[builder(start_fn)] id: &EntityId,
        #[builder(start_fn)] name: &AttributeName,
        #[builder(start_fn, into)] attribute: Change<Attribute<Patch>, InstanceDeletion>,
        #[builder(into)] entity_type: Option<TypeSelectionBuilder>,
        #[builder(default)] form: Lossless,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<UpdateOutcome>, ClientError<T::Error>> {
        let entity_type: Option<TypeSelection> = built(entity_type)?;
        self.route
            .send_with_locality(
                locality,
                &PartialAttributeUpdate::builder()
                    .id(id)
                    .name(name)
                    .attribute(&attribute)
                    .maybe_entity_type(entity_type.as_ref())
                    .form(form)
                    .build(),
                tenant,
            )
            .await
    }

    /// Replace Attribute (clause 6.7.3.3): replaces one attribute instance.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the entity or the attribute does not exist or the call fails.
    #[builder(finish_fn = send)]
    pub async fn replace_attribute(
        &self,
        #[builder(start_fn)] id: &EntityId,
        #[builder(start_fn)] name: &AttributeName,
        #[builder(start_fn)] attribute: &Attribute<Input>,
        #[builder(into)] entity_type: Option<TypeSelectionBuilder>,
        #[builder(default)] form: Lossless,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<UpdateOutcome>, ClientError<T::Error>> {
        let entity_type: Option<TypeSelection> = built(entity_type)?;
        self.route
            .send_with_locality(
                locality,
                &ReplaceAttribute::builder()
                    .id(id)
                    .name(name)
                    .attribute(attribute)
                    .maybe_entity_type(entity_type.as_ref())
                    .form(form)
                    .build(),
                tenant,
            )
            .await
    }

    /// Delete Attribute (clause 6.7.3.2): removes the default instance, one dataset or every instance.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the entity or the attribute does not exist or the call fails.
    #[builder(finish_fn = send)]
    pub async fn delete_attribute(
        &self,
        #[builder(start_fn)] id: &EntityId,
        #[builder(start_fn)] name: &AttributeName,
        #[builder(default)] instances: InstanceSelection<'_>,
        #[builder(into)] entity_type: Option<TypeSelectionBuilder>,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<UpdateOutcome>, ClientError<T::Error>> {
        let entity_type: Option<TypeSelection> = built(entity_type)?;
        self.route
            .send_with_locality(
                locality,
                &DeleteAttribute::builder()
                    .id(id)
                    .name(name)
                    .instances(instances)
                    .maybe_entity_type(entity_type.as_ref())
                    .build(),
                tenant,
            )
            .await
    }

    /// Purge Entities (clause 6.4.3.3): deletes every matching entity, or some of their attributes; `entity_map`
    /// names an existing entity map that determines the entities purged (Table 6.4.3.3-2).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the selection is unbounded and not local (refused before sending), or the
    /// call fails.
    #[builder(finish_fn = send)]
    pub async fn purge(
        &self,
        #[builder(into)] entity_type: Option<SelectorTypeBuilder>,
        #[builder(default)] ids: Vec<EntityId>,
        #[builder(into)] id_pattern: Option<IdPattern>,
        #[builder(into)] q: Option<QueryBuilder>,
        #[builder(into)] geo_q: Option<GeoQueryBuilder>,
        #[builder(into)] scope_q: Option<ScopeQueryBuilder>,
        #[builder(into)] csf: Option<QueryBuilder>,
        #[builder(default)] drop: Vec<AttributeName>,
        #[builder(default)] keep: Vec<AttributeName>,
        #[builder(default)] locality: Locality,
        entity_map: Option<&NgsiLdEntityMap>,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<BatchOutcome>, ClientError<T::Error>> {
        let selection = SelectionBuilders {
            entity_type,
            q,
            geo_q,
            scope_q,
            csf,
            pick: None,
            omit: None,
            lang: None,
        }
        .build()?;
        let purge = PurgeEntities::builder()
            .maybe_entity_type(selection.entity_type.as_ref())
            .ids(&ids)
            .maybe_id_pattern(id_pattern.as_ref())
            .maybe_q(selection.q.as_ref())
            .maybe_geo_q(selection.geo_q.as_ref())
            .maybe_scope_q(selection.scope_q.as_ref())
            .maybe_csf(selection.csf.as_ref())
            .drop(&drop)
            .keep(&keep)
            .locality(locality)
            .maybe_entity_map(entity_map)
            .build();
        self.route.send_selection(&purge, tenant).await
    }
}
