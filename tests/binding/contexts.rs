//! The stored `@context` operations (clauses 5.13, 6.29 and 6.30).

use crate::{
    fixtures::{context, endpoint, endpoint_with, header_value, link_of, response},
    links::context_link,
};
use bytes::Bytes;
use headers::Header;
use http::{Method, Request, header};
use iri_rs::{IriBuf, IriRefBuf};
use ngsi_ld::{
    http::{
        BodyError,
        ContextDelivery,
        ResponseError,
        headers::{NgsiLdTenant, OptionalHeader},
        operation::{
            AddContext,
            ContextDeletion,
            ContextKindFilter,
            DeleteContext,
            ListContextDetails,
            ListContexts,
            Operation,
            ServeContext,
            ServeContextDetails,
        },
    },
    model::{Context, ContextEntry, ContextId, DocumentErrorKind, ErrorType, InlineContext, JsonType, MemberError, StoredContextKind, Tenant},
};
use pretty_assertions::assert_eq;
use std::str;
use url::Url;

/// `C`.
const C: &str = "urn:ngsi-ld:context:c1";

/// The body of Add @context with `inline`.
const INLINE_DOCUMENT: &str = r#"{"@context":{"speed":"https://example.org/speed"}}"#;

/// The metadata of a cached remote context, with every member.
const CACHED: &str = concat!(
    r#"{"URL":"https://example.org/context.jsonld","localId":"urn:ngsi-ld:context:c1","kind":"Cached","#,
    r#""createdAt":"2024-05-01T10:00:00Z","expiresAt":"2024-05-02T10:00:00Z","lastUsage":"2024-05-01T11:00:00Z","#,
    r#""numberOfHits":3,"extraInfo":{"etag":"x1"}}"#
);

/// The metadata of a context the broker hosts, with the mandatory members only.
const HOSTED: &str = concat!(
    r#"{"URL":"http://broker.test/ngsi-ld/v1/jsonldContexts/urn:ngsi-ld:context:c1","localId":"urn:ngsi-ld:context:c1","#,
    r#""kind":"Hosted","createdAt":"2024-05-01T10:00:00Z"}"#
);

/// `inline`.
fn inline() -> Context {
    Context::Single(ContextEntry::Inline(
        r#"{"speed":"https://example.org/speed"}"#.parse::<InlineContext>().unwrap(),
    ))
}

/// `C`.
fn c() -> ContextId {
    C.parse().unwrap()
}

/// The text of a request body.
fn body_text(request: &Request<Bytes>) -> &str {
    str::from_utf8(request.body()).unwrap()
}

/// The request an operation builds against the binding endpoint, on the default tenant.
fn built(operation: &impl Operation) -> Request<Bytes> {
    operation.to_request(&endpoint()).unwrap()
}

