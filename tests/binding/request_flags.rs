//! The request flags and inputs the specification lets a client state explicitly: `details=false` and
//! `reload=false` (Tables 6.25.3.1-1, 6.27.3.1-1, 6.29.3.2-1, 6.30.3.1-1 and 6.30.3.2-1), the members Add @context
//! sends beside the context (clause 5.13.2.3), the entity map inputs of Purge Entities and of the temporal
//! operations (Table 6.4.3.3-2, clauses 5.7.3.3 and 5.7.4.3), `timeproperty` without a time window (Table
//! 6.19.3.1-1), and the parameter that names a temporal representation (clause 6.3.12).

use crate::fixtures::{endpoint, header_value, json};
use bytes::Bytes;
use headers::Header;
use http::{HeaderValue, Method, Request};
use indexmap::IndexMap;
use iri_rs::{IriRefBuf, iri};
use jstrict::Value as Json;
use ngsi_ld::{
    http::{
        Coverage,
        RequestError,
        headers::{ContentRange, NgsiLdEntityMap, OptionalHeader},
        operation::{
            AddContext,
            Aggregated,
            ArgumentError,
            ContextDeletion,
            ContextKindFilter,
            DeleteContext,
            EntityMapRequest,
            ListContexts,
            Operation,
            PurgeEntities,
            QueryMethod,
            QueryTemporalEvolution,
            RepresentationSpelling,
            RepresentationToken,
            RetrieveAttributes,
            RetrieveEntityTypes,
            RetrieveTemporalEvolution,
            ServeContext,
            SimplifiedTemporal,
            SummaryDetails,
            SystemAttributes,
            TemporalSelection,
        },
    },
    model::{Context, ContextId, EntityId, Lossless, Value},
    query::{
        EntityMapReturn,
        Locality,
        SelectorType,
        temporal_query::{AggregationMethods, TemporalProperty, TemporalQuery},
    },
};
use pretty_assertions::assert_eq;
use std::num::NonZeroU64;

/// The request an operation builds against the binding endpoint.
fn built(operation: &impl Operation) -> Request<Bytes> {
    operation.to_request(&endpoint()).unwrap()
}

/// The query string of the request an operation builds.
fn query_string(operation: &impl Operation) -> Option<String> {
    built(operation).uri().query().map(str::to_owned)
}

/// The map at `/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1`.
fn entity_map() -> NgsiLdEntityMap {
    NgsiLdEntityMap::new(IriRefBuf::new("/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1".to_owned()).unwrap()).unwrap()
}

/// `B`.
fn b() -> EntityId {
    EntityId::from(iri!("urn:ngsi-ld:Vehicle:B9211"))
}

/// A retrieve of `id` with `selection`, no other parameter, in `format`.
fn retrieve<'a, F>(id: &'a EntityId, selection: TemporalSelection<'a>, format: F) -> RetrieveTemporalEvolution<'a, F> {
    RetrieveTemporalEvolution::builder().id(id).selection(selection).format(format).build()
}

/// A selection without a temporal query or any option.
const fn unbounded() -> TemporalSelection<'static> {
    TemporalSelection::Unbounded {
        time_property: None,
        last_n: None,
        aggregation_methods: None,
        aggregation_period: None,
    }
}

/// A query of the types `entity_type` with `temporal_q`, no other parameter, in `format`.
fn query<'a, F>(entity_type: &'a SelectorType, temporal_q: &'a TemporalQuery, format: F) -> QueryTemporalEvolution<'a, F> {
    QueryTemporalEvolution::builder()
        .entity_type(entity_type)
        .temporal_q(temporal_q)
        .format(format)
        .build()
}

#[test]
fn every_summary_request_may_state_details_false() {
    let explicit = SummaryDetails::Explicit;
    let types = RetrieveEntityTypes {
        details: explicit,
        ..RetrieveEntityTypes::default()
    };
    assert_eq!(query_string(&types).as_deref(), Some("details=false"));
    let attributes = RetrieveAttributes::builder().details(explicit).locality(Locality::Local).build();
    assert_eq!(query_string(&attributes).as_deref(), Some("details=false&local=true"));
    let hosted = ListContexts::builder().kind(ContextKindFilter::Hosted).details(explicit).build();
    assert_eq!(query_string(&hosted).as_deref(), Some("details=false&kind=Hosted"));
    let id: ContextId = "urn:ngsi-ld:context:c1".parse().unwrap();
    let serve = ServeContext::builder().id(&id).details(explicit).build();
    assert_eq!(query_string(&serve).as_deref(), Some("details=false"));
    assert_eq!(query_string(&RetrieveEntityTypes::default()), None);
    assert_eq!(query_string(&ListContexts::default()), None);
}

