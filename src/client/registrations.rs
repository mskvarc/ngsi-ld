//! The context source registration operations (NGSI-LD clauses 5.9.2 to 5.9.4, 5.10.1 and 5.10.2), as builders that
//! finish with `send()`.

use crate::{
    client::{Answering, Client, ClientError, OutcomeOnly, Transport, arguments::built},
    http::{
        Page,
        ResourceCreated,
        operation::{DeleteRegistration, QueryRegistrations, RegisterContextSource, ResultsCount, RetrieveRegistration, SystemAttributes, UpdateRegistration},
    },
    model::{AttributeName, EntityId, Input, Output, Tenant},
    query::{
        IdPattern,
        geoquery::{GeoQuery, builders::GeoQueryBuilder},
        language_filter::{LanguageFilter, builders::LanguageFilterBuilder},
        q::{Query, builders::QueryBuilder},
        scope_query::{ScopeQuery, builders::ScopeQueryBuilder},
        temporal_query::{TemporalQuery, builders::TemporalQueryBuilder},
        type_selection::{TypeSelection, builders::TypeSelectionBuilder},
    },
    registration::{CSourceRegistration, RegistrationFragment, RegistrationId},
};
use bon::bon;

/// The context source registration operations of a [`Client`], from [`Client::registrations`].
///
/// Each operation is a builder that finishes with `send()`.
///
/// ```
/// # use bytes::Bytes;
/// # use http::{Request, Response, StatusCode, header::LOCATION};
/// # use std::convert::Infallible;
/// # struct Offline;
/// # impl ngsi_ld::client::Transport for Offline {
/// #     type Error = Infallible;
/// #     async fn send(&self, request: Request<Bytes>) -> Result<Response<Bytes>, Infallible> {
/// #         let mut response = Response::new(Bytes::from_static(b"[]"));
/// #         if request.method() == http::Method::POST {
/// #             *response.status_mut() = StatusCode::CREATED;
/// #             response.headers_mut().insert(LOCATION, http::HeaderValue::from_static("/ngsi-ld/v1/csourceRegistrations/urn:ngsi-ld:ContextSourceRegistration:1"));
/// #         }
/// #         Ok(response)
/// #     }
/// # }
/// use anyhow::Context;
/// use ngsi_ld::{client::Client, registration::{CSourceRegistration, RegistrationInfo}};
///
/// async fn register_speed_source(client: &Client<Offline>) -> anyhow::Result<()> {
///     let registration = CSourceRegistration::builder("http://my.csource.org:1026")
///         .information([RegistrationInfo::builder()
///             .entities(["Vehicle"])
///             .property_names(["speed"])
///             .build()?])
///         .build()?;
///     let created = client.registrations().register(&registration).send().await?;
///     let id = created.id().context("the broker named no registration")?;
///     let sources = client.registrations().query().entity_type("Vehicle").send().await?;
///     assert!(sources.items().is_empty());
///     client.registrations().delete(&id).send().await?;
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Registrations<'c, T, M = OutcomeOnly> {
    /// The client the calls go through.
    client: &'c Client<T, M>,
}

impl<'c, T, M> Registrations<'c, T, M> {
    /// The handle of `client`.
    pub(crate) const fn new(client: &'c Client<T, M>) -> Self {
        Self { client }
    }
}

