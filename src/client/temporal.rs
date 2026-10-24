//! The Temporal API operations (NGSI-LD clauses 5.6.11 to 5.6.16, 5.7.3 and 5.7.4), as builders that finish with
//! `send()`.

use crate::{
    client::{Answering, ClientError, OutcomeOnly, Transport, arguments::built, route::Route},
    http::{
        TemporalUpsertOutcome,
        operation::{
            AddTemporalAttributes,
            ArgumentError,
            CreateOrUpdateSimplifiedTemporalEvolution,
            CreateOrUpdateTemporalEvolution,
            DeleteAttributeInstance,
            DeleteTemporalAttribute,
            DeleteTemporalEvolution,
            InstanceSelection,
            ModifyAttributeInstance,
            TemporalSelection,
        },
    },
    model::{
        Attribute,
        AttributeName,
        Duration,
        EntityId,
        Input,
        InstanceId,
        Lossless,
        Tenant,
        evolution::{SimplifiedTemporalInput, TemporalAttributes, TemporalEntity},
    },
    query::{
        Locality,
        temporal_query::{AggregationMethods, TemporalProperty, TemporalQuery, builders::TemporalQueryBuilder},
    },
};
use bon::bon;
use std::num::NonZeroU64;

pub(super) mod querying;
mod ranges;
pub(super) mod retrieving;

/// The Temporal API operations of a [`Client`](crate::client::Client), from
/// [`Client::temporal`](crate::client::Client::temporal).
///
/// Each operation is a builder that finishes with `send()`.
///
/// Retrieve and Query Temporal Evolution exist once per representation (`retrieve`, `retrieve_values`,
/// `retrieve_aggregated`, and the same for `query`) and once per completeness (`retrieve_projected` and the
/// others), so the type of the answer follows the format asked for. A broker that cannot answer at once answers 206
/// Partial Content (clause 6.3.10): the answer's [`Coverage`](crate::http::Coverage) says so and gives the
/// temporal query of the rest of the range.
///
/// ```
/// # use bytes::Bytes;
/// # use http::{Request, Response, StatusCode};
/// # use std::convert::Infallible;
/// # struct Offline;
/// # impl ngsi_ld::client::Transport for Offline {
/// #     type Error = Infallible;
/// #     async fn send(&self, _: Request<Bytes>) -> Result<Response<Bytes>, Infallible> {
/// #         let mut response = Response::new(Bytes::from_static(br#"[{"id": "urn:ngsi-ld:Vehicle:B9211", "type": "Vehicle", "speed": [{"type": "Property", "value": 80, "observedAt": "2018-08-01T12:05:00Z"}]}]"#));
/// #         *response.status_mut() = StatusCode::OK;
/// #         Ok(response)
/// #     }
/// # }
/// use ngsi_ld::{
///     client::Client,
///     query::{projection, temporal_query},
/// };
///
/// async fn recent_speeds(client: &Client<Offline>) -> anyhow::Result<usize> {
///     let page = client
///         .temporal()
///         .query(temporal_query::after("2018-08-01T12:00:00Z".parse()?).last_n(10))
///         .entity_type("Vehicle")
///         .pick(projection::attributes(["speed"]))
///         .send()
///         .await?;
///     Ok(page.items().len())
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Temporal<'c, T, M = OutcomeOnly> {
    /// Where the calls go: the client, and the snapshot they are aimed at.
    route: Route<'c, T, M>,
}

impl<'c, T, M> Temporal<'c, T, M> {
    /// The handle sending its calls through `route`.
    pub(crate) const fn new(route: Route<'c, T, M>) -> Self {
        Self { route }
    }
}

/// The arguments of Retrieve Temporal Evolution that select instances, before they are resolved into a
/// [`TemporalSelection`].
struct SelectionArguments {
    /// The temporal query, built.
    temporal_q: Option<TemporalQuery>,
    /// `timeproperty` without a temporal query.
    time_property: Option<TemporalProperty>,
    /// `lastN` without a temporal query.
    last_n: Option<u64>,
    /// `aggrMethods` without a temporal query.
    aggregation_methods: Option<AggregationMethods>,
    /// `aggrPeriodDuration` without a temporal query.
    aggregation_period: Option<Duration>,
}

impl SelectionArguments {
    /// Gathers the arguments, building the temporal query.
    fn new<E>(
        temporal_q: Option<TemporalQueryBuilder>,
        time_property: Option<TemporalProperty>,
        last_n: Option<u64>,
        aggregation_methods: Option<AggregationMethods>,
        aggregation_period: Option<Duration>,
    ) -> Result<Self, ClientError<E>> {
        Ok(Self {
            temporal_q: built(temporal_q)?,
            time_property,
            last_n,
            aggregation_methods,
            aggregation_period,
        })
    }

