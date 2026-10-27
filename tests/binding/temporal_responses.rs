//! How the answers of the temporal operations are read (clauses 6.18 to 6.22 and 6.3.10).

use crate::{
    fixtures::{context, endpoint, response},
    links::{link_field, link_value},
    temporal_fixtures::{T1, T2, T3, b, evolution, instance_id, speed, time, window},
};
use headers::Header;
use http::{HeaderValue, header};
use iri_rs::IriRefBuf;
use ngsi_ld::{
    http::{
        BodyError,
        Coverage,
        ResponseContext,
        ResponseError,
        TemporalUpsertOutcome,
        headers::{ContentRange, ContentRangeSize, NgsiLdResultsCount, OptionalHeader, PreferenceApplied, RelationType},
        operation::{
            AddTemporalAttributes,
            Aggregated,
            CreateOrUpdateTemporalEvolution,
            DeleteAttributeInstance,
            DeleteTemporalAttribute,
            DeleteTemporalEvolution,
            InstanceSelection,
            ModifyAttributeInstance,
            Operation,
            QueryTemporalEvolution,
            RetrieveTemporalEvolution,
            SimplifiedTemporal,
            TemporalSelection,
        },
    },
    model::{
        Attribute,
        Complete,
        DocumentErrorKind,
        EntityId,
        ErrorType,
        Input,
        Lossless,
        MemberError,
        Property,
        evolution::{Evolution, TemporalAttributes},
    },
    query::{
        SelectorType,
        temporal_query::{AggregationMethods, TemporalQuery},
    },
};
use pretty_assertions::assert_eq;
use std::num::NonZeroU64;

/// `B`'s evolution as a broker returns it: two `speed` instances, each with an `instanceId`.
const RETRIEVED: &str = concat!(
    r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","speed":["#,
    r#"{"type":"Property","value":120,"observedAt":"2018-08-01T12:03:00Z","instanceId":"urn:ngsi-ld:attribute:instance:1"},"#,
    r#"{"type":"Property","value":80,"observedAt":"2018-08-01T12:05:00Z","instanceId":"urn:ngsi-ld:attribute:instance:2"}]}"#
);

/// `B`'s evolution in simplified temporal representation.
const VALUES: &str =
    r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","speed":{"type":"Property","values":[[120,"2018-08-01T12:03:00Z"],[80,"2018-08-01T12:05:00Z"]]}}"#;

/// `B`'s evolution in aggregated temporal representation.
const AGGREGATED: &str =
    r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","speed":{"type":"Property","max":[[120,"2018-08-01T12:03:00Z","2018-08-01T12:07:00Z"]]}}"#;

/// A retrieve of `id` with `window`, in `format`.
fn retrieve<'a, F>(id: &'a EntityId, window: &'a TemporalQuery, format: F) -> RetrieveTemporalEvolution<'a, F, Complete> {
    RetrieveTemporalEvolution::builder()
        .id(id)
        .selection(TemporalSelection::Query(window))
        .format(format)
        .build()
}

/// A GET query of `Vehicle` with `window`, normalized.
fn query<'a>(entity_type: &'a SelectorType, window: &'a TemporalQuery) -> QueryTemporalEvolution<'a, Lossless, Complete> {
    QueryTemporalEvolution::builder()
        .entity_type(entity_type)
        .temporal_q(window)
        .format(Lossless::Normalized)
        .build()
}

/// The status of an unexpected-status error.
fn is_unexpected<T>(result: &Result<T, ResponseError>) -> bool {
    matches!(result, Err(ResponseError::UnexpectedStatus { .. }))
}

#[test]
fn create_or_update_reads_created_with_its_location_or_updated() {
    let entity = evolution();
    let operation = CreateOrUpdateTemporalEvolution::builder().entity(&entity).build();
    let location = "/ngsi-ld/v1/temporal/entities/urn:ngsi-ld:Vehicle:B9211";
    assert_eq!(
        operation.decode(&context(), response(201, &[(&header::LOCATION, location)], "")).unwrap(),
        TemporalUpsertOutcome::Created {
            location: OptionalHeader::Valid(IriRefBuf::new(location.to_owned()).unwrap())
        }
    );
    assert_eq!(
        operation.decode(&context(), response(201, &[], "")).unwrap(),
        TemporalUpsertOutcome::Created {
            location: OptionalHeader::Absent
        }
    );
    assert_eq!(operation.decode(&context(), response(204, &[], "")).unwrap(), TemporalUpsertOutcome::Updated);
    assert!(is_unexpected(&operation.decode(&context(), response(200, &[], ""))));
}