#[bon]
impl<T: Transport, M: Answering> Registrations<'_, T, M> {
    /// Register Context Source (clause 6.8.3.1); the broker names a registration sent without an `id` (clause
    /// 5.9.2.4), which [`ResourceCreated::id`] reads.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the broker refuses the registration (`AlreadyExists`, `BadRequestData`,
    /// `Conflict`) or the call fails.
    #[builder(finish_fn = send)]
    pub async fn register(
        &self,
        #[builder(start_fn)] registration: &CSourceRegistration<Input>,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<ResourceCreated<RegistrationId>>, ClientError<T::Error>> {
        self.client.send_on_tenant(&RegisterContextSource { registration }, tenant).await
    }

    /// Update Context Source Registration (clause 6.9.3.2): applies a fragment to the registration.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the registration does not exist, the broker refuses the fragment, or the call
    /// fails.
    #[builder(finish_fn = send)]
    pub async fn update(
        &self,
        #[builder(start_fn)] id: &RegistrationId,
        #[builder(start_fn)] fragment: &RegistrationFragment,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<()>, ClientError<T::Error>> {
        self.client.send_on_tenant(&UpdateRegistration { id, fragment }, tenant).await
    }

    /// Retrieve Context Source Registration (clause 6.9.3.1).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the registration does not exist, the answer is not a registration, or the call
    /// fails.
    #[builder(finish_fn = send)]
    pub async fn retrieve(
        &self,
        #[builder(start_fn)] id: &RegistrationId,
        #[builder(default)] system_attributes: SystemAttributes,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<CSourceRegistration<Output>>, ClientError<T::Error>> {
        self.client
            .send_on_tenant(&RetrieveRegistration::builder().id(id).system_attributes(system_attributes).build(), tenant)
            .await
    }

    /// Query Context Source Registrations (clause 6.8.3.2): the first page; [`Client::pages`] walks the following
    /// ones.
    ///
    /// At least an entity type, an attribute, a `q` or a geoquery is needed (clause 5.10.2.4), the temporal query
    /// may not carry `lastN` or an aggregation, and `limit` 0 needs `count` (clause 6.3.13); `send()` refuses the
    /// request otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when an argument is invalid, the request is refused before sending, an element of
    /// the answer is not a registration, or the call fails.
    #[builder(finish_fn = send)]
    pub async fn query(
        &self,
        #[builder(into)] entity_type: Option<TypeSelectionBuilder>,
        #[builder(default)] ids: Vec<EntityId>,
        #[builder(into)] id_pattern: Option<IdPattern>,
        #[builder(default)] attrs: Vec<AttributeName>,
        #[builder(into)] q: Option<QueryBuilder>,
        #[builder(into)] csf: Option<QueryBuilder>,
        #[builder(into)] geo_q: Option<GeoQueryBuilder>,
        #[builder(into)] temporal_q: Option<TemporalQueryBuilder>,
        #[builder(into)] scope_q: Option<ScopeQueryBuilder>,
        #[builder(into)] lang: Option<LanguageFilterBuilder>,
        #[builder(default)] system_attributes: SystemAttributes,
        limit: Option<u32>,
        offset: Option<u32>,
        #[builder(default)] count: ResultsCount,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<Page<CSourceRegistration<Output>>>, ClientError<T::Error>> {
        let entity_type: Option<TypeSelection> = built(entity_type)?;
        let q: Option<Query> = built(q)?;
        let csf: Option<Query> = built(csf)?;
        let geo_q: Option<GeoQuery> = built(geo_q)?;
        let temporal_q: Option<TemporalQuery> = built(temporal_q)?;
        let scope_q: Option<ScopeQuery> = built(scope_q)?;
        let lang: Option<LanguageFilter> = built(lang)?;
        let query = QueryRegistrations::builder()
            .maybe_entity_type(entity_type.as_ref())
            .ids(&ids)
            .maybe_id_pattern(id_pattern.as_ref())
            .attrs(&attrs)
            .maybe_q(q.as_ref())
            .maybe_csf(csf.as_ref())
            .maybe_geo_q(geo_q.as_ref())
            .maybe_temporal_q(temporal_q.as_ref())
            .maybe_scope_q(scope_q.as_ref())
            .maybe_lang(lang.as_ref())
            .system_attributes(system_attributes)
            .maybe_limit(limit)
            .maybe_offset(offset)
            .count(count)
            .build();
        self.client.send_on_tenant(&query, tenant).await
    }

    /// Delete Context Source Registration (clause 6.9.3.3).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the registration does not exist or the call fails.
    #[builder(finish_fn = send)]
    pub async fn delete(&self, #[builder(start_fn)] id: &RegistrationId, tenant: Option<&Tenant>) -> Result<M::Answer<()>, ClientError<T::Error>> {
        self.client.send_on_tenant(&DeleteRegistration { id }, tenant).await
    }
}
