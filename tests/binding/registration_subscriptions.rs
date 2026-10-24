//! The Context Source Registration Subscription operations (clauses 5.11, 6.12 and 6.13): the subscription
//! operations, bound to `csourceSubscriptions`.

use crate::{
    fixtures::{context, endpoint, endpoint_with, header_value, link_of, response},
    links::{context_link, link_field, link_value},
};
use bytes::Bytes;
use headers::Header;
use http::{Method, Request, header};
use ngsi_ld::{
    http::{
        BodyError,
        ContextDelivery,
        Page,
        ResponseContext,
        ResponseError,
        headers::{NgsiLdResultsCount, NgsiLdTenant, RelationType},
        operation::{
            CreateSubscription,
            DeleteSubscription,
            Operation,
            QuerySubscriptions,
            ResultsCount,
            RetrieveSubscription,
            SystemAttributes,
            UpdateSubscription,
        },
    },
    model::{Input, MemberError, Output, Tenant},
    query::temporal_query::TemporalQuery,
    subscription::{Activation, ReadErrorKind, RegistrationSubscription, Subscription, SubscriptionFragment, SubscriptionId},
};
use pretty_assertions::assert_eq;
use std::str;

/// `RS`.
const RS: &str = "urn:ngsi-ld:Subscription:registrations";

/// The text of `watch`, a Context Source Registration Subscription for the registrations of vehicles.
const WATCH: &str = r#"{"type":"Subscription","entities":[{"type":"Vehicle"}],"notification":{"endpoint":{"uri":"http://my.endpoint.org/registrations"}}}"#;

/// `RS` as a broker returns it, with a temporal query.
const RETRIEVED: &str = concat!(
    r#"{"id":"urn:ngsi-ld:Subscription:registrations","type":"Subscription","entities":[{"type":"Vehicle"}],"#,
    r#""temporalQ":{"timerel":"after","timeAt":"2024-05-01T10:00:00Z"},"#,
    r#""notification":{"endpoint":{"uri":"http://my.endpoint.org/registrations"}},"status":"active"}"#
);

/// `watch`.
fn watch() -> Subscription<Input, RegistrationSubscription> {
    Subscription::parse(WATCH).unwrap()
}

/// The path of `RS` below the API root.
fn watch_path() -> String {
    format!("/ngsi-ld/v1/csourceSubscriptions/{RS}")
}

/// Parses a subscription id.
fn id(text: &str) -> SubscriptionId {
    text.parse().unwrap()
}

/// The text of a request body.
fn body_text(request: &Request<Bytes>) -> &str {
    str::from_utf8(request.body()).unwrap()
}

/// `widen`: a fragment of a registration subscription setting its temporal query.
fn widen() -> SubscriptionFragment<RegistrationSubscription> {
    SubscriptionFragment::registration_builder()
        .temporal_q(TemporalQuery::after("2024-05-01T10:00:00Z".parse().unwrap()))
        .activation(Activation::Paused)
        .build()
        .unwrap()
}

#[test]
fn create_posts_a_registration_subscription_to_csource_subscriptions() {
    let request = CreateSubscription { subscription: &watch() }.to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::POST);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/csourceSubscriptions");
    assert_eq!(request.uri().query(), None);
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(header_value(&request, &header::ACCEPT), None);
    assert_eq!(body_text(&request), WATCH);
    let request = CreateSubscription { subscription: &watch() }
        .to_request(&endpoint_with(ContextDelivery::LdJsonBody))
        .unwrap();
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/ld+json"));
    let expected = format!(r#"{},"@context":"https://example.org/context.jsonld"}}"#, WATCH.strip_suffix('}').unwrap());
    assert_eq!(body_text(&request), expected);
}

