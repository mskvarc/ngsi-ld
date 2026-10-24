//! How Query Entities and Retrieve Entity name their entities and their representation (clauses 4.17, 5.2.33,
//! 6.3.7, 6.4.3.2 and 6.23.3.1), and how an inline join reads linked entities a projection narrowed (clauses 4.21
//! and 4.5.23.2).

use crate::{
    fixtures::{context, endpoint, json, response, vehicle_id},
    query_projected,
    query_vehicles,
    retrieve_entity,
};
use iri_rs::iri;
use ngsi_ld::{
    http::{
        RequestError,
        operation::{
            ArgumentError,
            GeoJson,
            InlineJoin,
            Operation,
            QueryEntities,
            QueryMethod,
            RepresentationSpelling,
            RetrieveEntity,
            Simplified,
            SystemAttributes,
        },
    },
    model::{Attribute, AttributeName, Entity, EntityId, Instances, Lossless, OneOrMany, Output, Projected, Representation},
    query::{
        EntitySelector,
        IdPattern,
        Join,
        Locality,
        SelectorType,
        builders::SelectorTypeBuilder,
        projection::{EntityMember, Projection},
    },
};
use pretty_assertions::assert_eq;

/// Two vehicle ids.
fn vehicles() -> [EntityId; 2] {
    [
        EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567")),
        EntityId::from(iri!("urn:ngsi-ld:Vehicle:B9211")),
    ]
}

/// The linked entity of the relationship `name` of `entity`.
fn linked<'a>(entity: &'a Entity<Output>, name: &str) -> &'a Entity<Output, Projected> {
    let name: AttributeName = name.parse().unwrap();
    let Some(Attribute::Relationship(relationship)) = entity.attribute(&name).map(Instances::first) else {
        panic!("`{name}` is not a Relationship");
    };
    relationship.output().entity.as_ref().and_then(OneOrMany::as_one).unwrap()
}

#[test]
fn several_selectors_are_sent_in_the_query_body_of_a_post_query() {
    let vehicle: SelectorType = "Vehicle".parse().unwrap();
    let [a4567, _] = vehicles();
    let ids = [a4567];
    let pattern: IdPattern = "^urn:ngsi-ld:Car:A.*$|^urn:ngsi-ld:Car:B.*$".parse().unwrap();
    let selectors = [EntitySelector::new("Car".parse().unwrap()).with_id_pattern(pattern), EntitySelector::all()];
    let query = QueryEntities {
        ids: &ids,
        selectors: &selectors,
        method: QueryMethod::Post,
        ..query_vehicles(Some(&vehicle), None, Lossless::Normalized)
    };
    let request = query.to_request(&endpoint()).unwrap();
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/entityOperations/query");
    assert_eq!(request.uri().query(), None);
    assert_eq!(
        json(request.body()),
        json(
            br#"{"type":"Query","entities":[
                {"type":"Vehicle","id":"urn:ngsi-ld:Vehicle:A4567"},
                {"type":"Car","idPattern":"^urn:ngsi-ld:Car:A.*$|^urn:ngsi-ld:Car:B.*$"},
                {"type":"*"}
            ]}"#
        )
    );
}

