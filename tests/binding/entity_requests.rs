//! The requests of the entity operations: method, path, query string, headers and body.

use crate::{
    fixtures::{endpoint, endpoint_with, header_value, json, link_of, vehicle, vehicle_body, vehicle_id},
    links::context_link,
    query_projected,
    query_vehicles,
    retrieve_entity,
};
use headers::Header;
use http::{Method, header};
use jstrict::{Unordered, Value as Json};
use ngsi_ld::{
    http::{
        ContextDelivery,
        RequestError,
        headers::NgsiLdTenant,
        operation::{ArgumentError, CreateEntity, DeleteEntity, MergeEntity, Operation, QueryEntities, QueryMethod, ReplaceEntity, ResultsCount, Simplified},
    },
    model::{EntityFragment, EntityId, Lossless, Tenant},
    query::{Locality, SelectorType, projection::Projection, q},
};
use pretty_assertions::assert_eq;

#[test]
fn create_with_json_with_link_sends_the_context_in_the_link_header_only() {
    let vehicle = vehicle();
    let request = CreateEntity::builder().entity(&vehicle).build().to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::POST);
    assert_eq!(request.uri().to_string(), "http://broker.test/ngsi-ld/v1/entities");
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(header_value(&request, &header::CONTENT_LENGTH), Some(request.body().len().to_string().as_str()));
    let body = json(request.body());
    assert!(body.as_object().unwrap().get("@context").next().is_none());
    assert_eq!(Unordered(body), Unordered(vehicle_body()));
}

#[test]
fn create_with_ld_json_body_sends_the_context_in_the_body_only() {
    let vehicle = vehicle();
    let request = CreateEntity::builder()
        .entity(&vehicle)
        .build()
        .to_request(&endpoint_with(ContextDelivery::LdJsonBody))
        .unwrap();
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/ld+json"));
    assert_eq!(link_of(&request), None);
    let body = json(request.body());
    let context: Vec<&Json> = body.as_object().unwrap().get("@context").collect();
    assert_eq!(context, [&Json::from("https://example.org/context.jsonld")]);
}

#[test]
fn retrieve_sends_get_with_the_link_header_and_accepts_json() {
    let id = vehicle_id();
    let request = retrieve_entity(&id, Lossless::Normalized).to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::GET);
    assert_eq!(request.uri().to_string(), "http://broker.test/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567");
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(header_value(&request, &header::ACCEPT), Some("application/json"));
    assert!(request.body().is_empty());
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), None);
}

#[test]
fn an_entity_id_with_slash_and_hash_is_percent_encoded_in_the_path() {
    let id: EntityId = "https://example.org/vehicles/A4567#front".parse().unwrap();
    let request = retrieve_entity(&id, Simplified).to_request(&endpoint()).unwrap();
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/entities/https:%2F%2Fexample.org%2Fvehicles%2FA4567%23front");
}

#[test]
fn query_prints_and_percent_encodes_type_q_and_format() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let q = q::attr("brandName").ne("Mercedes").build().unwrap();
    let request = query_vehicles(Some(&entity_type), Some(&q), Simplified).to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::GET);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/entities");
    assert_eq!(request.uri().query(), Some("type=Vehicle&q=brandName%21%3D%22Mercedes%22&format=simplified"));
}

#[test]
fn query_via_post_sends_a_query_body_and_keeps_format_in_the_query_string() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let q = q::attr("brandName").ne("Mercedes").build().unwrap();
    let query = QueryEntities {
        method: QueryMethod::Post,
        ..query_vehicles(Some(&entity_type), Some(&q), Simplified)
    };
    let request = query.to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::POST);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/entityOperations/query");
    assert_eq!(request.uri().query(), Some("format=simplified"));
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/json"));
    assert_eq!(
        Unordered(json(request.body())),
        Unordered(json(br#"{"type": "Query", "entities": [{"type": "Vehicle"}], "q": "brandName!=\"Mercedes\""}"#))
    );
}

#[test]
fn limit_and_count_are_sent() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let query = QueryEntities {
        limit: Some(2),
        count: ResultsCount::Include,
        ..query_vehicles(Some(&entity_type), None, Lossless::Normalized)
    };
    assert_eq!(query.to_request(&endpoint()).unwrap().uri().query(), Some("type=Vehicle&limit=2&count=true"));
}

