//! The subscription operations (clauses 5.8, 6.10 and 6.11).

use crate::{
    fixtures::{context, endpoint, endpoint_with, header_value, link_of, response},
    links::{context_link, link_field, link_value},
};
use bytes::Bytes;
use headers::Header;
use http::{Method, Request, StatusCode, header};
use iri_rs::IriRefBuf;
use ngsi_ld::{
    http::{
        BodyError,
        ContextDelivery,
        Page,
        RequestError,
        ResourceCreated,
        ResponseContext,
        ResponseError,
        headers::{NgsiLdResultsCount, NgsiLdTenant, OptionalHeader, RelationType},
        operation::{
            ArgumentError,
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
    model::{DateTime, DocumentError, ErrorType, Input, MemberError, Output, Tenant},
    subscription::{Activation, EntitySubscription, Problem as SubscriptionProblem, ReadErrorKind, Subscription, SubscriptionFragment, SubscriptionId},
};
use pretty_assertions::assert_eq;
use std::str;
use url::Url;

/// `S`.
const S: &str = "urn:ngsi-ld:Subscription:mySubscription";

/// The text of `vehicles`: a subscription to every vehicle, with the mandatory members only.
const VEHICLES: &str = r#"{"type":"Subscription","entities":[{"type":"Vehicle"}],"notification":{"endpoint":{"uri":"http://my.endpoint.org/notify"}}}"#;

/// `S` as a broker returns it, with its notification status and its system-generated members.
const RETRIEVED: &str = concat!(
    r#"{"id":"urn:ngsi-ld:Subscription:mySubscription","type":"Subscription","entities":[{"type":"Vehicle"}],"#,
    r#""notification":{"format":"normalized","endpoint":{"uri":"http://my.endpoint.org/notify"},"status":"ok","timesSent":3,"timesFailed":1,"#,
    r#""lastNotification":"2024-05-01T10:00:00Z","lastSuccess":"2024-05-01T10:00:00Z","lastFailure":"2024-04-30T10:00:00Z"},"#,
    r#""status":"active","createdAt":"2024-04-01T10:00:00Z","modifiedAt":"2024-04-02T10:00:00Z"}"#
);

/// The body of `pause` on `S`.
const PAUSED: &str = r#"{"id":"urn:ngsi-ld:Subscription:mySubscription","type":"Subscription","isActive":false,"expiresAt":"2024-05-01T10:00:00Z"}"#;

/// `vehicles`.
fn vehicles() -> Subscription<Input> {
    Subscription::parse(VEHICLES).unwrap()
}

/// `pause`: a fragment that pauses the subscription and sets its expiry.
fn pause() -> SubscriptionFragment {
    SubscriptionFragment::builder()
        .activation(Activation::Paused)
        .expires_at("2024-05-01T10:00:00Z".parse::<DateTime>().unwrap())
        .build()
        .unwrap()
}

/// Parses a subscription id.
fn id(text: &str) -> SubscriptionId {
    text.parse().unwrap()
}

/// The path of `id` below the API root.
fn subscription_path(encoded: &str) -> String {
    format!("/ngsi-ld/v1/subscriptions/{encoded}")
}

/// The text of a request body.
fn body_text(request: &Request<Bytes>) -> &str {
    str::from_utf8(request.body()).unwrap()
}

/// The 201 answer to Create with the `Location` given.
fn created(location: &str) -> ResourceCreated<SubscriptionId> {
    CreateSubscription { subscription: &vehicles() }
        .decode(&context(), response(201, &[(&header::LOCATION, location)], ""))
        .unwrap()
}

/// The reader error inside a body error.
fn subscription_error(error: ResponseError) -> DocumentError<ReadErrorKind<SubscriptionProblem>> {
    let ResponseError::Body { source, .. } = error else {
        panic!("expected a body error, got {error:?}")
    };
    let BodyError::Subscription(error) = *source else {
        panic!("expected a subscription error, got {source:?}")
    };
    error
}

#[test]
fn create_posts_the_subscription_with_the_json_headers_and_no_accept() {
    let request = CreateSubscription { subscription: &vehicles() }.to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::POST);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/subscriptions");
    assert_eq!(request.uri().query(), None);
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(header_value(&request, &header::CONTENT_LENGTH), Some(request.body().len().to_string().as_str()));
    assert_eq!(header_value(&request, &header::ACCEPT), None);
    assert_eq!(body_text(&request), VEHICLES);
}

#[test]
fn under_ld_json_body_the_subscription_carries_the_context_last() {
    let request = CreateSubscription { subscription: &vehicles() }
        .to_request(&endpoint_with(ContextDelivery::LdJsonBody))
        .unwrap();
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/ld+json"));
    assert_eq!(link_of(&request), None);
    let expected = format!(r#"{},"@context":"https://example.org/context.jsonld"}}"#, VEHICLES.strip_suffix('}').unwrap());
    assert_eq!(body_text(&request), expected);
}

#[test]
fn the_context_a_subscription_was_read_with_is_not_sent() {
    let subscription = Subscription::<Input>::parse(&format!(
        r#"{{"id":"{S}",{},"@context":"https://example.org/other.jsonld"}}"#,
        VEHICLES.strip_prefix('{').unwrap().strip_suffix('}').unwrap()
    ))
    .unwrap();
    let request = CreateSubscription { subscription: &subscription }.to_request(&endpoint()).unwrap();
    assert_eq!(body_text(&request), format!(r#"{{"id":"{S}",{}"#, VEHICLES.strip_prefix('{').unwrap()));
    assert_eq!(link_of(&request), Some(context_link()));
}

#[test]
fn update_patches_the_subscription_with_its_id_before_the_fragment() {
    let request = UpdateSubscription {
        id: &id(S),
        fragment: &pause(),
    }
    .to_request(&endpoint())
    .unwrap();
    assert_eq!(request.method(), Method::PATCH);
    assert_eq!(request.uri().path(), subscription_path(S));
    assert_eq!(request.uri().query(), None);
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(header_value(&request, &header::ACCEPT), None);
    assert_eq!(body_text(&request), PAUSED);
}

#[test]
fn under_ld_json_body_the_update_carries_the_context_last() {
    let request = UpdateSubscription {
        id: &id(S),
        fragment: &pause(),
    }
    .to_request(&endpoint_with(ContextDelivery::LdJsonBody))
    .unwrap();
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/ld+json"));
    assert_eq!(link_of(&request), None);
    let expected = format!(r#"{},"@context":"https://example.org/context.jsonld"}}"#, PAUSED.strip_suffix('}').unwrap());
    assert_eq!(body_text(&request), expected);
}

#[test]
fn retrieve_gets_the_subscription_asking_for_json_with_or_without_system_attributes() {
    let subscription = id(S);
    let request = RetrieveSubscription::builder()
        .id(&subscription)
        .kind(EntitySubscription)
        .build()
        .to_request(&endpoint())
        .unwrap();
    assert_eq!(request.method(), Method::GET);
    assert_eq!(request.uri().path(), subscription_path(S));
    assert_eq!(request.uri().query(), None);
    assert_eq!(header_value(&request, &header::ACCEPT), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), None);
    assert!(request.body().is_empty());
    let request = RetrieveSubscription::builder()
        .id(&subscription)
        .system_attributes(SystemAttributes::Include)
        .kind(EntitySubscription)
        .build()
        .to_request(&endpoint())
        .unwrap();
    assert_eq!(request.uri().query(), Some("options=sysAttrs"));
}

#[test]
fn an_id_with_slash_and_hash_is_one_encoded_path_segment() {
    let subscription = id("https://example.org/subscriptions/1#a");
    let expected = subscription_path("https:%2F%2Fexample.org%2Fsubscriptions%2F1%23a");
    let fragment = pause();
    let requests = [
        RetrieveSubscription::builder()
            .id(&subscription)
            .kind(EntitySubscription)
            .build()
            .to_request(&endpoint())
            .unwrap(),
        UpdateSubscription {
            id: &subscription,
            fragment: &fragment,
        }
        .to_request(&endpoint())
        .unwrap(),
        DeleteSubscription {
            id: &subscription,
            kind: EntitySubscription,
        }
        .to_request(&endpoint())
        .unwrap(),
    ];
    for request in requests {
        assert_eq!(request.uri().path(), expected);
    }
}

#[test]
fn query_subscriptions_gets_the_list_with_its_paging_parameters() {
    let request = QuerySubscriptions::<EntitySubscription>::default().to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::GET);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/subscriptions");
    assert_eq!(request.uri().query(), None);
    assert_eq!(header_value(&request, &header::ACCEPT), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    let request = QuerySubscriptions::builder()
        .system_attributes(SystemAttributes::Include)
        .limit(10)
        .offset(20)
        .count(ResultsCount::Include)
        .kind(EntitySubscription)
        .build()
        .to_request(&endpoint())
        .unwrap();
    assert_eq!(request.uri().query(), Some("options=sysAttrs&limit=10&offset=20&count=true"));
}

#[test]
fn a_limit_of_zero_needs_count() {
    let error = QuerySubscriptions {
        limit: Some(0),
        ..QuerySubscriptions::<EntitySubscription>::default()
    }
    .to_request(&endpoint())
    .unwrap_err();
    assert!(matches!(error, RequestError::Argument(ArgumentError::ZeroLimitWithoutCount)), "{error:?}");
    let request = QuerySubscriptions {
        limit: Some(0),
        count: ResultsCount::Include,
        ..QuerySubscriptions::<EntitySubscription>::default()
    }
    .to_request(&endpoint())
    .unwrap();
    assert_eq!(request.uri().query(), Some("limit=0&count=true"));
}

#[test]
fn delete_deletes_the_subscription_with_the_link_header_only() {
    let request = DeleteSubscription {
        id: &id(S),
        kind: EntitySubscription,
    }
    .to_request(&endpoint())
    .unwrap();
    assert_eq!(request.method(), Method::DELETE);
    assert_eq!(request.uri().path(), subscription_path(S));
    assert_eq!(request.uri().query(), None);
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(header_value(&request, &header::ACCEPT), None);
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), None);
    assert!(request.body().is_empty());
}

#[test]
fn every_subscription_operation_names_the_tenant() {
    let tenant: Tenant = "t1".parse().unwrap();
    let subscription = vehicles();
    let target = id(S);
    let fragment = pause();
    let requests = [
        CreateSubscription { subscription: &subscription }.to_request(&endpoint().with_tenant(tenant.clone())),
        UpdateSubscription {
            id: &target,
            fragment: &fragment,
        }
        .to_request(&endpoint().with_tenant(tenant.clone())),
        RetrieveSubscription::builder()
            .id(&target)
            .kind(EntitySubscription)
            .build()
            .to_request(&endpoint().with_tenant(tenant.clone())),
        QuerySubscriptions::<EntitySubscription>::default().to_request(&endpoint().with_tenant(tenant.clone())),
        DeleteSubscription {
            id: &target,
            kind: EntitySubscription,
        }
        .to_request(&endpoint().with_tenant(tenant)),
    ];
    for request in requests {
        assert_eq!(header_value(&request.unwrap(), NgsiLdTenant::name()), Some("t1"));
    }
}

#[test]
fn the_next_page_of_subscriptions_is_requested_on_the_tenant_of_the_query() {
    let tenant: Tenant = "t1".parse().unwrap();
    let query = QuerySubscriptions {
        limit: Some(1),
        ..QuerySubscriptions::<EntitySubscription>::default()
    };
    let request = query.to_request(&endpoint().with_tenant(tenant)).unwrap();
    let page = query
        .decode(
            &ResponseContext::new(&request),
            response(
                200,
                &[(
                    &header::LINK,
                    &link_field([link_value("/ngsi-ld/v1/subscriptions?limit=1&offset=1", RelationType::Next)]),
                )],
                &format!("[{RETRIEVED}]"),
            ),
        )
        .unwrap();
    let next = page.next_page().unwrap().to_request(&endpoint()).unwrap();
    assert_eq!(next.method(), Method::GET);
    assert_eq!(next.uri().to_string(), "http://broker.test/ngsi-ld/v1/subscriptions?limit=1&offset=1");
    assert_eq!(header_value(&next, &header::ACCEPT), Some("application/json"));
    assert_eq!(header_value(&next, NgsiLdTenant::name()), Some("t1"));
}

#[test]
fn a_201_names_the_created_subscription_in_its_location() {
    let outcome = created("/ngsi-ld/v1/subscriptions/urn:ngsi-ld:Subscription:mySubscription");
    assert_eq!(
        outcome.location.valid().map(IriRefBuf::as_str),
        Some("/ngsi-ld/v1/subscriptions/urn:ngsi-ld:Subscription:mySubscription")
    );
    assert_eq!(outcome.id(), Some(id(S)));
}

#[test]
fn the_id_is_the_last_path_segment_percent_decoded() {
    assert_eq!(
        created("/ngsi-ld/v1/subscriptions/https:%2F%2Fexample.org%2Fs%2F1").id(),
        Some(id("https://example.org/s/1"))
    );
    assert_eq!(
        created("http://broker.test/ngsi-ld/v1/subscriptions/urn:ngsi-ld:Subscription:mySubscription?x=1#f").id(),
        Some(id(S))
    );
}

#[test]
fn a_missing_unreadable_or_empty_location_names_no_subscription() {
    let absent = CreateSubscription { subscription: &vehicles() }
        .decode(&context(), response(201, &[], ""))
        .unwrap();
    assert_eq!(absent.location, OptionalHeader::Absent);
    assert_eq!(absent.id(), None);
    let malformed = created("a b");
    assert!(matches!(malformed.location, OptionalHeader::Malformed(_)), "{malformed:?}");
    assert_eq!(malformed.id(), None);
    let root = created("/");
    assert!(root.location.valid().is_some());
    assert_eq!(root.id(), None);
}

#[test]
fn create_answers_only_201() {
    for status in [200, 204, 207] {
        let error = CreateSubscription { subscription: &vehicles() }
            .decode(&context(), response(status, &[], ""))
            .unwrap_err();
        assert!(
            matches!(error, ResponseError::UnexpectedStatus { status: found, .. } if found.as_u16() == status),
            "{error:?}"
        );
    }
}

#[test]
fn create_reports_already_exists_and_bad_request_data() {
    for (status, kind, error_type) in [
        (409, "AlreadyExists", ErrorType::AlreadyExists),
        (400, "BadRequestData", ErrorType::BadRequestData),
    ] {
        let body = format!(r#"{{"type":"https://uri.etsi.org/ngsi-ld/errors/{kind}","title":"{kind}"}}"#);
        let error = CreateSubscription { subscription: &vehicles() }
            .decode(&context(), response(status, &[], &body))
            .unwrap_err();
        assert!(matches!(error, ResponseError::Problem { details, .. } if details.error_type == Some(error_type.clone())));
    }
}

#[test]
fn a_retrieved_subscription_is_read_with_its_output_members() {
    let subscription = id(S);
    let retrieved = RetrieveSubscription::builder()
        .id(&subscription)
        .kind(EntitySubscription)
        .build()
        .decode(&context(), response(200, &[], RETRIEVED))
        .unwrap();
    assert_eq!(retrieved, Subscription::<Output>::parse(RETRIEVED).unwrap());
    assert_eq!(retrieved.render().to_string(), RETRIEVED);
}

#[test]
fn a_retrieved_body_that_is_not_a_subscription_is_a_body_error() {
    let subscription = id(S);
    let retrieve = RetrieveSubscription::builder().id(&subscription).kind(EntitySubscription).build();
    let without_id = RETRIEVED.replacen(r#""id":"urn:ngsi-ld:Subscription:mySubscription","#, "", 1);
    let error = subscription_error(retrieve.decode(&context(), response(200, &[], &without_id)).unwrap_err());
    assert!(
        matches!(error.kind, ReadErrorKind::Member(MemberError::MissingMember { member: "id" })),
        "{error:?}"
    );
    let with_foo = RETRIEVED.replacen(r#""status":"active","#, r#""status":"active","foo":1,"#, 1);
    let error = subscription_error(retrieve.decode(&context(), response(200, &[], &with_foo)).unwrap_err());
    assert!(
        matches!(&error.kind, ReadErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "foo"),
        "{error:?}"
    );
    let error = retrieve.decode(&context(), response(200, &[], "not json")).unwrap_err();
    assert!(
        matches!(&error, ResponseError::Body { source, .. } if matches!(**source, BodyError::Json(_))),
        "{error:?}"
    );
}

#[test]
fn retrieve_update_and_delete_report_resource_not_found() {
    let subscription = id(S);
    let fragment = pause();
    let body = r#"{"type":"https://uri.etsi.org/ngsi-ld/errors/ResourceNotFound","title":"Not found"}"#;
    let errors = [
        RetrieveSubscription::builder()
            .id(&subscription)
            .kind(EntitySubscription)
            .build()
            .decode(&context(), response(404, &[], body))
            .unwrap_err(),
        UpdateSubscription {
            id: &subscription,
            fragment: &fragment,
        }
        .decode(&context(), response(404, &[], body))
        .unwrap_err(),
        DeleteSubscription {
            id: &subscription,
            kind: EntitySubscription,
        }
        .decode(&context(), response(404, &[], body))
        .unwrap_err(),
    ];
    for error in errors {
        assert!(
            matches!(&error, ResponseError::Problem { status: StatusCode::NOT_FOUND, details, .. } if details.error_type == Some(ErrorType::ResourceNotFound)),
            "{error:?}"
        );
    }
}

#[test]
fn update_and_delete_answer_only_204_and_retrieve_only_200() {
    let subscription = id(S);
    let fragment = pause();
    let update = UpdateSubscription {
        id: &subscription,
        fragment: &fragment,
    };
    let delete = DeleteSubscription {
        id: &subscription,
        kind: EntitySubscription,
    };
    let retrieve = RetrieveSubscription::builder().id(&subscription).kind(EntitySubscription).build();
    update.decode(&context(), response(204, &[], "")).unwrap();
    delete.decode(&context(), response(204, &[], "")).unwrap();
    let errors = [
        update.decode(&context(), response(200, &[], "")).unwrap_err(),
        delete.decode(&context(), response(200, &[], "")).unwrap_err(),
        retrieve.decode(&context(), response(201, &[], RETRIEVED)).unwrap_err(),
        retrieve.decode(&context(), response(204, &[], "")).unwrap_err(),
    ];
    for error in errors {
        assert!(matches!(error, ResponseError::UnexpectedStatus { .. }), "{error:?}");
    }
}

#[test]
fn a_page_of_subscriptions_keeps_their_order_its_links_and_its_total() {
    let second = RETRIEVED.replacen("mySubscription", "otherSubscription", 1);
    let page: Page<Subscription<Output>> = QuerySubscriptions::<EntitySubscription>::default()
        .decode(
            &context(),
            response(
                200,
                &[
                    (
                        &header::LINK,
                        &link_field([
                            link_value("/ngsi-ld/v1/subscriptions?limit=2&offset=2", RelationType::Next),
                            link_value("/ngsi-ld/v1/subscriptions?limit=2&offset=0", RelationType::Prev),
                        ]),
                    ),
                    (NgsiLdResultsCount::name(), "5"),
                ],
                &format!("[{RETRIEVED},{second}]"),
            ),
        )
        .unwrap();
    let ids: Vec<&str> = page.items().iter().map(|subscription| subscription.id().as_str()).collect();
    assert_eq!(ids, [S, "urn:ngsi-ld:Subscription:otherSubscription"]);
    assert_eq!(
        page.next().map(Url::as_str),
        Some("http://broker.test/ngsi-ld/v1/subscriptions?limit=2&offset=2")
    );
    assert_eq!(
        page.prev().map(Url::as_str),
        Some("http://broker.test/ngsi-ld/v1/subscriptions?limit=2&offset=0")
    );
    assert_eq!(page.total(), Some(5));
}

#[test]
fn a_query_answer_that_is_not_a_list_of_subscriptions_is_refused() {
    let query = QuerySubscriptions::<EntitySubscription>::default();
    let error = query.decode(&context(), response(200, &[], "{}")).unwrap_err();
    assert!(
        matches!(&error, ResponseError::Body { source, .. } if matches!(**source, BodyError::NotAnArray)),
        "{error:?}"
    );
    let without_notification = VEHICLES.replacen(r#","notification":{"endpoint":{"uri":"http://my.endpoint.org/notify"}}"#, "", 1);
    let second = format!(r#"{{"id":"urn:ngsi-ld:Subscription:other",{}"#, without_notification.strip_prefix('{').unwrap());
    let error = subscription_error(query.decode(&context(), response(200, &[], &format!("[{RETRIEVED},{second}]"))).unwrap_err());
    assert!(
        matches!(error.kind, ReadErrorKind::Member(MemberError::MissingMember { member: "notification" })),
        "{error:?}"
    );
    assert_eq!(error.path.to_string(), "1");
    for status in [201, 204] {
        let error = query.decode(&context(), response(status, &[], "[]")).unwrap_err();
        assert!(matches!(error, ResponseError::UnexpectedStatus { .. }), "{error:?}");
    }
}