/// A problem body of type `name`.
fn problem(name: &str) -> String {
    format!(r#"{{"type":"https://uri.etsi.org/ngsi-ld/errors/{name}","title":"{name}"}}"#)
}

/// Whether `error` is a problem of type `expected` with `status`.
fn is_problem(error: &ResponseError, expected: &ErrorType, status: u16) -> bool {
    matches!(error, ResponseError::Problem { status: found, details, .. } if *found == status && details.error_type.as_ref() == Some(expected))
}

#[test]
fn add_posts_the_local_context_as_json_ld_without_a_link_header() {
    let context = inline();
    let request = built(&AddContext::builder().context(&context).build());
    assert_eq!(request.method(), Method::POST);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/jsonldContexts");
    assert_eq!(request.uri().query(), None);
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/ld+json"));
    assert_eq!(link_of(&request), None);
    assert_eq!(header_value(&request, &header::ACCEPT), None);
    assert_eq!(
        header_value(&request, &header::CONTENT_LENGTH),
        Some(INLINE_DOCUMENT.len().to_string().as_str())
    );
    assert_eq!(body_text(&request), INLINE_DOCUMENT);
}

#[test]
fn add_never_adds_the_clients_own_context() {
    let context = inline();
    let with_link = built(&AddContext::builder().context(&context).build());
    let in_body = AddContext::builder()
        .context(&context)
        .build()
        .to_request(&endpoint_with(ContextDelivery::LdJsonBody))
        .unwrap();
    assert_eq!(in_body.headers(), with_link.headers());
    assert_eq!(body_text(&in_body), INLINE_DOCUMENT);
}

#[test]
fn add_is_sent_without_the_profile_an_endpoint_states_for_its_context() {
    let context = inline();
    let add = AddContext::builder().context(&context).build();
    let profile = IriBuf::new("https://uri.etsi.org/ngsi-ld/profile/version").unwrap();
    for delivery in [ContextDelivery::JsonWithLink, ContextDelivery::LdJsonBody, ContextDelivery::MergePatchWithLink] {
        let profiled = endpoint_with(delivery).with_context_profile(profile.clone()).unwrap();
        let request = add.to_request(&profiled).unwrap();
        assert_eq!(request.headers(), built(&add).headers(), "{delivery}");
        assert_eq!(body_text(&request), INLINE_DOCUMENT, "{delivery}");
    }
}

#[test]
fn list_asks_for_urls_or_details_of_one_kind_or_every_kind() {
    let every = built(&ListContexts::default());
    assert_eq!(every.method(), Method::GET);
    assert_eq!(every.uri().path(), "/ngsi-ld/v1/jsonldContexts");
    assert_eq!(every.uri().query(), None);
    assert_eq!(header_value(&every, &header::ACCEPT), Some("application/json"));
    assert_eq!(link_of(&every), Some(context_link()));
    let hosted = ListContexts {
        kind: Some(ContextKindFilter::Hosted),
        ..ListContexts::default()
    };
    assert_eq!(built(&hosted).uri().query(), Some("kind=Hosted"));
    let cached = ListContextDetails::builder().kind(ContextKindFilter::Cached).build();
    assert_eq!(built(&cached).uri().query(), Some("details=true&kind=Cached"));
    assert_eq!(built(&ListContextDetails::default()).uri().query(), Some("details=true"));
}

#[test]
fn serve_names_the_context_in_one_path_segment() {
    let id = c();
    let serve = built(&ServeContext::builder().id(&id).build());
    assert_eq!(serve.method(), Method::GET);
    assert_eq!(serve.uri().path(), "/ngsi-ld/v1/jsonldContexts/urn:ngsi-ld:context:c1");
    assert_eq!(serve.uri().query(), None);
    assert_eq!(header_value(&serve, &header::ACCEPT), Some("application/json"));
    assert_eq!(built(&ServeContextDetails { id: &id }).uri().query(), Some("details=true"));
    let cached: ContextId = "https://example.org/context.jsonld".parse().unwrap();
    assert_eq!(
        built(&ServeContext::builder().id(&cached).build()).uri().path(),
        "/ngsi-ld/v1/jsonldContexts/https:%2F%2Fexample.org%2Fcontext.jsonld"
    );
}

#[test]
fn delete_removes_or_reloads_the_context() {
    let id = c();
    let delete = built(&DeleteContext::builder().id(&id).build());
    assert_eq!(delete.method(), Method::DELETE);
    assert_eq!(delete.uri().path(), "/ngsi-ld/v1/jsonldContexts/urn:ngsi-ld:context:c1");
    assert_eq!(delete.uri().query(), None);
    assert!(delete.body().is_empty());
    assert_eq!(header_value(&delete, &header::ACCEPT), None);
    let reload = built(&DeleteContext::builder().id(&id).deletion(ContextDeletion::Reload).build());
    assert_eq!(reload.uri().query(), Some("reload=true"));
}

#[test]
fn every_stored_context_operation_names_its_tenant() {
    let tenant: Tenant = "t1".parse().unwrap();
    let context = inline();
    let id = c();
    let requests = [
        AddContext::builder()
            .context(&context)
            .build()
            .to_request(&endpoint().with_tenant(tenant.clone())),
        ListContexts::default().to_request(&endpoint().with_tenant(tenant.clone())),
        ServeContext::builder().id(&id).build().to_request(&endpoint().with_tenant(tenant.clone())),
        DeleteContext::builder().id(&id).build().to_request(&endpoint().with_tenant(tenant)),
    ];
    for request in requests {
        assert_eq!(header_value(&request.unwrap(), NgsiLdTenant::name()), Some("t1"));
    }
}

#[test]
fn add_names_the_stored_context_by_the_location_of_its_201() {
    let stored = inline();
    let add = AddContext::builder().context(&stored).build();
    let location = format!("/ngsi-ld/v1/jsonldContexts/{C}");
    let created = add.decode(&context(), response(201, &[(&header::LOCATION, &location)], "")).unwrap();
    assert_eq!(created.location, OptionalHeader::Valid(IriRefBuf::new(location).unwrap()));
    assert_eq!(created.id(), Some(c()));
    let absent = add.decode(&context(), response(201, &[], "")).unwrap();
    assert_eq!(absent.id(), None);
    assert_eq!(absent.location, OptionalHeader::Absent);
    let error = add.decode(&context(), response(200, &[], "")).unwrap_err();
    assert!(matches!(error, ResponseError::UnexpectedStatus { .. }), "{error:?}");
    let error = add
        .decode(
            &context(),
            response(400, &[(&header::CONTENT_TYPE, "application/json")], &problem("BadRequestData")),
        )
        .unwrap_err();
    assert!(is_problem(&error, &ErrorType::BadRequestData, 400), "{error:?}");
}

#[test]
fn list_reads_urls_or_metadata() {
    let urls = ListContexts::default()
        .decode(
            &context(),
            response(
                200,
                &[],
                r#"["https://example.org/context.jsonld","http://broker.test/ngsi-ld/v1/jsonldContexts/urn:ngsi-ld:context:c1"]"#,
            ),
        )
        .unwrap();
    assert_eq!(
        urls,
        [
            Url::parse("https://example.org/context.jsonld").unwrap(),
            Url::parse("http://broker.test/ngsi-ld/v1/jsonldContexts/urn:ngsi-ld:context:c1").unwrap()
        ]
    );
    let error = ListContexts::default().decode(&context(), response(200, &[], "[1]")).unwrap_err();
    let ResponseError::Body { source, .. } = error else {
        panic!("expected a body error, got {error:?}")
    };
    let BodyError::Document(error) = *source else {
        panic!("expected a document error, got {source:?}")
    };
    assert!(matches!(
        error.kind,
        DocumentErrorKind::Member(MemberError::WrongShape {
            expected: JsonType::String,
            ..
        })
    ));
    assert_eq!(error.path.to_string(), "0");
    let details = ListContextDetails::default()
        .decode(&context(), response(200, &[], &format!("[{HOSTED}]")))
        .unwrap();
    assert_eq!(details.len(), 1);
    assert_eq!(details[0].kind, StoredContextKind::Hosted);
}

#[test]
fn serve_reads_the_context_or_its_metadata_and_a_cached_context_is_a_problem() {
    let id = c();
    let served = ServeContext::builder()
        .id(&id)
        .build()
        .decode(&context(), response(200, &[], INLINE_DOCUMENT))
        .unwrap();
    assert_eq!(served, inline());
    let metadata = ServeContextDetails { id: &id }.decode(&context(), response(200, &[], CACHED)).unwrap();
    assert_eq!(metadata.number_of_hits, Some(3));
    let error = ServeContext::builder()
        .id(&id)
        .build()
        .decode(
            &context(),
            response(422, &[(&header::CONTENT_TYPE, "application/json")], &problem("OperationNotSupported")),
        )
        .unwrap_err();
    assert!(is_problem(&error, &ErrorType::OperationNotSupported, 422), "{error:?}");
    let error = ServeContext::builder()
        .id(&id)
        .build()
        .decode(
            &context(),
            response(404, &[(&header::CONTENT_TYPE, "application/json")], &problem("ResourceNotFound")),
        )
        .unwrap_err();
    assert!(is_problem(&error, &ErrorType::ResourceNotFound, 404), "{error:?}");
}

#[test]
fn delete_answers_204_and_a_failed_reload_is_a_problem() {
    let id = c();
    let delete = DeleteContext::builder().id(&id).build();
    delete.decode(&context(), response(204, &[], "")).unwrap();
    let error = delete.decode(&context(), response(200, &[], "")).unwrap_err();
    assert!(matches!(error, ResponseError::UnexpectedStatus { .. }), "{error:?}");
    let reload = DeleteContext::builder().id(&id).deletion(ContextDeletion::Reload).build();
    let error = reload
        .decode(
            &context(),
            response(504, &[(&header::CONTENT_TYPE, "application/json")], &problem("LdContextNotAvailable")),
        )
        .unwrap_err();
    assert!(is_problem(&error, &ErrorType::LdContextNotAvailable, 504), "{error:?}");
}