#[test]
fn delete_context_may_state_reload_false() {
    let id: ContextId = "urn:ngsi-ld:context:c1".parse().unwrap();
    let explicit = built(&DeleteContext::builder().id(&id).deletion(ContextDeletion::RemoveExplicitly).build());
    assert_eq!(explicit.method(), Method::DELETE);
    assert_eq!(explicit.uri().query(), Some("reload=false"));
    let removal = DeleteContext::builder().id(&id).build();
    assert_eq!(query_string(&removal), None);
}

#[test]
fn add_context_sends_the_members_beside_the_context_in_order() {
    let context = Context::from(IriRefBuf::new("https://example.org/context.jsonld").unwrap());
    let members = IndexMap::from([
        ("note".parse().unwrap(), Value::from("discarded")),
        ("extra".parse().unwrap(), Value::from([1, 2])),
    ]);
    let request = built(&AddContext::builder().context(&context).extra_members(&members).build());
    assert_eq!(
        json(request.body()),
        json(br#"{"@context":"https://example.org/context.jsonld","note":"discarded","extra":[1,2]}"#)
    );
    let none = built(&AddContext::builder().context(&context).build());
    assert_eq!(json(none.body()), json(br#"{"@context":"https://example.org/context.jsonld"}"#));
}

#[test]
fn purge_entities_names_its_entity_map_in_the_header_only() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let map = entity_map();
    let purge = PurgeEntities::builder().entity_type(&entity_type).entity_map(&map).build();
    let request = built(&purge);
    assert_eq!(request.method(), Method::DELETE);
    assert_eq!(request.uri().query(), Some("type=Vehicle"));
    assert_eq!(
        header_value(&request, NgsiLdEntityMap::name()),
        Some("/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1")
    );
}

#[test]
fn retrieve_temporal_evolution_ranks_the_last_instances_by_a_time_property_without_a_window() {
    let id = b();
    let selection = TemporalSelection::Unbounded {
        time_property: Some(TemporalProperty::ModifiedAt),
        last_n: NonZeroU64::new(2),
        aggregation_methods: None,
        aggregation_period: None,
    };
    assert_eq!(
        query_string(&retrieve(&id, selection, Lossless::Normalized)).as_deref(),
        Some("timeproperty=modifiedAt&lastN=2")
    );
    let deleted = TemporalSelection::Unbounded {
        time_property: Some(TemporalProperty::DeletedAt),
        last_n: None,
        aggregation_methods: None,
        aggregation_period: None,
    };
    assert_eq!(
        query_string(&retrieve(&id, deleted, SimplifiedTemporal)).as_deref(),
        Some("timeproperty=deletedAt&format=temporalValues")
    );
}

#[test]
fn retrieve_temporal_evolution_carries_the_entity_map_inputs_of_clause_5_7_3_3() {
    let id = b();
    let request = EntityMapRequest::returned_for("PT1H".parse().unwrap()).reusing(entity_map());
    let operation = RetrieveTemporalEvolution {
        entity_map: Some(&request),
        ..retrieve(&id, unbounded(), Lossless::Normalized)
    };
    let sent = built(&operation);
    assert_eq!(sent.uri().query(), Some("entityMap=true&entityMapLifetime=PT1H"));
    assert_eq!(
        header_value(&sent, NgsiLdEntityMap::name()),
        Some("/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1")
    );
}

#[test]
fn query_temporal_evolution_sends_the_flag_and_the_header_together_in_both_bindings() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let window = TemporalQuery::after("2018-08-01T12:03:00Z".parse().unwrap());
    let request = EntityMapRequest::returned().reusing(entity_map());
    let get = QueryTemporalEvolution {
        entity_map: Some(&request),
        ..query(&entity_type, &window, Lossless::Normalized)
    };
    let sent = built(&get);
    assert_eq!(
        sent.uri().query(),
        Some("type=Vehicle&timerel=after&timeAt=2018-08-01T12%3A03%3A00Z&entityMap=true")
    );
    assert_eq!(
        header_value(&sent, NgsiLdEntityMap::name()),
        Some("/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1")
    );
    let not_returned = EntityMapRequest {
        returned: EntityMapReturn::NotReturned,
        ..EntityMapRequest::reuse(entity_map())
    };
    let post = QueryTemporalEvolution {
        entity_map: Some(&not_returned),
        method: QueryMethod::Post,
        ..query(&entity_type, &window, Lossless::Normalized)
    };
    let sent = built(&post);
    let body = json(sent.body());
    let object = body.as_object().unwrap();
    assert_eq!(object.get("entityMap").collect::<Vec<_>>(), [&Json::from(false)]);
    assert_eq!(
        header_value(&sent, NgsiLdEntityMap::name()),
        Some("/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1")
    );
}

#[test]
fn the_temporal_representation_is_named_in_format_options_or_both() {
    let id = b();
    let options = RetrieveTemporalEvolution {
        spelling: RepresentationSpelling::default().in_options(),
        system_attributes: SystemAttributes::Include,
        ..retrieve(&id, unbounded(), SimplifiedTemporal)
    };
    assert_eq!(query_string(&options).as_deref(), Some("options=temporalValues,sysAttrs"));
    let methods: AggregationMethods = "totalCount".parse().unwrap();
    let aggregated = TemporalSelection::Unbounded {
        time_property: None,
        last_n: None,
        aggregation_methods: Some(&methods),
        aggregation_period: None,
    };
    let deprecated = RetrieveTemporalEvolution {
        spelling: RepresentationSpelling::default().in_options(),
        ..retrieve(&id, aggregated, Aggregated)
    };
    assert_eq!(query_string(&deprecated).as_deref(), Some("aggrMethods=totalCount&options=aggregatedValues"));
    let both = RetrieveTemporalEvolution {
        spelling: RepresentationSpelling::default().format_over_options(RepresentationToken::AggregatedValues),
        ..retrieve(&id, unbounded(), SimplifiedTemporal)
    };
    assert!(matches!(
        both.to_request(&endpoint()),
        Err(RequestError::Argument(ArgumentError::AggregationWithoutMethods))
    ));
    let both = RetrieveTemporalEvolution {
        spelling: RepresentationSpelling::default().format_over_options(RepresentationToken::AggregatedValues),
        ..retrieve(&id, aggregated, SimplifiedTemporal)
    };
    assert_eq!(
        query_string(&both).as_deref(),
        Some("aggrMethods=totalCount&format=temporalValues&options=aggregatedValues")
    );
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let window = TemporalQuery::after("2018-08-01T12:03:00Z".parse().unwrap());
    let get = QueryTemporalEvolution {
        spelling: RepresentationSpelling::default().in_options(),
        ..query(&entity_type, &window, SimplifiedTemporal)
    };
    assert_eq!(
        query_string(&get).as_deref(),
        Some("type=Vehicle&timerel=after&timeAt=2018-08-01T12%3A03%3A00Z&options=temporalValues")
    );
    let post = QueryTemporalEvolution {
        spelling: RepresentationSpelling::default().format_over_options(RepresentationToken::TemporalValues),
        method: QueryMethod::Post,
        ..query(&entity_type, &window, Lossless::Normalized)
    };
    assert!(matches!(
        post.to_request(&endpoint()),
        Err(RequestError::Argument(ArgumentError::OptionsKeywordWithoutFormat {
            keyword: RepresentationToken::TemporalValues
        }))
    ));
    let concise = QueryTemporalEvolution {
        spelling: RepresentationSpelling::default().format_over_options(RepresentationToken::TemporalValues),
        method: QueryMethod::Post,
        ..query(&entity_type, &window, Lossless::Concise)
    };
    assert_eq!(query_string(&concise).as_deref(), Some("format=concise&options=temporalValues"));
}

#[test]
fn the_rest_of_an_unbounded_range_keeps_its_time_property() {
    let value = HeaderValue::from_static("DateTime 2018-08-01T13:00:00Z-2018-08-01T12:05:00Z/2");
    let range = ContentRange::decode(&mut [&value].into_iter()).unwrap();
    let partial = Coverage::Partial(OptionalHeader::Valid(range));
    let selection = TemporalSelection::Unbounded {
        time_property: Some(TemporalProperty::ModifiedAt),
        last_n: NonZeroU64::new(2),
        aggregation_methods: None,
        aggregation_period: None,
    };
    assert_eq!(
        partial.following_query(selection),
        Some(
            TemporalQuery::before("2018-08-01T12:05:00Z".parse().unwrap())
                .with_time_property(TemporalProperty::ModifiedAt)
                .with_last_n(NonZeroU64::new(2).unwrap())
        )
    );
}