#[test]
fn a_zero_limit_without_count_is_refused_before_sending() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let query = QueryEntities {
        limit: Some(0),
        ..query_vehicles(Some(&entity_type), None, Lossless::Normalized)
    };
    assert!(matches!(
        query.to_request(&endpoint()),
        Err(RequestError::Argument(ArgumentError::ZeroLimitWithoutCount))
    ));
    let counted = QueryEntities {
        limit: Some(0),
        count: ResultsCount::Include,
        ..query_vehicles(Some(&entity_type), None, Lossless::Normalized)
    };
    assert!(counted.to_request(&endpoint()).is_ok());
}

#[test]
fn a_query_without_type_or_q_is_refused_unless_local() {
    assert!(matches!(
        query_vehicles(None, None, Lossless::Normalized).to_request(&endpoint()),
        Err(RequestError::Argument(ArgumentError::UnboundedQuery))
    ));
    let local = QueryEntities {
        locality: Locality::Local,
        ..query_vehicles(None, None, Lossless::Normalized)
    };
    assert_eq!(local.to_request(&endpoint()).unwrap().uri().query(), Some("local=true"));
}

#[test]
fn a_tenant_is_sent_in_the_ngsild_tenant_header() {
    let id = vehicle_id();
    let tenant: Tenant = "t1".parse().unwrap();
    let request = DeleteEntity::builder().id(&id).build().to_request(&endpoint().with_tenant(tenant)).unwrap();
    assert_eq!(header_value(&request, NgsiLdTenant::name()), Some("t1"));
}

#[test]
fn the_default_tenant_sends_no_tenant_header() {
    let id = vehicle_id();
    let request = DeleteEntity::builder().id(&id).build().to_request(&endpoint()).unwrap();
    assert!(request.headers().get(NgsiLdTenant::name()).is_none());
}

#[test]
fn merge_sends_patch_with_the_fragment() {
    let id = vehicle_id();
    let fragment = EntityFragment::builder().property("speed", 0).delete("isParked").build().unwrap();
    let request = MergeEntity::builder()
        .id(&id)
        .fragment(&fragment)
        .form(Lossless::Concise)
        .build()
        .to_request(&endpoint())
        .unwrap();
    assert_eq!(request.method(), Method::PATCH);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567");
    assert_eq!(
        Unordered(json(request.body())),
        Unordered(json(br#"{"speed": 0, "isParked": "urn:ngsi-ld:null"}"#))
    );
}

#[test]
fn replace_sends_put_to_the_entity() {
    let vehicle = vehicle();
    let request = ReplaceEntity::builder().entity(&vehicle).build().to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::PUT);
    assert_eq!(request.uri().path(), format!("/ngsi-ld/v1/entities/{}", vehicle.id));
}

#[test]
fn delete_sends_delete_with_the_link_header_and_no_body() {
    let id = vehicle_id();
    let request = DeleteEntity::builder().id(&id).build().to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::DELETE);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567");
    assert_eq!(link_of(&request), Some(context_link()));
    assert!(request.body().is_empty());
    assert!(request.headers().get(header::CONTENT_TYPE).is_none());
}

#[test]
fn query_via_post_sends_each_picked_term_as_one_element() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let pick: Projection = "temperature,humidity".parse().unwrap();
    let query = QueryEntities {
        method: QueryMethod::Post,
        pick: Some(&pick),
        omit: Some(&pick),
        ..query_projected(Some(&entity_type), None, Lossless::Normalized)
    };
    let request = query.to_request(&endpoint()).unwrap();
    assert_eq!(
        Unordered(json(request.body())),
        Unordered(json(
            br#"{"type": "Query", "entities": [{"type": "Vehicle"}], "pick": ["temperature", "humidity"], "omit": ["temperature", "humidity"]}"#
        ))
    );
}

#[test]
fn a_query_via_get_writes_list_type_and_projection_separators_literally() {
    let entity_type: SelectorType = "(Home;Vehicle)|Motorhome".parse().unwrap();
    let pick: Projection = "temperature,humidity".parse().unwrap();
    let ids: [EntityId; 2] = ["urn:ngsi-ld:Vehicle:A4567".parse().unwrap(), "urn:ngsi-ld:Vehicle:B,9211".parse().unwrap()];
    let query = QueryEntities {
        ids: &ids,
        pick: Some(&pick),
        ..query_projected(Some(&entity_type), None, Lossless::Normalized)
    };
    assert_eq!(
        query.to_request(&endpoint()).unwrap().uri().query(),
        Some("type=(Home;Vehicle)%7CMotorhome&id=urn%3Angsi-ld%3AVehicle%3AA4567,urn%3Angsi-ld%3AVehicle%3AB%2C9211&pick=temperature,humidity")
    );
}
