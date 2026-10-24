//! Query Subscriptions (NGSI-LD clauses 5.8.4 and 6.10.3.2) and Query Context Source Registration Subscriptions
//! (clauses 5.11.5 and 6.12.3.2), and the requests for their following pages.

use crate::{
    http::{
        Endpoint,
        Page,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            Operation,
            ResultsCount,
            SystemAttributes,
            parameters::{check_limit, push_paging},
            sealed::{BuildsRequest, Sealed},
            subscription::{reader, resource::SubscriptionResource},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
    },
    model::{NgsiLdMediaType, Output},
    subscription::{EntitySubscription, Subscription, SubscriptionKind},
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// `GET /subscriptions` (NGSI-LD clause 6.10.3.2), or `GET /csourceSubscriptions` (clause 6.12.3.2) for the kind
/// [`RegistrationSubscription`](crate::subscription::RegistrationSubscription): lists the subscriptions, one
/// page at a time.
///
/// Tables 6.10.3.2-1 and 6.12.3.2-1 list `limit`; clause 4.12 makes both queries paginated operations, so they also
/// take `offset` (Table 6.3.10-2) and `count` (Table 6.3.13-1), and clause 6.3.11 adds `options=sysAttrs`.
/// `limit=0` without `count` is refused before sending, as clause 6.3.13 makes it a `BadRequest`.
///
/// ```
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, QuerySubscriptions, ResultsCount},
///     },
///     subscription::EntitySubscription,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let first_ten = QuerySubscriptions::builder()
///     .limit(10)
///     .count(ResultsCount::Include)
///     .kind(EntitySubscription)
///     .build();
/// let request = first_ten.to_request(&endpoint)?;
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/subscriptions?limit=10&count=true"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, bon::Builder)]
pub struct QuerySubscriptions<K: SubscriptionKind = EntitySubscription> {
    /// Whether `createdAt` and `modifiedAt` are included (clause 6.3.11).
    #[builder(default)]
    pub system_attributes: SystemAttributes,
    /// The page size (clause 6.3.10).
    pub limit: Option<u32>,
    /// The index of the first subscription of the page (clause 6.3.10).
    pub offset: Option<u32>,
    /// Whether the total is reported (clause 6.3.13).
    #[builder(default)]
    pub count: ResultsCount,
    /// The kind of subscription, which decides the resource and how the answer is read.
    pub kind: K,
}

impl<K: SubscriptionKind> Sealed for QuerySubscriptions<K> {}

impl<K: SubscriptionResource> Operation for QuerySubscriptions<K> {
    type Outcome = Page<Subscription<Output, K>>;

    fn decode(&self, context: &ResponseContext, response: Response<Bytes>) -> Result<Page<Subscription<Output, K>>, ResponseError> {
        reader().decode_page(context, response)
    }
}

impl<K: SubscriptionResource> BuildsRequest for QuerySubscriptions<K> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        check_limit(self.limit, self.count)?;
        let mut query = QueryString::default();
        self.system_attributes.push_query(&mut query);
        push_paging(&mut query, self.limit, self.offset, self.count);
        RequestParts::resource(Method::GET, ResourcePath::subscriptions(K::COLLECTION), query)
            .accepting(NgsiLdMediaType::Json)
            .into_request(endpoint, scope)
    }
}

page_operation!(
    impl<K: SubscriptionResource> for Subscription<Output, K> => |context, response| reader().decode_page(context, response),
    NgsiLdMediaType::Json
);
