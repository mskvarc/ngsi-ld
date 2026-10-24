//! Delete Subscription (NGSI-LD clauses 5.8.5 and 6.11.3.3) and Delete Context Source Registration Subscription
//! (clauses 5.11.6 and 6.13.3.3).

use crate::{
    http::{
        Endpoint,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            Operation,
            no_content::decode_no_content,
            sealed::{BuildsRequest, Sealed},
            subscription::resource::SubscriptionResource,
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
    },
    subscription::{EntitySubscription, SubscriptionId, SubscriptionKind},
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// `DELETE /subscriptions/{subscriptionId}` (NGSI-LD clause 6.11.3.3), or
/// `DELETE /csourceSubscriptions/{subscriptionId}` (clause 6.13.3.3) for the kind
/// [`RegistrationSubscription`](crate::subscription::RegistrationSubscription): deletes a subscription,
/// which notifies no more.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{DeleteSubscription, Operation},
///     },
///     subscription::{RegistrationSubscription, SubscriptionId},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = SubscriptionId::from(iri!("urn:ngsi-ld:Subscription:registrations"));
/// let delete = DeleteSubscription {
///     id: &id,
///     kind: RegistrationSubscription,
/// };
/// let request = delete.to_request(&endpoint)?;
/// assert_eq!(request.method(), "DELETE");
/// assert_eq!(
///     request.uri(),
///     concat!(
///         "http://broker.test/ngsi-ld/v1/csourceSubscriptions",
///         "/urn:ngsi-ld:Subscription:registrations"
///     )
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteSubscription<'a, K: SubscriptionKind = EntitySubscription> {
    /// The subscription.
    pub id: &'a SubscriptionId,
    /// The kind of subscription, which decides the resource.
    pub kind: K,
}

impl<K: SubscriptionKind> Sealed for DeleteSubscription<'_, K> {}

impl<K: SubscriptionResource> Operation for DeleteSubscription<'_, K> {
    type Outcome = ();

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<(), ResponseError> {
        decode_no_content(response)
    }
}

impl<K: SubscriptionResource> BuildsRequest for DeleteSubscription<'_, K> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        RequestParts::resource(Method::DELETE, ResourcePath::subscription(K::COLLECTION, self.id), QueryString::default()).into_request(endpoint, scope)
    }
}
