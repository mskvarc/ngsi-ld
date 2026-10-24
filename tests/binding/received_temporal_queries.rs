//! Every type (`*`) and whole id patterns on the temporal query, Purge Entities, the entity map creations and the
//! registration query (Tables 6.18.3.2-1, 6.4.3.3-1, 6.8.3.2-1, clause 5.14.4.3); the temporal query strings read back
//! in typed form (Tables 6.18.3.2-1 and 6.19.3.1-1); and a POST body that is exactly the `Query` document rendered
//! (clause 5.2.23).

use crate::{fixtures::endpoint, query_vehicles};
use iri_rs::iri;
use ngsi_ld::{
    http::{
        QueryParameter,
        RequestError,
        operation::{
            Aggregated,
            ArgumentError,
            CreateEntityMap,
            EntityMapRequest,
            Operation,
            ParameterError,
            PurgeEntities,
            QueryEntities,
            QueryEntitiesParameters,
            QueryMethod,
            QueryRegistrations,
            QueryTemporalEvolution,
            QueryTemporalEvolutionParameters,
            QueryTemporalEvolutionPostParameters,
            RepresentationSpelling,
            RepresentationToken,
            ResultsCount,
            RetrieveTemporalEvolution,
            RetrieveTemporalEvolutionParameters,
            Simplified,
            SimplifiedTemporal,
            SystemAttributes,
            TemporalRepresentation,
            TemporalSelection,
            TemporalSelectionParameters,
        },
    },
    model::{AttributeName, DocumentError, EntityId, Lossless, MemberError, evolution::TemporalAttributeName},
    query::{
        EntitySplitting,
        IdPattern,
        Join,
        Locality,
        SelectorType,
        temporal_query::{TemporalProperty, TemporalQuery, TemporalQueryError},
        type_selection::TypeSelection,
    },
    snapshot::{EntityQuery, QueryDocument, TemporalEvolutionQuery},
};
use pretty_assertions::assert_eq;
use std::{
    num::{NonZeroU32, NonZeroU64},
    str,
};

/// A GET query of the temporal evolution of `entity_type`, within `temporal_q`, with no other parameter.
fn temporal_query<'a, F>(entity_type: Option<&'a SelectorType>, temporal_q: &'a TemporalQuery, format: F) -> QueryTemporalEvolution<'a, F> {
    QueryTemporalEvolution::builder()
        .maybe_entity_type(entity_type)
        .temporal_q(temporal_q)
        .format(format)
        .build()
}

/// A purge of `entity_type`, with no other parameter.
fn purge(entity_type: Option<&SelectorType>) -> PurgeEntities<'_> {
    PurgeEntities::builder().maybe_entity_type(entity_type).build()
}

/// The query string of the request `operation` builds.
fn query_string(operation: &impl Operation) -> Option<String> {
    operation.to_request(&endpoint()).unwrap().uri().query().map(str::to_owned)
}

/// A whole id pattern with a top-level alternation.
fn either() -> IdPattern {
    "^urn:ngsi-ld:Vehicle:A.*$|^urn:ngsi-ld:Vehicle:B.*$".parse().unwrap()
}

#[test]
fn a_temporal_query_reads_back_with_every_type_a_whole_id_pattern_and_both_representation_parameters() {
    let every = SelectorType::All;
    let pattern = either();
    let window = TemporalQuery::between("2024-05-01T10:00:00Z".parse().unwrap(), "2024-05-02T10:00:00Z".parse().unwrap())
        .with_time_property(TemporalProperty::ModifiedAt)
        .with_last_n(NonZeroU64::new(4).unwrap())
        .with_aggregation_methods("min,max".parse().unwrap())
        .with_aggregation_period("PT1H".parse().unwrap());
    let query = QueryTemporalEvolution {
        id_pattern: Some(&pattern),
        limit: Some(5),
        count: ResultsCount::Include,
        system_attributes: SystemAttributes::Include,
        spelling: RepresentationSpelling::default().format_over_options(RepresentationToken::TemporalValues),
        ..temporal_query(Some(&every), &window, Aggregated)
    };
    let sent = query_string(&query).unwrap();
    assert!(sent.starts_with("type=*&idPattern=%5Eurn%3Angsi-ld%3AVehicle%3AA.%2A%24%7C%5E"), "{sent}");
    let received = QueryTemporalEvolutionParameters::parse(&sent).unwrap();
    assert_eq!(received.selection.entity_type, Some(SelectorType::All));
    assert_eq!(received.selection.id_pattern.as_ref(), Some(&pattern));
    assert_eq!(received.temporal_q, window);
    assert_eq!(received.representation, TemporalRepresentation::Aggregated);
    assert_eq!(received.spelling, query.spelling);
    assert_eq!(received.system_attributes, SystemAttributes::Include);
    assert_eq!((received.limit, received.count), (Some(5), ResultsCount::Include));
    let distributed = QueryTemporalEvolution {
        locality: Locality::Distributed,
        ..query
    };
    assert!(matches!(
        distributed.to_request(&endpoint()),
        Err(RequestError::Argument(ArgumentError::AllTypesNotLocal))
    ));
}

