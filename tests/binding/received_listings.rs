//! The listings read back in typed form, as a broker reads the requests of Query Context Source Registrations,
//! Query Subscriptions, List @contexts and the discovery operations (Tables 6.8.3.2-1, 6.10.3.2-1, 6.12.3.2-1,
//! 6.25.3.1-1, 6.27.3.1-1 and 6.29.3.2-1), and the entity map creations whose parameters are those of a query.

use crate::fixtures::endpoint;
use ngsi_ld::{
    http::{
        QueryParameter,
        operation::{
            ContextKindFilter,
            CreateEntityMap,
            CreateTemporalEntityMap,
            Details,
            DiscoveryParameters,
            ListContextDetails,
            ListContexts,
            ListContextsParameters,
            Operation,
            ParameterError,
            QueryEntitiesParameters,
            QueryRegistrations,
            QueryRegistrationsParameters,
            QuerySubscriptions,
            QuerySubscriptionsParameters,
            QueryTemporalEvolutionParameters,
            ResultsCount,
            RetrieveAttributeInfo,
            RetrieveAttributes,
            RetrieveEntityTypes,
            SummaryDetails,
            SystemAttributes,
            TemporalSelectionParameters,
        },
    },
    model::{AttributeName, EntityId},
    query::{
        IdPattern,
        Locality,
        SelectorType,
        language_filter::LanguageFilter,
        q::{self, Query},
        temporal_query::{TemporalProperty, TemporalQuery},
        type_selection::TypeSelection,
    },
    subscription::{EntitySubscription, RegistrationSubscription},
};
use pretty_assertions::assert_eq;

/// The query string `operation` writes.
fn written(operation: &impl Operation) -> String {
    operation.to_request(&endpoint()).unwrap().uri().query().unwrap_or_default().to_owned()
}

/// Why `query` is refused by `parse`.
fn refused<T>(parse: impl FnOnce(&str) -> Result<T, ParameterError>, query: &str) -> ParameterError {
    match parse(query) {
        Ok(_) => panic!("{query} was read"),
        Err(error) => error,
    }
}

#[test]
fn every_query_registrations_parameter_reads_back_as_the_value_sent() {
    let entity_type: TypeSelection = "Vehicle".parse().unwrap();
    let ids: [EntityId; 1] = ["urn:ngsi-ld:Vehicle:A4567".parse().unwrap()];
    let id_pattern: IdPattern = "urn:ngsi-ld:Vehicle:.*".parse().unwrap();
    let attrs: [AttributeName; 2] = ["speed".parse().unwrap(), "isParked".parse().unwrap()];
    let q: Query = q::attr("speed").gt(50).build().unwrap();
    let csf: Query = q::attr("mode").eq("inclusive").build().unwrap();
    let lang: LanguageFilter = "en".parse().unwrap();
    let temporal = TemporalQuery::after("2024-05-01T10:00:00Z".parse().unwrap());
    let query = QueryRegistrations {
        entity_type: Some(&entity_type),
        ids: &ids,
        id_pattern: Some(&id_pattern),
        attrs: &attrs,
        q: Some(&q),
        csf: Some(&csf),
        temporal_q: Some(&temporal),
        lang: Some(&lang),
        system_attributes: SystemAttributes::Include,
        limit: Some(10),
        offset: Some(20),
        count: ResultsCount::Include,
        ..QueryRegistrations::default()
    };
    let received = QueryRegistrationsParameters::parse(&written(&query)).unwrap();
    assert_eq!(received.selection.entity_type.as_ref().map(ToString::to_string).as_deref(), Some("Vehicle"));
    assert_eq!(received.selection.ids, ids);
    assert_eq!(received.selection.id_pattern.as_ref(), Some(&id_pattern));
    assert_eq!(received.selection.attrs, attrs);
    assert_eq!(received.selection.q.as_ref(), Some(&q));
    assert_eq!(received.selection.csf.as_ref(), Some(&csf));
    assert_eq!(received.selection.lang.as_ref(), Some(&lang));
    assert_eq!(received.temporal, TemporalSelectionParameters::Query(temporal));
    assert_eq!(received.system_attributes, SystemAttributes::Include);
    assert_eq!((received.limit, received.offset, received.count), (Some(10), Some(20), ResultsCount::Include));
}

#[test]
fn a_time_property_alone_and_a_geometry_property_read_back() {
    let received = QueryRegistrationsParameters::parse("type=Vehicle&timeproperty=modifiedAt&geometryProperty=site").unwrap();
    let TemporalSelectionParameters::Unbounded { time_property, last_n, .. } = received.temporal else {
        panic!("expected a selection without a time bound");
    };
    assert_eq!((time_property, last_n), (Some(TemporalProperty::ModifiedAt), None));
    assert_eq!(received.geometry_property, Some("site".parse().unwrap()));
}

