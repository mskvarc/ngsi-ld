//! The Retrieve Entity and Query Entities parameters of linked entity retrieval, dataset selection, language
//! filtering, ordering, value expansion, entity splitting and EntityMaps, and how their responses are decoded.

use crate::{
    fixtures::{context, endpoint, header_value, json, response, vehicle, vehicle_body, vehicle_id},
    query_vehicles,
    retrieve_entity,
};
use headers::Header;
use iri_rs::IriRefBuf;
use jstrict::{Print, Unordered, Value as Json};
use ngsi_ld::{
    http::{
        BodyError,
        ResponseError,
        headers::{NgsiLdEntityMap, OptionalHeader, PreferenceApplied},
        operation::{EntityMapRequest, InlineJoin, Operation, QueryEntities, QueryMethod, RetrieveEntity, Simplified},
    },
    model::{
        AttributeName,
        DatasetId,
        EntityId,
        Lossless,
        geometry::{LineString, Position, Shape},
    },
    query::{EntitySplitting, Join, SelectorType, language_filter::LanguageFilter, ordering},
};
use pretty_assertions::assert_eq;
use std::num::NonZeroU32;

/// The entity map the broker hands out in these tests.
fn entity_map() -> NgsiLdEntityMap {
    NgsiLdEntityMap::new(IriRefBuf::new("/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1").unwrap()).unwrap()
}

/// Two dataset ids: the default instance and the GPS one.
fn dataset_ids() -> [DatasetId; 2] {
    ["@none".parse().unwrap(), "urn:ngsi-ld:dataset:gps".parse().unwrap()]
}

/// One entity already encountered.
fn building() -> [EntityId; 1] {
    ["urn:ngsi-ld:Building:1".parse().unwrap()]
}

/// The query string of the request `operation` builds.
fn query_string(operation: &impl Operation) -> Option<String> {
    operation.to_request(&endpoint()).unwrap().uri().query().map(str::to_owned)
}

#[test]
fn retrieve_selects_dataset_ids_including_the_default_instance() {
    let id = vehicle_id();
    let dataset_ids = dataset_ids();
    let retrieve = RetrieveEntity {
        dataset_ids: &dataset_ids,
        ..retrieve_entity(&id, Lossless::Normalized)
    };
    assert_eq!(query_string(&retrieve).as_deref(), Some("datasetId=%40none,urn%3Angsi-ld%3Adataset%3Agps"));
}

#[test]
fn retrieve_sends_a_language_filter_with_its_syntax_literal() {
    let id = vehicle_id();
    let lang: LanguageFilter = "fr-CH,fr;q=0.9".parse().unwrap();
    let retrieve = RetrieveEntity {
        lang: Some(&lang),
        ..retrieve_entity(&id, Simplified)
    };
    assert_eq!(query_string(&retrieve).as_deref(), Some("lang=fr-CH,fr;q=0.9&format=simplified"));
}

#[test]
fn retrieve_sends_an_inline_join_with_its_level_and_the_entities_encountered() {
    let id = vehicle_id();
    let contained_by = building();
    let retrieve = RetrieveEntity {
        join: Some(InlineJoin::Inline),
        join_level: NonZeroU32::new(2),
        contained_by: &contained_by,
        ..retrieve_entity(&id, Lossless::Concise)
    };
    assert_eq!(
        query_string(&retrieve).as_deref(),
        Some("join=inline&joinLevel=2&containedBy=urn%3Angsi-ld%3ABuilding%3A1&format=concise")
    );
}

#[test]
fn a_flattened_retrieve_sends_join_flat_and_reads_an_array() {
    let id = vehicle_id();
    let retrieve = RetrieveEntity::flattened_builder().id(&id).format(Lossless::Normalized).build();
    assert_eq!(query_string(&retrieve).as_deref(), Some("join=flat"));
    let body = Json::Array(vec![vehicle_body(), vehicle_body()].into()).compact_print().to_string();
    let retrieved = retrieve.decode(&context(), response(200, &[], &body)).unwrap();
    assert_eq!(retrieved.body().len(), 2);
    let error = retrieve
        .decode(&context(), response(200, &[], &vehicle_body().compact_print().to_string()))
        .unwrap_err();
    assert!(
        matches!(&error, ResponseError::Body { source, .. } if matches!(**source, BodyError::NotAnArray)),
        "{error:?}"
    );
}

#[test]
fn returning_an_entity_map_sends_the_flag_and_the_lifetime() {
    let id = vehicle_id();
    let request = EntityMapRequest::returned_for("PT1H".parse().unwrap());
    let retrieve = RetrieveEntity {
        entity_map: Some(&request),
        ..retrieve_entity(&id, Lossless::Normalized)
    };
    let sent = retrieve.to_request(&endpoint()).unwrap();
    assert_eq!(sent.uri().query(), Some("entityMap=true&entityMapLifetime=PT1H"));
    assert_eq!(header_value(&sent, NgsiLdEntityMap::name()), None);
}

