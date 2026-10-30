//! The entity map and source identity operations (clauses 5.14, 5.15 and 6.32 to 6.35).

use crate::{
    fixtures::{context, endpoint, endpoint_with, header_value, link_of, response},
    links::context_link,
};
use bytes::Bytes;
use headers::Header;
use http::{Method, Request, header};
use ngsi_ld::{
    http::{
        BodyError,
        ContextDelivery,
        RequestError,
        ResponseError,
        headers::{NgsiLdEntityMap, NgsiLdTenant, OptionalHeader, PreferenceApplied},
        operation::{
            ArgumentError,
            CreateEntityMap,
            CreateTemporalEntityMap,
            DeleteEntityMap,
            Operation,
            QueryMethod,
            RetrieveEntityMap,
            RetrieveSourceIdentity,
            UpdateEntityMap,
        },
    },
    model::{DateTime, DocumentErrorKind, EntityId, ErrorType, MemberError, OneOrMany, Tenant},
    query::{
        EntitySelector,
        EntitySplitting,
        Join,
        Locality,
        SelectorType,
        geoquery::GeoQuery,
        language_filter::LanguageFilter,
        projection::Projection,
        q::Query,
        scope_query::ScopeQuery,
        temporal_query::TemporalQuery,
    },
    registration::{EntityMapId, EntitySource},
};
use pretty_assertions::assert_eq;
use std::{num::NonZeroU32, str};

/// `M`.
const M: &str = "urn:ngsi-ld:EntityMap:1";

/// `T`.
const T: &str = "2030-01-01T00:00:00Z";

/// `MAP`.
const MAP: &str = concat!(
    r#"{"id":"urn:ngsi-ld:EntityMap:1","type":"EntityMap","expiresAt":"2030-01-01T00:00:00Z","#,
    r#""entityMap":{"urn:ngsi-ld:Vehicle:A4567":["@none","urn:ngsi-ld:ContextSourceRegistration:r1"],"#,
    r#""urn:ngsi-ld:Vehicle:B9211":["urn:ngsi-ld:ContextSourceRegistration:r1"]},"#,
    r#""linkedMaps":{"urn:ngsi-ld:ContextSourceRegistration:r1":"urn:ngsi-ld:EntityMap:remote7"}}"#
);

/// `ID`.
const ID: &str = concat!(
    r#"{"id":"urn:ngsi-ld:ContextSource:cs1","type":"ContextSourceIdentity","contextSourceAlias":"broker-1","#,
    r#""contextSourceUptime":"P1DT2H","contextSourceTimeAt":"2024-05-01T10:00:00Z","#,
    r#""contextSourceExtras":{"version":"1.2","tenants":["t1"]}}"#
);

/// The location of `M`.
const LOCATION: &str = "/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1";

/// `M`.
fn m() -> EntityMapId {
    M.parse().unwrap()
}

/// `T`.
fn t() -> DateTime {
    T.parse().unwrap()
}

/// The type selection `Vehicle`.
fn vehicle_type() -> SelectorType {
    "Vehicle".parse().unwrap()
}

/// A temporal query: after `2024-05-01T10:00:00Z`, the last 5 instances.
fn after_last_five() -> TemporalQuery {
    TemporalQuery::after("2024-05-01T10:00:00Z".parse().unwrap()).with_last_n(5.try_into().unwrap())
}

/// A creation for Query Entities of `entity_type`, with no other parameter.
fn create(entity_type: Option<&SelectorType>) -> CreateEntityMap<'_> {
    CreateEntityMap {
        entity_type,
        ..CreateEntityMap::default()
    }
}

/// A creation for Query Temporal Evolution of `entity_type` over `temporal_q`, with no other parameter.
fn create_temporal<'a>(entity_type: Option<&'a SelectorType>, temporal_q: &'a TemporalQuery) -> CreateTemporalEntityMap<'a> {
    CreateTemporalEntityMap::builder().maybe_entity_type(entity_type).temporal_q(temporal_q).build()
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

/// The kind of the document error a body error holds.
fn document_error(error: ResponseError) -> DocumentErrorKind {
    let ResponseError::Body { source, .. } = error else {
        panic!("expected a body error, got {error:?}")
    };
    let BodyError::Document(error) = *source else {
        panic!("expected a document error, got {source:?}")
    };
    error.kind
}

/// Whether `error` is an unexpected success status.
fn is_unexpected(error: &ResponseError) -> bool {
    matches!(error, ResponseError::UnexpectedStatus { .. })
}

#[test]
fn retrieve_gets_the_map_resource() {
    let id = m();
    let request = built(&RetrieveEntityMap { id: &id });
    assert_eq!(request.method(), Method::GET);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1");
    assert_eq!(request.uri().query(), None);
    assert_eq!(header_value(&request, &header::ACCEPT), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), None);
    assert!(request.body().is_empty());
}