#[test]
fn a_query_registrations_string_with_what_its_table_does_not_define_is_refused() {
    for query in [
        "type=Vehicle&lastN=3",
        "type=Vehicle&expandValues=speed",
        "type=Vehicle&datasetId=urn:ngsi-ld:d",
        "type=Vehicle&local=true",
        "type=Vehicle&format=simplified",
    ] {
        assert!(
            matches!(refused(QueryRegistrationsParameters::parse, query), ParameterError::Unknown { .. }),
            "{query}"
        );
    }
    assert!(matches!(
        refused(QueryRegistrationsParameters::parse, "type=A&type=B"),
        ParameterError::Repeated {
            parameter: QueryParameter::Type
        }
    ));
    assert!(matches!(
        refused(QueryRegistrationsParameters::parse, "type=A&count=yes"),
        ParameterError::NotABoolean {
            parameter: QueryParameter::Count
        }
    ));
    assert!(matches!(
        refused(QueryRegistrationsParameters::parse, "type=A&limit=-1"),
        ParameterError::Integer {
            parameter: QueryParameter::Limit,
            ..
        }
    ));
    assert!(matches!(
        refused(QueryRegistrationsParameters::parse, "type=A&options=keyValues"),
        ParameterError::UnknownToken {
            parameter: QueryParameter::Options,
            ..
        }
    ));
    assert!(matches!(
        refused(QueryRegistrationsParameters::parse, "type=A&timerel=between&timeAt=2024-05-01T10:00:00Z"),
        ParameterError::TemporalQuery { .. }
    ));
}

#[test]
fn a_registration_query_without_the_bound_clause_5_10_2_4_asks_for_is_still_read() {
    let received = QueryRegistrationsParameters::parse("limit=5").unwrap();
    assert_eq!(received.selection.entity_type, None);
    assert_eq!(received.limit, Some(5));
}

#[test]
fn every_query_subscriptions_parameter_reads_back_for_both_kinds() {
    let entities = QuerySubscriptions::builder()
        .kind(EntitySubscription)
        .system_attributes(SystemAttributes::Include)
        .limit(10)
        .offset(20)
        .count(ResultsCount::Include)
        .build();
    let registrations = QuerySubscriptions::builder()
        .kind(RegistrationSubscription)
        .system_attributes(SystemAttributes::Include)
        .limit(10)
        .offset(20)
        .count(ResultsCount::Include)
        .build();
    for query in [written(&entities), written(&registrations)] {
        let received = QuerySubscriptionsParameters::parse(&query).unwrap();
        assert_eq!(received.system_attributes, SystemAttributes::Include);
        assert_eq!((received.limit, received.offset, received.count), (Some(10), Some(20), ResultsCount::Include));
    }
    let none = QuerySubscriptionsParameters::parse("").unwrap();
    assert_eq!(
        (none.system_attributes, none.limit, none.offset, none.count),
        (SystemAttributes::Omit, None, None, ResultsCount::Omit)
    );
}

#[test]
fn a_query_subscriptions_string_with_what_its_table_does_not_define_is_refused() {
    assert!(matches!(
        refused(QuerySubscriptionsParameters::parse, "type=Vehicle"),
        ParameterError::Unknown { .. }
    ));
    assert!(matches!(
        refused(QuerySubscriptionsParameters::parse, "limit=1&limit=2"),
        ParameterError::Repeated {
            parameter: QueryParameter::Limit
        }
    ));
    assert!(matches!(
        refused(QuerySubscriptionsParameters::parse, "options=sysAttrs,keyValues"),
        ParameterError::UnknownToken {
            parameter: QueryParameter::Options,
            ..
        }
    ));
    assert!(matches!(
        refused(QuerySubscriptionsParameters::parse, "limit=ten"),
        ParameterError::Integer {
            parameter: QueryParameter::Limit,
            ..
        }
    ));
}