#[test]
fn a_retrieve_without_a_time_bound_reads_back_its_standing_alone_members() {
    let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:B9211"));
    let returned = EntityMapRequest::returned();
    let retrieve = RetrieveTemporalEvolution::builder()
        .id(&id)
        .selection(TemporalSelection::Unbounded {
            time_property: Some(TemporalProperty::ObservedAt),
            last_n: NonZeroU64::new(3),
            aggregation_methods: None,
            aggregation_period: None,
        })
        .entity_map(&returned)
        .format(SimplifiedTemporal)
        .spelling(RepresentationSpelling::default().in_options())
        .build();
    let sent = query_string(&retrieve).unwrap();
    assert_eq!(sent, "timeproperty=observedAt&lastN=3&entityMap=true&options=temporalValues");
    let received = RetrieveTemporalEvolutionParameters::parse(&sent).unwrap();
    assert_eq!(
        received.selection,
        TemporalSelectionParameters::Unbounded {
            time_property: Some(TemporalProperty::ObservedAt),
            last_n: NonZeroU64::new(3),
            aggregation_methods: None,
            aggregation_period: None,
        }
    );
    assert_eq!(received.representation, TemporalRepresentation::Simplified);
    assert_eq!(received.spelling, RepresentationSpelling::default().in_options());
}

#[test]
fn a_received_temporal_query_names_the_scope_in_attrs_and_a_query_of_entities_does_not() {
    let query = QueryTemporalEvolutionParameters::parse("type=Vehicle&attrs=speed,scope&timerel=after&timeAt=2018-08-01T12%3A00%3A00Z").unwrap();
    assert_eq!(
        query.selection.attrs,
        [
            TemporalAttributeName::from("speed".parse::<AttributeName>().unwrap()),
            TemporalAttributeName::Scope
        ]
    );
    let retrieve = RetrieveTemporalEvolutionParameters::parse("attrs=scope").unwrap();
    assert_eq!(retrieve.attrs, [TemporalAttributeName::Scope]);
    assert!(matches!(
        QueryEntitiesParameters::parse("type=Vehicle&attrs=speed,scope").unwrap_err(),
        ParameterError::Name {
            parameter: QueryParameter::Attrs,
            ..
        }
    ));
}

#[test]
fn an_invalid_temporal_query_string_is_refused_naming_the_parameter() {
    let missing = QueryTemporalEvolutionParameters::parse("type=Vehicle").unwrap_err();
    assert!(matches!(
        missing,
        ParameterError::TemporalQuery {
            source: DocumentError {
                kind: TemporalQueryError::Member(MemberError::MissingMember { member: "timerel" }),
                ..
            }
        }
    ));
    let without_end = QueryTemporalEvolutionParameters::parse("timerel=between&timeAt=2024-05-01T10%3A00%3A00Z").unwrap_err();
    assert!(matches!(without_end, ParameterError::TemporalQuery { .. }));
    assert!(matches!(
        RetrieveTemporalEvolutionParameters::parse("lastN=0").unwrap_err(),
        ParameterError::TemporalQuery {
            source: DocumentError {
                kind: TemporalQueryError::LastN,
                ..
            }
        }
    ));
    assert!(matches!(
        RetrieveTemporalEvolutionParameters::parse("format=keyValues").unwrap_err(),
        ParameterError::UnknownToken {
            parameter: QueryParameter::Format,
            ..
        }
    ));
    assert!(matches!(
        RetrieveTemporalEvolutionParameters::parse("limit=1").unwrap_err(),
        ParameterError::Unknown { name } if name == "limit"
    ));
}

#[test]
fn a_purge_of_every_type_is_local_and_takes_a_whole_id_pattern() {
    let every = SelectorType::All;
    assert_eq!(query_string(&purge(Some(&every))).as_deref(), Some("type=*"));
    let distributed = PurgeEntities {
        locality: Locality::Distributed,
        ..purge(Some(&every))
    };
    assert!(matches!(
        distributed.to_request(&endpoint()),
        Err(RequestError::Argument(ArgumentError::AllTypesNotLocal))
    ));
    let vehicle: SelectorType = "Vehicle".parse().unwrap();
    let pattern: IdPattern = "^a$|^b$".parse().unwrap();
    let patterned = PurgeEntities {
        id_pattern: Some(&pattern),
        ..purge(Some(&vehicle))
    };
    assert_eq!(query_string(&patterned).as_deref(), Some("type=Vehicle&idPattern=%5Ea%24%7C%5Eb%24"));
}