#[test]
fn the_operations_that_change_an_evolution_read_204_only() {
    let id = b();
    let name = speed();
    let instance_id = instance_id();
    let instance: Attribute<Input> = Property::builder(90).build().unwrap().into();
    let attributes = TemporalAttributes::<Input>::new();
    let add = AddTemporalAttributes::builder().id(&id).attributes(&attributes).build();
    let delete_attribute = DeleteTemporalAttribute::builder().id(&id).name(&name).instances(InstanceSelection::All).build();
    let modify = ModifyAttributeInstance::builder()
        .id(&id)
        .name(&name)
        .instance_id(&instance_id)
        .instance(&instance)
        .build();
    let delete_instance = DeleteAttributeInstance {
        id: &id,
        name: &name,
        instance_id: &instance_id,
    };
    let delete = DeleteTemporalEvolution { id: &id };
    let decoders: [&dyn Fn(u16) -> Result<(), ResponseError>; 5] = [
        &|status| add.decode(&context(), response(status, &[], "")),
        &|status| delete_attribute.decode(&context(), response(status, &[], "")),
        &|status| modify.decode(&context(), response(status, &[], "")),
        &|status| delete_instance.decode(&context(), response(status, &[], "")),
        &|status| delete.decode(&context(), response(status, &[], "")),
    ];
    for decode in decoders {
        assert_eq!(decode(204).unwrap(), ());
        assert!(is_unexpected(&decode(200)));
    }
}

#[test]
fn a_retrieved_evolution_keeps_every_instance_and_covers_the_whole_range() {
    let id = b();
    let window = window();
    let retrieved = retrieve(&id, &window, Lossless::Normalized)
        .decode(&context(), response(200, &[], RETRIEVED))
        .unwrap();
    assert_eq!(retrieved.body().attribute(&speed()).map(Evolution::len), Some(2));
    assert_eq!(retrieved.coverage(), &Coverage::Whole);
    assert_eq!(retrieved.body().render(Lossless::Normalized).to_string(), RETRIEVED);
}

#[test]
fn a_206_reads_the_range_and_gives_the_query_of_the_rest() {
    let id = b();
    let window = window();
    let range = "DateTime 2018-08-01T12:03:00Z-2018-08-01T12:05:00Z/*";
    let retrieved = retrieve(&id, &window, Lossless::Normalized)
        .decode(&context(), response(206, &[(&header::CONTENT_RANGE, range)], RETRIEVED))
        .unwrap();
    assert_eq!(retrieved.status().as_u16(), 206);
    let content_range = retrieved.coverage().content_range().unwrap();
    assert_eq!((content_range.start(), content_range.end()), (&time(T1), &time(T2)));
    assert_eq!(content_range.size(), ContentRangeSize::Unknown);
    assert_eq!(
        retrieved.coverage().following_query(TemporalSelection::Query(&window)),
        Some(TemporalQuery::between(time(T2), time(T3)))
    );
}

