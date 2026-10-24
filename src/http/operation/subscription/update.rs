//! Update Subscription (NGSI-LD clauses 5.8.2 and 6.11.3.2) and Update Context Source Registration Subscription
//! (clauses 5.11.3 and 6.13.3.2).

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
    subscription::{EntitySubscription, SubscriptionFragment, SubscriptionId, SubscriptionKind},
};
use bytes::Bytes;
use http::{Method, Request, Response};
use jstrict::{Object, Value as Json};

/// `PATCH /subscriptions/{subscriptionId}` (NGSI-LD clause 6.11.3.2), or
/// `PATCH /csourceSubscriptions/{subscriptionId}` (clause 6.13.3.2) for a fragment of kind
/// [`RegistrationSubscription`](crate::subscription::RegistrationSubscription): applies a fragment to a
/// subscription.
///
/// The body is the fragment preceded by the subscription's `id`: Tables 6.11.3.2-1 and 6.13.3.2-1 describe it as a
/// fragment "including id, type and any other subscription field to be changed". The id is the one the path names,
/// so the fragment never changes it.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, UpdateSubscription},
///     },
///     subscription::{Activation, SubscriptionFragment, SubscriptionId},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = SubscriptionId::from(iri!("urn:ngsi-ld:Subscription:mySubscription"));
/// let pause = SubscriptionFragment::builder()
///     .activation(Activation::Paused)
///     .build()?;
/// let request = UpdateSubscription {
///     id: &id,
///     fragment: &pause,
/// }
/// .to_request(&endpoint)?;
/// assert_eq!(request.method(), "PATCH");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/subscriptions/urn:ngsi-ld:Subscription:mySubscription"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateSubscription<'a, K: SubscriptionKind = EntitySubscription> {
    /// The subscription.
    pub id: &'a SubscriptionId,
    /// The changes.
    pub fragment: &'a SubscriptionFragment<K>,
}

impl<K: SubscriptionKind> Sealed for UpdateSubscription<'_, K> {}

impl<K: SubscriptionResource> Operation for UpdateSubscription<'_, K> {
    type Outcome = ();

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<(), ResponseError> {
        decode_no_content(response)
    }
}

impl<K: SubscriptionResource> BuildsRequest for UpdateSubscription<'_, K> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        let mut body = Object::new();
        body.push("id".into(), Json::from(self.id.as_str()));
        body.extend(self.fragment.to_object());
        RequestParts::resource(Method::PATCH, ResourcePath::subscription(K::COLLECTION, self.id), QueryString::default())
            .with_body(body)
            .into_request(endpoint, scope)
    }
}
