//! The operations of the NGSI-LD API, one request type each, with the parameters of its clause 6 table as typed
//! fields.
//!
//! Every type implements [`Operation`]: it builds its HTTP request through the crate's one request pipeline and reads the response into the outcomes its table lists.
//!
//! The binding is sans-IO: an operation turns into an `http::Request` against an [`Endpoint`](crate::http::Endpoint), and an
//! `http::Response`, however it was obtained, turns into the operation's outcome.
//!
//! ```
//! use bytes::Bytes;
//! use http::{Response, StatusCode};
//! use iri_rs::iri;
//! use ngsi_ld::{
//!     http::{
//!         CreateOutcome,
//!         Endpoint,
//!         ResponseContext,
//!         operation::{CreateEntity, Operation},
//!     },
//!     model::Entity,
//! };
//! use url::Url;
//!
//! let endpoint = Endpoint::builder()
//!     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
//!     .build()?;
//! let vehicle = Entity::builder()
//!     .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
//!     .entity_type("Vehicle")
//!     .property("speed", 80)
//!     .build()?;
//! let create = CreateEntity::builder().entity(&vehicle).build();
//!
//! let request = create.to_request(&endpoint)?;
//! assert_eq!(request.method(), "POST");
//! assert_eq!(request.uri(), "http://broker.test/ngsi-ld/v1/entities");
//!
//! let context = ResponseContext::new(&request);
//! let mut response = Response::new(Bytes::new());
//! *response.status_mut() = StatusCode::CREATED;
//! let outcome = create.decode(&context, response)?;
//! assert!(matches!(outcome, CreateOutcome::Created { .. }));
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! # Operations
//!
//! [`Operation`], [`Simplified`], [`ArgumentError`], [`PageRequest`].
//!
//! # Entities
//!
//! [`CreateEntity`], [`RetrieveEntity`], [`RetrieveEntityParameters`], [`QueryEntities`], [`QueryEntitiesParameters`],
//! [`QueryEntitiesPostParameters`], [`ReplaceEntity`], [`MergeEntity`], [`MergeSimplifiedEntity`], [`DeleteEntity`],
//! [`PurgeEntities`], [`PurgeEntitiesParameters`], [`GeoJson`], [`InlineJoin`], [`Flattened`].
//!
//! # Attributes
//!
//! [`AppendAttributes`], [`AppendAttributesParameters`], [`UpdateAttributes`], [`PartialAttributeUpdate`],
//! [`ReplaceAttribute`], [`DeleteAttribute`], [`DeleteAttributeParameters`], [`DeleteAll`], [`InstanceSelection`].
//!
//! # Batch operations
//!
//! The elements of a batch are sent in the order given, never deduplicated or sorted: a broker processes the instances
//! of one entity in array order, oldest first (clauses 4.6.6 and 5.5.11), so the caller's order is the chronological
//! order. An empty batch is refused before it is sent, since every batch operation answers it with a `BadRequestData`.
//!
//! [`BatchCreate`], [`BatchUpsert`], [`BatchUpsertParameters`], [`BatchUpdate`], [`BatchUpdateParameters`],
//! [`BatchMerge`], [`BatchDelete`].
//!
//! # Temporal evolution
//!
//! Retrieve and Query Temporal Evolution choose the representation by type, as Retrieve and Query Entities do: a
//! [`Lossless`](crate::model::Lossless) format answers with [`TemporalEntity`](crate::model::evolution::TemporalEntity) values,
//! [`SimplifiedTemporal`] with [`SimplifiedTemporalEntity`](crate::model::evolution::SimplifiedTemporalEntity) values and
//! [`Aggregated`] with [`AggregatedTemporalEntity`](crate::model::evolution::AggregatedTemporalEntity) values (clause
//! 6.3.12). A broker that cannot answer at once answers 206 Partial Content with a `Content-Range` (clause 6.3.10),
//! read into the [`Coverage`](crate::http::Coverage) of the answer.
//!
//! [`CreateOrUpdateTemporalEvolution`], [`CreateOrUpdateSimplifiedTemporalEvolution`], [`AddTemporalAttributes`],
//! [`DeleteTemporalAttribute`], [`ModifyAttributeInstance`], [`DeleteAttributeInstance`], [`DeleteTemporalEvolution`],
//! [`RetrieveTemporalEvolution`], [`QueryTemporalEvolution`], [`TemporalSelection`], [`TemporalRepresentation`],
//! [`SimplifiedTemporal`], [`Aggregated`], [`TemporalSelectionParameters`], [`RetrieveTemporalEvolutionParameters`],
//! [`QueryTemporalEvolutionParameters`], [`QueryTemporalEvolutionPostParameters`].
//!
//! # Subscriptions
//!
//! Each request type is generic over the [kind of subscription](crate::subscription::SubscriptionKind), which decides
//! the resource: `subscriptions` for context information subscriptions, `csourceSubscriptions` for Context Source
//! Registration Subscriptions. A subscription, a registration and a snapshot name attributes and entity types as terms,
//! so their requests carry the client's `@context` as every other request does (clause 6.3.5); the `@context` a
//! document was read with is not sent.
//!
//! [`CreateSubscription`], [`RetrieveSubscription`], [`QuerySubscriptions`], [`UpdateSubscription`],
//! [`DeleteSubscription`].
//!
//! # Registrations
//!
//! The registry operations are always local (clause 6.3.18), so none takes `local`.
//!
//! [`RegisterContextSource`], [`RetrieveRegistration`], [`QueryRegistrations`], [`UpdateRegistration`],
//! [`DeleteRegistration`].
//!
//! # Discovery
//!
//! `details=true` changes the shape of the answer (Tables 6.25.3.1-2 and 6.27.3.1-2), so the summary and the detailed
//! retrieval are separate request types with separate outcomes. Every request is a GET, so the client's context goes in
//! the `Link` header (clause 6.3.5), where it decides the short names of the answer.
//!
//! [`RetrieveEntityTypes`], [`RetrieveEntityTypeDetails`], [`RetrieveEntityTypeInfo`], [`RetrieveAttributes`],
//! [`RetrieveAttributeDetails`], [`RetrieveAttributeInfo`], [`RetrieveSourceIdentity`].
//!
//! # Stored contexts
//!
//! The body of Add @context is a JSON-LD local context, sent as `application/ld+json` without the client's own context
//! (clause 6.3.5). As for discovery, `details=true` has its own request types.
//!
//! [`AddContext`], [`ListContexts`], [`ListContextDetails`], [`ContextKindFilter`], [`ServeContext`],
//! [`ServeContextDetails`], [`DeleteContext`], [`ContextDeletion`].
//!
//! # Entity maps
//!
//! An entity map (clauses 4.5.25, 5.2.39 and 5.14) records which context source registrations hold which entities
//! during a distributed operation, so the following requests of the same consumption, the next page of a query for
//! instance, reach the same sources.
//!
//! [`CreateEntityMap`], [`CreateTemporalEntityMap`], [`RetrieveEntityMap`], [`UpdateEntityMap`], [`DeleteEntityMap`],
//! [`EntityMapRequest`], [`EntityMapLifetime`], [`EntityMapLifetimeError`].
//!
//! # Snapshots
//!
//! [`OnSnapshot`] aims an operation of the Core or Temporal API at a snapshot.
//!
//! [`CreateSnapshot`], [`CloneSnapshot`], [`RetrieveSnapshot`], [`UpdateSnapshot`], [`DeleteSnapshot`],
//! [`PurgeSnapshots`], [`OnSnapshot`], [`SnapshotOperation`].
//!
//! # Parameters
//!
//! [`SystemAttributes`], [`ResultsCount`], [`SummaryDetails`], [`QueryMethod`], [`Overwrite`], [`UpsertMode`],
//! [`LocalScopeOperation`], [`WithLocality`], [`RepresentationSpelling`], [`RepresentationToken`],
//! [`RepresentationParameter`], [`NormalizedSpelling`], [`SimplifiedSpelling`].
//!
//! # Received requests
//!
//! A context source reads the query string of a request a broker forwards to it (clause 5.7.2.4) in typed form: the
//! parameters are split at `&`, each name is one of the operation's table, and each value is percent-decoded as IETF
//! RFC 3986 section 2.1 says, so a `+` is a `+` (the crate's own requests escape it as `%2B`). A comma separated list
//! is split at its literal commas before its items are decoded, so an escaped comma (`%2C`) stays inside its item.
//!
//! [`SelectionParameters`], [`AssemblyParameters`], [`ParameterError`].
//!
//! # Builders
//!
//! The builders of these types are in [`builders`].