#[test]
fn update_patches_the_id_type_and_expiry() {
    let (id, expires_at) = (m(), t());
    let request = built(&UpdateEntityMap {
        id: &id,
        expires_at: &expires_at,
    });
    assert_eq!(request.method(), Method::PATCH);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1");
    assert_eq!(request.uri().query(), None);
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(header_value(&request, &header::ACCEPT), None);
    assert_eq!(
        body_text(&request),
        r#"{"id":"urn:ngsi-ld:EntityMap:1","type":"EntityMap","expiresAt":"2030-01-01T00:00:00Z"}"#
    );
}

#[test]
fn update_under_ld_json_carries_the_context_in_the_body() {
    let (id, expires_at) = (m(), t());
    let request = UpdateEntityMap {
        id: &id,
        expires_at: &expires_at,
    }
    .to_request(&endpoint_with(ContextDelivery::LdJsonBody))
    .unwrap();
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/ld+json"));
    assert_eq!(link_of(&request), None);
    assert_eq!(
        body_text(&request),
        r#"{"id":"urn:ngsi-ld:EntityMap:1","type":"EntityMap","expiresAt":"2030-01-01T00:00:00Z","@context":"https://example.org/context.jsonld"}"#
    );
}

#[test]
fn delete_removes_the_map_and_every_operation_encodes_the_id_as_one_segment() {
    let id = m();
    let request = built(&DeleteEntityMap { id: &id });
    assert_eq!(request.method(), Method::DELETE);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1");
    assert_eq!(header_value(&request, &header::ACCEPT), None);
    assert_eq!(link_of(&request), Some(context_link()));
    assert!(request.body().is_empty());
    let odd: EntityMapId = "https://example.org/maps/1#a".parse().unwrap();
    let expires_at = t();
    for request in [
        built(&RetrieveEntityMap { id: &odd }),
        built(&UpdateEntityMap {
            id: &odd,
            expires_at: &expires_at,
        }),
        built(&DeleteEntityMap { id: &odd }),
    ] {
        assert_eq!(request.uri().path(), "/ngsi-ld/v1/entityMaps/https:%2F%2Fexample.org%2Fmaps%2F1%23a");
    }
}