#[test]
fn a_206_reads_an_attribute_that_has_no_instance_in_its_range() {
    let id = b();
    let window = window();
    let body = RETRIEVED.replacen("]}", r#"],"rpm":[]}"#, 1);
    let range = "DateTime 2018-08-01T12:03:00Z-2018-08-01T12:05:00Z/*";
    let retrieved = retrieve(&id, &window, Lossless::Normalized)
        .decode(&context(), response(206, &[(&header::CONTENT_RANGE, range)], &body))
        .unwrap();
    assert_eq!(retrieved.body().attribute(&"rpm".parse().unwrap()).map(Evolution::len), Some(0));
    assert_eq!(retrieved.body().attribute(&speed()).map(Evolution::len), Some(2));
    assert_eq!(retrieved.body().render(Lossless::Normalized).to_string(), body);
}

#[test]
fn a_206_without_a_readable_range_still_carries_the_entity() {
    let id = b();
    let window = window();
    let operation = retrieve(&id, &window, Lossless::Normalized);
    let malformed = operation
        .decode(&context(), response(206, &[(&header::CONTENT_RANGE, "bytes 0-1/2")], RETRIEVED))
        .unwrap();
    assert_eq!(
        malformed.coverage(),
        &Coverage::Partial(OptionalHeader::Malformed(vec![HeaderValue::from_static("bytes 0-1/2")]))
    );
    let absent = operation.decode(&context(), response(206, &[], RETRIEVED)).unwrap();
    assert_eq!(absent.coverage(), &Coverage::Partial(OptionalHeader::Absent));
    assert_eq!(malformed.body(), absent.body());
}

#[test]
fn temporal_values_and_aggregated_answers_read_into_their_own_types() {
    let id = b();
    let window = window();
    let values = retrieve(&id, &window, SimplifiedTemporal)
        .decode(&context(), response(200, &[], VALUES))
        .unwrap();
    assert_eq!(values.body().render().to_string(), VALUES);
    let aggregated_window = window.clone().with_aggregation_methods("max".parse::<AggregationMethods>().unwrap());
    let aggregated = retrieve(&id, &aggregated_window, Aggregated)
        .decode(&context(), response(200, &[], AGGREGATED))
        .unwrap();
    assert_eq!(aggregated.body().render().to_string(), AGGREGATED);
    let amended = retrieve(&id, &window, Lossless::Normalized)
        .decode(&context(), response(203, &[(PreferenceApplied::name(), "ngsi-ld=1.8.1")], RETRIEVED))
        .unwrap();
    assert_eq!(amended.preference_applied().map(ToString::to_string).as_deref(), Some("1.8.1"));
    assert_eq!((values.status().as_u16(), amended.status().as_u16()), (200, 203));
}

#[test]
fn a_query_page_reads_its_entities_links_count_and_coverage() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let window = window();
    let operation = query(&entity_type, &window);
    let sent = operation.to_request(&endpoint()).unwrap();
    let context = ResponseContext::new(&sent);
    let body = format!("[{RETRIEVED},{}]", RETRIEVED.replace("B9211", "A4567"));
    let page = operation
        .decode(
            &context,
            response(
                200,
                &[
                    (&header::LINK, &link_field([link_value("?type=Vehicle&offset=2", RelationType::Next)])),
                    (NgsiLdResultsCount::name(), "2"),
                ],
                &body,
            ),
        )
        .unwrap();
    let ids: Vec<String> = page.items().iter().map(|entity| entity.id.to_string()).collect();
    assert_eq!(ids, ["urn:ngsi-ld:Vehicle:B9211", "urn:ngsi-ld:Vehicle:A4567"]);
    assert_eq!(
        page.next().map(ToString::to_string).as_deref(),
        Some("http://broker.test/ngsi-ld/v1/temporal/entities?type=Vehicle&offset=2")
    );
    assert_eq!(page.total(), Some(2));
    assert_eq!(page.coverage(), &Coverage::Whole);
    let partial = operation
        .decode(
            &context,
            response(206, &[(&header::CONTENT_RANGE, "DateTime 2018-08-01T12:07:00Z-2018-08-01T12:03:00Z/5")], &body),
        )
        .unwrap();
    assert_eq!(
        partial.coverage().content_range().map(ContentRange::size),
        Some(ContentRangeSize::LastN(NonZeroU64::new(5).unwrap()))
    );
}

#[test]
fn a_query_or_retrieve_answer_out_of_shape_is_a_typed_error() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let window = window();
    let operation = query(&entity_type, &window);
    let not_an_array = operation.decode(&context(), response(200, &[], "{}")).unwrap_err();
    assert!(matches!(not_an_array, ResponseError::Body { source, .. } if matches!(*source, BodyError::NotAnArray)));
    let without_id = format!(r#"[{RETRIEVED},{{"type":"Vehicle"}}]"#);
    let ResponseError::Body { source, .. } = operation.decode(&context(), response(200, &[], &without_id)).unwrap_err() else {
        panic!("expected a body error")
    };
    let BodyError::Document(error) = *source else {
        panic!("expected a document error, got {source:?}")
    };
    assert_eq!(error.path.to_string(), "1");
    assert!(matches!(error.kind, DocumentErrorKind::Member(MemberError::MissingMember { member: "id" })));
    assert!(is_unexpected(&operation.decode(&context(), response(204, &[], ""))));
    let id = b();
    let retrieve = retrieve(&id, &window, Lossless::Normalized);
    assert!(is_unexpected(&retrieve.decode(&context(), response(204, &[], ""))));
    let problem = r#"{"type":"https://uri.etsi.org/ngsi-ld/errors/ResourceNotFound","title":"not found"}"#;
    let error = retrieve.decode(&context(), response(404, &[], problem)).unwrap_err();
    assert!(matches!(error, ResponseError::Problem { details, .. } if details.error_type == Some(ErrorType::ResourceNotFound)));
}