#[test]
fn update_retrieve_query_and_delete_address_csource_subscriptions() {
    let target = id(RS);
    let fragment = widen();
    let update = UpdateSubscription {
        id: &target,
        fragment: &fragment,
    }
    .to_request(&endpoint())
    .unwrap();
    assert_eq!(update.method(), Method::PATCH);
    assert_eq!(update.uri().path(), watch_path());
    assert_eq!(
        body_text(&update),
        concat!(
            r#"{"id":"urn:ngsi-ld:Subscription:registrations","type":"Subscription","#,
            r#""temporalQ":{"timerel":"after","timeAt":"2024-05-01T10:00:00Z"},"isActive":false}"#
        )
    );
    let retrieve = RetrieveSubscription::builder()
        .id(&target)
        .system_attributes(SystemAttributes::Include)
        .kind(RegistrationSubscription)
        .build()
        .to_request(&endpoint())
        .unwrap();
    assert_eq!(retrieve.method(), Method::GET);
    assert_eq!(retrieve.uri().path(), watch_path());
    assert_eq!(retrieve.uri().query(), Some("options=sysAttrs"));
    assert_eq!(header_value(&retrieve, &header::ACCEPT), Some("application/json"));
    let query = QuerySubscriptions::builder()
        .system_attributes(SystemAttributes::Include)
        .limit(10)
        .offset(20)
        .count(ResultsCount::Include)
        .kind(RegistrationSubscription)
        .build()
        .to_request(&endpoint())
        .unwrap();
    assert_eq!(query.uri().path(), "/ngsi-ld/v1/csourceSubscriptions");
    assert_eq!(query.uri().query(), Some("options=sysAttrs&limit=10&offset=20&count=true"));
    let delete = DeleteSubscription {
        id: &target,
        kind: RegistrationSubscription,
    }
    .to_request(&endpoint())
    .unwrap();
    assert_eq!(delete.method(), Method::DELETE);
    assert_eq!(delete.uri().path(), watch_path());
    assert!(delete.body().is_empty());
}

#[test]
fn the_next_page_of_registration_subscriptions_stays_on_the_tenant() {
    let tenant: Tenant = "t1".parse().unwrap();
    let query = QuerySubscriptions {
        limit: Some(1),
        ..QuerySubscriptions::<RegistrationSubscription>::default()
    };
    let request = query.to_request(&endpoint().with_tenant(tenant)).unwrap();
    assert_eq!(header_value(&request, NgsiLdTenant::name()), Some("t1"));
    let page = query
        .decode(
            &ResponseContext::new(&request),
            response(
                200,
                &[(
                    &header::LINK,
                    &link_field([link_value("/ngsi-ld/v1/csourceSubscriptions?limit=1&offset=1", RelationType::Next)]),
                )],
                &format!("[{RETRIEVED}]"),
            ),
        )
        .unwrap();
    let next = page.next_page().unwrap().to_request(&endpoint()).unwrap();
    assert_eq!(next.uri().to_string(), "http://broker.test/ngsi-ld/v1/csourceSubscriptions?limit=1&offset=1");
    assert_eq!(header_value(&next, &header::ACCEPT), Some("application/json"));
    assert_eq!(header_value(&next, NgsiLdTenant::name()), Some("t1"));
}

#[test]
fn create_and_retrieve_read_a_registration_subscription() {
    let created = CreateSubscription { subscription: &watch() }
        .decode(&context(), response(201, &[(&header::LOCATION, &watch_path())], ""))
        .unwrap();
    assert_eq!(created.id(), Some(id(RS)));
    let target = id(RS);
    let retrieve = RetrieveSubscription::builder().id(&target).kind(RegistrationSubscription).build();
    let retrieved: Subscription<Output, RegistrationSubscription> = retrieve.decode(&context(), response(200, &[], RETRIEVED)).unwrap();
    assert_eq!(retrieved.id(), &target);
    assert_eq!(retrieved.temporal_q(), Some(&TemporalQuery::after("2024-05-01T10:00:00Z".parse().unwrap())));
    let with_csf = RETRIEVED.replacen(r#""status":"active""#, r#""csf":"mode==\"inclusive\"","status":"active""#, 1);
    let error = retrieve.decode(&context(), response(200, &[], &with_csf)).unwrap_err();
    let ResponseError::Body { source, .. } = error else {
        panic!("expected a body error, got {error:?}")
    };
    let BodyError::Subscription(error) = *source else {
        panic!("expected a subscription error, got {source:?}")
    };
    assert!(
        matches!(&error.kind, ReadErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "csf"),
        "{error:?}"
    );
}

#[test]
fn a_query_reads_a_page_of_registration_subscriptions() {
    let page: Page<Subscription<Output, RegistrationSubscription>> = QuerySubscriptions::<RegistrationSubscription>::default()
        .decode(&context(), response(200, &[(NgsiLdResultsCount::name(), "1")], &format!("[{RETRIEVED}]")))
        .unwrap();
    assert_eq!(page.items().len(), 1);
    assert_eq!(page.items()[0].id().as_str(), RS);
    assert_eq!(page.total(), Some(1));
}