#[test]
fn reusing_an_entity_map_sends_its_location_in_the_header_only() {
    let id = vehicle_id();
    let request = EntityMapRequest::reuse(entity_map());
    let retrieve = RetrieveEntity {
        entity_map: Some(&request),
        ..retrieve_entity(&id, Lossless::Normalized)
    };
    let sent = retrieve.to_request(&endpoint()).unwrap();
    assert_eq!(sent.uri().query(), None);
    assert_eq!(
        header_value(&sent, NgsiLdEntityMap::name()),
        Some("/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1")
    );
}

#[test]
fn a_join_level_without_a_linking_join_and_contained_by_without_a_level_are_sent() {
    let id = vehicle_id();
    let contained_by = building();
    for join in [None, Some(InlineJoin::None)] {
        let retrieve = RetrieveEntity {
            join,
            join_level: NonZeroU32::new(2),
            ..retrieve_entity(&id, Lossless::Normalized)
        };
        assert!(query_string(&retrieve).is_some_and(|query| query.ends_with("joinLevel=2")));
    }
    let retrieve = RetrieveEntity {
        join: Some(InlineJoin::Inline),
        contained_by: &contained_by,
        ..retrieve_entity(&id, Lossless::Normalized)
    };
    assert_eq!(query_string(&retrieve).as_deref(), Some("join=inline&containedBy=urn%3Angsi-ld%3ABuilding%3A1"));
}

#[test]
fn a_get_query_sends_every_parameter_with_its_encoding() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let expand: [AttributeName; 1] = ["gender".parse().unwrap()];
    let json_keys: [AttributeName; 1] = ["parkingTickets".parse().unwrap()];
    let dataset_ids = dataset_ids();
    let lang: LanguageFilter = "en".parse().unwrap();
    let contained_by = building();
    let ordering = ordering::by("name").then_descending("age").collation("de-u-co-phonebk").build().unwrap();
    let request = EntityMapRequest::returned();
    let query = QueryEntities {
        expand_values: &expand,
        json_keys: &json_keys,
        dataset_ids: &dataset_ids,
        lang: Some(&lang),
        join: Some(Join::Flat),
        join_level: NonZeroU32::new(2),
        contained_by: &contained_by,
        ordering: Some(&ordering),
        split_entities: EntitySplitting::Split,
        entity_map: Some(&request),
        ..query_vehicles(Some(&entity_type), None, Lossless::Normalized)
    };
    assert_eq!(
        query_string(&query).as_deref(),
        Some(concat!(
            "type=Vehicle&expandValues=gender&jsonKeys=parkingTickets&datasetId=%40none,urn%3Angsi-ld%3Adataset%3Agps",
            "&lang=en&join=flat&joinLevel=2&containedBy=urn%3Angsi-ld%3ABuilding%3A1",
            "&orderBy=name,age;desc&collation=de-u-co-phonebk&splitEntities=true&entityMap=true",
        ))
    );
}

#[test]
fn an_ordering_by_distance_sends_its_origin() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let point = Shape::Point(Position::new(13.3986, 52.5547).unwrap());
    let nearest = ordering::nearest("location").origin(point).build().unwrap();
    let query = QueryEntities {
        ordering: Some(&nearest),
        ..query_vehicles(Some(&entity_type), None, Simplified)
    };
    assert_eq!(
        query_string(&query).as_deref(),
        Some("type=Vehicle&orderBy=location;dist-asc&orderFrom=%5B13.3986%2C52.5547%5D&format=simplified")
    );
    let positions = vec![Position::new(13.3986, 52.5547).unwrap(), Position::new(13.4, 52.6).unwrap()];
    let line = Shape::LineString(LineString::new(positions).unwrap());
    let farthest = ordering::farthest("location").origin(line).build().unwrap();
    let query = QueryEntities {
        ordering: Some(&farthest),
        ..query_vehicles(Some(&entity_type), None, Simplified)
    };
    assert_eq!(
        query_string(&query).as_deref(),
        Some(concat!(
            "type=Vehicle&orderBy=location;dist-desc&orderGeometry=LineString",
            "&orderFrom=%5B%5B13.3986%2C52.5547%5D%2C%5B13.4%2C52.6%5D%5D&format=simplified",
        ))
    );
}

