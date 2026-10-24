//! Retrieve Subscription (NGSI-LD clauses 5.8.3 and 6.11.3.1) and Retrieve Context Source Registration
//! Subscription (clauses 5.11.4 and 6.13.3.1).

use crate::{
    http::{
        Endpoint,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            Operation,
            SystemAttributes,
            sealed::{BuildsRequest, Sealed},
            subscription::{reader, resource::SubscriptionResource},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
    },
    model::{NgsiLdMediaType, Output},
    subscription::{EntitySubscription, Subscription, SubscriptionId, SubscriptionKind},
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// `GET /subscriptions/{subscriptionId}` (NGSI-LD clause 6.11.3.1), or
/// `GET /csourceSubscriptions/{subscriptionId}` (clause 6.13.3.1) for the kind
/// [`RegistrationSubscription`](crate::subscription::RegistrationSubscription): reads a subscription.
///
/// The subscription is read strictly: a member Table 5.2.12-1 does not define, or a member of the other kind,
/// fails the response.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, RetrieveSubscription},
///     },
///     subscription::{EntitySubscription, SubscriptionId},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = SubscriptionId::from(iri!("urn:ngsi-ld:Subscription:mySubscription"));
/// let request = RetrieveSubscription::builder()
///     .id(&id)
///     .kind(EntitySubscription)
///     .build()
///     .to_request(&endpoint)?;
/// assert_eq!(request.method(), "GET");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/subscriptions/urn:ngsi-ld:Subscription:mySubscription"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, bon::Builder)]
pub struct RetrieveSubscription<'a, K: SubscriptionKind = EntitySubscription> {
    /// The subscription.
    pub id: &'a SubscriptionId,
    /// Whether `createdAt` and `modifiedAt` are included (clause 6.3.11).
    #[builder(default)]
    pub system_attributes: SystemAttributes,
    /// The kind of subscription, which decides the resource and how the answer is read.
    pub kind: K,
}

impl<K: SubscriptionKind> Sealed for RetrieveSubscription<'_, K> {}

impl<K: SubscriptionResource> Operation for RetrieveSubscription<'_, K> {
    type Outcome = Subscription<Output, K>;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<Subscription<Output, K>, ResponseError> {
        reader().decode_one(response)
    }
}

impl<K: SubscriptionResource> BuildsRequest for RetrieveSubscription<'_, K> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        let mut query = QueryString::default();
        self.system_attributes.push_query(&mut query);
        RequestParts::resource(Method::GET, ResourcePath::subscription(K::COLLECTION, self.id), query)
            .accepting(NgsiLdMediaType::Json)
            .into_request(endpoint, scope)
    }
}