#[test]
fn selectors_alone_bound_a_post_query_and_without_a_type_parameter_need_no_type() {
    let selectors = [
        EntitySelector::new("Vehicle".parse().unwrap()).with_ids(OneOrMany::try_from(vehicles().to_vec()).unwrap()),
        EntitySelector::new("Car".parse().unwrap()),
    ];
    let query = QueryEntities {
        selectors: &selectors,
        method: QueryMethod::Post,
        ..query_vehicles(None, None, Simplified)
    };
    let request = query.to_request(&endpoint()).unwrap();
    assert_eq!(request.uri().query(), Some("format=simplified"));
    assert_eq!(
        json(request.body()),
        json(br#"{"type":"Query","entities":[{"type":"Vehicle","id":["urn:ngsi-ld:Vehicle:A4567","urn:ngsi-ld:Vehicle:B9211"]},{"type":"Car"}]}"#)
    );
    let ids = vehicles();
    let untyped = QueryEntities { ids: &ids, ..query };
    assert!(matches!(
        untyped.to_request(&endpoint()),
        Err(RequestError::Argument(ArgumentError::SelectorWithoutType))
    ));
}

#[test]
fn a_get_query_carries_a_selector_given_alone_as_its_parameters() {
    let pattern: IdPattern = "^urn:ngsi-ld:Vehicle:.*$".parse().unwrap();
    let selectors = [EntitySelector::new("Vehicle".parse().unwrap())
        .with_ids(OneOrMany::try_from(vehicles().to_vec()).unwrap())
        .with_id_pattern(pattern)];
    let query = QueryEntities {
        selectors: &selectors,
        ..query_vehicles(None, None, Lossless::Normalized)
    };
    assert_eq!(
        query.to_request(&endpoint()).unwrap().uri().query(),
        Some("type=Vehicle&id=urn%3Angsi-ld%3AVehicle%3AA4567,urn%3Angsi-ld%3AVehicle%3AB9211&idPattern=%5Eurn%3Angsi-ld%3AVehicle%3A.%2A%24")
    );
}

#[test]
fn a_get_query_refuses_more_than_one_selection() {
    let vehicle: SelectorType = "Vehicle".parse().unwrap();
    let car = [EntitySelector::new("Car".parse().unwrap())];
    let beside_type = QueryEntities {
        selectors: &car,
        ..query_vehicles(Some(&vehicle), None, Lossless::Normalized)
    };
    assert!(matches!(
        beside_type.to_request(&endpoint()),
        Err(RequestError::Argument(ArgumentError::SeveralSelectorsInQueryString))
    ));
    let two = [EntitySelector::new("Car".parse().unwrap()), EntitySelector::new("Bus".parse().unwrap())];
    let two_selectors = QueryEntities {
        selectors: &two,
        ..query_vehicles(None, None, Lossless::Normalized)
    };
    assert!(matches!(
        two_selectors.to_request(&endpoint()),
        Err(RequestError::Argument(ArgumentError::SeveralSelectorsInQueryString))
    ));
    let posted = QueryEntities {
        method: QueryMethod::Post,
        ..two_selectors
    };
    assert!(posted.to_request(&endpoint()).is_ok());
}

#[test]
fn every_type_is_sent_as_a_star_and_makes_the_query_local() {
    let every = SelectorType::All;
    let query = query_vehicles(Some(&every), None, Lossless::Normalized);
    assert_eq!(query.to_request(&endpoint()).unwrap().uri().query(), Some("type=*"));
    let posted = QueryEntities {
        method: QueryMethod::Post,
        ..query.clone()
    };
    assert_eq!(
        json(posted.to_request(&endpoint()).unwrap().body()),
        json(br#"{"type":"Query","entities":[{"type":"*"}]}"#)
    );
    let distributed = QueryEntities {
        locality: Locality::Distributed,
        ..query
    };
    assert!(matches!(
        distributed.to_request(&endpoint()),
        Err(RequestError::Argument(ArgumentError::AllTypesNotLocal))
    ));
    let every_selector = [EntitySelector::all()];
    let distributed_selector = QueryEntities {
        selectors: &every_selector,
        method: QueryMethod::Post,
        locality: Locality::Distributed,
        ..query_vehicles(None, None, Lossless::Normalized)
    };
    assert!(matches!(
        distributed_selector.to_request(&endpoint()),
        Err(RequestError::Argument(ArgumentError::AllTypesNotLocal))
    ));
}

#[test]
fn an_id_pattern_with_a_top_level_alternation_is_sent_whole() {
    let vehicle: SelectorType = "Vehicle".parse().unwrap();
    let pattern: IdPattern = "^a$|^b$".parse().unwrap();
    let query = QueryEntities {
        id_pattern: Some(&pattern),
        ..query_vehicles(Some(&vehicle), None, Lossless::Normalized)
    };
    assert_eq!(
        query.to_request(&endpoint()).unwrap().uri().query(),
        Some("type=Vehicle&idPattern=%5Ea%24%7C%5Eb%24")
    );
    let posted = QueryEntities {
        method: QueryMethod::Post,
        ..query
    };
    assert_eq!(
        json(posted.to_request(&endpoint()).unwrap().body()),
        json(br#"{"type":"Query","entities":[{"type":"Vehicle","idPattern":"^a$|^b$"}]}"#)
    );
}

#[test]
fn every_representation_spelling_of_table_6_3_7_1_is_sent() {
    let vehicle: SelectorType = "Vehicle".parse().unwrap();
    let spelled = |spelling: RepresentationSpelling| {
        QueryEntities {
            spelling,
            system_attributes: SystemAttributes::Include,
            ..query_vehicles(Some(&vehicle), None, Simplified)
        }
        .to_request(&endpoint())
        .unwrap()
        .uri()
        .query()
        .map(str::to_owned)
    };
    let base = RepresentationSpelling::default();
    assert_eq!(spelled(base).as_deref(), Some("type=Vehicle&format=simplified&options=sysAttrs"));
    assert_eq!(spelled(base.key_values()).as_deref(), Some("type=Vehicle&format=keyValues&options=sysAttrs"));
    assert_eq!(
        spelled(base.in_options().key_values()).as_deref(),
        Some("type=Vehicle&options=keyValues,sysAttrs")
    );
    let id = vehicle_id();
    let normalized = RetrieveEntity {
        spelling: base.explicit_normalized(),
        ..retrieve_entity(&id, Lossless::Normalized)
    };
    assert_eq!(normalized.to_request(&endpoint()).unwrap().uri().query(), Some("format=normalized"));
    let concise = RetrieveEntity {
        spelling: base.in_options(),
        ..retrieve_entity(&id, Lossless::Concise)
    };
    assert_eq!(concise.to_request(&endpoint()).unwrap().uri().query(), Some("options=concise"));
    let site: AttributeName = "site".parse().unwrap();
    let geo_json = RetrieveEntity {
        spelling: base.in_options().key_values(),
        ..retrieve_entity(
            &id,
            GeoJson {
                form: Simplified,
                geometry_property: Some(&site),
            },
        )
    };
    assert_eq!(
        geo_json.to_request(&endpoint()).unwrap().uri().query(),
        Some("geometryProperty=site&options=keyValues")
    );
}

/// The vehicle of clause 4.5.23.2 (Annex C.2.2) as a broker answers `pick=id,type,isParked{name}` with an inline
/// join: the projection applies to the linked entity too, which keeps only `name`.
const NARROWED_INLINE: &str = r#"[{
    "id": "urn:ngsi-ld:Vehicle:A4567",
    "type": "Vehicle",
    "isParked": {
        "type": "Relationship",
        "objectType": "OffStreetParking",
        "object": "urn:ngsi-ld:OffStreetParking:Downtown1",
        "entity": {
            "name": {"value": "Top Parking", "type": "Property"}
        },
        "observedAt": "2017-07-29T12:00:04Z"
    }
}]"#;

#[test]
fn an_inline_join_reads_linked_entities_a_nested_projection_narrowed() {
    let vehicle: SelectorType = "Vehicle".parse().unwrap();
    let pick: Projection = "id,type,isParked{name}".parse().unwrap();
    let query = QueryEntities {
        pick: Some(&pick),
        join: Some(Join::Inline),
        ..query_vehicles(Some(&vehicle), None, Lossless::Normalized)
    };
    assert_eq!(
        query.to_request(&endpoint()).unwrap().uri().query(),
        Some("type=Vehicle&pick=id,type,isParked%7Bname%7D&join=inline")
    );
    let page = query.decode(&context(), response(200, &[], NARROWED_INLINE)).unwrap();
    let parking = linked(&page.items()[0], "isParked");
    assert_eq!(parking.id, None);
    assert_eq!(parking.entity_type, None);
    assert_eq!(parking.attributes.len(), 1);
}

#[test]
fn a_projected_inline_join_reads_linked_entities_keeping_only_their_id() {
    let vehicle: SelectorType = "Vehicle".parse().unwrap();
    let pick: Projection = "isParked{name,id},id".parse().unwrap();
    let query = QueryEntities {
        pick: Some(&pick),
        join: Some(Join::Inline),
        ..query_projected(Some(&vehicle), None, Lossless::Concise)
    };
    assert_eq!(
        query.to_request(&endpoint()).unwrap().uri().query(),
        Some("type=Vehicle&pick=isParked%7Bname,id%7D,id&join=inline&format=concise")
    );
    let body = r#"[{"id":"urn:ngsi-ld:Vehicle:A4567","isParked":{"object":"urn:ngsi-ld:OffStreetParking:Downtown1","entity":{"id":"urn:ngsi-ld:OffStreetParking:Downtown1","name":"Top Parking"}}}]"#;
    let page = query.decode(&context(), response(200, &[], body)).unwrap();
    let entity = &page.items()[0];
    assert_eq!(entity.entity_type, None);
    let name: AttributeName = "isParked".parse().unwrap();
    let Some(Attribute::Relationship(relationship)) = entity.attribute(&name).map(Instances::first) else {
        panic!("not a Relationship");
    };
    let parking = relationship.output().entity.as_ref().and_then(OneOrMany::as_one).unwrap();
    assert_eq!(parking.id.as_ref().map(EntityId::as_str), Some("urn:ngsi-ld:OffStreetParking:Downtown1"));
    assert_eq!(parking.entity_type, None);
    assert_eq!(
        json(entity.render(Representation::Concise).to_string().as_bytes()),
        json(br#"{"id":"urn:ngsi-ld:Vehicle:A4567","isParked":{"object":"urn:ngsi-ld:OffStreetParking:Downtown1","entity":{"id":"urn:ngsi-ld:OffStreetParking:Downtown1","name":"Top Parking"}}}"#)
    );
}

#[test]
fn an_inline_retrieve_reads_linked_entities_of_a_list_relationship_without_id_and_type() {
    let id = vehicle_id();
    let pick: Projection = "id,type,route{name}".parse().unwrap();
    let retrieve = RetrieveEntity {
        pick: Some(&pick),
        join: Some(InlineJoin::Inline),
        ..retrieve_entity(&id, Lossless::Normalized)
    };
    let body = r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","route":{"type":"ListRelationship","objectType":"City","objectList":[{"object":"urn:ngsi-ld:City:Antwerp"},{"object":"urn:ngsi-ld:City:Rotterdam"}],"entityList":[{"name":{"type":"Property","value":"Antwerp"}},{"name":{"type":"Property","value":"Rotterdam"}}]}}"#;
    let entity = retrieve.decode(&context(), response(200, &[], body)).unwrap().into_body();
    let route: AttributeName = "route".parse().unwrap();
    let Some(Attribute::ListRelationship(relationship)) = entity.attribute(&route).map(Instances::first) else {
        panic!("not a ListRelationship");
    };
    let cities = relationship.output().entity_list.as_ref().unwrap();
    assert_eq!(cities.len(), 2);
    assert!(cities.iter().all(|city| city.id.is_none() && city.entity_type.is_none()));
    assert_eq!(json(entity.render(Representation::Normalized).to_string().as_bytes()), json(body.as_bytes()));
}

#[test]
fn a_flattened_join_for_complete_entities_refuses_a_nested_projection_removing_id_or_type() {
    let vehicle: SelectorType = "Vehicle".parse().unwrap();
    let narrowing: Projection = "id,type,isParked{name}".parse().unwrap();
    let flat = QueryEntities {
        pick: Some(&narrowing),
        join: Some(Join::Flat),
        ..query_vehicles(Some(&vehicle), None, Lossless::Normalized)
    };
    assert!(matches!(
        flat.to_request(&endpoint()),
        Err(RequestError::Argument(ArgumentError::ProjectionRemovesMember { member: EntityMember::Id }))
    ));
    let keeping: Projection = "id,type,isParked{id,type,name}".parse().unwrap();
    let kept = QueryEntities {
        pick: Some(&keeping),
        ..flat.clone()
    };
    assert!(kept.to_request(&endpoint()).is_ok());
    let omitting: Projection = "isParked{type}".parse().unwrap();
    let omitted = QueryEntities {
        pick: None,
        omit: Some(&omitting),
        ..flat.clone()
    };
    assert!(matches!(
        omitted.to_request(&endpoint()),
        Err(RequestError::Argument(ArgumentError::ProjectionRemovesMember { member: EntityMember::Type }))
    ));
    let projected = QueryEntities {
        pick: Some(&narrowing),
        join: Some(Join::Flat),
        ..query_projected(Some(&vehicle), None, Lossless::Normalized)
    };
    assert!(projected.to_request(&endpoint()).is_ok());
}

#[test]
fn a_type_selection_cannot_hold_every_type_beside_another() {
    for text in ["*|Vehicle", "(*;A)", "Vehicle,*"] {
        assert!(text.parse::<SelectorType>().is_err(), "{text}");
    }
    assert!(SelectorTypeBuilder::from("*").build().is_err());
    let alone = [EntitySelector::all()];
    let query = QueryEntities {
        selectors: &alone,
        ..query_vehicles(None, None, Lossless::Normalized)
    };
    assert_eq!(query.to_request(&endpoint()).unwrap().uri().query(), Some("type=*"));
}