pub use crate::http::operation::{
    append_attributes::{AppendAttributes, parameters::AppendAttributesParameters},
    argument_error::ArgumentError,
    batch::{
        create::BatchCreate,
        delete::BatchDelete,
        merge::BatchMerge,
        parameters::{BatchUpdateParameters, BatchUpsertParameters},
        update::BatchUpdate,
        upsert::BatchUpsert,
    },
    binding::Operation,
    create_entity::CreateEntity,
    delete_attribute::{
        DeleteAttribute,
        InstanceSelection,
        parameters::{DeleteAll, DeleteAttributeParameters},
    },
    delete_entity::DeleteEntity,
    discovery::{
        attribute_details::RetrieveAttributeDetails,
        attribute_info::RetrieveAttributeInfo,
        attributes::RetrieveAttributes,
        entity_type_details::RetrieveEntityTypeDetails,
        entity_type_info::RetrieveEntityTypeInfo,
        entity_types::RetrieveEntityTypes,
    },
    entity_format::Simplified,
    entity_map::{
        EntityMapLifetime,
        EntityMapLifetimeError,
        EntityMapRequest,
        create::CreateEntityMap,
        create_temporal::CreateTemporalEntityMap,
        delete::DeleteEntityMap,
        retrieve::RetrieveEntityMap,
        update::UpdateEntityMap,
    },
    geo_json::GeoJson,
    linked_entities::{Flattened, InlineJoin},
    locality::{LocalScopeOperation, WithLocality},
    merge_entity::{MergeEntity, MergeSimplifiedEntity},
    page_request::PageRequest,
    parameters::{Overwrite, QueryMethod, ResultsCount, SummaryDetails, SystemAttributes, UpsertMode},
    partial_attribute_update::PartialAttributeUpdate,
    purge_entities::{PurgeEntities, parameters::PurgeEntitiesParameters},
    query_entities::{
        QueryEntities,
        parameters::{QueryEntitiesParameters, QueryEntitiesPostParameters},
    },
    received_parameters::{
        ParameterError,
        selection::{AssemblyParameters, SelectionParameters},
    },
    registration::{
        delete::DeleteRegistration,
        query::QueryRegistrations,
        register::RegisterContextSource,
        retrieve::RetrieveRegistration,
        update::UpdateRegistration,
    },
    replace_attribute::ReplaceAttribute,
    replace_entity::ReplaceEntity,
    representation_spelling::{NormalizedSpelling, RepresentationParameter, RepresentationSpelling, RepresentationToken, SimplifiedSpelling},
    retrieve_entity::{RetrieveEntity, parameters::RetrieveEntityParameters},
    snapshot::{
        clone::CloneSnapshot,
        create::CreateSnapshot,
        delete::DeleteSnapshot,
        purge::PurgeSnapshots,
        retrieve::RetrieveSnapshot,
        update::UpdateSnapshot,
    },
    snapshot_target::{OnSnapshot, SnapshotOperation},
    source_identity::RetrieveSourceIdentity,
    stored_context::{
        add::AddContext,
        delete::{ContextDeletion, DeleteContext},
        list::{ContextKindFilter, ListContextDetails, ListContexts},
        serve::{ServeContext, ServeContextDetails},
    },
    subscription::{
        create::CreateSubscription,
        delete::DeleteSubscription,
        query::QuerySubscriptions,
        retrieve::RetrieveSubscription,
        update::UpdateSubscription,
    },
    temporal::{
        Aggregated,
        SimplifiedTemporal,
        TemporalRepresentation,
        TemporalSelection,
        add_attributes::AddTemporalAttributes,
        create_or_update::{CreateOrUpdateSimplifiedTemporalEvolution, CreateOrUpdateTemporalEvolution},
        delete::DeleteTemporalEvolution,
        delete_attribute::DeleteTemporalAttribute,
        delete_instance::DeleteAttributeInstance,
        modify_instance::ModifyAttributeInstance,
        parameters::{
            QueryTemporalEvolutionParameters,
            QueryTemporalEvolutionPostParameters,
            RetrieveTemporalEvolutionParameters,
            TemporalSelectionParameters,
        },
        query::QueryTemporalEvolution,
        retrieve::RetrieveTemporalEvolution,
    },
    update_attributes::UpdateAttributes,
};
pub mod builders;
#[macro_use]
mod deletion;
#[macro_use]
mod entity_selection;
mod append_attributes;
mod argument_error;
mod batch;
mod binding;
mod create_entity;
mod delete_attribute;
mod delete_entity;
mod discovery;
mod entity_format;
mod entity_map;
mod entity_ordering;
mod geo_json;
mod geoquery;
mod linked_entities;
mod locality;
mod merge_entity;
mod no_content;
#[macro_use]
mod page_request;
mod parameters;
mod partial_attribute_update;
mod projection_rule;
mod purge_entities;
mod query_entities;
mod received_parameters;
mod registration;
mod replace_attribute;
mod replace_entity;
mod representation_spelling;
mod retrieve_entity;
mod sealed;
mod snapshot;
mod snapshot_target;
mod source_identity;
mod stored_context;
mod stored_document;
pub(crate) mod subscription;
mod temporal;
mod update_attributes;
