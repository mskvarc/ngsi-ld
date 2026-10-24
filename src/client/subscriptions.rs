//! The subscription operations (NGSI-LD clauses 5.8.1 to 5.8.5 and 5.11.2 to 5.11.6), as builders that finish with
//! `send()`.

use crate::{
    client::{Answering, ClientError, OutcomeOnly, Transport, route::Route},
    http::{
        Page,
        ResourceCreated,
        operation::{
            CreateSubscription,
            DeleteSubscription,
            QuerySubscriptions,
            ResultsCount,
            RetrieveSubscription,
            SystemAttributes,
            UpdateSubscription,
            subscription::resource::SubscriptionResource,
        },
    },
    model::{Input, Output, Tenant},
    subscription::{EntitySubscription, Subscription, SubscriptionFragment, SubscriptionId, SubscriptionKind},
};
use bon::bon;

/// The subscription operations of a [`Client`](crate::client::Client) for subscriptions of kind `K`: from
/// [`Client::subscriptions`](crate::client::Client::subscriptions) for context information subscriptions
/// (`subscriptions`, clauses 6.10 and 6.11), from
/// [`Client::registration_subscriptions`](crate::client::Client::registration_subscriptions) for Context Source
/// Registration Subscriptions (`csourceSubscriptions`, clauses 6.12 and 6.13).
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
/// #     async fn send(&self, _: Request<Bytes>) -> Result<Response<Bytes>, Infallible> {
/// #         let mut response = Response::new(Bytes::new());
/// #         *response.status_mut() = StatusCode::CREATED;
/// #         response.headers_mut().insert(
/// #             LOCATION,
/// #             http::HeaderValue::from_static(
/// #                 "/ngsi-ld/v1/subscriptions/urn:ngsi-ld:Subscription:1",
/// #             ),
/// #         );
/// #         Ok(response)
/// #     }
/// # }
/// use anyhow::Context;
/// use ngsi_ld::{
///     client::Client,
///     query::q,
///     subscription::{
///         Activation,
///         Endpoint,
///         NotificationParams,
///         Subscription,
///         SubscriptionFragment,
///     },
/// };
///
/// async fn watch_fast_vehicles(client: &Client<Offline>) -> anyhow::Result<()> {
///     let subscription = Subscription::builder()
///         .entities(["Vehicle"])
///         .watched_attributes(["speed"])
///         .q(q::attr("speed").gt(50).build()?)
///         .notification(
///             NotificationParams::builder(
///                 Endpoint::builder("http://my.endpoint.org/notify").build()?,
///             )
///             .build()?,
///         )
///         .build()?;
///     let created = client.subscriptions().create(&subscription).send().await?;
///     let id = created.id().context("the broker named no subscription")?;
///     let pause = SubscriptionFragment::builder()
///         .activation(Activation::Paused)
///         .build()?;
///     client.subscriptions().update(&id, &pause).send().await?;
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Subscriptions<'c, T, K: SubscriptionKind = EntitySubscription, M = OutcomeOnly> {
    /// Where the calls go: the client, and the snapshot they are aimed at.
    route: Route<'c, T, M>,
    /// The kind of subscription, which decides the resource.
    kind: K,
}

impl<'c, T, K: SubscriptionKind, M> Subscriptions<'c, T, K, M> {
    /// The handle sending its calls through `route`, for subscriptions of kind `kind`.
    pub(crate) const fn new(route: Route<'c, T, M>, kind: K) -> Self {
        Self { route, kind }
    }
}

#[bon]
impl<T: Transport, K: SubscriptionResource, M: Answering> Subscriptions<'_, T, K, M> {
    /// Create Subscription (clauses 6.10.3.1 and 6.12.3.1); the broker names a subscription sent without an `id`
    /// (clauses 5.8.1.4 and 5.11.2.4), which [`ResourceCreated::id`] reads.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the broker refuses the subscription (`AlreadyExists`, `BadRequestData`) or the
    /// call fails.
    #[builder(finish_fn = send)]
    pub async fn create(
        &self,
        #[builder(start_fn)] subscription: &Subscription<Input, K>,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<ResourceCreated<SubscriptionId>>, ClientError<T::Error>> {
        self.route.send(&CreateSubscription { subscription }, tenant).await
    }

    /// Update Subscription (clauses 6.11.3.2 and 6.13.3.2): applies a fragment to the subscription.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the subscription does not exist, the broker refuses the fragment, or the call
    /// fails.
    #[builder(finish_fn = send)]
    pub async fn update(
        &self,
        #[builder(start_fn)] id: &SubscriptionId,
        #[builder(start_fn)] fragment: &SubscriptionFragment<K>,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<()>, ClientError<T::Error>> {
        self.route.send(&UpdateSubscription { id, fragment }, tenant).await
    }

    /// Retrieve Subscription (clauses 6.11.3.1 and 6.13.3.1).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the subscription does not exist, the answer is not a subscription of kind `K`,
    /// or the call fails.
    #[builder(finish_fn = send)]
    pub async fn retrieve(
        &self,
        #[builder(start_fn)] id: &SubscriptionId,
        #[builder(default)] system_attributes: SystemAttributes,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<Subscription<Output, K>>, ClientError<T::Error>> {
        let retrieve = RetrieveSubscription::builder()
            .id(id)
            .system_attributes(system_attributes)
            .kind(self.kind)
            .build();
        self.route.send(&retrieve, tenant).await
    }

    /// Query Subscriptions (clauses 6.10.3.2 and 6.12.3.2): the first page;
    /// [`Client::pages`](crate::client::Client::pages) walks the following ones.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when `limit` is 0 without `count` (refused before sending, clause 6.3.13), an
    /// element of the answer is not a subscription of kind `K`, or the call fails.
    #[builder(finish_fn = send)]
    pub async fn query(
        &self,
        #[builder(default)] system_attributes: SystemAttributes,
        limit: Option<u32>,
        offset: Option<u32>,
        #[builder(default)] count: ResultsCount,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<Page<Subscription<Output, K>>>, ClientError<T::Error>> {
        let query = QuerySubscriptions::builder()
            .system_attributes(system_attributes)
            .maybe_limit(limit)
            .maybe_offset(offset)
            .count(count)
            .kind(self.kind)
            .build();
        self.route.send(&query, tenant).await
    }

    /// Delete Subscription (clauses 6.11.3.3 and 6.13.3.3).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the subscription does not exist or the call fails.
    #[builder(finish_fn = send)]
    pub async fn delete(&self, #[builder(start_fn)] id: &SubscriptionId, tenant: Option<&Tenant>) -> Result<M::Answer<()>, ClientError<T::Error>> {
        self.route.send(&DeleteSubscription { id, kind: self.kind }, tenant).await
    }
}
