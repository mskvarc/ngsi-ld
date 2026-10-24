//! Create Subscription (NGSI-LD clauses 5.8.1 and 6.10.3.1) and Create Context Source Registration Subscription
//! (clauses 5.11.2 and 6.12.3.1).

use crate::{
    http::{
        Endpoint,
        RequestError,
        ResourceCreated,
        ResponseContext,
        ResponseError,
        operation::{
            Operation,
            sealed::{BuildsRequest, Sealed},
            subscription::resource::SubscriptionResource,
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
        response::classify,
    },
    model::Input,
    subscription::{EntitySubscription, Subscription, SubscriptionId, SubscriptionKind},
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// Creates a subscription.
///
/// Its request is `POST /subscriptions` (NGSI-LD clause 6.10.3.1), or `POST /csourceSubscriptions` (clause 6.12.3.1)
/// for a subscription of kind [`RegistrationSubscription`](crate::subscription::RegistrationSubscription). A
/// subscription sent without an `id` is given one by the broker (clauses 5.8.1.4 and 5.11.2.4), which
/// [`ResourceCreated::id`] reads from the `Location` of the answer.
///
/// ```
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{CreateSubscription, Operation},
///     },
///     subscription::{Endpoint as NotificationEndpoint, NotificationParams, Subscription},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let notify = NotificationParams::builder(
///     NotificationEndpoint::builder("http://my.endpoint.org/notify").build()?,
/// )
/// .build()?;
/// let vehicles = Subscription::builder()
///     .entities(["Vehicle"])
///     .notification(notify)
///     .build()?;
/// let request = CreateSubscription {
///     subscription: &vehicles,
/// }
/// .to_request(&endpoint)?;
/// assert_eq!(request.method(), "POST");
/// assert_eq!(request.uri(), "http://broker.test/ngsi-ld/v1/subscriptions");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateSubscription<'a, K: SubscriptionKind = EntitySubscription> {
    /// The subscription.
    pub subscription: &'a Subscription<Input, K>,
}

impl<K: SubscriptionKind> Sealed for CreateSubscription<'_, K> {}

impl<K: SubscriptionResource> Operation for CreateSubscription<'_, K> {
    type Outcome = ResourceCreated<SubscriptionId>;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<ResourceCreated<SubscriptionId>, ResponseError> {
        ResourceCreated::decode(&classify(response)?)
    }
}

impl<K: SubscriptionResource> BuildsRequest for CreateSubscription<'_, K> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        RequestParts::resource(Method::POST, ResourcePath::subscriptions(K::COLLECTION), QueryString::default())
            .with_body(self.subscription.to_object())
            .into_request(endpoint, scope)
    }
}