    /// The selection: the temporal query, or `timeproperty`, `lastN` and the aggregation standing alone. Both
    /// together are refused, since the temporal query carries them itself (Table 5.2.21-1), and so is a `lastN` of 0.
    fn selection<E>(&self) -> Result<TemporalSelection<'_>, ClientError<E>> {
        let last_n = self
            .last_n
            .map(|count| NonZeroU64::new(count).ok_or(ClientError::Request(ArgumentError::ZeroLastN.into())))
            .transpose()?;
        let standing_alone = self.time_property.is_some() || last_n.is_some() || self.aggregation_methods.is_some() || self.aggregation_period.is_some();
        match &self.temporal_q {
            Some(_) if standing_alone => Err(ClientError::Request(ArgumentError::InstanceOptionsBesideTemporalQuery.into())),
            Some(query) => Ok(TemporalSelection::Query(query)),
            None => Ok(TemporalSelection::Unbounded {
                time_property: self.time_property,
                last_n,
                aggregation_methods: self.aggregation_methods.as_ref(),
                aggregation_period: self.aggregation_period,
            }),
        }
    }
}

#[bon]
impl<T: Transport, M: Answering> Temporal<'_, T, M> {
    /// Create or Update Temporal Evolution of an Entity (clause 6.18.3.1): creates the temporal evolution, or adds
    /// the instances sent to the one that exists.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the broker refuses the temporal evolution or the call fails.
    #[builder(finish_fn = send)]
    pub async fn create_or_update(
        &self,
        #[builder(start_fn)] entity: &TemporalEntity<Input>,
        #[builder(default)] form: Lossless,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<TemporalUpsertOutcome>, ClientError<T::Error>> {
        self.route
            .send_with_locality(locality, &CreateOrUpdateTemporalEvolution::builder().entity(entity).form(form).build(), tenant)
            .await
    }

    /// Create or Update Temporal Evolution of an Entity (clause 6.18.3.1) with the temporal evolution in simplified
    /// temporal representation (clause 5.2.20).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the broker refuses the temporal evolution or the call fails.
    #[builder(finish_fn = send)]
    pub async fn create_or_update_simplified(
        &self,
        #[builder(start_fn)] entity: &SimplifiedTemporalInput,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<TemporalUpsertOutcome>, ClientError<T::Error>> {
        self.route
            .send_with_locality(locality, &CreateOrUpdateSimplifiedTemporalEvolution { entity }, tenant)
            .await
    }

    /// Add Attributes to Temporal Evolution of an Entity (clause 6.20.3.1).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the temporal evolution does not exist, the broker refuses the instances, or
    /// the call fails.
    #[builder(finish_fn = send)]
    pub async fn add_attributes(
        &self,
        #[builder(start_fn)] id: &EntityId,
        #[builder(start_fn)] attributes: &TemporalAttributes<Input>,
        #[builder(default)] form: Lossless,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<()>, ClientError<T::Error>> {
        self.route
            .send_with_locality(
                locality,
                &AddTemporalAttributes::builder().id(id).attributes(attributes).form(form).build(),
                tenant,
            )
            .await
    }

    /// Delete Attribute from Temporal Evolution of an Entity (clause 6.21.3.1): removes the evolution of the
    /// default instance, of one dataset, or of every instance.
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
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<()>, ClientError<T::Error>> {
        self.route
            .send_with_locality(
                locality,
                &DeleteTemporalAttribute::builder().id(id).name(name).instances(instances).build(),
                tenant,
            )
            .await
    }

    /// Modify Attribute Instance in Temporal Evolution of an Entity (clause 6.22.3.1): replaces the instance
    /// `instance_id` of the attribute `name` with `instance`.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the entity, the attribute or the instance does not exist, the broker refuses
    /// the instance, or the call fails.
    #[builder(finish_fn = send)]
    pub async fn modify_instance(
        &self,
        #[builder(start_fn)] id: &EntityId,
        #[builder(start_fn)] name: &AttributeName,
        #[builder(start_fn)] instance_id: &InstanceId,
        #[builder(start_fn)] instance: &Attribute<Input>,
        #[builder(default)] form: Lossless,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<()>, ClientError<T::Error>> {
        let modify = ModifyAttributeInstance::builder()
            .id(id)
            .name(name)
            .instance_id(instance_id)
            .instance(instance)
            .form(form)
            .build();
        self.route.send_with_locality(locality, &modify, tenant).await
    }

    /// Delete Attribute Instance from Temporal Evolution of an Entity (clause 6.22.3.2).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the entity, the attribute or the instance does not exist or the call fails.
    #[builder(finish_fn = send)]
    pub async fn delete_instance(
        &self,
        #[builder(start_fn)] id: &EntityId,
        #[builder(start_fn)] name: &AttributeName,
        #[builder(start_fn)] instance_id: &InstanceId,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<()>, ClientError<T::Error>> {
        self.route
            .send_with_locality(locality, &DeleteAttributeInstance { id, name, instance_id }, tenant)
            .await
    }

    /// Delete Temporal Evolution of an Entity (clause 6.19.3.2).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the temporal evolution does not exist or the call fails.
    #[builder(finish_fn = send)]
    pub async fn delete(
        &self,
        #[builder(start_fn)] id: &EntityId,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<()>, ClientError<T::Error>> {
        self.route.send_with_locality(locality, &DeleteTemporalEvolution { id }, tenant).await
    }
}
