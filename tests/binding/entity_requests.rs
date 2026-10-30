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
use iri_rs::iri;
use jstrict::{Unordered, Value as Json};
use ngsi_ld::{
    http::{
        ContextDelivery,
        RequestError,
        headers::NgsiLdTenant,
        operation::{
            AppendAttributes,
            ArgumentError,
            CreateEntity,
            DeleteEntity,
            EntityIdMember,
            MergeEntity,
            MergeSimplifiedEntity,
            Operation,
            QueryEntities,
            QueryMethod,
            ReplaceEntity,
            ResultsCount,
            Simplified,
            UpdateAttributes,
        },
    },
    model::{
        Entity,
        EntityFragment,
        EntityId,
        LanguageMap,
        LanguageProperty,
        LanguageText,
        ListRelationship,
        Lossless,
        Output,
        Representation,
        SimplifiedFragment,
        Tenant,
    },
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
fn a_concise_create_writes_an_object_list_of_uris_beside_a_language_entry_of_several_strings() {
    let cities = ["urn:ngsi-ld:City:Antwerp", "urn:ngsi-ld:City:Rotterdam"].map(|id| id.parse::<EntityId>().unwrap());
    let names = LanguageMap::try_from_pairs([("fr", LanguageText::Multiple(vec!["Grand Place".to_owned(), "Grote Markt".to_owned()]))]).unwrap();
    let route = Entity::builder()
        .id(iri!("urn:ngsi-ld:Route:R1"))
        .entity_type("Route")
        .list_relationship("passesThrough", ListRelationship::builder(cities.to_vec()).build().unwrap())
        .language_property("square", LanguageProperty::builder(names).build().unwrap())
        .build()
        .unwrap();
    let request = CreateEntity::builder()
        .entity(&route)
        .form(Lossless::Concise)
        .build()
        .to_request(&endpoint())
        .unwrap();
    let body = concat!(
        r#"{"id":"urn:ngsi-ld:Route:R1","type":"Route","#,
        r#""passesThrough":{"objectList":["urn:ngsi-ld:City:Antwerp","urn:ngsi-ld:City:Rotterdam"]},"#,
        r#""square":{"languageMap":{"fr":["Grand Place","Grote Markt"]}}}"#
    );
    assert_eq!(json(request.body()), json(body.as_bytes()));
    let read = Entity::<Output>::parse(body, Lossless::Concise).unwrap();
    assert_eq!(read.render(Representation::Concise).to_string(), body);
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
fn a_fragment_naming_an_entity_other_than_its_target_is_refused_before_sending() {
    let id = vehicle_id();
    let other = || "urn:ngsi-ld:Vehicle:B9211".parse::<EntityId>().unwrap();
    let elsewhere = EntityFragment::builder().id(other()).property("speed", 0).build().unwrap();
    let named_elsewhere = EntityFragment::for_entity(other(), "Vehicle").property("speed", 0).build().unwrap();
    let simplified_elsewhere = SimplifiedFragment::builder().id(other()).value("speed", 0).build().unwrap();
    let refusals = [
        MergeEntity::builder().id(&id).fragment(&elsewhere).build().to_request(&endpoint()),
        MergeEntity::builder().id(&id).fragment(&named_elsewhere).build().to_request(&endpoint()),
        UpdateAttributes::builder().id(&id).fragment(&elsewhere).build().to_request(&endpoint()),
        AppendAttributes::builder().id(&id).fragment(&named_elsewhere).build().to_request(&endpoint()),
        MergeSimplifiedEntity::builder()
            .id(&id)
            .fragment(&simplified_elsewhere)
            .build()
            .to_request(&endpoint()),
    ];
    for refusal in refusals {
        assert!(
            matches!(refusal, Err(RequestError::Argument(ArgumentError::FragmentNamesAnotherEntity))),
            "{refusal:?}"
        );
    }
}

#[test]
fn a_fragment_naming_its_target_or_no_entity_is_sent_with_its_id_as_given() {
    let id = vehicle_id();
    let named = EntityFragment::builder().id(vehicle_id()).property("speed", 0).build().unwrap();
    let unnamed = EntityFragment::builder().property("speed", 0).build().unwrap();
    let simplified = SimplifiedFragment::builder().id(vehicle_id()).value("speed", 0).build().unwrap();
    let merge = MergeEntity::builder().id(&id).fragment(&named).build().to_request(&endpoint()).unwrap();
    assert_eq!(json(merge.body()).as_object().unwrap().get("id").next(), Some(&Json::from(id.as_str())));
    let update = UpdateAttributes::builder().id(&id).fragment(&unnamed).build().to_request(&endpoint()).unwrap();
    assert!(!json(update.body()).as_object().unwrap().contains_key("id"));
    let append = AppendAttributes::builder().id(&id).fragment(&named).build().to_request(&endpoint()).unwrap();
    assert_eq!(append.uri().path(), "/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567/attrs");
    let simplified = MergeSimplifiedEntity::builder()
        .id(&id)
        .fragment(&simplified)
        .build()
        .to_request(&endpoint())
        .unwrap();
    assert_eq!(json(simplified.body()).as_object().unwrap().get("id").next(), Some(&Json::from(id.as_str())));
}

#[test]
fn replace_sends_put_to_the_entity() {
    let vehicle = vehicle();
    let request = ReplaceEntity::builder().entity(&vehicle).build().to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::PUT);
    assert_eq!(request.uri().path(), format!("/ngsi-ld/v1/entities/{}", vehicle.id));
}

#[test]
fn replace_can_leave_the_id_out_of_the_body_the_path_names() {
    let vehicle = vehicle();
    let written = ReplaceEntity::builder().entity(&vehicle).build().to_request(&endpoint()).unwrap();
    assert!(json(written.body()).as_object().unwrap().contains_key("id"));
    let omitted = ReplaceEntity::builder()
        .entity(&vehicle)
        .id_member(EntityIdMember::Omitted)
        .build()
        .to_request(&endpoint())
        .unwrap();
    assert_eq!(omitted.uri().path(), format!("/ngsi-ld/v1/entities/{}", vehicle.id));
    let body = json(omitted.body());
    assert!(!body.as_object().unwrap().contains_key("id"));
    assert!(body.as_object().unwrap().contains_key("type"));
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
