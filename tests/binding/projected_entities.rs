//! Projected entities (clauses 4.21 and 6.4.3.2): which projections a
//! request for complete entities refuses, and how the answers of projected requests are read.

use crate::{
    fixtures::{context, endpoint, response, vehicle_id},
    query_projected,
    query_vehicles,
    retrieve_entity,
    retrieve_projected,
};
use ngsi_ld::{
    http::{
        RequestError,
        operation::{ArgumentError, Operation, QueryEntities, RetrieveEntity, Simplified},
    },
    model::Lossless,
    query::{
        Join,
        SelectorType,
        projection::{EntityMember, Projection},
    },
};
use pretty_assertions::assert_eq;

#[test]
fn a_request_for_complete_entities_refuses_a_projection_removing_id_or_type() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let pick: Projection = "speed".parse().unwrap();
    let query = QueryEntities {
        pick: Some(&pick),
        ..query_vehicles(Some(&entity_type), None, Lossless::Normalized)
    };
    assert!(matches!(
        query.to_request(&endpoint()).unwrap_err(),
        RequestError::Argument(ArgumentError::ProjectionRemovesMember { member: EntityMember::Id })
    ));
    let id = vehicle_id();
    let omit: Projection = "type".parse().unwrap();
    let retrieve = RetrieveEntity {
        omit: Some(&omit),
        ..retrieve_entity(&id, Simplified)
    };
    assert!(matches!(
        retrieve.to_request(&endpoint()).unwrap_err(),
        RequestError::Argument(ArgumentError::ProjectionRemovesMember { member: EntityMember::Type })
    ));
    let keeping: Projection = "id,type,speed".parse().unwrap();
    let retrieve = RetrieveEntity {
        pick: Some(&keeping),
        ..retrieve_entity(&id, Simplified)
    };
    assert_eq!(
        retrieve.to_request(&endpoint()).unwrap().uri().query(),
        Some("pick=id,type,speed&format=simplified")
    );
}

#[test]
fn a_projected_query_sends_the_projection_and_reads_entities_without_id() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let pick: Projection = "speed".parse().unwrap();
    let query = QueryEntities {
        pick: Some(&pick),
        ..query_projected(Some(&entity_type), None, Lossless::Normalized)
    };
    assert_eq!(query.to_request(&endpoint()).unwrap().uri().query(), Some("type=Vehicle&pick=speed"));
    let page = query
        .decode(&context(), response(200, &[], r#"[{"speed":{"type":"Property","value":80}}]"#))
        .unwrap();
    assert_eq!(page.items().len(), 1);
    assert_eq!(page.items()[0].id, None);
    assert_eq!(page.items()[0].entity_type, None);
}

#[test]
fn a_projected_inline_join_sends_a_projection_removing_id_or_type() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let pick: Projection = "speed".parse().unwrap();
    let inline = QueryEntities {
        pick: Some(&pick),
        join: Some(Join::Inline),
        ..query_projected(Some(&entity_type), None, Lossless::Normalized)
    };
    assert_eq!(
        inline.to_request(&endpoint()).unwrap().uri().query(),
        Some("type=Vehicle&pick=speed&join=inline")
    );
    let flat = QueryEntities {
        join: Some(Join::Flat),
        ..inline.clone()
    };
    assert_eq!(flat.to_request(&endpoint()).unwrap().uri().query(), Some("type=Vehicle&pick=speed&join=flat"));
    let keeping: Projection = "id,type,speed".parse().unwrap();
    let inline_keeping = QueryEntities {
        pick: Some(&keeping),
        ..inline
    };
    assert!(inline_keeping.to_request(&endpoint()).is_ok());
}

#[test]
fn a_projected_simplified_retrieve_omits_id_and_type() {
    let id = vehicle_id();
    let omit: Projection = "id,type".parse().unwrap();
    let retrieve = RetrieveEntity {
        omit: Some(&omit),
        ..retrieve_projected(&id, Simplified)
    };
    assert_eq!(retrieve.to_request(&endpoint()).unwrap().uri().query(), Some("omit=id,type&format=simplified"));
    let entity = retrieve.decode(&context(), response(200, &[], r#"{"speed":80}"#)).unwrap().into_body();
    assert_eq!(entity.id, None);
    assert_eq!(entity.render().to_string(), r#"{"speed":80}"#);
}

#[test]
fn a_projected_flattened_retrieve_reads_every_entity_without_id() {
    let id = vehicle_id();
    let pick: Projection = "speed".parse().unwrap();
    let retrieve = RetrieveEntity::flattened_projected_builder()
        .id(&id)
        .pick(&pick)
        .format(Lossless::Concise)
        .build();
    assert_eq!(
        retrieve.to_request(&endpoint()).unwrap().uri().query(),
        Some("pick=speed&join=flat&format=concise")
    );
    let entities = retrieve
        .decode(&context(), response(200, &[], r#"[{"speed":80},{"speed":40}]"#))
        .unwrap()
        .into_body();
    assert_eq!(entities.len(), 2);
    assert!(entities.iter().all(|entity| entity.id.is_none()));
}