#[test]
fn a_post_query_carries_the_parameters_in_its_body() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let expand: [AttributeName; 2] = ["gender".parse().unwrap(), "colour".parse().unwrap()];
    let json_keys: [AttributeName; 1] = ["parkingTickets".parse().unwrap()];
    let dataset_ids = dataset_ids();
    let lang: LanguageFilter = "fr-CH,fr;q=0.9".parse().unwrap();
    let contained_by = building();
    let positions = vec![Position::new(13.3986, 52.5547).unwrap(), Position::new(13.4, 52.6).unwrap()];
    let line = Shape::LineString(LineString::new(positions).unwrap());
    let ordering = ordering::by("name")
        .then_nearest("location")
        .collation("und-u-ks-identic")
        .origin(line)
        .build()
        .unwrap();
    let request = EntityMapRequest::returned_for("P1D".parse().unwrap());
    let query = QueryEntities {
        expand_values: &expand,
        json_keys: &json_keys,
        dataset_ids: &dataset_ids,
        lang: Some(&lang),
        join: Some(Join::Inline),
        join_level: NonZeroU32::new(3),
        contained_by: &contained_by,
        ordering: Some(&ordering),
        split_entities: EntitySplitting::Whole,
        entity_map: Some(&request),
        method: QueryMethod::Post,
        limit: Some(5),
        ..query_vehicles(Some(&entity_type), None, Lossless::Normalized)
    };
    let sent = query.to_request(&endpoint()).unwrap();
    assert_eq!(sent.uri().query(), Some("limit=5"));
    assert_eq!(
        Unordered(json(sent.body())),
        Unordered(json(
            br#"{
                "type": "Query",
                "entities": [{"type": "Vehicle"}],
                "expandValues": "gender,colour",
                "jsonKeys": "parkingTickets",
                "datasetId": ["@none", "urn:ngsi-ld:dataset:gps"],
                "lang": "fr-CH,fr;q=0.9",
                "join": "inline",
                "joinLevel": 3,
                "containedBy": ["urn:ngsi-ld:Building:1"],
                "ordering": {
                    "orderBy": ["name", "location;dist-asc"],
                    "collation": "und-u-ks-identic",
                    "geometry": "LineString",
                    "coordinates": [[13.3986, 52.5547], [13.4, 52.6]]
                },
                "splitEntities": false,
                "entityMap": true,
                "entityMapLifetime": "P1D"
            }"#
        ))
    );
}

#[test]
fn a_post_query_reusing_an_entity_map_sends_the_header_and_no_flag() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let request = EntityMapRequest::reuse(entity_map());
    let query = QueryEntities {
        entity_map: Some(&request),
        method: QueryMethod::Post,
        ..query_vehicles(Some(&entity_type), None, Lossless::Normalized)
    };
    let sent = query.to_request(&endpoint()).unwrap();
    assert_eq!(
        header_value(&sent, NgsiLdEntityMap::name()),
        Some("/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1")
    );
    assert_eq!(
        Unordered(json(sent.body())),
        Unordered(json(br#"{"type": "Query", "entities": [{"type": "Vehicle"}]}"#))
    );
}

#[test]
fn a_query_sends_a_join_level_beside_a_join_of_none() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let query = QueryEntities {
        join: Some(Join::None),
        join_level: NonZeroU32::new(1),
        ..query_vehicles(Some(&entity_type), None, Lossless::Normalized)
    };
    assert_eq!(query_string(&query).as_deref(), Some("type=Vehicle&join=%40none&joinLevel=1"));
}

#[test]
fn a_retrieved_entity_keeps_the_entity_map_and_the_applied_version() {
    let id = vehicle_id();
    let retrieve = retrieve_entity(&id, Lossless::Normalized);
    let headers = [
        (NgsiLdEntityMap::name(), "/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1"),
        (PreferenceApplied::name(), "ngsi-ld=1.8.1"),
    ];
    let retrieved = retrieve
        .decode(&context(), response(203, &headers, &vehicle_body().compact_print().to_string()))
        .unwrap();
    assert_eq!(retrieved.entity_map(), &OptionalHeader::Valid(entity_map()));
    assert_eq!(retrieved.preference_applied().map(ToString::to_string).as_deref(), Some("1.8.1"));
    assert_eq!(retrieved.into_body().id, vehicle().id);
}

#[test]
fn a_page_keeps_the_entity_map_or_its_malformed_lines() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let query = query_vehicles(Some(&entity_type), None, Lossless::Normalized);
    let valid = query
        .decode(
            &context(),
            response(201, &[(NgsiLdEntityMap::name(), "/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1")], "[]"),
        )
        .unwrap();
    assert_eq!(valid.entity_map(), &OptionalHeader::Valid(entity_map()));
    let malformed = query.decode(&context(), response(200, &[(NgsiLdEntityMap::name(), "a b")], "[]")).unwrap();
    assert!(malformed.entity_map().is_malformed());
    let absent = query.decode(&context(), response(200, &[], "[]")).unwrap();
    assert!(absent.entity_map().is_absent());
}