#[test]
fn an_entity_map_creation_and_a_registration_query_take_every_type_and_a_whole_id_pattern() {
    let every = SelectorType::All;
    let pattern: IdPattern = "^a$|^b$".parse().unwrap();
    let creation = CreateEntityMap {
        entity_type: Some(&every),
        id_pattern: Some(&pattern),
        ..CreateEntityMap::default()
    };
    assert_eq!(query_string(&creation).as_deref(), Some("type=*&idPattern=%5Ea%24%7C%5Eb%24"));
    let vehicles: TypeSelection = "Vehicle".parse().unwrap();
    let registrations = QueryRegistrations {
        entity_type: Some(&vehicles),
        id_pattern: Some(&pattern),
        ..QueryRegistrations::default()
    };
    assert_eq!(query_string(&registrations).as_deref(), Some("type=Vehicle&idPattern=%5Ea%24%7C%5Eb%24"));
}

#[test]
fn a_post_query_body_is_exactly_the_query_document_rendered() {
    let vehicle: SelectorType = "Vehicle".parse().unwrap();
    let ids = [EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"))];
    let contained_by = ids.clone();
    let entity_map = EntityMapRequest::returned_for("PT1H".parse().unwrap());
    let query = QueryEntities {
        ids: &ids,
        join: Some(Join::Flat),
        join_level: NonZeroU32::new(2),
        contained_by: &contained_by,
        split_entities: EntitySplitting::Whole,
        entity_map: Some(&entity_map),
        method: QueryMethod::Post,
        ..query_vehicles(Some(&vehicle), None, Simplified)
    };
    let request = query.to_request(&endpoint()).unwrap();
    let body = str::from_utf8(request.body()).unwrap();
    assert_eq!(QueryDocument::<EntityQuery>::parse(body).unwrap().render().to_string(), body);
    let window = TemporalQuery::after("2024-05-01T10:00:00Z".parse().unwrap());
    let temporal = QueryTemporalEvolution {
        method: QueryMethod::Post,
        ..temporal_query(Some(&vehicle), &window, Lossless::Normalized)
    };
    let request = temporal.to_request(&endpoint()).unwrap();
    let body = str::from_utf8(request.body()).unwrap();
    assert_eq!(
        body,
        r#"{"type":"Query","entities":[{"type":"Vehicle"}],"temporalQ":{"timerel":"after","timeAt":"2024-05-01T10:00:00Z"}}"#
    );
    assert_eq!(QueryDocument::<TemporalEvolutionQuery>::parse(body).unwrap().render().to_string(), body);
    let forwarded = r#"{"type":"Query","entities":[{"type":"Vehicle"}],"containedBy":["urn:ngsi-ld:Vehicle:A4567"]}"#;
    assert_eq!(QueryDocument::<EntityQuery>::parse(forwarded).unwrap().contained_by(), ids);
}

/// The query string of a POST temporal query holds only what its `Query` body cannot (clause 6.24.3.1): the GET
/// decoder needs the `timerel` the body carries, so only the POST decoder reads it.
#[test]
fn a_post_temporal_query_string_reads_back_with_the_post_decoder_only() {
    let vehicle: SelectorType = "Vehicle".parse().unwrap();
    let window = TemporalQuery::after("2024-05-01T10:00:00Z".parse().unwrap());
    let query = QueryTemporalEvolution {
        system_attributes: SystemAttributes::Include,
        limit: Some(5),
        offset: Some(10),
        count: ResultsCount::Include,
        locality: Locality::Local,
        method: QueryMethod::Post,
        ..temporal_query(Some(&vehicle), &window, SimplifiedTemporal)
    };
    let sent = query_string(&query).unwrap();
    let received = QueryTemporalEvolutionPostParameters::parse(&sent).unwrap();
    assert_eq!(received.representation, TemporalRepresentation::Simplified);
    assert_eq!(received.system_attributes, SystemAttributes::Include);
    assert_eq!(
        (received.limit, received.offset, received.count, received.locality),
        (Some(5), Some(10), ResultsCount::Include, Locality::Local)
    );
    assert!(matches!(
        QueryTemporalEvolutionParameters::parse(&sent),
        Err(ParameterError::TemporalQuery {
            source: DocumentError {
                kind: TemporalQueryError::Member(MemberError::MissingMember { member: "timerel" }),
                ..
            }
        })
    ));
    for selection in ["type=Vehicle", "timerel=after", "attrs=speed"] {
        assert!(
            matches!(QueryTemporalEvolutionPostParameters::parse(selection), Err(ParameterError::Unknown { .. })),
            "{selection}"
        );
    }
}