#[test]
fn a_get_creation_sends_the_query_parameters_and_never_the_ignored_ones() {
    let entity_type = vehicle_type();
    let q: Query = "speed>50".parse().unwrap();
    let request = built(&CreateEntityMap {
        q: Some(&q),
        lifetime: Some("PT1H".parse().unwrap()),
        ..create(Some(&entity_type))
    });
    assert_eq!(request.method(), Method::GET);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/entityMaps");
    assert_eq!(request.uri().query(), Some("type=Vehicle&q=speed%3E50&entityMapLifetime=PT1H"));
    assert_eq!(header_value(&request, &header::ACCEPT), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(header_value(&request, NgsiLdEntityMap::name()), None);
}

#[test]
fn a_post_creation_sends_a_query_body() {
    let entity_type = vehicle_type();
    let q: Query = "speed>50".parse().unwrap();
    let request = built(&CreateEntityMap {
        q: Some(&q),
        lifetime: Some("PT1H".parse().unwrap()),
        method: QueryMethod::Post,
        ..create(Some(&entity_type))
    });
    assert_eq!(request.method(), Method::POST);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/entityMaps");
    assert_eq!(request.uri().query(), None);
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/json"));
    assert_eq!(header_value(&request, &header::ACCEPT), Some("application/json"));
    assert_eq!(
        body_text(&request),
        r#"{"type":"Query","entities":[{"type":"Vehicle"}],"q":"speed>50","entityMapLifetime":"PT1H"}"#
    );
}

#[test]
fn a_post_creation_carries_several_selectors_each_as_the_caller_spelled_it() {
    let selectors = [
        EntitySelector::new("Vehicle".parse().unwrap()).with_ids(OneOrMany::try_from(vec!["urn:ngsi-ld:Vehicle:A4567".parse::<EntityId>().unwrap()]).unwrap()),
        EntitySelector::new("Car".parse().unwrap()),
    ];
    let request = built(&CreateEntityMap {
        selectors: &selectors,
        method: QueryMethod::Post,
        ..CreateEntityMap::default()
    });
    assert_eq!(request.uri().query(), None);
    assert_eq!(
        body_text(&request),
        r#"{"type":"Query","entities":[{"type":"Vehicle","id":["urn:ngsi-ld:Vehicle:A4567"]},{"type":"Car"}]}"#
    );
}

#[test]
fn a_get_creation_carries_one_selector_given_alone_and_refuses_more() {
    let one = [EntitySelector::new("Vehicle".parse().unwrap())];
    let alone = CreateEntityMap {
        selectors: &one,
        ..CreateEntityMap::default()
    };
    assert_eq!(built(&alone).uri().query(), Some("type=Vehicle"));
    let two = [EntitySelector::new("Car".parse().unwrap()), EntitySelector::new("Bus".parse().unwrap())];
    let several = CreateEntityMap {
        selectors: &two,
        ..CreateEntityMap::default()
    };
    assert!(matches!(
        several.to_request(&endpoint()),
        Err(RequestError::Argument(ArgumentError::SeveralSelectorsInQueryString))
    ));
}

#[test]
fn a_temporal_creation_carries_several_selectors_in_a_post_body_and_refuses_them_in_a_get() {
    let temporal_q = after_last_five();
    let selectors = [EntitySelector::new("Car".parse().unwrap()), EntitySelector::new("Bus".parse().unwrap())];
    let several = CreateTemporalEntityMap {
        selectors: &selectors,
        ..create_temporal(None, &temporal_q)
    };
    assert!(matches!(
        several.to_request(&endpoint()),
        Err(RequestError::Argument(ArgumentError::SeveralSelectorsInQueryString))
    ));
    let request = built(&CreateTemporalEntityMap {
        method: QueryMethod::Post,
        ..several
    });
    assert_eq!(
        body_text(&request),
        concat!(
            r#"{"type":"Query","entities":[{"type":"Car"},{"type":"Bus"}],"#,
            r#""temporalQ":{"timerel":"after","timeAt":"2024-05-01T10:00:00Z","lastN":5}}"#
        )
    );
}

#[test]
fn a_get_creation_encodes_every_parameter_it_takes() {
    let ids: [EntityId; 2] = ["urn:ngsi-ld:Vehicle:A4567".parse().unwrap(), "urn:ngsi-ld:Vehicle:B9211".parse().unwrap()];
    let geo_q = GeoQuery::parse(r#"{"georel":"near;maxDistance==2000","geometry":"Point","coordinates":[8,40]}"#).unwrap();
    let scope_q: ScopeQuery = "/Madrid".parse().unwrap();
    let pick: Projection = "speed".parse().unwrap();
    let dataset_ids = ["@none".parse().unwrap()];
    let lang: LanguageFilter = "en".parse().unwrap();
    let create = CreateEntityMap {
        ids: &ids,
        geo_q: Some(&geo_q),
        scope_q: Some(&scope_q),
        pick: Some(&pick),
        dataset_ids: &dataset_ids,
        lang: Some(&lang),
        join: Some(Join::Flat),
        join_level: NonZeroU32::new(2),
        split_entities: EntitySplitting::Split,
        locality: Locality::Local,
        ..CreateEntityMap::default()
    };
    assert_eq!(
        built(&create).uri().query(),
        Some(concat!(
            "id=urn%3Angsi-ld%3AVehicle%3AA4567,urn%3Angsi-ld%3AVehicle%3AB9211&georel=near;maxDistance==2000&geometry=Point",
            "&coordinates=%5B8%2C40%5D&scopeQ=/Madrid&pick=speed&datasetId=%40none&lang=en&join=flat&joinLevel=2",
            "&splitEntities=true&local=true"
        ))
    );
}

#[test]
fn an_unbounded_creation_is_refused_and_a_join_level_without_a_join_is_sent() {
    let unbounded = CreateEntityMap::default().to_request(&endpoint());
    assert!(matches!(unbounded, Err(RequestError::Argument(ArgumentError::UnboundedQuery))));
    let entity_type = vehicle_type();
    let levelled = CreateEntityMap {
        join_level: NonZeroU32::new(2),
        ..create(Some(&entity_type))
    };
    assert!(
        levelled
            .to_request(&endpoint())
            .unwrap()
            .uri()
            .query()
            .is_some_and(|query| query.contains("joinLevel=2"))
    );
    let local = CreateEntityMap {
        locality: Locality::Local,
        ..CreateEntityMap::default()
    };
    assert_eq!(built(&local).uri().query(), Some("local=true"));
}

#[test]
fn a_get_temporal_creation_sends_the_temporal_query() {
    let (entity_type, temporal_q) = (vehicle_type(), after_last_five());
    let request = built(&CreateTemporalEntityMap {
        lifetime: Some("PT1H".parse().unwrap()),
        ..create_temporal(Some(&entity_type), &temporal_q)
    });
    assert_eq!(request.method(), Method::GET);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/temporal/entityMaps");
    assert_eq!(
        request.uri().query(),
        Some("type=Vehicle&timerel=after&timeAt=2024-05-01T10%3A00%3A00Z&lastN=5&entityMapLifetime=PT1H")
    );
    assert_eq!(header_value(&request, &header::ACCEPT), Some("application/json"));
}

#[test]
fn a_post_temporal_creation_carries_the_temporal_query_in_the_body() {
    let (entity_type, temporal_q) = (vehicle_type(), after_last_five());
    let request = built(&CreateTemporalEntityMap {
        lifetime: Some("PT1H".parse().unwrap()),
        locality: Locality::Local,
        method: QueryMethod::Post,
        ..create_temporal(Some(&entity_type), &temporal_q)
    });
    assert_eq!(request.method(), Method::POST);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/temporal/entityMaps");
    assert_eq!(request.uri().query(), Some("local=true"));
    assert_eq!(
        body_text(&request),
        concat!(
            r#"{"type":"Query","entities":[{"type":"Vehicle"}],"#,
            r#""temporalQ":{"timerel":"after","timeAt":"2024-05-01T10:00:00Z","lastN":5},"entityMapLifetime":"PT1H"}"#
        )
    );
}

#[test]
fn an_unbounded_temporal_creation_is_refused() {
    let temporal_q = after_last_five();
    let unbounded = create_temporal(None, &temporal_q).to_request(&endpoint());
    assert!(matches!(unbounded, Err(RequestError::Argument(ArgumentError::UnboundedQuery))));
}

#[test]
fn the_source_identity_is_a_get_without_parameters() {
    let request = built(&RetrieveSourceIdentity);
    assert_eq!(request.method(), Method::GET);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/info/sourceIdentity");
    assert_eq!(request.uri().query(), None);
    assert_eq!(header_value(&request, &header::ACCEPT), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert!(request.body().is_empty());
}

#[test]
fn every_operation_names_the_tenant() {
    let tenant: Tenant = "t1".parse().unwrap();
    let (id, expires_at, entity_type, temporal_q) = (m(), t(), vehicle_type(), after_last_five());
    let requests = [
        RetrieveEntityMap { id: &id }.to_request(&endpoint().with_tenant(tenant.clone())),
        UpdateEntityMap {
            id: &id,
            expires_at: &expires_at,
        }
        .to_request(&endpoint().with_tenant(tenant.clone())),
        DeleteEntityMap { id: &id }.to_request(&endpoint().with_tenant(tenant.clone())),
        create(Some(&entity_type)).to_request(&endpoint().with_tenant(tenant.clone())),
        create_temporal(Some(&entity_type), &temporal_q).to_request(&endpoint().with_tenant(tenant.clone())),
        RetrieveSourceIdentity.to_request(&endpoint().with_tenant(tenant)),
    ];
    for request in requests {
        assert_eq!(header_value(&request.unwrap(), NgsiLdTenant::name()), Some("t1"));
    }
}

#[test]
fn retrieve_reads_the_map_strictly() {
    let id = m();
    let retrieve = RetrieveEntityMap { id: &id };
    let map = retrieve.decode(&context(), response(200, &[], MAP)).unwrap();
    assert_eq!(map.id, id);
    assert_eq!(map.render().to_string(), MAP);
    let array = retrieve.decode(&context(), response(200, &[], "[]")).unwrap_err();
    assert!(matches!(document_error(array), DocumentErrorKind::Member(MemberError::NotAnObject)));
    let missing = retrieve
        .decode(
            &context(),
            response(404, &[(&header::CONTENT_TYPE, "application/json")], &problem("ResourceNotFound")),
        )
        .unwrap_err();
    assert!(is_problem(&missing, &ErrorType::ResourceNotFound, 404));
}

#[test]
fn a_creation_reads_the_map_and_its_location_whatever_the_header() {
    let entity_type = vehicle_type();
    let create = create(Some(&entity_type));
    let created = create.decode(&context(), response(201, &[(NgsiLdEntityMap::name(), LOCATION)], MAP)).unwrap();
    assert_eq!(created.status().as_u16(), 201);
    assert_eq!(created.body().render().to_string(), MAP);
    assert_eq!(
        created.body().entity_map.values().next().map(|sources| sources[0].clone()),
        Some(EntitySource::Local)
    );
    assert_eq!(created.entity_map().valid().and_then(NgsiLdEntityMap::id), Some(m()));
    let absent = create.decode(&context(), response(201, &[], MAP)).unwrap();
    assert_eq!(absent.entity_map(), &OptionalHeader::Absent);
    assert_eq!(absent.body().id, m());
    let malformed = create.decode(&context(), response(201, &[(NgsiLdEntityMap::name(), "a b")], MAP)).unwrap();
    assert!(matches!(malformed.entity_map(), OptionalHeader::Malformed(_)));
}

#[test]
fn only_the_get_creation_for_query_entities_answers_203() {
    let entity_type = vehicle_type();
    let get = create(Some(&entity_type));
    let amended = get
        .decode(
            &context(),
            response(203, &[(NgsiLdEntityMap::name(), LOCATION), (PreferenceApplied::name(), "ngsi-ld=1.8.1")], MAP),
        )
        .unwrap();
    assert_eq!(amended.preference_applied().map(ToString::to_string).as_deref(), Some("1.8.1"));
    assert_eq!(amended.status().as_u16(), 203);
    let post = CreateEntityMap {
        method: QueryMethod::Post,
        ..create(Some(&entity_type))
    };
    assert!(is_unexpected(&post.decode(&context(), response(203, &[], MAP)).unwrap_err()));
    assert!(is_unexpected(&get.decode(&context(), response(200, &[], MAP)).unwrap_err()));
}

#[test]
fn a_temporal_creation_answers_201_only() {
    let (entity_type, temporal_q) = (vehicle_type(), after_last_five());
    let create = create_temporal(Some(&entity_type), &temporal_q);
    assert_eq!(create.decode(&context(), response(201, &[], MAP)).unwrap().body().id, m());
    assert!(is_unexpected(&create.decode(&context(), response(203, &[], MAP)).unwrap_err()));
    let refused = create
        .decode(
            &context(),
            response(400, &[(&header::CONTENT_TYPE, "application/json")], &problem("BadRequestData")),
        )
        .unwrap_err();
    assert!(is_problem(&refused, &ErrorType::BadRequestData, 400));
}

#[test]
fn a_created_map_without_its_expiry_fails_the_answer() {
    let entity_type = vehicle_type();
    let error = create(Some(&entity_type))
        .decode(&context(), response(201, &[], r#"{"id":"urn:ngsi-ld:EntityMap:1","type":"EntityMap"}"#))
        .unwrap_err();
    assert!(matches!(
        document_error(error),
        DocumentErrorKind::Member(MemberError::MissingMember { member: "expiresAt" })
    ));
}

#[test]
fn update_and_delete_answer_204_only() {
    let (id, expires_at) = (m(), t());
    let update = UpdateEntityMap {
        id: &id,
        expires_at: &expires_at,
    };
    let delete = DeleteEntityMap { id: &id };
    assert_eq!(update.decode(&context(), response(204, &[], "")).unwrap(), ());
    assert_eq!(delete.decode(&context(), response(204, &[], "")).unwrap(), ());
    assert!(is_unexpected(&delete.decode(&context(), response(200, &[], "")).unwrap_err()));
    let missing = update
        .decode(
            &context(),
            response(404, &[(&header::CONTENT_TYPE, "application/json")], &problem("ResourceNotFound")),
        )
        .unwrap_err();
    assert!(is_problem(&missing, &ErrorType::ResourceNotFound, 404));
}

#[test]
fn the_source_identity_is_read_and_not_implemented_is_a_problem_of_another_type() {
    let identity = RetrieveSourceIdentity.decode(&context(), response(200, &[], ID)).unwrap();
    assert_eq!(identity.alias.as_ref(), "broker-1");
    assert_eq!(identity.render().to_string(), ID);
    let unsupported = RetrieveSourceIdentity
        .decode(
            &context(),
            response(501, &[(&header::CONTENT_TYPE, "application/json")], &problem("NotImplemented")),
        )
        .unwrap_err();
    assert!(matches!(
        unsupported,
        ResponseError::Problem { status, ref details, .. } if status == 501 && matches!(details.error_type, Some(ErrorType::Other(_)))
    ));
    assert!(is_unexpected(&RetrieveSourceIdentity.decode(&context(), response(204, &[], "")).unwrap_err()));
}

#[test]
fn a_creation_the_broker_does_not_implement_is_a_problem() {
    let entity_type = vehicle_type();
    let error = create(Some(&entity_type))
        .decode(
            &context(),
            response(501, &[(&header::CONTENT_TYPE, "application/json")], &problem("NotImplemented")),
        )
        .unwrap_err();
    assert!(matches!(error, ResponseError::Problem { status, .. } if status == 501));
}