#[test]
fn list_contexts_reads_back_its_kind_and_details() {
    let hosted = ListContexts::builder()
        .kind(ContextKindFilter::Hosted)
        .details(SummaryDetails::Explicit)
        .build();
    let received = ListContextsParameters::parse(&written(&hosted)).unwrap();
    assert_eq!(
        (received.details, received.kind),
        (Details::Summary(SummaryDetails::Explicit), Some(ContextKindFilter::Hosted))
    );
    let cached = ListContextDetails::builder().kind(ContextKindFilter::Cached).build();
    let received = ListContextsParameters::parse(&written(&cached)).unwrap();
    assert_eq!((received.details, received.kind), (Details::Detailed, Some(ContextKindFilter::Cached)));
    let bare = ListContextsParameters::parse("").unwrap();
    assert_eq!((bare.details, bare.kind), (Details::Summary(SummaryDetails::Implied), None));
    for kind in ["Cached", "Hosted", "ImplicitlyCreated"] {
        assert_eq!(
            ListContextsParameters::parse(&format!("kind={kind}"))
                .unwrap()
                .kind
                .map(|kind| kind.to_string())
                .as_deref(),
            Some(kind)
        );
    }
}

#[test]
fn list_contexts_refuses_a_kind_the_table_does_not_define_and_other_parameters() {
    assert!(matches!(
        refused(ListContextsParameters::parse, "kind=Remote"),
        ParameterError::UnknownToken {
            parameter: QueryParameter::Kind,
            ..
        }
    ));
    assert!(matches!(
        refused(ListContextsParameters::parse, "kind=hosted"),
        ParameterError::UnknownToken { .. }
    ));
    assert!(matches!(
        refused(ListContextsParameters::parse, "details=1"),
        ParameterError::NotABoolean {
            parameter: QueryParameter::Details
        }
    ));
    assert!(matches!(refused(ListContextsParameters::parse, "local=true"), ParameterError::Unknown { .. }));
    assert!(matches!(
        refused(ListContextsParameters::parse, "kind=Hosted&kind=Cached"),
        ParameterError::Repeated {
            parameter: QueryParameter::Kind
        }
    ));
}

#[test]
fn the_discovery_requests_read_back_their_details_and_locality() {
    let types = RetrieveEntityTypes::builder()
        .details(SummaryDetails::Explicit)
        .locality(Locality::Local)
        .build();
    let received = DiscoveryParameters::parse(&written(&types)).unwrap();
    assert_eq!(
        (received.details, received.locality),
        (Details::Summary(SummaryDetails::Explicit), Locality::Local)
    );
    let attribute: AttributeName = "speed".parse().unwrap();
    let info = RetrieveAttributeInfo::builder().attribute(&attribute).locality(Locality::Distributed).build();
    assert_eq!(DiscoveryParameters::parse(&written(&info)).unwrap().locality, Locality::Distributed);
    let attributes = RetrieveAttributes::default();
    let bare = DiscoveryParameters::parse(&written(&attributes)).unwrap();
    assert_eq!(
        (bare.details, bare.locality),
        (Details::Summary(SummaryDetails::Implied), Locality::Unspecified)
    );
    assert_eq!(DiscoveryParameters::parse("details=true").unwrap().details, Details::Detailed);
}

#[test]
fn the_discovery_requests_refuse_what_their_tables_do_not_define() {
    assert!(matches!(refused(DiscoveryParameters::parse, "limit=1"), ParameterError::Unknown { .. }));
    assert!(matches!(
        refused(DiscoveryParameters::parse, "details=maybe"),
        ParameterError::NotABoolean {
            parameter: QueryParameter::Details
        }
    ));
    assert!(matches!(
        refused(DiscoveryParameters::parse, "local=2"),
        ParameterError::NotABoolean {
            parameter: QueryParameter::Local
        }
    ));
    assert!(matches!(
        refused(DiscoveryParameters::parse, "details=true&details=false"),
        ParameterError::Repeated {
            parameter: QueryParameter::Details
        }
    ));
}

#[test]
fn the_entity_map_creations_read_back_with_the_parameters_of_a_query() {
    let every = SelectorType::All;
    let creation = CreateEntityMap::builder()
        .entity_type(&every)
        .lifetime("PT1H".parse().unwrap())
        .locality(Locality::Local)
        .build();
    let received = QueryEntitiesParameters::parse(&written(&creation)).unwrap();
    assert_eq!(received.selection.entity_type, Some(SelectorType::All));
    assert_eq!(received.assembly.entity_map_lifetime, Some("PT1H".parse().unwrap()));
    assert_eq!(received.locality, Locality::Local);

    let vehicles: SelectorType = "Vehicle".parse().unwrap();
    let since = TemporalQuery::after("2024-05-01T10:00:00Z".parse().unwrap());
    let temporal = CreateTemporalEntityMap::builder().entity_type(&vehicles).temporal_q(&since).build();
    let received = QueryTemporalEvolutionParameters::parse(&written(&temporal)).unwrap();
    assert_eq!(received.temporal_q, since);
    assert_eq!(received.selection.entity_type, Some(vehicles));
}
